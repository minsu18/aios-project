# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

"""
inference_router.py
===================
AI-OS 추론 경로 결정기 (InferenceRouter).

## 역할
IntentParser의 각 백엔드가 반환한 결과를 받아
최종 추론 경로(Rule-Based / On-Device / Cloud)를 결정.

## 경로 결정 알고리즘
```
사용자 입력
    │
    ▼
[1] RuleBasedBackend.infer()
    ├─ confidence >= rule_threshold → 반환 (즉시)
    └─ confidence < threshold
         │
         ▼
    [2] OnDeviceBackend.infer()  (Phi-4-mini)
         ├─ available? / latency OK?
         │    ├─ confidence >= on_device_threshold → 반환
         │    └─ confidence < threshold
         │         │
         │         ▼ (prefer_privacy=False 이면)
         │    [3] CloudApiBackend.infer()  (Claude API)
         │         └─ 결과 반환
         │
         └─ unavailable / timeout → CloudApi로 폴백
```

## 참조
- Phi-4-mini: https://huggingface.co/microsoft/phi-4-mini-instruct
- Chain of Responsibility: GoF (1994)
- llama.cpp Python 바인딩: https://github.com/abetlen/llama-cpp-python
"""

from __future__ import annotations

import logging
import os
import time
from abc import ABC, abstractmethod
from typing import Optional

from .models import (
    InferenceBackend,
    IntentObject,
    IntentType,
    ResourceType,
    RouterConfig,
)

# 모듈 레벨 로거
logger = logging.getLogger(__name__)


# ─────────────────────────────────────────────
#  추상 백엔드 인터페이스 (Strategy 패턴)
# ─────────────────────────────────────────────

class InferenceBackendInterface(ABC):
    """추론 백엔드 추상 인터페이스.

    각 백엔드(규칙 기반 / 온디바이스 / 클라우드)는 이 인터페이스를
    구현하며, InferenceRouter가 Strategy 패턴으로 동적으로 선택.

    ## 구현 규칙
    1. `infer()` 는 항상 `IntentObject` 를 반환 (실패 시 UNKNOWN)
    2. `is_available()` 은 빠른 사전 검사 (< 1ms)
    3. 예외는 내부에서 처리 후 UNKNOWN IntentObject 반환
    """

    @abstractmethod
    def infer(self, text: str) -> IntentObject:
        """자연어 텍스트에서 의도를 추론.

        Args:
            text: 사용자 입력 자연어 문자열

        Returns:
            IntentObject: 추론 결과 (실패 시 UNKNOWN 타입)
        """
        ...

    @abstractmethod
    def is_available(self) -> bool:
        """이 백엔드가 현재 사용 가능한지 확인.

        Returns:
            True: 즉시 사용 가능
            False: 모델 미로드 / API 키 없음 등
        """
        ...

    @property
    @abstractmethod
    def backend_type(self) -> InferenceBackend:
        """백엔드 종류 식별자."""
        ...


# ─────────────────────────────────────────────
#  규칙 기반 백엔드 (RuleBasedBackend)
# ─────────────────────────────────────────────

