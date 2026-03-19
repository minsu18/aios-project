#!/usr/bin/env bash
# AIOS RPi Simulation — Run Raspberry Pi kernel in QEMU (raspi4b)
#
# Usage: ./tools/simulate-rpi.sh [--sd IMAGE.img] [--raspi3b] [--llama]
#
# Builds kernel8.img, then runs QEMU with raspi4b machine.
# Serial output appears in the terminal.
#
# Options:
#   --sd IMAGE.img  Use SD card image (from tools/make-sd-image.sh).
#   --raspi3b       Use raspi3b machine (QEMU SD may work; raspi4b often fails).
#   --llama         Build with llama feature (load_model from SD; run build-llama-baremetal.sh first).
#
# Exit: Ctrl+A then X
#
# Requirements:
#   - Rust: rustup target add aarch64-unknown-none
#   - ARM toolchain: aarch64-none-elf (or clang + llvm-objcopy)
#   - QEMU 9.0+: qemu-system-aarch64 (with raspi4b support)

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SD_IMAGE=""
MACHINE="raspi4b"
LLAMA=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --sd)
      SD_IMAGE="$2"
      shift 2
      ;;
    --raspi3b)
      MACHINE="raspi3b"
      shift
      ;;
    --llama)
      LLAMA="1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

echo "=== AIOS RPi Simulation (QEMU $MACHINE) ==="
echo ""

# Build kernel (raspi3b needs --raspi3; --llama enables load_model; --sd uses sdhost_first then sdhci_first)
FEATURES=""
[[ "$MACHINE" == raspi3b ]] && FEATURES="raspi3"
[[ -n "$LLAMA" ]] && FEATURES="${FEATURES:+$FEATURES }llama"
[[ -n "$SD_IMAGE" ]] && FEATURES="${FEATURES:+$FEATURES }sdhost_first sdhci_first"
[[ -n "$SD_IMAGE" ]] && [[ -n "$LLAMA" ]] && FEATURES="${FEATURES:+$FEATURES }sd_debug"
BUILD_ARGS=()
[[ -n "$FEATURES" ]] && BUILD_ARGS=(--features "$FEATURES")
./tools/build-rpi.sh "${BUILD_ARGS[@]}"

ELF="$ROOT/target/aarch64-unknown-none/release/kernel-rpi"
IMG="$ROOT/target/aarch64-unknown-none/release/kernel8.img"

# QEMU accepts both ELF and raw binary; prefer kernel8.img for real SD card parity
if [[ -f "$IMG" ]]; then
  KERNEL="$IMG"
elif [[ -f "$ELF" ]]; then
  KERNEL="$ELF"
else
  echo "Error: kernel build failed (no $IMG or $ELF)"
  exit 1
fi

# Check for qemu-system-aarch64
if ! command -v qemu-system-aarch64 &>/dev/null; then
  echo "Error: qemu-system-aarch64 not found."
  echo "Install:"
  echo "  macOS:   brew install qemu"
  echo "  Ubuntu:  sudo apt install qemu-system-aarch64"
  echo "  Fedora:  sudo dnf install qemu-system-aarch64"
  exit 1
fi

# raspi3b: 1 GiB RAM, cortex-a53; raspi4b: 2G RAM, cortex-a72
if [[ "$MACHINE" == raspi3b ]]; then
  RAM="1G"
  CPU="cortex-a53"
else
  RAM="2G"
  CPU="cortex-a72"
fi
QEMU_ARGS=(
  -M "$MACHINE"
  -m "$RAM"
  -cpu "$CPU"
  -smp 4
  -kernel "$KERNEL"
  -serial mon:stdio
  -display none
)

if [[ -n "$SD_IMAGE" ]]; then
  if [[ ! -f "$SD_IMAGE" ]]; then
    echo "SD image not found: $SD_IMAGE"
    echo "Create one: ./tools/make-sd-image.sh [MODEL.GGUF] target/aios-sd.img"
    exit 1
  fi
  QEMU_ARGS+=(-drive "if=sd,file=$SD_IMAGE,format=raw")
  echo "SD: $SD_IMAGE"
fi

echo "Booting in QEMU (Ctrl+A then X to exit)..."
echo ""

exec qemu-system-aarch64 "${QEMU_ARGS[@]}"
