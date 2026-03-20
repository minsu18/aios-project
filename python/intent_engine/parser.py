# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

"""
intent_engine.parser
====================

사용자 자연어 입력을 구조화된 IntentObject로 변환하는 모듈.

## 아키텍처 위치
    User Input (자연어)
        │
        ▼
    IntentParser.parse(text)
        │
        ├─ 온디바이스 추론: Phi-4-mini / Gemma-3 (llama.cpp / ONNX)
        └─ 클라우드 추론:  Claude API (Anthropic)
        │
        ▼
    IntentObject
        │
        ▼
    Skill Orchestrator → HAL Command

## M0 구현 범위
- IntentObject 데이터 타입 정의 (dataclass + Enum)
- IntentParser 클래스 뼈대 (규칙 기반 fallback 포함)
- 온디바이스 / 클라우드 추론 라우팅 인터페이스 (stub)

## M1 구현 예정
- Phi-4-mini / Gemma-3 실제 연동
- Claude API 실제 연동
- 신뢰도 기반 자동 라우팅
"""

from __future__ import annotations

import re
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Optional


# ─────────────────────────────────────────────
#  섹션 1: Intent 타입 정의 (Enum)
# ─────────────────────────────────────────────

class IntentType(Enum):
    """
    사용자 의도(Intent)의 최상위 분류.

    HAL 명령 생성 시 어떤 HalCommand 변형을 사용할지 결정하는 기준.
    """
    # 리소스 상태 조회 ("메모리 얼마나 사용 중이야?")
    QUERY = auto()
    # 리소스 제어 명령 ("메모리 정리해줘")
    CONTROL = auto()
    # 파일/스토리지 관련 작업 ("파일 정리해줘", "사진 폴더 열어줘")
    FILE_OPERATION = auto()
    # 미디어 재생/제어 ("음악 틀어줘", "볼륨 줄여줘")
    MEDIA = auto()
    # 통신/네트워크 작업 ("이메일 보내줘", "Wi-Fi 연결해줘")
    COMMUNICATION = auto()
    # 시스템 설정 변경 ("밝기 올려줘", "절전 모드 켜줘")
    SYSTEM_CONFIG = auto()
    # 파서가 의도를 파악하지 못한 경우
    UNKNOWN = auto()


class ActionType(Enum):
    """
    Intent 안에서의 구체적 행동 종류.

    IntentType과 조합하여 정확한 HAL 명령을 생성.
    예: IntentType.QUERY + ActionType.GET_STATE → HalCommand::QueryState
    """
    # 상태/정보 조회
    GET_STATE = auto()
    # 리소스 할당 / 시작
    ALLOCATE = auto()
    # 리소스 해제 / 중지
    FREE = auto()
    # 파일/디렉토리 열기
    OPEN = auto()
    # 파일/디렉토리 닫기
    CLOSE = auto()
    # 이동/이름변경
    MOVE = auto()
    # 삭제
    DELETE = auto()
    # 재생 시작
    PLAY = auto()
    # 일시정지
    PAUSE = auto()
    # 정지
    STOP = auto()
    # 볼륨/밝기 등 증가
    INCREASE = auto()
    # 볼륨/밝기 등 감소
    DECREASE = auto()
    # 설정 변경
    SET = auto()
    # 전송/발신
    SEND = auto()
    # 알 수 없는 행동
    UNKNOWN = auto()


class ResourceTarget(Enum):
    """
    Intent가 대상으로 하는 하드웨어/소프트웨어 리소스.

    ai-hal의 ResourceType과 대응됨.
    """
    MEMORY = auto()
    CPU = auto()
    STORAGE = auto()
    GPU = auto()
    AUDIO = auto()
    CAMERA = auto()
    NETWORK = auto()
    DISPLAY = auto()
    # 특정 리소스가 명확하지 않은 경우
    UNSPECIFIED = auto()


