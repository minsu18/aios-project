# Roadmap

## Phase 1 ✅

- Project structure, AI core prototype, skill runtime
- MCP-compatible spec, skill tool invocation
- Pluggable inference (placeholder, OpenAI, Anthropic)
- App Store CLI (install, remove)

## Phase 2 ✅

- x86-64 kernel, HAL, QEMU boot
- Driver structure (camera, audio, comms)

## Phase 3

- VM simulation, Raspberry Pi kernel
- Multimodal I/O (STT, Vision)
- [ ] Drivers: real hardware bindings

## Phase 4 ✅

- App Store: browse, install-from-registry, update
- Remote registry URL

## Phase 5

- [x] Offline-first (AIOS_OFFLINE=1)
- [x] On-device LLM: Ollama, Transformers.js
- [x] HAL gpu.inference interface
- [ ] HAL llama.cpp FFI (bare-metal)