class RuleBasedBackend(InferenceBackendInterface):
    """정규식 + 키워드 매칭 기반 빠른 의도 추론 백엔드.

    ## 알고리즘
    1. 입력 텍스트 소문자 변환 + 공백 정규화
    2. `_INTENT_RULES` 테이블에서 키워드 매칭 순회
    3. 매칭된 키워드 수 / 전체 키워드 수 = 기본 신뢰도
    4. 파라미터 추출 (`_extract_parameters`)
    5. 임계값 미달 시 UNKNOWN 반환

    ## 성능 목표
    - 지연: < 1 ms (정규식 없이 순수 문자열 검색)
    - 정확도: 단순 명령에 대해 > 95%
    """

    # ── 의도 규칙 테이블 ──────────────────────────
    # 구조: {IntentType: {"keywords": [...], "resource": ResourceType, "weight": float}}
    _INTENT_RULES: list[dict] = [
        # 메모리 상태 조회
        {
            "intent": IntentType.QUERY_STATE,
            "resource": ResourceType.MEMORY,
            "keywords": ["메모리", "memory", "ram", "상태", "state", "조회", "얼마", "남았"],
            "weight": 1.0,
        },
        # CPU 상태 조회
        {
            "intent": IntentType.QUERY_STATE,
            "resource": ResourceType.CPU,
            "keywords": ["cpu", "프로세서", "processor", "코어", "core", "사용률", "usage",
                         "확인", "상태", "조회"],
            "weight": 1.0,
        },
        # 스토리지 상태 조회
        {
            "intent": IntentType.QUERY_STATE,
            "resource": ResourceType.STORAGE,
            "keywords": ["디스크", "disk", "저장", "storage", "스토리지", "용량", "파일시스템",
                         "확인", "상태", "조회"],
            "weight": 1.0,
        },
        # 메모리 할당
        {
            "intent": IntentType.ALLOCATE_MEMORY,
            "resource": ResourceType.MEMORY,
            "keywords": ["할당", "alloc", "allocate", "mmap", "확보", "reserve"],
            "weight": 1.2,
        },
        # 메모리 해제
        {
            "intent": IntentType.FREE_MEMORY,
            "resource": ResourceType.MEMORY,
            "keywords": ["해제", "free", "release", "munmap", "반환", "deallocate"],
            "weight": 1.2,
        },
        # CPU 힌트
        {
            "intent": IntentType.CPU_HINT,
            "resource": ResourceType.CPU,
            "keywords": ["어피니티", "affinity", "고정", "pin", "스케줄", "schedule",
                         "힌트", "hint", "cpu", "설정", "우선순위"],
            "weight": 1.0,
        },
        # 파일 열기 (읽기)
        {
            "intent": IntentType.OPEN_FILE,
            "resource": ResourceType.STORAGE,
            "keywords": ["열어", "open", "읽어", "read", "불러", "load"],
            "weight": 1.0,
        },
        # 파일 쓰기
        {
            "intent": IntentType.WRITE_FILE,
            "resource": ResourceType.STORAGE,
            "keywords": ["저장", "save", "write", "써", "기록", "쓰기"],
            "weight": 1.0,
        },
    ]

    def infer(self, text: str) -> IntentObject:
        """키워드 매칭으로 의도 추론.

        ## 알고리즘
        1. 텍스트 정규화 (소문자, 공백 제거)
        2. 각 규칙에 대해 매칭 키워드 수 계산
        3. 점수 = (매칭 수 / 전체 키워드 수) × weight
        4. 최고 점수 규칙 선택
        5. 파라미터 추출 후 IntentObject 반환

        Args:
            text: 사용자 입력 문자열

        Returns:
            IntentObject (신뢰도 포함)
        """
        start = time.perf_counter()
        normalized = text.lower().strip()

        best_score = 0.0
        best_rule: dict | None = None

        for rule in self._INTENT_RULES:
            matched = sum(1 for kw in rule["keywords"] if kw in normalized)
            if matched == 0:
                continue
            # 신뢰도 = 매칭 비율 × 가중치 (최대 1.0으로 클리핑)
            score = min(1.0, (matched / len(rule["keywords"])) * rule["weight"] * 2.0)
            if score > best_score:
                best_score = score
                best_rule = rule

        latency_ms = (time.perf_counter() - start) * 1000

        if best_rule is None or best_score < 0.3:
            return IntentObject.unknown(text, InferenceBackend.RULE_BASED)

        params = self._extract_parameters(normalized, best_rule["intent"])

        return IntentObject(
            intent_type=best_rule["intent"],
            resource_type=best_rule["resource"],
            parameters=params,
            confidence=best_score,
            raw_input=text,
            backend_used=InferenceBackend.RULE_BASED,
            latency_ms=latency_ms,
        )

    def _extract_parameters(
        self, text: str, intent: IntentType
    ) -> dict:
        """의도 타입별 파라미터 추출.

        ## 지원 파라미터
        - ALLOCATE_MEMORY: size_bytes (숫자 + 단위 파싱)
        - CPU_HINT: preferred_core (숫자 파싱)
        - OPEN_FILE / WRITE_FILE: path (따옴표 또는 공백으로 추출)
        - QUERY_STATE: detailed (상세 키워드 존재 시 True)

        Args:
            text: 정규화된 입력 문자열
            intent: 파싱된 의도 타입

        Returns:
            파라미터 딕셔너리
        """
        import re

        params: dict = {}

        if intent == IntentType.ALLOCATE_MEMORY:
            # "4096 바이트", "4 kb", "1 mb", "2gb" 등 파싱
            size_pattern = re.search(
                r"(\d+(?:\.\d+)?)\s*(gb|mb|kb|바이트|bytes?|b)\b", text
            )
            if size_pattern:
                num = float(size_pattern.group(1))
                unit = size_pattern.group(2).lower()
                multipliers = {"gb": 2**30, "mb": 2**20, "kb": 2**10,
                               "b": 1, "바이트": 1, "bytes": 1, "byte": 1}
                params["size_bytes"] = int(num * multipliers.get(unit, 1))
            else:
                # 단순 숫자 (바이트 단위로 간주)
                num_match = re.search(r"\b(\d+)\b", text)
                if num_match:
                    params["size_bytes"] = int(num_match.group(1))
                else:
                    params["size_bytes"] = 4096  # 기본값: 1 페이지
            params["alignment"] = 4096
            params["shared"] = "공유" in text or "shared" in text

        elif intent == IntentType.CPU_HINT:
            # "코어 3에 고정", "core 2" 등 파싱
            core_match = re.search(r"(?:코어|core)\s*(\d+)", text)
            if core_match:
                params["preferred_core"] = int(core_match.group(1))
            pid_match = re.search(r"(?:pid|프로세스)\s*(\d+)", text)
            params["pid"] = int(pid_match.group(1)) if pid_match else 0
            params["priority"] = 128  # 기본 우선순위

        elif intent in (IntentType.OPEN_FILE, IntentType.WRITE_FILE):
            # 따옴표 내 경로, 또는 "/" 로 시작하는 경로 파싱
            path_match = re.search(r"[\"'](.*?)[\"']", text) or re.search(
                r"(/[\w./\-_]+)", text
            )
            if path_match:
                params["path"] = path_match.group(1)
            params["create_if_missing"] = intent == IntentType.WRITE_FILE

        elif intent == IntentType.QUERY_STATE:
            params["detailed"] = any(
                kw in text for kw in ["상세", "detail", "verbose", "자세"]
            )

        return params

    def is_available(self) -> bool:
        """규칙 기반 백엔드는 항상 사용 가능."""
        return True

    @property
    def backend_type(self) -> InferenceBackend:
        return InferenceBackend.RULE_BASED