class InferenceBackend(Enum):
    """
    Intent 분석에 사용된 추론 백엔드.

    감사 로그 및 비용 추적에 사용.
    """
    # 규칙 기반 (모델 없음, M0 기본)
    RULE_BASED = "rule_based"
    # 온디바이스: Phi-4-mini (Microsoft, 3.8B params)
    PHI4_MINI = "phi4_mini"
    # 온디바이스: Gemma-3 (Google)
    GEMMA3 = "gemma3"
    # 클라우드: Claude API (Anthropic)
    CLAUDE_API = "claude_api"


# ─────────────────────────────────────────────
#  섹션 2: Intent 데이터 타입 정의
# ─────────────────────────────────────────────

@dataclass
class IntentObject:
    """
    파서가 생성하는 구조화된 사용자 의도 객체.

    이 객체는 Skill Orchestrator로 전달되어
    HAL Command 시퀀스로 변환된다.

    Attributes:
        intent_type: 의도의 최상위 분류 (QUERY, CONTROL, ...)
        action:      구체적 행동 (GET_STATE, PLAY, ...)
        target:      대상 리소스 (MEMORY, AUDIO, ...)
        raw_text:    원본 사용자 입력 (디버깅/감사용)
        confidence:  파싱 신뢰도 0.0 ~ 1.0
        params:      추가 파라미터 (경로, 볼륨 값 등)
        backend:     분석에 사용된 추론 백엔드
        parsed_at:   파싱 완료 시각 (Unix timestamp)
    """
    intent_type: IntentType
    action: ActionType
    target: ResourceTarget
    raw_text: str
    confidence: float
    params: dict[str, Any] = field(default_factory=dict)
    backend: InferenceBackend = InferenceBackend.RULE_BASED
    parsed_at: float = field(default_factory=time.time)

    def is_confident(self, threshold: float = 0.7) -> bool:
        """
        신뢰도가 임계값 이상인지 확인.

        Args:
            threshold: 신뢰도 임계값 (기본 0.7)

        Returns:
            bool: 신뢰도 >= threshold이면 True
        """
        return self.confidence >= threshold

    def to_dict(self) -> dict[str, Any]:
        """
        직렬화용 딕셔너리 변환.

        ai-core-bridge를 통해 Rust HAL로 전송 시 사용.

        Returns:
            dict: JSON 직렬화 가능한 딕셔너리
        """
        return {
            "intent_type": self.intent_type.name,
            "action": self.action.name,
            "target": self.target.name,
            "raw_text": self.raw_text,
            "confidence": self.confidence,
            "params": self.params,
            "backend": self.backend.value,
            "parsed_at": self.parsed_at,
        }

    @classmethod
    def unknown(cls, raw_text: str) -> "IntentObject":
        """
        의도를 파악하지 못했을 때 반환하는 UNKNOWN Intent 생성.

        Args:
            raw_text: 원본 사용자 입력

        Returns:
            IntentObject: UNKNOWN 타입의 IntentObject
        """
        return cls(
            intent_type=IntentType.UNKNOWN,
            action=ActionType.UNKNOWN,
            target=ResourceTarget.UNSPECIFIED,
            raw_text=raw_text,
            confidence=0.0,
        )


# ─────────────────────────────────────────────
#  섹션 3: 추론 백엔드 인터페이스 (추상 클래스)
# ─────────────────────────────────────────────

class InferenceBackendInterface(ABC):
    """
    추론 백엔드 추상 인터페이스.

    온디바이스(Phi-4-mini, Gemma-3)와 클라우드(Claude API) 모두
    이 인터페이스를 구현하여 IntentParser에 주입됨.

    알고리즘: Strategy 패턴 (GoF)
    참조: https://refactoring.guru/design-patterns/strategy
    """

    @abstractmethod
    def infer(self, text: str, context: Optional[dict]) -> IntentObject:
        """
        자연어 텍스트를 IntentObject로 변환.

        Args:
            text:    사용자 입력 텍스트
            context: 이전 대화 컨텍스트 (Context Manager에서 전달)

        Returns:
            IntentObject: 변환된 의도 객체
        """
        ...

    @abstractmethod
    def is_available(self) -> bool:
        """
        이 백엔드가 현재 사용 가능한지 확인.

        온디바이스: 모델 파일 존재 여부
        클라우드: API 키 존재 및 네트워크 연결 여부

        Returns:
            bool: 사용 가능하면 True
        """
        ...

    @property
    @abstractmethod
    def backend_type(self) -> InferenceBackend:
        """이 백엔드의 타입 반환."""
        ...


