# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project

"""
test_models.py
==============
models.py 데이터 클래스 단위 테스트.

## 테스트 대상
- IntentType, ResourceType, InferenceBackend, HalCommandType 열거형
- IntentObject: 생성, to_dict/from_dict, is_confident, unknown 팩토리
- HalCommandModel: 생성, to_dict/from_dict
- ParseResult: 생성, is_executable, to_dict
- RouterConfig: 기본값, 필드 검증

## 실행
```bash
pytest python/intent_engine/tests/test_models.py -v
```
"""

from __future__ import annotations

import time

import pytest

from ..models import (
    HalCommandModel,
    HalCommandType,
    InferenceBackend,
    IntentObject,
    IntentType,
    ParseResult,
    ResourceType,
    RouterConfig,
)


# ─────────────────────────────────────────────
#  열거형 테스트
# ─────────────────────────────────────────────

class TestEnums:
    """열거형 값 및 직렬화 테스트."""

    def test_intent_type_values(self):
        """IntentType 모든 값이 문자열 형태인지 확인."""
        for member in IntentType:
            assert isinstance(member.value, str), (
                f"{member.name}의 value가 str이 아님: {type(member.value)}"
            )

    def test_resource_type_rust_compatible(self):
        """ResourceType 값이 Rust ResourceType 변형 이름과 일치하는지 확인.

        Rust ai_hal::ResourceType과 JSON 직렬화로 통신하므로
        대소문자까지 정확히 일치해야 함.
        """
        # Rust ResourceType 열거형 변형 이름 (PascalCase)
        expected = {
            "Memory", "Cpu", "Storage", "Gpu",
            "Network", "Audio", "Camera", "Display", "Unknown",
        }
        actual = {member.value for member in ResourceType}
        assert actual == expected

    def test_inference_backend_values(self):
        """InferenceBackend 열거형 값이 소문자 snake_case인지 확인."""
        for member in InferenceBackend:
            assert member.value == member.value.lower(), (
                f"{member.name} value가 소문자가 아님: {member.value}"
            )

    def test_hal_command_type_pascal_case(self):
        """HalCommandType 값이 PascalCase인지 확인.

        Rust HalCommand 열거형 변형 이름은 PascalCase.
        """
        for member in HalCommandType:
            value = member.value
            assert value[0].isupper(), (
                f"{member.name} value가 대문자로 시작하지 않음: {value}"
            )


# ─────────────────────────────────────────────
#  IntentObject 테스트
# ─────────────────────────────────────────────

