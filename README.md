# AIOS — AI-Native Operating System

> An operating system that runs **only with AI**. No traditional apps — AI directly controls all device functions.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Status**: Not yet validated on physical hardware. This project aims to align with the all-AI device development direction.

## Vision

AIOS is an **AI-only OS**: the system operates solely through AI, with no conventional app launcher or app-based workflows.

- **AI-only interface**: All interactions go through AI (text, voice, photo, video)
- **App Store**: A marketplace for adding diverse AI capabilities — browse, install, and manage skills that extend what the AI can do
- **Hybrid compute**: Built-in on-device AI for basics; cloud for heavy tasks or external data

```
┌─────────────────────────────────────────────────────────┐
│  User: Text · Voice · Image · Video                      │
└─────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────┐
│  AI Core — Intent · Task Decomposition · Orchestration   │
└─────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ On-Device    │    │  App Store   │    │  Cloud       │
│ AI (basic)   │    │  (skills/    │    │  AI (heavy)  │
│              │    │  AI add-ons) │    │              │
└──────────────┘    └──────────────┘    └──────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              ▼
┌─────────────────────────────────────────────────────────┐
│  HAL (Hardware Abstraction Layer)                        │
│  Memory · CPU · GPU · Comms · Camera · Speaker · Mic     │
└─────────────────────────────────────────────────────────┘
```

## Project Structure

```
aios-project/
├── kernel/          # Minimal microkernel
├── hal/             # Hardware Abstraction Layer
├── ai-core/         # AI interface core
├── skills/          # Skill runtime (App Store items = installable skills)
├── drivers/         # Hardware drivers
├── tools/           # Build · emulation tools
└── docs/            # Design docs
```

## Roadmap

| Phase | Goal |
|-------|------|
| **Phase 1** | AI core + skill runtime (host validation) |
| **Phase 2** | HAL + minimal kernel (x86-64 / ARM) |
| **Phase 3** | Bare-metal boot · drivers · multimodal I/O |
| **Phase 4** | App Store — browse, install, manage AI add-ons |

## Development

- **Rust** (nightly): kernel, HAL, drivers
- **Python/TypeScript**: AI core prototype (Phase 1)

## Getting Started

```bash
git clone https://github.com/minsu18/aios-project.git
cd aios-project

# Phase 1: AI core + skill runtime (host validation)
cd prototype && npm install && npm run demo

# List loaded skills and MCP tools
cd prototype && npm run skills

# Phase 2: Kernel + QEMU boot (requires Rust nightly, qemu-system-x86_64)
cargo run -p aios-boot

# Rust crates (kernel, HAL, ai-core)
cargo build
```

> **Push to GitHub**: See [docs/GITHUB_SETUP.md](docs/GITHUB_SETUP.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT License — [LICENSE](LICENSE)