# ─────────────────────────────────────────────
#  섹션 4: 규칙 기반 폴백 파서 (M0 기본 구현)
# ─────────────────────────────────────────────

class RuleBasedBackend(InferenceBackendInterface):
    """
    규칙 기반 Intent 파서 (M0 기본 구현).

    실제 LLM 없이 키워드 매칭으로 Intent를 추론.
    신뢰도는 낮지만 모델 없이도 동작 가능한 fallback.

    M1에서 Phi-4-mini / Claude API로 교체 예정.

    알고리즘:
        키워드 → (IntentType, ActionType, ResourceTarget) 매핑 테이블
        정규식으로 파라미터 추출 (볼륨 값, 경로 등)
        매칭 수에 따라 신뢰도 계산
    """

    # 리소스 키워드 매핑 테이블
    # 형식: {ResourceTarget: [키워드 목록]}
    _RESOURCE_KEYWORDS: dict[ResourceTarget, list[str]] = {
        ResourceTarget.MEMORY: ["메모리", "램", "ram", "memory"],
        ResourceTarget.CPU: ["cpu", "씨피유", "프로세서", "processor", "코어"],
        ResourceTarget.STORAGE: ["디스크", "저장", "파일", "폴더", "disk", "storage", "ssd"],
        ResourceTarget.AUDIO: ["음악", "소리", "볼륨", "오디오", "audio", "music", "sound", "volume"],
        ResourceTarget.NETWORK: ["네트워크", "wifi", "인터넷", "network", "연결"],
        ResourceTarget.DISPLAY: ["화면", "밝기", "디스플레이", "display", "brightness"],
        ResourceTarget.GPU: ["gpu", "그래픽", "graphic"],
    }

    # 행동 키워드 매핑 테이블
    # 형식: {ActionType: [키워드 목록]}
    _ACTION_KEYWORDS: dict[ActionType, list[str]] = {
        ActionType.GET_STATE: ["얼마나", "상태", "현재", "보여", "알려", "확인", "status", "show", "check"],
        ActionType.PLAY: ["틀어", "재생", "play", "시작"],
        ActionType.PAUSE: ["일시정지", "멈춰", "pause"],
        ActionType.STOP: ["종료", "꺼", "stop", "끄"],
        ActionType.INCREASE: ["올려", "높여", "증가", "up", "increase"],
        ActionType.DECREASE: ["줄여", "낮춰", "감소", "down", "decrease"],
        ActionType.FREE: ["정리", "해제", "청소", "clean", "free", "clear"],
        ActionType.OPEN: ["열어", "접근", "open"],
        ActionType.DELETE: ["삭제", "지워", "제거", "delete", "remove"],
        ActionType.SEND: ["보내", "전송", "send"],
    }

    # 의도 타입 → 행동 타입 연관 매핑
    _INTENT_FROM_ACTION: dict[ActionType, IntentType] = {
        ActionType.GET_STATE: IntentType.QUERY,
        ActionType.PLAY: IntentType.MEDIA,
        ActionType.PAUSE: IntentType.MEDIA,
        ActionType.STOP: IntentType.MEDIA,
        ActionType.INCREASE: IntentType.SYSTEM_CONFIG,
        ActionType.DECREASE: IntentType.SYSTEM_CONFIG,
        ActionType.FREE: IntentType.CONTROL,
        ActionType.OPEN: IntentType.FILE_OPERATION,
        ActionType.DELETE: IntentType.FILE_OPERATION,
        ActionType.SEND: IntentType.COMMUNICATION,
    }

    def infer(self, text: str, context: Optional[dict] = None) -> IntentObject:
        """
        키워드 매칭으로 Intent를 추론.

        알고리즘:
            1. 텍스트 정규화 (소문자 변환)
            2. ResourceTarget 매칭
            3. ActionType 매칭
            4. IntentType 결정 (ActionType 기반)
            5. 파라미터 추출 (볼륨 값, 경로 등 정규식)
            6. 신뢰도 계산 (매칭 항목 수 기반)

        Args:
            text:    사용자 입력
            context: 이전 대화 컨텍스트 (규칙 기반에서는 미사용)

        Returns:
            IntentObject: 추론된 의도 객체
        """
        normalized = text.lower().strip()

        # 리소스 매칭
        matched_resource = ResourceTarget.UNSPECIFIED
        resource_match_count = 0
        for resource, keywords in self._RESOURCE_KEYWORDS.items():
            for kw in keywords:
                if kw in normalized:
                    matched_resource = resource
                    resource_match_count += 1
                    break

        # 행동 매칭
        matched_action = ActionType.UNKNOWN
        action_match_count = 0
        for action, keywords in self._ACTION_KEYWORDS.items():
            for kw in keywords:
                if kw in normalized:
                    matched_action = action
                    action_match_count += 1
                    break

        # 의도 타입 결정
        matched_intent = self._INTENT_FROM_ACTION.get(
            matched_action, IntentType.UNKNOWN
        )

        # 파라미터 추출: 볼륨 수치 (예: "볼륨 50으로", "volume to 80")
        params: dict[str, Any] = {}
        volume_match = re.search(r"(\d+)\s*%?", normalized)
        if volume_match and matched_resource == ResourceTarget.AUDIO:
            params["value"] = int(volume_match.group(1))

        # 파일 경로 추출 (따옴표로 감싼 경우)
        path_match = re.search(r'["\']([^"\']+)["\']', text)
        if path_match:
            params["path"] = path_match.group(1)

        # 신뢰도 계산
        # - 리소스 + 행동 모두 매칭: 0.75
        # - 행동만 매칭: 0.45
        # - 리소스만 매칭: 0.35
        # - 둘 다 미매칭: 0.0
        confidence = 0.0
        if resource_match_count > 0 and action_match_count > 0:
            confidence = 0.75
        elif action_match_count > 0:
            confidence = 0.45
        elif resource_match_count > 0:
            confidence = 0.35

        # 알 수 없는 의도 처리
        if matched_intent == IntentType.UNKNOWN or matched_action == ActionType.UNKNOWN:
            return IntentObject.unknown(text)

        return IntentObject(
            intent_type=matched_intent,
            action=matched_action,
            target=matched_resource,
            raw_text=text,
            confidence=confidence,
            params=params,
            backend=InferenceBackend.RULE_BASED,
        )

    def is_available(self) -> bool:
        """규칙 기반 파서는 항상 사용 가능."""
        return True

    @property
    def backend_type(self) -> InferenceBackend:
        return InferenceBackend.RULE_BASED


