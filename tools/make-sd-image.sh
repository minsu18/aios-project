#!/bin/bash
# Create FAT32 SD image for QEMU or real RPi.
# Usage: ./tools/make-sd-image.sh [MODEL.GGUF path] [output image path]
#
# Copies MODEL.GGUF (or MODEL.GGU for 8.3) and SKILL.md into a 64MB FAT32 image.
# Requirements: dosfstools (mkfs.vfat), optionally mtools (mcopy) for file copy.
#
# QEMU with SD: ./tools/simulate-rpi.sh --sd target/aios-sd.img

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [[ $# -eq 1 && "$1" == *.img ]]; then
    OUT="$1"
    MODEL_PATH=""
elif [[ $# -ge 2 ]]; then
    MODEL_PATH="$1"
    OUT="$2"
else
    OUT="${2:-$ROOT/target/aios-sd.img}"
    MODEL_PATH="${1:-}"
fi
SIZE_MB=64
RESERVED=2048  # sectors for MBR + alignment

mkdir -p "$(dirname "$OUT")"

# Create blank image
echo "Creating ${SIZE_MB}MB image: $OUT"
dd if=/dev/zero of="$OUT" bs=1M count="$SIZE_MB" 2>/dev/null

# Write MBR with one FAT32 partition (start sector $RESERVED, type 0x0C)
python3 - "$OUT" "$RESERVED" << 'PY'
import sys
out = sys.argv[1]
reserved = int(sys.argv[2])
total_sectors = 64 * 1024 * 2  # 64MB / 512

with open(out, "r+b") as f:
    # Boot signature at 510-511
    f.seek(510)
    f.write(b'\x55\xaa')
    # Partition 1 at 0x1BE: boot 80, type 0C, start, size
    f.seek(0x1BE)
    start = reserved
    num_sectors = total_sectors - reserved
    f.write(bytes([
        0x80, 1, 1, 0,       # bootable, CHS start
        0x0C, 0xFE, 0xFF, 0xFF,  # type FAT32 LBA, CHS end
        start & 0xFF, (start >> 8) & 0xFF, (start >> 16) & 0xFF, (start >> 24) & 0xFF,
        num_sectors & 0xFF, (num_sectors >> 8) & 0xFF, (num_sectors >> 16) & 0xFF, (num_sectors >> 24) & 0xFF,
    ]))
PY

# Format partition: extract to temp, format, write back
PART_OFFSET=$((RESERVED * 512))
PART_SIZE=$(((SIZE_MB * 1024 * 1024) - PART_OFFSET))
TMP_PART="$(mktemp)"
trap "rm -f $TMP_PART" EXIT

dd if="$OUT" of="$TMP_PART" skip="$RESERVED" bs=512 count=$((PART_SIZE / 512)) 2>/dev/null

if command -v mkfs.vfat &>/dev/null; then
    # dosfstools (macOS: brew install dosfstools, Linux: apt install dosfstools)
    mkfs.vfat -F 32 -n AIOS -s 1 -S 512 "$TMP_PART"
else
    echo "Error: need mkfs.vfat (dosfstools)."
    echo "  macOS: brew install dosfstools"
    echo "  Linux: sudo apt install dosfstools"
    exit 1
fi

dd if="$TMP_PART" of="$OUT" seek="$RESERVED" bs=512 conv=notrunc 2>/dev/null

# Copy SKILL.md first (clusters 3,4...); MODEL uses 5+ to avoid overlap
if [[ -f "$ROOT/SKILL.md" ]]; then
    if command -v mcopy &>/dev/null; then
        export MTOOLS_SKIP_CHECK=1
        mcopy -i "$OUT"@@${PART_OFFSET} "$ROOT/SKILL.md" "::/SKILL.MD"
    else
        # Fallback: inject SKILL.md without mtools (matches mkfs.vfat -s 1 layout)
        python3 - "$OUT" "$PART_OFFSET" "$ROOT/SKILL.md" << 'PY2'
import sys
img_path, part_off, skill_path = sys.argv[1], int(sys.argv[2]), sys.argv[3]
with open(skill_path, "rb") as f:
    data = f.read()
RES, NFAT, SPF, ROOT_CL = 32, 2, 993, 2
fat_start = part_off + RES * 512
data_start = part_off + (RES + NFAT * SPF) * 512
root_sec = data_start + (ROOT_CL - 2) * 512
n_clusters = max(1, (len(data) + 511) // 512)
first_cl = 3
name = b"SKILL   MD "
ent = bytearray(32)
ent[0:11] = name
ent[11] = 0x20
ent[26:28] = first_cl.to_bytes(2, "little")
ent[28:32] = len(data).to_bytes(4, "little")
with open(img_path, "r+b") as f:
    f.seek(root_sec + 32)
    f.write(ent)
    for i in range(n_clusters):
        sec = data_start + (first_cl + i - 2) * 512
        f.seek(sec)
        chunk = data[i*512:(i+1)*512].ljust(512, b"\0")
        f.write(chunk)
    for fa in range(2):
        base = fat_start + fa * SPF * 512
        for i in range(n_clusters):
            cl = first_cl + i
            f.seek(base + cl * 4)
            val = (first_cl + i + 1) if i + 1 < n_clusters else 0x0FFFFFFF
            f.write(val.to_bytes(4, "little"))
print("  SKILL.md injected (Python fallback)")
PY2
    fi
fi

# Copy MODEL.GGUF if provided (clusters 5+ to avoid overlap with SKILL)
if [[ -n "$MODEL_PATH" && -f "$MODEL_PATH" ]]; then
    echo "Adding $(basename "$MODEL_PATH")..."
    if command -v mcopy &>/dev/null; then
        export MTOOLS_SKIP_CHECK=1
        mcopy -i "$OUT"@@${PART_OFFSET} "$MODEL_PATH" "::/MODEL.GGU"
        echo "  Copied as MODEL.GGU (8.3)"
    else
        MODEL_SIZE=$(stat -f%z "$MODEL_PATH" 2>/dev/null || stat -c%s "$MODEL_PATH" 2>/dev/null)
        if [[ -n "$MODEL_SIZE" && "$MODEL_SIZE" -le 262144 ]]; then
            python3 - "$OUT" "$PART_OFFSET" "$MODEL_PATH" << 'PYMODEL'
import sys
img_path, part_off, model_path = sys.argv[1], int(sys.argv[2]), sys.argv[3]
with open(model_path, "rb") as f:
    data = f.read()
RES, NFAT, SPF = 32, 2, 993
fat_start = part_off + RES * 512
data_start = part_off + (RES + NFAT * SPF) * 512
root_sec = data_start
n_clusters = max(1, (len(data) + 511) // 512)
first_cl = 5  # SKILL uses 3,4
name = b"MODEL   GGU"
ent = bytearray(32)
ent[0:11] = name
ent[11] = 0x20
ent[26:28] = first_cl.to_bytes(2, "little")
ent[28:32] = len(data).to_bytes(4, "little")
with open(img_path, "r+b") as f:
    f.seek(root_sec + 64)
    f.write(ent)
    for i in range(n_clusters):
        sec = data_start + (first_cl + i - 2) * 512
        f.seek(sec)
        chunk = data[i*512:(i+1)*512].ljust(512, b"\0")
        f.write(chunk)
    for fa in range(2):
        base = fat_start + fa * SPF * 512
        for i in range(n_clusters):
            cl = first_cl + i
            f.seek(base + cl * 4)
            val = (first_cl + i + 1) if i + 1 < n_clusters else 0x0FFFFFFF
            f.write(val.to_bytes(4, "little"))
print("  MODEL.GGU injected (Python fallback, max 256KB)")
PYMODEL
        else
            echo "  Install mtools (brew install mtools) for models >256KB, or use a smaller model."
        fi
    fi
fi

echo "Done: $OUT"
echo "  QEMU: ./tools/simulate-rpi.sh --sd $OUT"
echo "  Real RPi: dd if=$OUT of=/dev/sdX bs=4M"
