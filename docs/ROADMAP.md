# AIOS Roadmap

## Phase 1 ✅
- [x] Project structure
- [x] AI core prototype (TypeScript)
- [x] Skill runtime prototype
- [x] MCP-compatible spec
- [x] Skill tool invocation (invokeTool, example.get_time, example.echo)
- [x] Pluggable inference backend (placeholder; openai/anthropic hooks ready)
- [x] Multimodal input API (text; voice/image/video placeholders)
- [x] App Store CLI (install, remove skills)

## Phase 2 ✅
- [x] x86-64 minimal kernel
- [x] HAL implementation (interface)
- [x] QEMU boot
- [x] Driver structure (camera, audio, comms traits)

## Phase 3
- [x] VM simulation (configurable specs, boot + prototype)
- [x] Raspberry Pi 3/4 kernel (kernel8.img, UART serial)
- [x] Drivers: camera, audio, comms (real hardware bindings — `--features host` on Linux)
- [x] Multimodal I/O pipeline (STT Whisper, Vision API)

## Phase 4
- [x] App Store — install, remove skills
- [x] App Store — browse registry, install-from-registry
- [x] App Store — remote registry URL (default GitHub raw), update command

## Phase 5
- [x] Offline-first: AIOS_OFFLINE=1 forces all inference on-device
- [x] Architecture: built-in on-device AI, works without network
- [x] On-device LLM (prototype): Ollama, Transformers.js backends
- [x] HAL gpu.inference: interface + llama.cpp design (docs/HAL_LLAMA_CPP.md)
- [x] HAL llama.cpp: llama-cpp-2 crate integration

## Phase 6 (RPi bare-metal)
- [x] RPi QEMU simulation (raspi4b)
- [x] HAL-kernel integration (hal-bare: timer, inference stub)
- [x] Skill runtime on RPi (help, time, weather, calc, ask, mem, sd, uptime, cpuinfo, reboot)
- [x] Bump allocator (128KB default; 64MB with `--features llama`)
- [x] Driver bridge (camera, audio) for prototype on Linux
- [x] Serial bridge (host Ollama for `ask` via UART protocol)
- [x] Block device (SD/EMMC2 init, read, CSD capacity)
- [x] FAT32 parser (MBR, root dir, read SKILL.md)
- [x] SKILL.md frontmatter parsing and tool registration (`load`, `skills`, `skill.tool`)
- [x] Prototype ↔ kernel bridge (`npm run simulate:rpi`)
