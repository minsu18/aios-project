# AI-OS — AI-Native Operating System

> **"앱이 없는 OS — AI가 곧 OS다"**
> An operating system where AI *is* the OS. No traditional app layer — AI Core directly controls all hardware via HAL.

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust](https://img.shields.io/badge/HAL-Rust%201.75+-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/AI%20Core-Python%203.11+-blue.svg)](https://python.org)
[![Status](https://img.shields.io/badge/Status-M0%20Development-yellow.svg)](https://github.com/minsu18/aios-project)
[![Architecture](https://img.shields.io/badge/Docs-Architecture-6366f1.svg)](docs/architecture.md)

> **현재 상태**: M0 개발 중 — HAL API trait 정의 및 기반 구조 확립 단계

---

## 비전 (Vision)

AI-OS는 전통적인 앱 레이어를 **완전히 제거**한 AI 네이티브 운영체제입니다.

기존 OS는 `사용자 → 앱 → OS API → 커널 → 하드웨어` 구조를 가집니다.
AI-OS는 이 구조를 혁신하여 `사용자 의도 → AI Core → HAL → 하드웨어`로 단순화합니다.

```
┌───────────────────────────────────────────────────────┐
│          User: 자연어 · 음성 · 이미지 · 제스처         │
└───────────────────────────────────────────────────────┘
                          │
                          ▼
┌───────────────────────────────────────────────────────┐
│                    AI CORE                            │
│   Intent Parser → Skill Orchestrator                  │
│   온디바이스 (Phi-4-mini / Gemma-3)                   │
│   클라우드   (Claude API)                             │
└───────────────────────────────────────────────────────┘
                          │  HAL Command
                          ▼
┌───────────────────────────────────────────────────────┐
│         Hardware Abstraction Layer (Rust)             │
│   Memory HAL │ CPU HAL │ Storage HAL │ GPU HAL …     │
└───────────────────────────────────────────────────────┘
                          │  syscall / eBPF
                          ▼
              Linux Kernel → Hardware
```

---

## 선행기술 대비 차별점

| 선행기술 | 구조 | AI-OS 차이점 |
|---------|------|------------|
| Rutgers AIOS | 기존 OS 위에 LLM 올림 | OS 자체가 AI (레이어 역전) |
| KernelAGI (2025) | 앱 레이어 유지 | 앱 레이어 **완전 제거** |
| IBM US11379110 | 앱이 전제됨 | 앱 개념 자체 없음 |
| Google AppFunctions | AI가 앱 함수 호출 | 앱 자체가 없음 |

---

## 문서

| 문서 | 설명 |
|------|------|
| [**Architecture**](docs/architecture.md) | 전체 레이어 구조, 데이터 흐름, STRIDE 보안, Glossary |
| [**Roadmap**](docs/roadmap.md) | M0→M3 마일스톤별 태스크, GitHub Projects 칸반 가이드 |
| [HAL API](docs/HAL_LLAMA_CPP.md) | HAL 인터페이스 레퍼런스 |
| [Skill Creation](docs/SKILL_CREATION.md) | Skill 작성 가이드 |
| [RPi 부팅](docs/BOOT_ON_RPI4.md) | Raspberry Pi 4 배포 가이드 |

---

## 프로젝트 구조

```
aios-project/
├── LICENSE                       # AGPL-3.0
├── README.md
├── CONTRIBUTING.md
├── Makefile
├── Cargo.toml                    # Rust workspace 루트
├── .github/
│   ├── workflows/
│   │   ├── ci-rust.yml           # HAL Rust 빌드/테스트
│   │   ├── ci-python.yml         # Intent Engine 테스트
│   │   └── ci-ui.yml             # AI Shell UI 빌드
│   └── ISSUE_TEMPLATE/
├── crates/
│   ├── ai-hal/                   # ⭐ HAL 직접 제어 레이어 (M0 핵심)
│   ├── ai-core-bridge/           # AI Core ↔ HAL 브리지 (M1)
│   └── skill-runtime/            # Skill 샌드박스 (M2)
├── python/
│   └── intent_engine/            # Intent 분석 → HAL 명령 생성
│       ├── parser.py             # ⭐ IntentParser (M0 핵심)
│       ├── hal_codegen.py        # (M1)
│       └── inference_router.py   # (M1)
├── ui/
│   └── ai-shell/                 # AI Shell UI (M2)
└── docs/
    ├── architecture.md           # 전체 아키텍처
    ├── hal-api.md                 # HAL API 레퍼런스
    ├── getting-started.md
    └── roadmap.md
```

---

## 마일스톤 (Roadmap)

| 마일스톤 | 기간 | 목표 |
|---------|------|------|
| **M0** ← 현재 | 1~2개월 | 레포 구조, HAL trait 정의, CI/CD |
| **M1** | 3~6개월 | Memory/CPU/Storage HAL 구현, AI Core 프로토타입 |
| **M2** | 6~12개월 | GPU/Camera/Audio HAL, Skill 런타임, AI Shell |
| **M3** | 12개월~ | 안정화, Skill 마켓, 외부 기여자 온보딩 |

---

## 기술 스택

| 영역 | 기술 |
|------|------|
| HAL / Kernel | Rust (cargo workspace), eBPF |
| AI Core | Python 3.11+, uv |
| UI | TypeScript + React 18 + Tailwind CSS |
| 빌드 | Makefile + GitHub Actions |
| 테스트 환경 | QEMU + Linux 6.x |
| 온디바이스 모델 | Phi-4-mini, Gemma-3 (llama.cpp / ONNX Runtime) |
| 클라우드 모델 | Claude API (Anthropic) |

---

## 시작하기 (Getting Started)

### 사전 요구사항

- Rust 1.75+ (`rustup update stable`)
- Python 3.11+ with [uv](https://github.com/astral-sh/uv)
- QEMU 8.x (테스트 환경)

### 설치 및 빌드

```bash
# 레포 클론
git clone https://github.com/minsu18/aios-project.git
cd aios-project

# HAL 크레이트 빌드 및 테스트 (M0)
cargo test -p ai-hal --features mock

# Python Intent Engine 실행 (M0)
cd python
uv pip install -e .
python -m intent_engine.parser

# 전체 빌드
cargo build --workspace
```

### 빠른 동작 확인

```bash
# Intent 파서 동작 확인
python python/intent_engine/parser.py

# HAL mock 테스트
cargo test -p ai-hal --features mock -- --nocapture
```

---

## 기여하기 (Contributing)

[CONTRIBUTING.md](CONTRIBUTING.md)를 먼저 읽어주세요.

- 커밋 메시지: [Conventional Commits](https://www.conventionalcommits.org/) 형식 사용
- 코드 주석: 한국어로 작성
- 모든 소스 파일 상단에 AGPL-3.0 헤더 포함 필수

---

## 라이선스 (License)

**AGPL-3.0-or-later** — [LICENSE](LICENSE)

Copyright (C) 2025 minsu18 <https://github.com/minsu18>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published
by the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

> 듀얼 라이선싱(상업용 라이선스)은 미래 옵션으로 유지됩니다.
> Commercial licensing is reserved as a future option.
