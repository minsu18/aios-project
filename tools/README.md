# AIOS Tools

## QEMU Boot

```bash
# From project root
cargo run -p aios-boot
```

Builds the x86-64 kernel, creates a BIOS bootable disk image, and launches QEMU. Serial output appears in the terminal ("AIOS kernel booted.").

**Requirements:**

- Rust nightly (`rust-toolchain.toml` sets this)
- `qemu-system-x86_64`
- `llvm-tools`: `rustup component add llvm-tools-preview`

## Other

- Skill install utilities — planned