# ─────────────────────────────────────────────
#  온디바이스 백엔드 (Phi-4-mini)
# ─────────────────────────────────────────────

class OnDeviceBackend(InferenceBackendInterface):
    """Phi-4-mini 기반 온디바이스 추론 백엔드.

    ## 구현 상태
    - **M0**: 스텁(stub) 구현 — 모델 로드 없이 UNKNOWN 반환
    - **M1**: llama-cpp-python으로 실제 추론 구현 예정

    ## M1 구현 계획
    1. `llama_cpp.Llama` 로 GGUF 모델 로드 (4-bit 양자화)
    2. 시스템 프롬프트: "다음 사용자 입력의 의도를 JSON으로 분류하시오..."
    3. 출력 JSON 파싱 → IntentObject 변환
    4. 지연 > `max_on_device_latency_ms` 시 타임아웃

    ## 참조
    - Phi-4-mini: https://huggingface.co/microsoft/phi-4-mini-instruct
    - llama-cpp-python: https://github.com/abetlen/llama-cpp-python
    - GGUF 포맷: https://github.com/ggml-org/ggml/blob/master/docs/gguf.md
    """

    # 분류 시스템 프롬프트 (M1에서 사용)
    _SYSTEM_PROMPT = """당신은 AI-OS 의도 분류기입니다.
사용자 입력을 분석하여 다음 JSON 포맷으로만 응답하세요:
{"intent_type": "<타입>", "resource_type": "<리소스>", "confidence": <0.0~1.0>, "parameters": {}}

가능한 intent_type 값: query_state, allocate_memory, free_memory, cpu_hint, open_file, write_file, register_skill, unknown
가능한 resource_type 값: Memory, Cpu, Storage, Gpu, Network, Unknown"""

    def __init__(self, config: RouterConfig) -> None:
        """OnDeviceBackend 초기화.

        Args:
            config: RouterConfig (모델 경로, 컨텍스트 길이 등)
        """
        self._config = config
        # M1에서 llama_cpp.Llama 인스턴스로 교체
        self._model: object | None = None
        self._loaded = False

        logger.info(
            "[OnDeviceBackend] M0 스텁 초기화 완료. "
            "M1에서 Phi-4-mini 로드 예정: %s",
            config.on_device_model_path,
        )

    def _try_load_model(self) -> bool:
        """모델 파일 존재 확인 및 (M1 이후) 실제 로드 시도.

        ## M1 구현 계획
        ```python
        from llama_cpp import Llama
        self._model = Llama(
            model_path=self._config.on_device_model_path,
            n_ctx=self._config.context_length,
            n_gpu_layers=-1,  # GPU 전체 오프로드
        )
        self._loaded = True
        ```

        Returns:
            True: 모델 로드 성공 (M0에서는 항상 False)
        """
        model_path = self._config.on_device_model_path
        if not os.path.exists(model_path):
            logger.debug("[OnDeviceBackend] 모델 파일 없음: %s", model_path)
            return False

        # M0: 파일 존재해도 실제 로드하지 않음 (의존성 없음)
        logger.info("[OnDeviceBackend] 모델 파일 발견 (M1에서 로드 예정): %s", model_path)
        return False  # M1에서 True로 변경

    def infer(self, text: str) -> IntentObject:
        """Phi-4-mini로 의도 추론 (M0: 스텁).

        ## M0 동작
        - 항상 UNKNOWN IntentObject 반환 (신뢰도 0.0)
        - 실제 추론 없음

        ## M1 동작 계획
        1. 모델이 로드되지 않았으면 `_try_load_model()` 재시도
        2. 시스템 프롬프트 + 사용자 입력으로 추론 요청
        3. JSON 응답 파싱 → IntentObject 변환
        4. 타임아웃 초과 시 UNKNOWN 반환

        Args:
            text: 사용자 입력 자연어 문자열

        Returns:
            IntentObject (M0: 항상 UNKNOWN)
        """
        # M0: 스텁 — 실제 추론 미구현
        # TODO(M1): Phi-4-mini llama_cpp 추론 구현
        logger.debug("[OnDeviceBackend] M0 스텁 호출 — UNKNOWN 반환. 입력: %r", text[:50])
        return IntentObject.unknown(text, InferenceBackend.ON_DEVICE)

    def is_available(self) -> bool:
        """온디바이스 백엔드 사용 가능 여부.

        M0: 모델 파일 유무 확인만 수행.
        M1: 실제 모델 로드 상태 확인.

        Returns:
            True: 모델 파일 존재 (M0)
        """
        return os.path.exists(self._config.on_device_model_path)

    @property
    def backend_type(self) -> InferenceBackend:
        return InferenceBackend.ON_DEVICE


