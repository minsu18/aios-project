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
OUT="${2:-$ROOT/target/aios-sd.img}"
MODEL_PATH="$1"
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

# Copy MODEL.GGUF if provided
if [[ -n "$MODEL_PATH" && -f "$MODEL_PATH" ]]; then
    echo "Adding $(basename "$MODEL_PATH")..."
    if command -v mcopy &>/dev/null; then
        export MTOOLS_SKIP_CHECK=1
        mcopy -i "$OUT"@@${PART_OFFSET} "$MODEL_PATH" "::/MODEL.GGU"
        echo "  Copied as MODEL.GGU (8.3)"
    else
        echo "  Install mtools (brew install mtools) to auto-copy, or copy manually to SD."
    fi
fi

# Copy SKILL.md if present
if [[ -f "$ROOT/SKILL.md" ]]; then
    if command -v mcopy &>/dev/null; then
        export MTOOLS_SKIP_CHECK=1
        mcopy -i "$OUT"@@${PART_OFFSET} "$ROOT/SKILL.md" "::/SKILL.MD"
    fi
fi

echo "Done: $OUT"
echo "  QEMU: ./tools/simulate-rpi.sh --sd $OUT"
echo "  Real RPi: dd if=$OUT of=/dev/sdX bs=4M"
