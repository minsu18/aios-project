# AIOS — Next Steps (Without Real Device Testing)

Overview of remaining work, ordered by priority. Real RPi 4 hardware validation is **deferred** (requires USB-TTL and physical setup).

---

## 1. Completed (Current State)

- QEMU raspi3b/raspi4b simulation with SD
- `sd`, `load`, `load_model` commands — QEMU SDHCI fallback (built-in skill, skip model load)
- Skill runtime, SKILL.md parsing, tool registration
- FAT32 parser, MBR/BPB, read from SD image
- Serial bridge (host Ollama for `ask`)
- llama.cpp bare-metal build scaffold, `--features llama`, heap 64MB
- `load_model` flow: SD read → `init_from_memory` → inference (blocked by QEMU SD read)

---

## 2. Remaining Work (Excluding Real Device)

### 2.1 High Priority — On-Device `load_model` (QEMU Path)

**Goal:** Run full `load_model` + `ask` in QEMU.

**Issue:** QEMU SDHCI does not return valid block data. `load_model` currently exits early with "Use real RPi 4".

**Options (QEMU only, no real HW):**

1. **Embed small GGUF in kernel** — Link a tiny model (e.g. ~2MB) into the binary so `load_model` can init from memory without SD. Validates the inference path.
2. **RAM disk / initrd** — QEMU `-initrd` with a FAT image; kernel reads "SD" from RAM. Bypasses SDHCI.
3. **Fix QEMU SDHCI** — Debug why SDMA buffer stays empty; may need QEMU source changes or different register usage. High effort.

**Recommended:** Option 1 (embedded model) for quick validation; Option 2 for realistic SD-like flow.

---

### 2.2 Medium Priority — Documentation & Scripts

| Task | Description |
|------|-------------|
| ROADMAP update | Mark "Real RPi 4 boot" as deferred; add "QEMU-only load_model" path |
| SD_SETUP.md | Note that `cargo clean` removes `target/aios-sd.img`; suggest `make-sd-image.sh` after clean |
| simulate-rpi.sh | Optional: auto-create SD image if missing (e.g. `target/aios-sd.img` not found → run `make-sd-image.sh`) |

---

### 2.3 Lower Priority — HAL / Kernel Polish

| Area | Notes |
|------|-------|
| hal/src | TODOs: audio, camera, memory, cpu, gpu stubs; driver registration |
| kernel/src | Scheduler, memory, interrupts (x86 kernel) |
| HDMI/framebuffer | Optional display for RPi; useful when real device is available |

---

### 2.4 Deferred (Needs Real Hardware)

| Task | Dependency |
|------|-------------|
| Real RPi 4 boot | USB-TTL, SD card, physical RPi 4 |
| EMMC2 `sd` / `load` on real device | Above |
| On-device `load_model` on real device | Above + GGUF on SD |

---

## 3. Suggested Order

1. **Document** — Update ROADMAP, SD_SETUP, NEXT_STEPS (this file).
2. **QEMU load_model** — Implement embedded-GGUF or initrd path so `load_model` + `ask` works in QEMU.
3. **Script polish** — Optional auto-create SD image in `simulate-rpi.sh`.
4. **HAL stubs** — When touching prototype or host inference.

---

## 4. Quick Reference

```bash
# Build (no llama)
./tools/build-rpi.sh

# Build with llama (load_model)
./tools/build-llama-baremetal.sh   # once
./tools/build-rpi.sh --features llama

# Simulate
./tools/simulate-rpi.sh --sd target/aios-sd.img --llama --raspi3b

# Create SD image (after cargo clean)
./tools/make-sd-image.sh target/aios-sd.img
```
