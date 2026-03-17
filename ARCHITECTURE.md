# AIOS Architecture

## 1. Design Principles

1. **AI-Only OS**: The system operates solely through AI. No traditional apps — AI is the only interface.
2. **App Store**: A marketplace for diverse AI add-ons. Users browse, install, and manage skills that extend what the AI can do.
3. **Hybrid Inference**: Basic tasks run on-device; heavy workloads are routed to the cloud.
4. **Minimal Kernel**: The kernel handles only scheduling, memory, and I/O.

## 2. Layer Structure

```
┌────────────────────────────────────────────────────────────┐
│ Layer 4: Multimodal I/O (text, voice, image, video)         │
├────────────────────────────────────────────────────────────┤
│ Layer 3: AI Core — intent, task decomposition, orchestration│
├────────────────────────────────────────────────────────────┤
│ Layer 2: Skill Runtime + App Store — load, install, isolate │
├────────────────────────────────────────────────────────────┤
│ Layer 1: HAL — memory, CPU, GPU, network, sensors, audio    │
├────────────────────────────────────────────────────────────┤
│ Layer 0: Microkernel — tasks, memory, interrupts           │
└────────────────────────────────────────────────────────────┘
```

## 3. AI Core

- **Input**: text, voice (WAV), image, video
- **Output**: HAL calls, skill calls, multimodal responses
- **Routing**:
  - Simple intents (time, weather, calculator) → on-device
  - Complex reasoning, external data → cloud

## 4. App Store & Skill/MCP Modules

- **App Store**: Browse, install, and manage AI add-ons (skills). Provides diverse capabilities for varied AI usage.
- **Skill format**: SKILL.md + tool definitions (MCP-compatible)
- **Install path**: `~/.aios/skills/` or per-project
- **Isolation**: sandbox or permission-based access control

## 5. HAL (Hardware Abstraction Layer)

| Resource | Abstraction |
|----------|-------------|
| Memory | alloc, free, mmap |
| CPU | task_create, yield |
| GPU | inference, render |
| Network | net_send, net_recv |
| Camera | capture_image, capture_video |
| Speaker | audio_play |
| Microphone | audio_capture |

## 6. Microkernel

- Process/thread scheduling
- Virtual memory
- Interrupt handlers
- IPC (inter-process communication)
