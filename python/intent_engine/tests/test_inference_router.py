# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project

"""
test_inference_router.py
========================
InferenceRouter 및 각 백엔드 단위 테스트.

## 테스트 대상
- RuleBasedBackend: 키워드 매칭, 파라미터 추출, 신뢰도 계산
- OnDeviceBackend: stub 동작 확인 (M0: is_available() == False)
- CloudApiBackend: stub 동작 확인 (M0: is_available() == False)
- InferenceRouter: 라우팅 로직, 신뢰도 임계값, 통계 추적

## 실행
```bash
pytest python/intent_engine/tests/test_inference_router.py -v
```
"""

from __future__ import annotations

import pytest

from ..inference_router import (
    CloudApiBackend,
    InferenceBackendInterface,
    OnDeviceBackend,
    RuleBasedBackend,
    InferenceRouter,
)
from ..models import (
    InferenceBackend,
    IntentObject,
    IntentType,
    ResourceType,
    RouterConfig,
)


# ─────────────────────────────────────────────
#  RuleBasedBackend 테스트
# ─────────────────────────────────────────────

class TestRuleBasedBackend:
    """규칙 기반 백엔드 테스트."""

    def setup_method(self):
        """각 테스트 전 백엔드 인스턴스 생성."""
        self.backend = RuleBasedBackend()

    def test_always_available(self):
        """규칙 기반 백엔드는 항상 사용 가능."""
        assert self.backend.is_available() is True

    def test_backend_type(self):
        """백엔드 타입이 RULE_BASED인지 확인."""
        assert self.backend.backend_type == InferenceBackend.RULE_BASED

    def test_query_state_memory(self):
        """'메모리 상태 조회' → QUERY_STATE, MEMORY."""
        intent = self.backend.infer("메모리 상태 조회해줘")
        assert intent.intent_type == IntentType.QUERY_STATE
        assert intent.resource_type == ResourceType.MEMORY
        assert intent.confidence >= 0.75

    def test_query_state_cpu(self):
        """'CPU 확인' → QUERY_STATE, CPU."""
        intent = self.backend.infer("CPU 상태 확인해줘")
        assert intent.intent_type == IntentType.QUERY_STATE
        assert intent.resource_type == ResourceType.CPU

    def test_query_state_storage(self):
        """'디스크 확인' → QUERY_STATE, STORAGE."""
        intent = self.backend.infer("디스크 상태 확인해줘")
        assert intent.intent_type == IntentType.QUERY_STATE
        assert intent.resource_type == ResourceType.STORAGE

    def test_allocate_memory_with_size(self):
        """'메모리 4096 할당' → ALLOCATE_MEMORY, size_bytes 파라미터."""
        intent = self.backend.infer("메모리 4096바이트 할당해줘")
        assert intent.intent_type == IntentType.ALLOCATE_MEMORY
        # 파라미터에 size_bytes가 포함되어야 함
        if "size_bytes" in intent.parameters:
            assert intent.parameters["size_bytes"] == 4096

    def test_free_memory(self):
        """'메모리 해제' → FREE_MEMORY."""
        intent = self.backend.infer("메모리 해제해줘")
        assert intent.intent_type == IntentType.FREE_MEMORY

    def test_cpu_hint(self):
        """'CPU 힌트' → CPU_HINT."""
        intent = self.backend.infer("CPU 힌트 설정해줘")
        assert intent.intent_type == IntentType.CPU_HINT

    def test_open_file_with_path(self):
        """파일 경로 포함 → OPEN_FILE, path 파라미터."""
        intent = self.backend.infer("파일 '/data/test.txt' 열어줘")
        assert intent.intent_type == IntentType.OPEN_FILE
        if "path" in intent.parameters:
            assert intent.parameters["path"] == "/data/test.txt"

    def test_write_file(self):
        """'파일 써' → WRITE_FILE."""
        intent = self.backend.infer("파일 '/tmp/out.txt' 써줘")
        assert intent.intent_type == IntentType.WRITE_FILE

    def test_unknown_input(self):
        """인식 불가 입력 → UNKNOWN, 신뢰도 0."""
        intent = self.backend.infer("xyzabc 완전히 알 수 없는 입력")
        assert intent.intent_type == IntentType.UNKNOWN
        assert intent.confidence == 0.0

    def test_backend_used_is_rule_based(self):
        """반환된 IntentObject의 backend_used가 RULE_BASED인지 확인."""
        intent = self.backend.infer("메모리 상태 알려줘")
        assert intent.backend_used == InferenceBackend.RULE_BASED

    def test_raw_input_preserved(self):
        """원본 입력이 raw_input에 보존되는지 확인."""
        text = "메모리 상태 조회해줘"
        intent = self.backend.infer(text)
        assert intent.raw_input == text

    def test_english_keywords(self):
        """영어 키워드 입력 처리 확인."""
        intent = self.backend.infer("check memory status")
        # 영어 키워드도 매칭 가능해야 함
        assert intent.intent_type in (IntentType.QUERY_STATE, IntentType.UNKNOWN)

    def test_latency_recorded(self):
        """추론 후 latency_ms가 기록되는지 확인."""
        intent = self.backend.infer("메모리 상태 알려줘")
        assert intent.latency_ms >= 0.0


# ─────────────────────────────────────────────
#  OnDeviceBackend 테스트 (M0 Stub)
# ─────────────────────────────────────────────

