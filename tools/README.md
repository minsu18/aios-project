# AIOS Tools

## VM Simulation

Boot AIOS kernel in QEMU with configurable specs, then run the AI prototype:

```bash
# From project root
./tools/simulate.sh --cpus 2 --memory 512

# Or via prototype
cd prototype && npm run simulate
```

**Options:**
- `--cpus N` — VM CPU count (default: 2)
- `--memory M` — VM RAM in MB (default: 512)
- `--vm-only` — Run QEMU only; Ctrl+A then X to exit
- `--no-vm` — Skip QEMU, simulated boot + prototype only

**Without QEMU/Rust:** `npm run simulate:no-vm` — ASCII boot simulation then interactive prototype.

## QEMU Boot (standalone)

```bash
# From project root
cargo run -p aios-boot

# With custom specs
AIOS_VM_CPUS=4 AIOS_VM_MEM=1024 cargo run -p aios-boot
```

Builds the x86-64 kernel, creates a BIOS bootable disk image, and launches QEMU. Serial output shows boot sequence.

**Requirements:**

- Rust nightly (`rust-toolchain.toml` sets this)
- `qemu-system-x86_64`
- `llvm-tools`: `rustup component add llvm-tools-preview`
