#!/usr/bin/env bash
# AIOS RPi Simulation — Run Raspberry Pi kernel in QEMU (raspi4b)
#
# Usage: ./tools/simulate-rpi.sh
#
# Builds kernel8.img, then runs QEMU with raspi4b machine.
# Serial output appears in the terminal.
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

echo "=== AIOS RPi Simulation (QEMU raspi4b) ==="
echo ""

# Build kernel
./tools/build-rpi.sh

ELF="$ROOT/target/aarch64-unknown-none/release/kernel"
KERNEL="${ELF}8.img"

# QEMU accepts both ELF and raw binary; prefer kernel8.img for real SD card parity
if [[ -f "$KERNEL" ]]; then
  :
elif [[ -f "$ELF" ]]; then
  KERNEL="$ELF"
else
  echo "Error: kernel build failed (no $KERNEL or $ELF)"
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

echo "Booting in QEMU (Ctrl+A then X to exit)..."
echo ""

exec qemu-system-aarch64 \
  -M raspi4b \
  -m 2G \
  -cpu cortex-a72 \
  -smp 4 \
  -kernel "$KERNEL" \
  -serial mon:stdio \
  -display none
