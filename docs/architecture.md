# AI-OS 아키텍처 설계 문서

> SPDX-License-Identifier: AGPL-3.0-or-later
> Copyright (C) 2025 minsu18 <https://github.com/minsu18>
> Project : AI-OS — https://github.com/minsu18/aios-project

---

## 1. 핵심 철학

> **"앱이 없는 OS — AI가 곧 OS다"**

전통적인 OS는 `사용자 → 앱 → OS API → 커널 → 하드웨어` 구조를 가진다.
AI-OS는 앱 레이어를 완전히 제거하고, AI Core가 HAL(Hardware Abstraction Layer)을 직접 제어한다.

---

## 2. 전체 레이어 다이어그램

```
┌─────────────────────────────────────────────────────────────────┐
│                        USER INTENT LAYER                        │
│                                                                 │
│   자연어 입력 / 음성 / 제스처 / 시선 추적 등 모든 입력 형태      │
│   "음악 틀어줘", "파일 정리해줘", "메일 써줘"                    │
└────────────────────────────┬────────────────────────────────────┘
                             │  Intent (자연어 → 구조화 명령)
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                         AI CORE LAYER                           │
│                                                                 │
│  ┌─────────────────┐  ┌──────────────────┐  ┌───────────────┐  │
│  │  Intent Parser  │  │ Context Manager  │  │   Inference   │  │
│  │  (parser.py)    │  │ (context.py)     │  │   Router      │  │
│  │                 │  │                  │  │ (router.py)   │  │
│  │ 자연어 분석      │  │ 세션/상태 관리   │  │               │  │
│  │ Intent 객체 생성 │  │ 이전 명령 참조   │  │ 온디바이스:   │  │
│  └────────┬────────┘  └────────┬─────────┘  │ Phi-4-mini   │  │
│           │                   │             │ Gemma-3      │  │
│           └───────────────────┘             │               │  │
│                       │                     │ 클라우드:     │  │
│                       ▼                     │ Claude API   │  │
│              ┌─────────────────┐            └───────────────┘  │
│              │ Skill           │                                │
│              │ Orchestrator    │                                │
│              │ (orchestrator.py│                                │
│              │                 │                                │
│              │ HAL 명령 시퀀스  │                                │
│              │ 계획 및 실행    │                                │
│              └────────┬────────┘                               │
└───────────────────────┼─────────────────────────────────────────┘
                        │  HAL Command (Rust FFI / IPC)
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                    AI-CORE BRIDGE LAYER                         │
│                  (crates/ai-core-bridge)                        │
│                                                                 │
│   Python AI Core ↔ Rust HAL 간 직렬화/역직렬화 브리지           │
│   Protocol Buffers / MessagePack 기반 IPC                       │
│   HAL 명령 검증 및 권한 확인 (pre-flight check)                 │
└────────────────────────┬────────────────────────────────────────┘
                         │  Validated HAL Command
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  HARDWARE ABSTRACTION LAYER                     │
│                      (crates/ai-hal)                            │
│                                                                 │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐    │
│  │  MemoryHal   │ │   CpuHal     │ │     StorageHal       │    │
│  │              │ │              │ │                      │    │
│  │ 메모리 할당  │ │ 프로세스 제어 │ │ 파일시스템 직접 제어  │    │
│  │ 맵핑/언맵핑  │ │ 스케줄링 힌트 │ │ 블록 I/O             │    │
│  └──────────────┘ └──────────────┘ └──────────────────────┘    │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐    │
│  │  NetworkHal  │ │   GpuHal     │ │     AudioHal (M2)    │    │
│  │              │ │  (M2 이후)   │ │       (M2 이후)      │    │
│  │ 소켓 추상화  │ │ GPGPU 제어   │ │ 오디오 스트림 제어    │    │
│  └──────────────┘ └──────────────┘ └──────────────────────┘    │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              AiHalInterface (Rust Trait)                 │   │
│  │  - execute_command(cmd: HalCommand) -> HalResult         │   │
│  │  - query_state(resource: ResourceType) -> ResourceState  │   │
│  │  - register_skill(skill: SkillManifest) -> SkillToken    │   │
│  └─────────────────────────────────────────────────────────┘   │
└────────────────────────┬────────────────────────────────────────┘
                         │  syscall / eBPF
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LINUX KERNEL LAYER                           │
│                                                                 │
│   Linux 6.x  |  eBPF Programs  |  Kernel Modules (선택적)      │
│   syscall 인터셉트 → HAL 이벤트로 변환                          │
└─────────────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                      HARDWARE LAYER                             │
│         CPU │ Memory │ Storage │ GPU │ Network │ Audio          │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. 선행기술 대비 차별점

| 선행기술 | 구조 | AI-OS |
|---------|------|-------|
| Rutgers AIOS | 기존 OS 위에 LLM 올림 | OS 자체가 AI (레이어 역전) |
| KernelAGI (2025) | 앱 레이어 유지 | 앱 레이어 **완전 제거** |
| IBM US11379110 | 앱이 전제됨 | 앱 개념 자체 없음 |
| Google AppFunctions | AI가 앱 함수 호출 | 앱 자체가 없음, HAL 직접 제어 |

---

## 4. 데이터 흐름 (상세)

```
사용자 입력: "현재 메모리 사용량 보여줘"
    │
    ▼ [Intent Parser]
    IntentObject {
        intent_type: Query,
        target: ResourceType::Memory,
        action: Action::GetState,
        params: {},
        confidence: 0.97
    }
    │
    ▼ [Skill Orchestrator]
    HalCommand::QueryState {
        resource: ResourceType::Memory,
        detail_level: Full
    }
    │
    ▼ [ai-core-bridge] → 직렬화 + 권한 검증
    │
    ▼ [ai-hal::MemoryHal::query_state()]
    ResourceState::Memory {
        total_bytes: 16_000_000_000,
        used_bytes: 8_200_000_000,
        free_bytes: 7_800_000_000,
        ...
    }
    │
    ▼ [AI Core] → 자연어 응답 생성
    "현재 메모리: 총 16GB 중 8.2GB 사용 중 (51%)"
```

---

## 5. Skill 격리 모델

```
┌─────────────────────────────────────┐
│         Skill Runtime               │
│    (crates/skill-runtime)           │
│                                     │
│  ┌─────────┐    ┌─────────────────┐ │
│  │ Skill A │    │ Capability ACL  │ │
│  │ sandbox │───▶│                 │ │
│  └─────────┘    │ - memory: read  │ │
│                 │ - cpu: hint     │ │
│  ┌─────────┐    │ - storage: /tmp │ │
│  │ Skill B │    │   only          │ │
│  │ sandbox │───▶│ - network: deny │ │
│  └─────────┘    └─────────────────┘ │
└─────────────────────────────────────┘
```

각 Skill은 독립 메모리 공간에서 실행되며, HAL 접근은 capability token으로만 허용.

---

## 6. M0 → M3 아키텍처 진화 계획

| 마일스톤 | 구현 범위 |
|---------|---------|
| **M0** | HAL trait 정의, CI 기반, Intent 파서 뼈대 |
| **M1** | MemoryHal / CpuHal / StorageHal 구현, AI Core 프로토타입 |
| **M2** | GpuHal / AudioHal / CameraHal, Skill Runtime, AI Shell UI |
| **M3** | 안정화, Skill 마켓, 외부 기여자 온보딩 |
