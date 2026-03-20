# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project

"""
test_hal_codegen.py
===================
HalCommandGenerator 단위 테스트.

## 테스트 대상
- 각 IntentType별 명령 생성 핸들러
- 명령 수, 타입, 파라미터 검증
- UNKNOWN 의도 → 빈 목록
- 기본값 파라미터 처리

## 실행
```bash
pytest python/intent_engine/tests/test_hal_codegen.py -v
```
"""

from __future__ import annotations

import pytest

from ..hal_codegen import HalCommandGenerator
from ..models import (
    HalCommandModel,
    HalCommandType,
    InferenceBackend,
    IntentObject,
    IntentType,
    ResourceType,
)


def make_intent(
    intent_type: IntentType,
    resource_type: ResourceType = ResourceType.MEMORY,
    confidence: float = 0.9,
    parameters: dict | None = None,
) -> IntentObject:
    """테스트용 IntentObject 생성 헬퍼."""
    return IntentObject(
        intent_type=intent_type,
        resource_type=resource_type,
        confidence=confidence,
        raw_input="테스트 입력",
        backend_used=InferenceBackend.RULE_BASED,
        parameters=parameters or {},
    )


class TestHalCommandGenerator:
    """HalCommandGenerator 전체 테스트."""

    def setup_method(self):
        """각 테스트 전 생성기 인스턴스 생성."""
        self.gen = HalCommandGenerator()

    # ── QUERY_STATE ────────────────────────────

    def test_query_state_generates_one_command(self):
        """QUERY_STATE → 명령 1개 생성."""
        intent = make_intent(IntentType.QUERY_STATE, ResourceType.MEMORY)
        cmds = self.gen.generate(intent)
        assert len(cmds) == 1

    def test_query_state_command_type(self):
        """QUERY_STATE 명령 타입이 QUERY_STATE인지 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.QUERY_STATE, ResourceType.CPU)
        )
        assert cmds[0].command_type == HalCommandType.QUERY_STATE

    def test_query_state_resource_in_params(self):
        """QUERY_STATE 파라미터에 resource 키 포함 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.QUERY_STATE, ResourceType.STORAGE)
        )
        assert "resource" in cmds[0].parameters
        assert cmds[0].parameters["resource"] == "Storage"

    def test_query_state_detailed_default_false(self):
        """QUERY_STATE 기본 detailed=False 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.QUERY_STATE)
        )
        assert cmds[0].parameters.get("detailed") is False

    def test_query_state_detailed_override(self):
        """QUERY_STATE parameters에 detailed=True 전달 시 적용 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.QUERY_STATE, parameters={"detailed": True})
        )
        assert cmds[0].parameters["detailed"] is True

    # ── ALLOCATE_MEMORY ────────────────────────

    def test_allocate_memory_generates_two_commands(self):
        """ALLOCATE_MEMORY → 명령 2개 생성 (QueryState + AllocateMemory)."""
        cmds = self.gen.generate(make_intent(IntentType.ALLOCATE_MEMORY))
        assert len(cmds) == 2

    def test_allocate_memory_first_command_is_query(self):
        """AllocateMemory 첫 번째 명령이 선행 QueryState인지 확인."""
        cmds = self.gen.generate(make_intent(IntentType.ALLOCATE_MEMORY))
        assert cmds[0].command_type == HalCommandType.QUERY_STATE

    def test_allocate_memory_second_command_is_allocate(self):
        """AllocateMemory 두 번째 명령이 AllocateMemory인지 확인."""
        cmds = self.gen.generate(make_intent(IntentType.ALLOCATE_MEMORY))
        assert cmds[1].command_type == HalCommandType.ALLOCATE_MEMORY

    def test_allocate_memory_default_size(self):
        """파라미터 없을 때 기본 size_bytes=4096 사용 확인."""
        cmds = self.gen.generate(make_intent(IntentType.ALLOCATE_MEMORY))
        alloc_cmd = cmds[1]
        assert alloc_cmd.parameters["size_bytes"] == 4096

    def test_allocate_memory_custom_size(self):
        """사용자 지정 size_bytes 파라미터 전달 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.ALLOCATE_MEMORY, parameters={"size_bytes": 8192})
        )
        assert cmds[1].parameters["size_bytes"] == 8192

    def test_allocate_memory_shared_flag(self):
        """shared 파라미터 전달 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.ALLOCATE_MEMORY, parameters={"shared": True})
        )
        assert cmds[1].parameters["shared"] is True

    # ── FREE_MEMORY ────────────────────────────

    def test_free_memory_generates_one_command(self):
        """FREE_MEMORY → 명령 1개 생성."""
        cmds = self.gen.generate(make_intent(IntentType.FREE_MEMORY))
        assert len(cmds) == 1

    def test_free_memory_command_type(self):
        """FREE_MEMORY 명령 타입 확인."""
        cmds = self.gen.generate(make_intent(IntentType.FREE_MEMORY))
        assert cmds[0].command_type == HalCommandType.FREE_MEMORY

    def test_free_memory_handle_id_default(self):
        """파라미터 없을 때 handle_id=0 기본값 확인."""
        cmds = self.gen.generate(make_intent(IntentType.FREE_MEMORY))
        assert cmds[0].parameters["handle_id"] == 0

    def test_free_memory_custom_handle(self):
        """사용자 지정 handle_id 전달 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.FREE_MEMORY, parameters={"handle_id": 42})
        )
        assert cmds[0].parameters["handle_id"] == 42

    # ── CPU_HINT ───────────────────────────────

    def test_cpu_hint_generates_two_commands(self):
        """CPU_HINT → 명령 2개 생성 (QueryState + CpuSchedulingHint)."""
        cmds = self.gen.generate(
            make_intent(IntentType.CPU_HINT, ResourceType.CPU)
        )
        assert len(cmds) == 2

    def test_cpu_hint_first_command_is_query(self):
        """CPU_HINT 첫 번째 명령이 QueryState인지 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.CPU_HINT, ResourceType.CPU)
        )
        assert cmds[0].command_type == HalCommandType.QUERY_STATE

    def test_cpu_hint_second_command_type(self):
        """CPU_HINT 두 번째 명령이 CpuSchedulingHint인지 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.CPU_HINT, ResourceType.CPU)
        )
        assert cmds[1].command_type == HalCommandType.CPU_SCHEDULING_HINT

    def test_cpu_hint_preferred_core(self):
        """preferred_core 파라미터 전달 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.CPU_HINT, ResourceType.CPU,
                        parameters={"preferred_core": 3})
        )
        assert cmds[1].parameters["preferred_core"] == 3

    def test_cpu_hint_preferred_core_default_none(self):
        """파라미터 없을 때 preferred_core=None 기본값 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.CPU_HINT, ResourceType.CPU)
        )
        assert cmds[1].parameters["preferred_core"] is None

    # ── OPEN_FILE ──────────────────────────────

    def test_open_file_generates_two_commands(self):
        """OPEN_FILE → 명령 2개 생성 (QueryState + OpenStorageRead)."""
        cmds = self.gen.generate(
            make_intent(IntentType.OPEN_FILE, ResourceType.STORAGE)
        )
        assert len(cmds) == 2

    def test_open_file_second_command_type(self):
        """OPEN_FILE 두 번째 명령이 OpenStorageRead인지 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.OPEN_FILE, ResourceType.STORAGE)
        )
        assert cmds[1].command_type == HalCommandType.OPEN_STORAGE_READ

    def test_open_file_path_parameter(self):
        """파일 경로 파라미터 전달 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.OPEN_FILE, ResourceType.STORAGE,
                        parameters={"path": "/home/user/doc.txt"})
        )
        assert cmds[1].parameters["path"] == "/home/user/doc.txt"

    def test_open_file_default_path(self):
        """path 파라미터 없을 때 기본 경로 사용 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.OPEN_FILE, ResourceType.STORAGE)
        )
        assert "path" in cmds[1].parameters
        assert len(cmds[1].parameters["path"]) > 0

    # ── WRITE_FILE ─────────────────────────────

    def test_write_file_generates_two_commands(self):
        """WRITE_FILE → 명령 2개 생성 (QueryState + OpenStorageWrite)."""
        cmds = self.gen.generate(
            make_intent(IntentType.WRITE_FILE, ResourceType.STORAGE)
        )
        assert len(cmds) == 2

    def test_write_file_second_command_type(self):
        """WRITE_FILE 두 번째 명령이 OpenStorageWrite인지 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.WRITE_FILE, ResourceType.STORAGE)
        )
        assert cmds[1].command_type == HalCommandType.OPEN_STORAGE_WRITE

    def test_write_file_create_if_missing_default(self):
        """create_if_missing 기본값 True 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.WRITE_FILE, ResourceType.STORAGE)
        )
        assert cmds[1].parameters["create_if_missing"] is True

    # ── REGISTER_SKILL ─────────────────────────

    def test_register_skill_generates_one_command(self):
        """REGISTER_SKILL → 명령 1개 생성."""
        cmds = self.gen.generate(
            make_intent(IntentType.REGISTER_SKILL, parameters={
                "name": "test-skill",
                "version": "1.0.0",
                "description": "테스트 스킬",
            })
        )
        assert len(cmds) == 1

    def test_register_skill_command_type(self):
        """RegisterSkill 명령 타입 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.REGISTER_SKILL)
        )
        assert cmds[0].command_type == HalCommandType.REGISTER_SKILL

    def test_register_skill_parameters(self):
        """RegisterSkill 파라미터 전달 확인."""
        cmds = self.gen.generate(
            make_intent(IntentType.REGISTER_SKILL, parameters={
                "name": "my-skill",
                "version": "2.0.0",
            })
        )
        assert cmds[0].parameters["name"] == "my-skill"
        assert cmds[0].parameters["version"] == "2.0.0"

    # ── UNKNOWN ────────────────────────────────

    def test_unknown_generates_no_commands(self):
        """UNKNOWN 의도 → 빈 명령 목록 반환."""
        cmds = self.gen.generate(IntentObject.unknown("알 수 없는 입력"))
        assert cmds == []

    # ── 공통 검증 ──────────────────────────────

    def test_all_commands_are_hal_command_model(self):
        """생성된 모든 명령이 HalCommandModel 인스턴스인지 확인."""
        intents = [
            make_intent(IntentType.QUERY_STATE),
            make_intent(IntentType.ALLOCATE_MEMORY),
            make_intent(IntentType.CPU_HINT, ResourceType.CPU),
        ]
        for intent in intents:
            cmds = self.gen.generate(intent)
            for cmd in cmds:
                assert isinstance(cmd, HalCommandModel), (
                    f"{intent.intent_type} 명령이 HalCommandModel이 아님: {type(cmd)}"
                )

    def test_commands_have_valid_priority(self):
        """명령의 priority가 0 이상 정수인지 확인."""
        cmds = self.gen.generate(make_intent(IntentType.ALLOCATE_MEMORY))
        for cmd in cmds:
            assert isinstance(cmd.priority, int)
            assert cmd.priority >= 0

    def test_sequential_priority_order(self):
        """다중 명령 시 priority가 순서대로 증가하는지 확인."""
        cmds = self.gen.generate(make_intent(IntentType.ALLOCATE_MEMORY))
        priorities = [cmd.priority for cmd in cmds]
        assert priorities == sorted(priorities), (
            f"priority 순서가 잘못됨: {priorities}"
        )

    def test_requires_capability_matches_resource(self):
        """requires_capability가 의도의 resource_type과 일치하는지 확인."""
        intent = make_intent(IntentType.QUERY_STATE, ResourceType.CPU)
        cmds = self.gen.generate(intent)
        # 모든 명령의 requires_capability가 CPU인지 확인
        for cmd in cmds:
            assert cmd.requires_capability == ResourceType.CPU