# ─────────────────────────────────────────────
#  섹션 5: 클라우드 Claude API 백엔드 (Stub)
# ─────────────────────────────────────────────

class ClaudeApiBackend(InferenceBackendInterface):
    """
    Claude API를 사용하는 클라우드 추론 백엔드 (M1 구현 예정).

    M0에서는 stub으로만 존재하며, is_available()은 항상 False 반환.
    M1에서 실제 Anthropic SDK 연동 구현 예정.

    참조:
        - Anthropic API: https://docs.anthropic.com
        - anthropic Python SDK: https://github.com/anthropics/anthropic-sdk-python
    """

    def __init__(self, api_key: Optional[str] = None):
        """
        Args:
            api_key: Anthropic API 키. None이면 환경변수 ANTHROPIC_API_KEY 사용.
        """
        self._api_key = api_key
        # M1에서 실제 anthropic 클라이언트 초기화 예정
        # import anthropic
        # self._client = anthropic.Anthropic(api_key=api_key)

    def infer(self, text: str, context: Optional[dict] = None) -> IntentObject:
        """
        M0: 미구현 stub. NotImplementedError 발생.
        M1: Claude API 호출로 Intent 추론.

        Raises:
            NotImplementedError: M0에서는 항상 발생
        """
        raise NotImplementedError(
            "ClaudeApiBackend는 M1에서 구현 예정입니다. "
            "현재는 RuleBasedBackend 또는 온디바이스 백엔드를 사용하세요."
        )

    def is_available(self) -> bool:
        """M0에서는 항상 False (미구현)."""
        return False

    @property
    def backend_type(self) -> InferenceBackend:
        return InferenceBackend.CLAUDE_API


