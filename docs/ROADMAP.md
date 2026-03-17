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
- [ ] Bare-metal boot
- [ ] Drivers: camera, audio, comms (real hardware bindings)
- [x] Multimodal I/O pipeline (STT Whisper, Vision API)

## Phase 4
- [x] App Store — install, remove skills
- [x] App Store — browse registry, install-from-registry
- [ ] App Store — remote registry URL, manage (update)