# ─────────────────────────────────────────────
#  클라우드 API 백엔드 (Claude API)
# ─────────────────────────────────────────────

class CloudApiBackend(InferenceBackendInterface):
    """Claude API 기반 클라우드 추론 백엔드.

    ## 구현 상태
    - **M0**: 스텁 구현 — API 호출 없이 UNKNOWN 반환
    - **M1**: anthropic Python SDK로 실제 Claude API 호출 구현

    ## M1 구현 계획
    ```python
    import anthropic
    client = anthropic.Anthropic(api_key=os.environ[self._config.cloud_api_key_env])
    response = client.messages.create(
        model="claude-haiku-4-5-20251001",  # 저지연 모델 사용
        max_tokens=256,
        system=SYSTEM_PROMPT,
        messages=[{"role": "user", "content": text}]
    )
    # JSON 파싱 → IntentObject 변환
    ```

    ## 참조
    - Anthropic Python SDK: https://github.com/anthropics/anthropic-sdk-python
    - Claude API Docs: https://docs.anthropic.com
    """

    def __init__(self, config: RouterConfig) -> None:
        self._config = config
        self._api_key: str | None = os.environ.get(config.cloud_api_key_env)
        logger.info(
            "[CloudApiBackend] M0 스텁 초기화. API 키 설정 여부: %s",
            "설정됨" if self._api_key else "미설정",
        )

    def infer(self, text: str) -> IntentObject:
        """Claude API로 의도 추론 (M0: 스텁).

        Args:
            text: 사용자 입력 문자열

        Returns:
            IntentObject (M0: 항상 UNKNOWN)
        """
        # M0: 스텁 — 실제 API 호출 미구현
        # TODO(M1): anthropic SDK로 Claude API 호출 구현
        logger.debug("[CloudApiBackend] M0 스텁 호출 — UNKNOWN 반환.")
        return IntentObject.unknown(text, InferenceBackend.CLOUD_API)

    def is_available(self) -> bool:
        """클라우드 API 사용 가능 여부.

        Returns:
            True: API 키가 설정되어 있고 prefer_privacy=False
        """
        return (
            self._api_key is not None
            and not self._config.prefer_privacy
        )

    @property
    def backend_type(self) -> InferenceBackend:
        return InferenceBackend.CLOUD_API


