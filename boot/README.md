# AIOS Boot

Creates a bootable BIOS disk image from the kernel and launches QEMU.

## Usage

```bash
cargo run -p aios-boot
```

## Requirements

- Rust nightly (via `rust-toolchain.toml`)
- `llvm-tools-preview`: `rustup component add llvm-tools-preview`
- `x86_64-unknown-none` target: `rustup target add x86_64-unknown-none`
- QEMU: `qemu-system-x86_64`

## What it does

1. Builds `aios-kernel` for `x86_64-unknown-none`
2. Creates `bios.img` using the bootloader crate
3. Launches QEMU with serial output to the terminal
