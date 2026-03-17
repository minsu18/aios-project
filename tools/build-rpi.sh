#!/usr/bin/env bash
# Build AIOS kernel for Raspberry Pi 3/4 (kernel8.img)
#
# Requirements: rustup target add aarch64-unknown-none
#               aarch64-none-elf toolchain (gcc or binutils)
#
# Output: target/aarch64-unknown-none/release/kernel8.img

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "Building AIOS kernel for Raspberry Pi..."

cargo build -p aios-kernel-rpi --target aarch64-unknown-none --release

ELF="$ROOT/target/aarch64-unknown-none/release/kernel"
IMG="${ELF}8.img"

if command -v aarch64-none-elf-objcopy &>/dev/null; then
    aarch64-none-elf-objcopy -O binary "$ELF" "$IMG"
elif command -v llvm-objcopy &>/dev/null; then
    llvm-objcopy -O binary "$ELF" "$IMG"
else
    echo "Error: aarch64-none-elf-objcopy or llvm-objcopy required"
    exit 1
fi

echo "Built: $IMG"
echo ""
echo "To boot on Raspberry Pi:"
echo "  1. Use an SD card with Raspberry Pi OS boot files (start*.elf, fixup*.dat, config.txt)"
echo "  2. Copy kernel8.img to the SD card root (replacing existing)"
echo "  3. Connect UART: GPIO14 (TX) pin 8, GPIO15 (RX) pin 10, GND pin 6"
echo "  4. Use 115200 baud serial terminal to see output"