class TestIntentObject:
    """IntentObject 데이터클래스 테스트."""

    def _make_intent(self, **kwargs) -> IntentObject:
        """테스트용 IntentObject 기본 생성 헬퍼."""
        defaults = dict(
            intent_type=IntentType.QUERY_STATE,
            resource_type=ResourceType.MEMORY,
            confidence=0.9,
            raw_input="메모리 상태 조회",
            backend_used=InferenceBackend.RULE_BASED,
        )
        defaults.update(kwargs)
        return IntentObject(**defaults)

    def test_basic_creation(self):
        """기본 생성 및 필드 확인."""
        intent = self._make_intent()
        assert intent.intent_type == IntentType.QUERY_STATE
        assert intent.resource_type == ResourceType.MEMORY
        assert intent.confidence == 0.9
        assert intent.backend_used == InferenceBackend.RULE_BASED

    def test_default_fields(self):
        """기본값 필드 확인."""
        intent = self._make_intent()
        assert intent.parameters == {}
        assert intent.latency_ms == 0.0
        # timestamp는 현재 시각 근처여야 함
        assert abs(intent.timestamp - time.time()) < 5.0

    def test_is_confident_above_threshold(self):
        """신뢰도 임계값 이상일 때 True 반환."""
        intent = self._make_intent(confidence=0.8)
        assert intent.is_confident(threshold=0.6) is True
        assert intent.is_confident(threshold=0.8) is True

    def test_is_confident_below_threshold(self):
        """신뢰도 임계값 미만일 때 False 반환."""
        intent = self._make_intent(confidence=0.5)
        assert intent.is_confident(threshold=0.6) is False

    def test_is_confident_default_threshold(self):
        """기본 임계값(0.6) 동작 확인."""
        assert self._make_intent(confidence=0.6).is_confident() is True
        assert self._make_intent(confidence=0.59).is_confident() is False

    def test_to_dict_keys(self):
        """to_dict() 반환 딕셔너리 키 확인."""
        intent = self._make_intent()
        d = intent.to_dict()
        required_keys = {
            "intent_type", "resource_type", "parameters",
            "confidence", "raw_input", "backend_used",
            "latency_ms", "timestamp",
        }
        assert required_keys.issubset(d.keys())

    def test_to_dict_values_are_serializable(self):
        """to_dict() 반환값이 JSON 직렬화 가능한 타입인지 확인."""
        import json
        intent = self._make_intent(parameters={"size_bytes": 4096})
        # JSON 직렬화 가능하면 예외 없음
        json.dumps(intent.to_dict())

    def test_from_dict_roundtrip(self):
        """to_dict() → from_dict() 라운드트립 동작 확인."""
        original = self._make_intent(
            confidence=0.85,
            parameters={"detailed": True},
        )
        restored = IntentObject.from_dict(original.to_dict())

        assert restored.intent_type == original.intent_type
        assert restored.resource_type == original.resource_type
        assert restored.confidence == original.confidence
        assert restored.parameters == original.parameters
        assert restored.backend_used == original.backend_used

    def test_unknown_factory(self):
        """unknown() 팩토리 메서드 — UNKNOWN 타입, 신뢰도 0."""
        intent = IntentObject.unknown("알 수 없는 입력")
        assert intent.intent_type == IntentType.UNKNOWN
        assert intent.resource_type == ResourceType.UNKNOWN
        assert intent.confidence == 0.0
        assert intent.raw_input == "알 수 없는 입력"
        assert intent.is_confident() is False

    def test_unknown_with_backend(self):
        """unknown() 팩토리에 backend 인자 전달 확인."""
        intent = IntentObject.unknown("x", backend=InferenceBackend.ON_DEVICE)
        assert intent.backend_used == InferenceBackend.ON_DEVICE

    def test_repr_contains_key_info(self):
        """__repr__에 의도 타입, 리소스, 신뢰도가 포함되는지 확인."""
        intent = self._make_intent()
        r = repr(intent)
        assert "query_state" in r
        assert "Memory" in r
        assert "0.90" in r


# ─────────────────────────────────────────────
#  HalCommandModel 테스트
# ─────────────────────────────────────────────

class TestHalCommandModel:
    """HalCommandModel 데이터클래스 테스트."""

    def _make_cmd(self, **kwargs) -> HalCommandModel:
        """테스트용 HalCommandModel 기본 생성 헬퍼."""
        defaults = dict(
            command_type=HalCommandType.QUERY_STATE,
            parameters={"resource": "Memory", "detailed": False},
            requires_capability=ResourceType.MEMORY,
        )
        defaults.update(kwargs)
        return HalCommandModel(**defaults)

    def test_basic_creation(self):
        """기본 생성 및 필드 확인."""
        cmd = self._make_cmd()
        assert cmd.command_type == HalCommandType.QUERY_STATE
        assert cmd.requires_capability == ResourceType.MEMORY
        assert cmd.priority == 0

    def test_to_dict_structure(self):
        """to_dict() 반환 구조 확인 — ai-core-bridge 전송 포맷."""
        cmd = self._make_cmd()
        d = cmd.to_dict()
        assert d["command_type"] == "QueryState"
        assert d["requires_capability"] == "Memory"
        assert isinstance(d["parameters"], dict)
        assert isinstance(d["priority"], int)

    def test_from_dict_roundtrip(self):
        """to_dict() → from_dict() 라운드트립 확인."""
        original = self._make_cmd(
            command_type=HalCommandType.ALLOCATE_MEMORY,
            parameters={"size_bytes": 8192, "alignment": 4096, "shared": False},
            requires_capability=ResourceType.MEMORY,
            priority=1,
        )
        restored = HalCommandModel.from_dict(original.to_dict())
        assert restored.command_type == original.command_type
        assert restored.parameters == original.parameters
        assert restored.priority == original.priority

    def test_default_priority(self):
        """기본 우선순위가 0인지 확인."""
        cmd = self._make_cmd()
        assert cmd.priority == 0

    def test_repr(self):
        """__repr__ 포함 정보 확인."""
        cmd = self._make_cmd()
        r = repr(cmd)
        assert "QueryState" in r


