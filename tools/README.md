# AIOS Tools

## Raspberry Pi Build

Build `kernel8.img` for Raspberry Pi 3/4:

```bash
./tools/build-rpi.sh
```

Requires: `aarch64-none-elf` toolchain, `rustup target add aarch64-unknown-none`. See [kernel-rpi/README.md](../kernel-rpi/README.md).

## Raspberry Pi Simulation (QEMU)

Run the RPi kernel in QEMU without physical hardware:

```bash
./tools/simulate-rpi.sh
```

Uses `qemu-system-aarch64` with raspi4b machine. Serial output in terminal. Exit: **Ctrl+A** then **X**.

**Requirements:** QEMU 9.0+ with `qemu-system-aarch64` (macOS: `brew install qemu`)

**Note:** The `ask` command requires the Serial Bridge (no host LLM in standalone mode). Use `./tools/simulate-rpi-bridge.sh` for `ask` with Ollama.

### Serial Bridge (host inference)

Run with host Ollama for real LLM responses on `ask`:

```bash
./tools/simulate-rpi-bridge.sh
```

Requires: Node 18+, `ollama serve`, `ollama pull llama3.2`. Env: `AIOS_OLLAMA_HOST`, `AIOS_OLLAMA_MODEL`

**SD card (QEMU or real RPi):**

- **QEMU:** Create an SD image: `./tools/make-sd-image.sh [path/to/MODEL.GGUF]`. Run `./tools/simulate-rpi.sh --sd target/aios-sd.img`. If SD init times out on raspi4b, try `--raspi3b` (e.g. `./tools/simulate-rpi.sh --sd target/aios-sd.img --raspi3b`). The kernel tries EMMC2, EMMC1 (raspi4b/raspi3b), then sdhost.
- **Real RPi:** Put `SKILL.md` and optionally `MODEL.GGUF` in the SD root (first FAT32 partition). The `sd` command reports capacity and file presence.

## Driver Bridge (Linux)

Build the driver bridge for hardware capture (camera, mic) from the prototype:

```bash
cargo build -p aios-driver-bridge --release
```

**Requires Linux** (V4L2, ALSA). Then `voice capture` and `image capture` work in the prototype. See [driver-bridge/README.md](../driver-bridge/README.md).

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
