# AIOS Kernel — Raspberry Pi 3/4

Minimal aarch64 kernel for Raspberry Pi. PL011 UART I/O with rule-based conversation loop.

**Commands:** `help`, `time`, `clear`, `version`, `weather [loc]`, `calc <expr>`, `ask <q>`, or type to echo. `ask` calls HAL inference (stub until llama.cpp linked).

## Requirements

1. **Rust** (nightly): `rustup target add aarch64-unknown-none`
2. **ARM toolchain** (one of):
   - `aarch64-none-elf-gcc` (ARM GNU Toolchain)
   - `clang` with aarch64 target
   - `llvm-objcopy` or `aarch64-none-elf-objcopy` for kernel8.img

## Build

```bash
# From project root
./tools/build-rpi.sh
```

Output: `target/aarch64-unknown-none/release/kernel8.img`

## Simulate with QEMU (no hardware needed)

Run the kernel in QEMU using the `raspi4b` machine (QEMU 9.0+):

```bash
./tools/simulate-rpi.sh
```

Serial output appears in the terminal. Exit with **Ctrl+A** then **X**.

**Requirements:** `qemu-system-aarch64` (macOS: `brew install qemu`, Ubuntu: `apt install qemu-system-aarch64`)

## Boot on Raspberry Pi

1. **SD card**: Use a Raspberry Pi OS SD card (or any with boot partition containing `start*.elf`, `fixup*.dat`, `config.txt`).
2. **Replace kernel**: Copy `kernel8.img` to the SD card boot partition, replacing the existing one.
3. **UART**: Connect a USB–TTL adapter:
   - GPIO14 (UART TX) → RX on adapter
   - GPIO15 (UART RX) → TX on adapter
   - GND → GND
4. **Serial**: Open a terminal at 115200 baud (e.g. `screen /dev/ttyUSB0 115200`).
5. **Power on**: You should see the AIOS boot banner.

## Supported boards

- Raspberry Pi 3 (64-bit)
- Raspberry Pi 4
- Raspberry Pi 5 (same UART base for 4/5)
