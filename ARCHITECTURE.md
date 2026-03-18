# AIOS Architecture

## 1. Design Principles

1. **AI-Only OS**: The system operates solely through AI. No traditional apps — AI is the only interface.
2. **App Store**: A marketplace for diverse AI add-ons. Users browse, install, and manage skills that extend what the AI can do.
3. **Offline-First, Device-Native**: AIOS runs entirely on the terminal/device. A built-in on-device AI must always be available so the system works even when network is unavailable.
4. **Hybrid Inference**: Basic tasks run on-device; when online, heavier workloads may be routed to the cloud. When offline, all inference stays on-device.
5. **Minimal Kernel**: The kernel handles only scheduling, memory, and I/O.

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
- **Built-in On-Device AI**: A small model bundled with the OS (or installed) runs locally for intent classification and basic generation. No network required for core functionality.
- **Routing**:
  - Offline: all inference on-device (rule-based or local LLM fallback)
  - Online, simple intents (time, weather, calculator) → on-device
  - Online, complex reasoning or external data → optionally cloud

## 4. App Store & Skill/MCP Modules

- **App Store**: Browse, install, and manage AI add-ons (skills). Provides diverse capabilities for varied AI usage.
- **Skill format**: SKILL.md + tool definitions (MCP-compatible)
- **Install path**: `~/.aios/skills/` or per-project
- **Isolation**: sandbox or permission-based access control

## 5. HAL (Hardware Abstraction Layer)

**hal-bare** (`aios-hal-bare`): no_std subset for bare-metal kernel (kernel-rpi). Timer, init. Full `aios-hal` links when it supports no_std.

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
