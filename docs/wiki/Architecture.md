# Architecture

## Design Principles

1. **AI-Only OS** — AI is the only interface
2. **App Store** — Installable skills extend capabilities
3. **Offline-First** — Works without network; built-in on-device AI
4. **Hybrid Inference** — On-device for basics; cloud optional
5. **Minimal Kernel** — Scheduling, memory, I/O only

## Layer Structure

| Layer | Role |
|-------|------|
| 4. Multimodal I/O | text, voice, image, video |
| 3. AI Core | intent, task decomposition, orchestration |
| 2. Skill Runtime | load, install, isolate skills |
| 1. HAL | memory, CPU, GPU, network, sensors, audio |
| 0. Microkernel | tasks, memory, interrupts |

## HAL Resources

| Resource | Abstraction |
|----------|-------------|
| Memory | alloc, free, mmap |
| CPU | task_create, yield |
| GPU | inference, render |
| Network | net_send, net_recv |
| Camera | capture_image, capture_video |
| Speaker / Mic | audio_play, audio_capture |