# ─────────────────────────────────────────────
#  추론 라우터 (InferenceRouter)
# ─────────────────────────────────────────────

class InferenceRouter:
    """추론 경로 결정 및 백엔드 체인 실행기.

    ## 역할
    - 백엔드 우선순위 관리 (Rule → OnDevice → Cloud)
    - 신뢰도 기반 에스컬레이션 결정
    - 지연/가용성 기반 폴백 처리
    - 라우팅 통계 수집

    ## 사용 예시
    ```python
    config = RouterConfig(prefer_privacy=True)
    router = InferenceRouter(config)
    intent = router.route("메모리 상태 알려줘")
    print(intent.backend_used, intent.confidence)
    ```

    ## 알고리즘 (Chain of Responsibility)
    1. RuleBasedBackend 시도
       → confidence >= rule_confidence_threshold: 즉시 반환
    2. OnDeviceBackend 시도 (가용 시)
       → confidence >= on_device_confidence_threshold: 반환
    3. CloudApiBackend 시도 (가용 + !prefer_privacy 시)
       → 결과 반환 (신뢰도 무관)
    4. 모두 실패: UNKNOWN 반환

    참조:
    - Chain of Responsibility: GoF (1994), "Design Patterns", p.223
    """

    def __init__(self, config: Optional[RouterConfig] = None) -> None:
        """InferenceRouter 초기화.

        Args:
            config: 라우터 설정 (None이면 기본값 사용)
        """
        self._config = config or RouterConfig()

        # 백엔드 체인 초기화 (우선순위 순서)
        self._rule_backend = RuleBasedBackend()
        self._on_device_backend = OnDeviceBackend(self._config)
        self._cloud_backend = CloudApiBackend(self._config)

        # 라우팅 통계 카운터
        self._stats: dict[str, int] = {
            "rule_based_hits": 0,
            "on_device_hits": 0,
            "cloud_hits": 0,
            "unknown_fallbacks": 0,
            "total_requests": 0,
        }

        logger.info(
            "[InferenceRouter] 초기화 완료. "
            "rule_threshold=%.2f, on_device_threshold=%.2f, privacy=%s",
            self._config.rule_confidence_threshold,
            self._config.on_device_confidence_threshold,
            self._config.prefer_privacy,
        )

    def route(self, text: str) -> IntentObject:
        """텍스트를 분석하여 최적 경로로 의도 추론.

        ## 단계별 처리
        1. 입력 검증 (빈 문자열 → UNKNOWN)
        2. RuleBasedBackend 시도 → 임계값 통과 시 즉시 반환
        3. OnDeviceBackend 시도 → 임계값 통과 시 반환
        4. CloudApiBackend 시도 → 결과 반환
        5. 모두 낮은 신뢰도 → 최선의 결과 반환

        Args:
            text: 사용자 입력 자연어 문자열

        Returns:
            IntentObject: 최종 추론 결과
        """
        self._stats["total_requests"] += 1

        # 입력 검증
        if not text or not text.strip():
            logger.debug("[InferenceRouter] 빈 입력 → UNKNOWN 반환")
            return IntentObject.unknown("", InferenceBackend.RULE_BASED)

        # [단계 1] 규칙 기반 백엔드
        rule_result = self._rule_backend.infer(text)
        if rule_result.confidence >= self._config.rule_confidence_threshold:
            self._stats["rule_based_hits"] += 1
            logger.debug(
                "[InferenceRouter] RuleBased 성공 (confidence=%.2f): %s",
                rule_result.confidence, rule_result.intent_type.value
            )
            return rule_result

        # [단계 2] 온디바이스 백엔드 (Phi-4-mini)
        if self._on_device_backend.is_available():
            on_device_result = self._on_device_backend.infer(text)
            if on_device_result.confidence >= self._config.on_device_confidence_threshold:
                self._stats["on_device_hits"] += 1
                logger.debug(
                    "[InferenceRouter] OnDevice 성공 (confidence=%.2f)",
                    on_device_result.confidence
                )
                return on_device_result
        else:
            logger.debug("[InferenceRouter] OnDevice 백엔드 미가용")

        # [단계 3] 클라우드 API 백엔드
        if self._cloud_backend.is_available():
            cloud_result = self._cloud_backend.infer(text)
            if cloud_result.intent_type != IntentType.UNKNOWN:
                self._stats["cloud_hits"] += 1
                logger.debug("[InferenceRouter] Cloud API 성공")
                return cloud_result
        else:
            logger.debug(
                "[InferenceRouter] Cloud API 미가용 (privacy=%s, api_key=%s)",
                self._config.prefer_privacy,
                bool(os.environ.get(self._config.cloud_api_key_env))
            )

        # [단계 4] 폴백: 규칙 기반 결과라도 반환 (저신뢰도)
        self._stats["unknown_fallbacks"] += 1
        logger.info(
            "[InferenceRouter] 모든 백엔드 임계값 미달 — 최선 결과 반환 "
            "(rule_confidence=%.2f)", rule_result.confidence
        )
        # 규칙 기반 결과가 완전히 UNKNOWN이 아니면 저신뢰도로라도 반환
        return rule_result

    def get_stats(self) -> dict[str, int | float]:
        """라우팅 통계 반환.

        Returns:
            각 백엔드 히트 수, 총 요청 수, 히트율 포함 딕셔너리
        """
        total = max(1, self._stats["total_requests"])
        return {
            **self._stats,
            "rule_based_rate": self._stats["rule_based_hits"] / total,
            "on_device_rate": self._stats["on_device_hits"] / total,
            "cloud_rate": self._stats["cloud_hits"] / total,
            "unknown_rate": self._stats["unknown_fallbacks"] / total,
        }

    def reset_stats(self) -> None:
        """통계 카운터 초기화."""
        for key in self._stats:
            self._stats[key] = 0

    @property
    def config(self) -> RouterConfig:
        """현재 라우터 설정 반환."""
        return self._config