# ─────────────────────────────────────────────
#  ParseResult 테스트
# ─────────────────────────────────────────────

class TestParseResult:
    """ParseResult 데이터클래스 테스트."""

    def _make_intent(self, intent_type=IntentType.QUERY_STATE, confidence=0.9):
        return IntentObject(
            intent_type=intent_type,
            resource_type=ResourceType.MEMORY,
            confidence=confidence,
            raw_input="테스트 입력",
            backend_used=InferenceBackend.RULE_BASED,
        )

    def _make_cmd(self):
        return HalCommandModel(
            command_type=HalCommandType.QUERY_STATE,
            parameters={"resource": "Memory"},
            requires_capability=ResourceType.MEMORY,
        )

    def test_is_executable_true(self):
        """신뢰도 충분 + 명령 있음 → is_executable() True."""
        result = ParseResult(
            intent=self._make_intent(confidence=0.9),
            hal_commands=[self._make_cmd()],
        )
        assert result.is_executable() is True

    def test_is_executable_low_confidence(self):
        """신뢰도 부족 → is_executable() False."""
        result = ParseResult(
            intent=self._make_intent(confidence=0.3),
            hal_commands=[self._make_cmd()],
        )
        assert result.is_executable() is False

    def test_is_executable_no_commands(self):
        """명령 없음 → is_executable() False."""
        result = ParseResult(
            intent=self._make_intent(confidence=0.9),
            hal_commands=[],
        )
        assert result.is_executable() is False

    def test_is_executable_unknown_intent(self):
        """UNKNOWN 의도 → is_executable() False."""
        result = ParseResult(
            intent=self._make_intent(
                intent_type=IntentType.UNKNOWN,
                confidence=0.9,
            ),
            hal_commands=[self._make_cmd()],
        )
        assert result.is_executable() is False

    def test_to_dict_structure(self):
        """to_dict() 반환 구조 확인."""
        result = ParseResult(
            intent=self._make_intent(),
            hal_commands=[self._make_cmd()],
            total_latency_ms=12.5,
            response_text="메모리 상태를 조회합니다.",
        )
        d = result.to_dict()
        assert "intent" in d
        assert "hal_commands" in d
        assert d["total_latency_ms"] == 12.5
        assert d["response_text"] == "메모리 상태를 조회합니다."
        assert len(d["hal_commands"]) == 1


# ─────────────────────────────────────────────
#  RouterConfig 테스트
# ─────────────────────────────────────────────

class TestRouterConfig:
    """RouterConfig 데이터클래스 테스트."""

    def test_default_values(self):
        """기본값 확인."""
        cfg = RouterConfig()
        assert cfg.rule_confidence_threshold == 0.75
        assert cfg.on_device_confidence_threshold == 0.60
        assert cfg.prefer_privacy is False
        assert cfg.context_length == 2048
        assert cfg.max_on_device_latency_ms == 2000.0

    def test_custom_values(self):
        """사용자 지정 값 설정 확인."""
        cfg = RouterConfig(
            rule_confidence_threshold=0.8,
            prefer_privacy=True,
        )
        assert cfg.rule_confidence_threshold == 0.8
        assert cfg.prefer_privacy is True

    def test_cloud_api_key_env_default(self):
        """클라우드 API 키 환경변수 이름 기본값 확인."""
        cfg = RouterConfig()
        assert cfg.cloud_api_key_env == "ANTHROPIC_API_KEY"
