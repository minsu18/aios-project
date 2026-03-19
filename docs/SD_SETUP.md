# SD Card Setup for AIOS RPi

Setup and troubleshooting for SD/block device on Raspberry Pi (QEMU and real hardware).

## Quick Start

### QEMU Simulation

```bash
# Create 64MB SD image with SKILL.md (and optionally MODEL.GGU)
./tools/make-sd-image.sh target/aios-sd.img
# Or with a model: ./tools/make-sd-image.sh /path/to/model.gguf target/aios-sd.img

# Run QEMU with SD
./tools/simulate-rpi.sh --sd target/aios-sd.img
```

In the kernel prompt:

```
>> sd       # Probe SD, show block count and SKILL.md status
>> load     # Load SKILL.md from SD
>> skills   # List loaded tools
>> load_model   # (with --features llama) Load MODEL.GGU from SD
```

### Real RPi 4

1. Prepare SD card with Raspberry Pi OS boot files (start*.elf, fixup.dat, config.txt).
2. Copy `kernel8.img` to the SD root.
3. Optionally add a FAT32 partition with SKILL.md and MODEL.GGU.

---

## make-sd-image.sh

Creates a 64MB FAT32 image.

| Usage | Description |
|-------|-------------|
| `./tools/make-sd-image.sh [output.img]` | Output path only; copies SKILL.md from project root. |
| `./tools/make-sd-image.sh [model.gguf] [output.img]` | Also copies model as MODEL.GGU (8.3). |

**Requirements:**

- `mkfs.vfat` (dosfstools) — required.
- `mcopy` (mtools) — optional. Without it:
  - SKILL.md is injected via Python (always works).
  - MODEL.GGU is injected only if model ≤256KB (Python fallback).

```bash
# macOS
brew install dosfstools mtools

# Ubuntu
sudo apt install dosfstools mtools
```

---

## Troubleshooting

### "0 blocks" or block_count is wrong

**Cause:** QEMU SDHCI returns a minimal CSD that omits capacity.

**Fix:** The driver falls back to MBR partition size when CSD reports 0. Ensure the image has a valid MBR with a FAT32 (0x0B/0x0C) partition. `make-sd-image.sh` creates this automatically.

### "SD image not found: target/aios-sd.img"

- `cargo clean` removes the entire `target/` directory, including the SD image.
- **Fix:** Recreate with `./tools/make-sd-image.sh target/aios-sd.img`.

### "No SKILL.md in root"

- Image was created before SKILL.md existed, or mcopy was not available.
- **Fix:** Re-run `./tools/make-sd-image.sh target/aios-sd.img`. The script injects SKILL.md via Python when mtools is absent.

### SD init timeout in QEMU

- Ensure you pass `--sd target/aios-sd.img` so QEMU attaches the drive.
- raspi4b uses generic SDHCI (EMMC1); raspi3b can use sdhost.

### QEMU SDHCI: load / load_model use fallback

**Cause:** QEMU raspi3b/raspi4b SDHCI does not correctly transfer block data to our buffer (SDMA path returns invalid MBR). Block reads fail with "FAT32 error (MBR)".

**Workaround:** The kernel detects QEMU SDHCI (`is_sdhci()`) and:

- **sd** — skips block read, prints "SD: init OK (QEMU SDHCI - block read hangs; use real RPi 4 for full SD)".
- **load** — uses built-in SKILL.md instead of SD. Prints "load: QEMU SDHCI (block read invalid), using built-in SKILL.md".
- **load_model** — exits early with "load_model: QEMU SDHCI - block read invalid. Use real RPi 4 for model load".

On **real Raspberry Pi 4**, the kernel uses EMMC2 (not SdSdhci), so `load`, `load_model`, and full SD access work correctly.

### "MBR signature invalid" (got 00 00 or wrong bytes at 510,511)

- The SD controller may not have the card; QEMU raspi4b SD wiring can vary.
- **Try raspi3b:** `./tools/simulate-rpi.sh --sd target/aios-sd.img --llama --raspi3b`
- Recreate the image: `./tools/make-sd-image.sh target/aios-sd.img`

### Debug output

To enable SD init debug logs (CMD0, CMD8, ACMD41, controller selection):

```bash
./tools/build-rpi.sh --features sd_debug
./tools/simulate-rpi.sh --sd target/aios-sd.img
```

---

## load_model

Requires `--features llama` and a GGUF model on the SD image.

```bash
# With mtools (recommended; any model size)
./tools/make-sd-image.sh /path/to/llama.gguf target/aios-sd.img

# Without mtools (models ≤256KB only)
./tools/make-sd-image.sh /path/to/tiny.gguf target/aios-sd.img
```

Build and run with llama:

```bash
./tools/build-llama-baremetal.sh   # once (builds libllama.a)
./tools/simulate-rpi.sh --sd target/aios-sd.img --llama
>> load_model
>> ask hello
```

Use `--llama` so simulate builds with the llama feature; otherwise it builds without it.

See [docs/HAL_LLAMA_CPP_BAREMETAL.md](HAL_LLAMA_CPP_BAREMETAL.md) for full on-device LLM setup.