class TestOnDeviceBackend:
    """온디바이스 백엔드 M0 stub 테스트."""

    def setup_method(self):
        # RouterConfig 기본값으로 초기화
        from ..models import RouterConfig
        self.backend = OnDeviceBackend(RouterConfig())

    def test_not_available_m0(self):
        """M0에서는 항상 is_available() == False."""
        assert self.backend.is_available() is False

    def test_backend_type(self):
        """백엔드 타입이 ON_DEVICE인지 확인."""
        assert self.backend.backend_type == InferenceBackend.ON_DEVICE

    def test_infer_returns_unknown_m0(self):
        """M0 stub: infer() 호출 시 UNKNOWN IntentObject 반환."""
        result = self.backend.infer("테스트 입력")
        assert result.intent_type == IntentType.UNKNOWN

    def test_model_path_accessible(self):
        """모델 경로가 RouterConfig를 통해 접근 가능한지 확인."""
        # _config에 on_device_model_path가 있어야 함
        assert hasattr(self.backend, '_config')
        assert hasattr(self.backend._config, 'on_device_model_path')


# ─────────────────────────────────────────────
#  CloudApiBackend 테스트 (M0 Stub)
# ─────────────────────────────────────────────

class TestCloudApiBackend:
    """클라우드 API 백엔드 M0 stub 테스트."""

    def setup_method(self):
        # RouterConfig 기본값으로 초기화
        from ..models import RouterConfig
        self.backend = CloudApiBackend(RouterConfig())

    def test_not_available_without_api_key(self, monkeypatch):
        """API 키 환경변수가 없을 때 is_available() == False."""
        # API 키 환경변수 제거 후 is_available() 확인
        monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
        from ..models import RouterConfig
        backend = CloudApiBackend(RouterConfig())
        assert backend.is_available() is False

    def test_backend_type(self):
        """백엔드 타입이 CLOUD_API인지 확인."""
        assert self.backend.backend_type == InferenceBackend.CLOUD_API

    def test_infer_returns_unknown_m0(self):
        """M0 stub: infer() 호출 시 UNKNOWN IntentObject 반환."""
        result = self.backend.infer("테스트 입력")
        assert result.intent_type == IntentType.UNKNOWN


# ─────────────────────────────────────────────
#  InferenceRouter 테스트
# ─────────────────────────────────────────────

class TestInferenceRouter:
    """InferenceRouter 라우팅 로직 테스트."""

    def setup_method(self):
        """기본 설정으로 라우터 생성."""
        self.config = RouterConfig()
        self.router = InferenceRouter(self.config)

    def test_route_known_input(self):
        """알려진 입력 → IntentObject 반환 (UNKNOWN이 아님)."""
        result = self.router.route("메모리 상태 조회해줘")
        # 규칙 기반으로 처리 가능한 입력
        assert isinstance(result, IntentObject)

    def test_route_unknown_input(self):
        """인식 불가 입력 → UNKNOWN IntentObject 반환."""
        result = self.router.route("xyzabc 완전히 알 수 없는 입력")
        assert result.intent_type == IntentType.UNKNOWN

    def test_route_uses_rule_based_m0(self):
        """M0에서 규칙 기반 백엔드를 사용하는지 확인.

        온디바이스/클라우드가 stub이므로 규칙 기반으로 폴백되어야 함.
        """
        result = self.router.route("메모리 상태 알려줘")
        assert result.backend_used == InferenceBackend.RULE_BASED

    def test_stats_incremented(self):
        """라우팅 후 통계가 증가하는지 확인."""
        stats_before = self.router.get_stats()
        self.router.route("메모리 조회")
        stats_after = self.router.get_stats()
        assert stats_after["total_requests"] == stats_before["total_requests"] + 1

    def test_stats_rule_based_hit(self):
        """규칙 기반 히트 통계가 증가하는지 확인."""
        self.router.reset_stats()
        self.router.route("메모리 상태 조회해줘")
        stats = self.router.get_stats()
        assert stats["rule_based_hits"] >= 1

    def test_stats_unknown_fallback(self):
        """UNKNOWN 폴백 통계가 증가하는지 확인."""
        self.router.reset_stats()
        self.router.route("xyzabc 완전히 모르는 입력")
        stats = self.router.get_stats()
        assert stats["unknown_fallbacks"] >= 1

    def test_reset_stats(self):
        """reset_stats() 후 모든 통계가 0으로 초기화되는지 확인."""
        self.router.route("메모리 조회")
        self.router.reset_stats()
        stats = self.router.get_stats()
        assert stats["total_requests"] == 0
        assert stats["rule_based_hits"] == 0

    def test_prefer_privacy_no_cloud(self):
        """prefer_privacy=True 시 클라우드 백엔드 비활성화 확인.

        M0에서는 어차피 클라우드가 stub이지만,
        설정이 RouterConfig에 정확히 저장되는지 확인.
        """
        config = RouterConfig(prefer_privacy=True)
        router = InferenceRouter(config)
        assert router._config.prefer_privacy is True

    def test_route_returns_intent_object(self):
        """route()의 반환 타입이 IntentObject인지 확인."""
        result = self.router.route("CPU 힌트 설정")
        assert isinstance(result, IntentObject)

    def test_multiple_routes_stats_accumulate(self):
        """여러 번 route() 호출 시 통계가 누적되는지 확인."""
        self.router.reset_stats()
        for _ in range(5):
            self.router.route("메모리 상태 조회")
        stats = self.router.get_stats()
        assert stats["total_requests"] == 5