# ─────────────────────────────────────────────
#  섹션 6: 온디바이스 백엔드 (Stub)
# ─────────────────────────────────────────────

class OnDeviceBackend(InferenceBackendInterface):
    """
    온디바이스 LLM을 사용하는 추론 백엔드 (M1 구현 예정).

    지원 모델:
        - Phi-4-mini (Microsoft, 3.8B) — llama.cpp 또는 ONNX Runtime
        - Gemma-3 (Google) — llama.cpp 또는 ONNX Runtime

    M0에서는 stub으로만 존재.
    M1에서 llama.cpp Python 바인딩으로 구현 예정.

    참조:
        - Phi-4-mini: https://huggingface.co/microsoft/Phi-4-mini-instruct
        - Gemma-3: https://huggingface.co/google/gemma-3-1b-it
        - llama-cpp-python: https://github.com/abetlen/llama-cpp-python
    """

    def __init__(self, model: InferenceBackend = InferenceBackend.PHI4_MINI):
        """
        Args:
            model: 사용할 온디바이스 모델 (PHI4_MINI 또는 GEMMA3)
        """
        assert model in (InferenceBackend.PHI4_MINI, InferenceBackend.GEMMA3), \
            "온디바이스 모델은 PHI4_MINI 또는 GEMMA3만 지원"
        self._model = model
        # M1에서 실제 llama_cpp 로드 예정
        # from llama_cpp import Llama
        # self._llm = Llama(model_path=self._get_model_path())

    def infer(self, text: str, context: Optional[dict] = None) -> IntentObject:
        """M0: stub. NotImplementedError 발생."""
        raise NotImplementedError(
            f"{self._model.value} 백엔드는 M1에서 구현 예정입니다."
        )

    def is_available(self) -> bool:
        """M0에서는 항상 False (미구현)."""
        return False

    @property
    def backend_type(self) -> InferenceBackend:
        return self._model


# ─────────────────────────────────────────────
#  섹션 7: IntentParser (메인 파서 클래스)
# ─────────────────────────────────────────────

