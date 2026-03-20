# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

"""
models.py
=========
AI-OS Intent Engine 핵심 데이터 모델 정의.

## 설계 원칙
- 모든 모델은 `dataclass` 기반으로 불변(frozen=True) 설계
- JSON 직렬화 지원: `to_dict()` / `from_dict()` 메서드 포함
- HAL 레이어와의 계약(Contract): `HalCommandModel`은 Rust `HalCommand` 열거형과 1:1 대응

## 의존 관계
```
UserInput (str)
    → IntentObject        (Intent Engine 출력)
    → List[HalCommandModel]  (HalCommandGenerator 출력)
    → Rust ai-hal 호출   (ai-core-bridge 경유)
```

## 참조
- Phi-4-mini: https://huggingface.co/microsoft/phi-4-mini-instruct
- Chain of Responsibility: GoF Design Patterns (1994), p.223
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field, asdict
from enum import Enum, auto
from typing import Any


# ─────────────────────────────────────────────
#  열거형 타입
# ─────────────────────────────────────────────

class IntentType(Enum):
    """사용자 의도 분류 열거형.

    AI Core가 수행할 상위 레벨 작업을 나타낸다.
    HAL 명령 시퀀스 생성의 기준이 됨.

    ## 추가 규칙
    - M0: QUERY_STATE, ALLOCATE_MEMORY, CPU_HINT, OPEN_FILE, WRITE_FILE
    - M1+: SKILL_INSTALL, MODEL_LOAD, SENSOR_READ 추가 예정
    """

    # 리소스 현재 상태 조회 (읽기 전용)
    QUERY_STATE = "query_state"

    # 메모리 할당 요청 (mmap 기반)
    ALLOCATE_MEMORY = "allocate_memory"

    # 메모리 해제 요청
    FREE_MEMORY = "free_memory"

    # CPU 스케줄링 힌트 (어피니티 설정)
    CPU_HINT = "cpu_hint"

    # 파일 열기 (읽기)
    OPEN_FILE = "open_file"

    # 파일 열기 (쓰기 / 생성)
    WRITE_FILE = "write_file"

    # Skill 등록
    REGISTER_SKILL = "register_skill"

    # 인식 불가 / 신뢰도 부족
    UNKNOWN = "unknown"


class ResourceType(Enum):
    """HAL이 제어하는 하드웨어 리소스 분류.

    Rust `ai_hal::ResourceType`과 문자열 값이 동일해야 함.
    ai-core-bridge가 JSON 직렬화로 Rust 측에 전달.
    """

    MEMORY = "Memory"
    CPU = "Cpu"
    STORAGE = "Storage"
    GPU = "Gpu"
    NETWORK = "Network"
    AUDIO = "Audio"
    CAMERA = "Camera"
    DISPLAY = "Display"
    UNKNOWN = "Unknown"


class InferenceBackend(Enum):
    """추론 백엔드 종류.

    InferenceRouter가 결정한 실제 추론 경로를 나타냄.

    ## 우선순위 (낮은 지연 → 높은 지연)
    1. RULE_BASED  : 정규식 + 키워드 매칭 (< 1 ms)
    2. ON_DEVICE   : Phi-4-mini 로컬 추론 (< 500 ms)
    3. CLOUD_API   : Claude API 클라우드 추론 (네트워크 필요)
    """

    # 규칙 기반 (정규식 + 키워드)
    RULE_BASED = "rule_based"

    # 온디바이스 모델 (Phi-4-mini)
    ON_DEVICE = "on_device"

    # 클라우드 API (Claude API)
    CLOUD_API = "cloud_api"


class HalCommandType(Enum):
    """HAL 명령 종류.

    Rust `ai_hal::HalCommand` 열거형 변형과 1:1 대응.
    ai-core-bridge가 이 값을 기반으로 Rust 타입을 생성.
    """

    QUERY_STATE = "QueryState"
    ALLOCATE_MEMORY = "AllocateMemory"
    FREE_MEMORY = "FreeMemory"
    CPU_SCHEDULING_HINT = "CpuSchedulingHint"
    OPEN_STORAGE_READ = "OpenStorageRead"
    OPEN_STORAGE_WRITE = "OpenStorageWrite"
    REGISTER_SKILL = "RegisterSkill"


# ─────────────────────────────────────────────
#  핵심 데이터 클래스
# ─────────────────────────────────────────────

@dataclass
class IntentObject:
    """자연어 입력을 파싱한 의도 표현 객체.

    `IntentParser`가 생성하고 `HalCommandGenerator`가 소비한다.

    ## 필드 설명
    - `intent_type`: 파싱된 의도 분류
    - `resource_type`: 대상 하드웨어 리소스
    - `parameters`: 의도별 추가 파라미터 (자유형 dict)
    - `confidence`: 추론 신뢰도 (0.0 ~ 1.0)
    - `raw_input`: 원본 사용자 입력 문자열
    - `backend_used`: 실제 사용된 추론 백엔드
    - `latency_ms`: 파싱 소요 시간 (밀리초)
    - `timestamp`: 파싱 완료 시각 (Unix epoch, 초)

    ## 신뢰도 기준
    - >= 0.9: 고신뢰도 — 바로 HAL 실행
    - 0.6 ~ 0.9: 중신뢰도 — 확인 후 실행
    - < 0.6: 저신뢰도 — UNKNOWN 처리 또는 상위 백엔드로 에스컬레이션
    """

    # 파싱된 의도 분류
    intent_type: IntentType

    # 대상 리소스
    resource_type: ResourceType

    # 의도별 추가 파라미터
    # 예: {"size_bytes": 4096, "alignment": 4096}
    parameters: dict[str, Any] = field(default_factory=dict)

    # 추론 신뢰도 (0.0 ~ 1.0)
    confidence: float = 0.0

    # 원본 입력 문자열
    raw_input: str = ""

    # 실제 사용된 백엔드
    backend_used: InferenceBackend = InferenceBackend.RULE_BASED

    # 파싱 소요 시간 (밀리초)
    latency_ms: float = 0.0

    # 파싱 완료 시각 (Unix epoch)
    timestamp: float = field(default_factory=time.time)

    # ── 편의 메서드 ──────────────────────────────

    def is_confident(self, threshold: float = 0.6) -> bool:
        """신뢰도가 임계값 이상인지 확인.

        Args:
            threshold: 신뢰도 임계값 (기본 0.6)

        Returns:
            True: 충분한 신뢰도로 HAL 실행 가능
        """
        return self.confidence >= threshold

    def to_dict(self) -> dict[str, Any]:
        """JSON 직렬화를 위한 딕셔너리 변환.

        ai-core-bridge가 이 포맷으로 Rust 측에 전달.
        """
        return {
            "intent_type": self.intent_type.value,
            "resource_type": self.resource_type.value,
            "parameters": self.parameters,
            "confidence": self.confidence,
            "raw_input": self.raw_input,
            "backend_used": self.backend_used.value,
            "latency_ms": self.latency_ms,
            "timestamp": self.timestamp,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "IntentObject":
        """딕셔너리에서 IntentObject 생성.

        Args:
            data: `to_dict()` 포맷의 딕셔너리

        Returns:
            IntentObject 인스턴스
        """
        return cls(
            intent_type=IntentType(data["intent_type"]),
            resource_type=ResourceType(data["resource_type"]),
            parameters=data.get("parameters", {}),
            confidence=data.get("confidence", 0.0),
            raw_input=data.get("raw_input", ""),
            backend_used=InferenceBackend(data.get("backend_used", "rule_based")),
            latency_ms=data.get("latency_ms", 0.0),
            timestamp=data.get("timestamp", time.time()),
        )

    @classmethod
    def unknown(cls, raw_input: str, backend: InferenceBackend = InferenceBackend.RULE_BASED) -> "IntentObject":
        """인식 불가 의도 생성 팩토리 메서드.

        Args:
            raw_input: 원본 입력 문자열
            backend: 시도한 백엔드

        Returns:
            UNKNOWN 타입의 IntentObject (신뢰도 0.0)
        """
        return cls(
            intent_type=IntentType.UNKNOWN,
            resource_type=ResourceType.UNKNOWN,
            confidence=0.0,
            raw_input=raw_input,
            backend_used=backend,
        )

    def __repr__(self) -> str:
        return (
            f"IntentObject("
            f"type={self.intent_type.value!r}, "
            f"resource={self.resource_type.value!r}, "
            f"confidence={self.confidence:.2f}, "
            f"backend={self.backend_used.value!r})"
        )


@dataclass
class HalCommandModel:
    """HAL로 전송할 단일 명령 표현 모델.

    `HalCommandGenerator`가 생성하고 ai-core-bridge가 Rust
    `ai_hal::HalCommand` 열거형으로 변환.

    ## 직렬화 포맷 (JSON)
    ```json
    {
      "command_type": "QueryState",
      "parameters": {"resource": "Memory", "detailed": false},
      "requires_capability": "Memory",
      "priority": 0
    }
    ```

    ## 파라미터 규격 (command_type별)
    - QueryState: {"resource": str, "detailed": bool}
    - AllocateMemory: {"size_bytes": int, "alignment": int, "shared": bool}
    - FreeMemory: {"handle_id": int}
    - CpuSchedulingHint: {"pid": int, "priority": int, "preferred_core": int|None}
    - OpenStorageRead: {"path": str}
    - OpenStorageWrite: {"path": str, "create_if_missing": bool}
    - RegisterSkill: {"name": str, "version": str, "description": str, "capabilities": list[str]}
    """

    # HAL 명령 종류
    command_type: HalCommandType

    # 명령별 파라미터
    parameters: dict[str, Any] = field(default_factory=dict)

    # 이 명령 실행에 필요한 capability 리소스 타입
    requires_capability: ResourceType = ResourceType.MEMORY

    # 실행 우선순위 (낮을수록 높은 우선순위)
    priority: int = 0

    def to_dict(self) -> dict[str, Any]:
        """ai-core-bridge 전송용 직렬화."""
        return {
            "command_type": self.command_type.value,
            "parameters": self.parameters,
            "requires_capability": self.requires_capability.value,
            "priority": self.priority,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "HalCommandModel":
        """딕셔너리에서 HalCommandModel 생성."""
        return cls(
            command_type=HalCommandType(data["command_type"]),
            parameters=data.get("parameters", {}),
            requires_capability=ResourceType(data.get("requires_capability", "Memory")),
            priority=data.get("priority", 0),
        )

    def __repr__(self) -> str:
        return (
            f"HalCommandModel("
            f"type={self.command_type.value!r}, "
            f"params={self.parameters!r})"
        )


@dataclass
class ParseResult:
    """IntentParser의 최종 출력 묶음.

    의도 객체 + 생성된 HAL 명령 목록 + 메타데이터를 묶어
    ai-core-bridge가 Rust 측으로 전달하는 패키지.

    ## 사용 흐름
    ```python
    result = parser.parse("메모리 상태 조회해줘")
    if result.is_executable():
        bridge.execute(result.hal_commands, result.intent.parameters)
    ```
    """

    # 파싱된 의도 객체
    intent: IntentObject

    # 생성된 HAL 명령 시퀀스 (순서 보장)
    hal_commands: list[HalCommandModel] = field(default_factory=list)

    # 전체 파이프라인 소요 시간 (밀리초)
    total_latency_ms: float = 0.0

    # 사용자에게 보여줄 응답 텍스트 (선택)
    response_text: str = ""

    def is_executable(self) -> bool:
        """HAL 실행 가능한 상태인지 확인.

        Returns:
            True: 신뢰도 충분 + HAL 명령 최소 1개 이상
        """
        return (
            self.intent.is_confident()
            and len(self.hal_commands) > 0
            and self.intent.intent_type != IntentType.UNKNOWN
        )

    def to_dict(self) -> dict[str, Any]:
        """JSON 직렬화."""
        return {
            "intent": self.intent.to_dict(),
            "hal_commands": [cmd.to_dict() for cmd in self.hal_commands],
            "total_latency_ms": self.total_latency_ms,
            "response_text": self.response_text,
        }

    def __repr__(self) -> str:
        return (
            f"ParseResult("
            f"intent={self.intent!r}, "
            f"commands={len(self.hal_commands)}, "
            f"executable={self.is_executable()})"
        )


@dataclass
class RouterConfig:
    """InferenceRouter 설정 값 묶음.

    ## 필드 설명
    - `rule_confidence_threshold`: 규칙 기반 최소 신뢰도 (이하 → 온디바이스로 에스컬레이션)
    - `on_device_confidence_threshold`: 온디바이스 최소 신뢰도 (이하 → 클라우드로 에스컬레이션)
    - `on_device_model_path`: Phi-4-mini GGUF 모델 파일 경로
    - `cloud_api_key_env`: Claude API 키를 담은 환경 변수 이름
    - `max_latency_ms`: 온디바이스 추론 최대 허용 시간 (초과 → 클라우드로 폴백)
    - `prefer_privacy`: True이면 클라우드 API 비활성화 (온디바이스만 사용)
    """

    # 규칙 기반 최소 신뢰도 임계값
    rule_confidence_threshold: float = 0.75

    # 온디바이스 최소 신뢰도 임계값
    on_device_confidence_threshold: float = 0.60

    # Phi-4-mini 모델 파일 경로 (GGUF 포맷)
    # 참조: https://huggingface.co/microsoft/phi-4-mini-instruct
    on_device_model_path: str = "models/phi-4-mini-instruct.Q4_K_M.gguf"

    # Claude API 키 환경 변수 이름
    cloud_api_key_env: str = "ANTHROPIC_API_KEY"

    # 온디바이스 추론 최대 허용 지연 (밀리초)
    max_on_device_latency_ms: float = 2000.0

    # 프라이버시 모드: True → 클라우드 API 완전 비활성화
    prefer_privacy: bool = False

    # 온디바이스 모델 컨텍스트 길이 (토큰)
    context_length: int = 2048