class IntentParser:
    """
    AI-OS Intent 파서 — 사용자 자연어를 IntentObject로 변환.

    ## 라우팅 전략 (M0)
    1. 온디바이스 백엔드가 사용 가능하면 우선 사용
    2. 불가능하면 클라우드(Claude API) 백엔드 시도
    3. 둘 다 불가능하면 규칙 기반 폴백 사용

    ## 라우팅 전략 (M1 목표)
    - 짧은 단순 명령 (< 20 tokens): 온디바이스 모델
    - 복잡한 복합 명령: Claude API
    - 오프라인/저전력 모드: 규칙 기반 전용

    ## 사용 예시
    ```python
    parser = IntentParser()
    intent = parser.parse("메모리 사용량 보여줘")
    print(intent.intent_type)  # IntentType.QUERY
    print(intent.target)       # ResourceTarget.MEMORY
    print(intent.confidence)   # 0.75
    ```
    """

    def __init__(
        self,
        backends: Optional[list[InferenceBackendInterface]] = None,
        confidence_threshold: float = 0.7,
    ):
        """
        IntentParser 초기화.

        Args:
            backends: 사용할 추론 백엔드 목록 (우선순위 순).
                     None이면 기본값: [OnDeviceBackend, ClaudeApiBackend, RuleBasedBackend]
            confidence_threshold: 이 이상의 신뢰도일 때 결과를 확정으로 처리.
                                  미만이면 로그 경고 발생.
        """
        if backends is None:
            # 기본 백엔드 체인: 온디바이스 → 클라우드 → 규칙 기반
            self._backends: list[InferenceBackendInterface] = [
                OnDeviceBackend(InferenceBackend.PHI4_MINI),
                ClaudeApiBackend(),
                RuleBasedBackend(),
            ]
        else:
            self._backends = backends

        self._confidence_threshold = confidence_threshold
        # 파싱 통계 (디버깅 및 모니터링용)
        self._stats: dict[str, int] = {
            "total_parsed": 0,
            "low_confidence": 0,
            "unknown": 0,
        }

    def parse(self, text: str, context: Optional[dict] = None) -> IntentObject:
        """
        자연어 텍스트를 IntentObject로 변환.

        알고리즘 (Chain of Responsibility 패턴):
            백엔드 목록을 순서대로 시도하여
            사용 가능한 첫 번째 백엔드로 추론.
            모든 백엔드 실패 시 UNKNOWN 반환.

        참조:
            GoF Chain of Responsibility:
            https://refactoring.guru/design-patterns/chain-of-responsibility

        Args:
            text:    사용자 입력 자연어 텍스트
            context: 이전 대화 컨텍스트 딕셔너리 (Context Manager에서 전달)

        Returns:
            IntentObject: 추론된 의도. 실패 시 UNKNOWN 타입.

        Raises:
            ValueError: text가 빈 문자열인 경우
        """
        if not text or not text.strip():
            raise ValueError("입력 텍스트가 비어 있습니다.")

        self._stats["total_parsed"] += 1

        # 백엔드 체인을 순서대로 시도
        last_error: Optional[Exception] = None
        for backend in self._backends:
            if not backend.is_available():
                continue  # 사용 불가 백엔드 건너뜀

            try:
                intent = backend.infer(text, context)
                # 낮은 신뢰도 경고 (다음 백엔드 시도)
                if not intent.is_confident(self._confidence_threshold):
                    self._stats["low_confidence"] += 1
                    # TODO M1: 낮은 신뢰도면 다음 백엔드 시도 로직 추가
                return intent
            except NotImplementedError:
                continue  # Stub 백엔드 건너뜀
            except Exception as e:
                last_error = e
                continue  # 에러 발생 시 다음 백엔드 시도

        # 모든 백엔드 실패: UNKNOWN 반환
        self._stats["unknown"] += 1
        return IntentObject.unknown(text)

    def parse_batch(
        self, texts: list[str], context: Optional[dict] = None
    ) -> list[IntentObject]:
        """
        여러 텍스트를 일괄 파싱.

        M1에서 배치 추론으로 최적화 예정.

        Args:
            texts:   파싱할 텍스트 목록
            context: 공유 컨텍스트

        Returns:
            list[IntentObject]: 각 텍스트에 대한 IntentObject 목록
        """
        return [self.parse(text, context) for text in texts]

    @property
    def stats(self) -> dict[str, int]:
        """파싱 통계 반환 (읽기 전용)."""
        return dict(self._stats)

    def reset_stats(self) -> None:
        """파싱 통계 초기화."""
        for key in self._stats:
            self._stats[key] = 0


# ─────────────────────────────────────────────
#  섹션 8: 단위 테스트
# ─────────────────────────────────────────────

if __name__ == "__main__":
    """
    모듈 직접 실행 시 기본 동작 확인.
    실제 테스트는 tests/test_parser.py에서 pytest로 실행.
    """
    import json

    print("=" * 60)
    print("AI-OS IntentParser 동작 확인")
    print("=" * 60)

    # 규칙 기반 파서만 사용 (M0 기본)
    parser = IntentParser(backends=[RuleBasedBackend()])

    # 테스트 케이스
    test_cases = [
        "메모리 사용량 보여줘",
        "음악 틀어줘",
        "볼륨 50으로 줄여줘",
        "파일 정리해줘",
        "CPU 상태 확인해줘",
        "abcxyz 알 수 없는 입력",
    ]

    for text in test_cases:
        intent = parser.parse(text)
        print(f"\n입력: '{text}'")
        print(f"  → 의도: {intent.intent_type.name} / {intent.action.name} / {intent.target.name}")
        print(f"  → 신뢰도: {intent.confidence:.2f} | 백엔드: {intent.backend.value}")
        if intent.params:
            print(f"  → 파라미터: {intent.params}")

    print(f"\n통계: {json.dumps(parser.stats, ensure_ascii=False, indent=2)}")
