# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

"""
hal_codegen.py
==============
IntentObject → HAL 명령 시퀀스 생성기.

## 역할
IntentParser가 생성한 IntentObject를 받아
Rust ai-hal이 실행 가능한 HalCommandModel 목록을 생성.

## 설계 원칙
- 각 IntentType마다 전용 핸들러 메서드 (단일 책임 원칙)
- 핸들러는 `_generate_<intent_type>()` 형태로 명명
- 생성된 명령은 실행 순서가 보장된 리스트로 반환
- UNKNOWN 의도는 항상 빈 목록 반환

## 파라미터 기본값
- 메모리 할당: size_bytes=4096, alignment=4096, shared=False
- 파일 경로: "/tmp/aios_default" (parameters에 path가 없을 경우)
- CPU 힌트: priority=0, preferred_core=None

## 참조
- Rust HalCommand 열거형: crates/ai-hal/src/lib.rs
- HalCommandType → Rust 변형 1:1 대응 (models.py 참조)
"""

from __future__ import annotations

from typing import Any

from .models import (
    HalCommandModel,
    HalCommandType,
    IntentObject,
    IntentType,
    ResourceType,
)


# 메모리 할당 기본 크기 (4 KiB)
_DEFAULT_ALLOC_SIZE: int = 4096

# 메모리 정렬 기본값 (페이지 정렬, 4 KiB)
_DEFAULT_ALIGNMENT: int = 4096

# 파일 경로 기본값 (parameters에 path 키가 없을 때 사용)
_DEFAULT_FILE_PATH: str = "/tmp/aios_default"


class HalCommandGenerator:
    """IntentObject를 HAL 명령 시퀀스로 변환하는 코드 생성기.

    각 IntentType마다 전용 핸들러가 존재하며,
    `generate()` 메서드가 의도 타입에 따라 핸들러를 디스패치.

    ## 명령 수 기준
    - QUERY_STATE   : 명령 1개 (QueryState)
    - ALLOCATE_MEMORY: 명령 2개 (QueryState 선행 확인 + AllocateMemory)
    - FREE_MEMORY   : 명령 1개 (FreeMemory)
    - CPU_HINT      : 명령 2개 (QueryState 선행 확인 + CpuSchedulingHint)
    - OPEN_FILE     : 명령 2개 (QueryState 선행 확인 + OpenStorageRead)
    - WRITE_FILE    : 명령 2개 (QueryState 선행 확인 + OpenStorageWrite)
    - REGISTER_SKILL: 명령 1개 (RegisterSkill)
    - UNKNOWN       : 명령 0개

    알고리즘 참조:
        Command 패턴: GoF Design Patterns (1994), p.233
    """

    def generate(self, intent: IntentObject) -> list[HalCommandModel]:
        """IntentObject에서 HAL 명령 목록 생성.

        의도 타입에 따라 적절한 핸들러 메서드를 호출.
        UNKNOWN 의도는 항상 빈 목록 반환 (안전 기본값).

        Args:
            intent: InferenceRouter가 생성한 의도 객체

        Returns:
            list[HalCommandModel]: 순서 보장된 HAL 명령 목록.
                실행 불가 의도(UNKNOWN)는 빈 목록.
        """
        # 의도 타입 → 핸들러 디스패치 테이블
        # 새 IntentType 추가 시 이 테이블에 핸들러 등록 필요
        _dispatch: dict[IntentType, Any] = {
            IntentType.QUERY_STATE:    self._generate_query_state,
            IntentType.ALLOCATE_MEMORY: self._generate_allocate_memory,
            IntentType.FREE_MEMORY:    self._generate_free_memory,
            IntentType.CPU_HINT:       self._generate_cpu_hint,
            IntentType.OPEN_FILE:      self._generate_open_file,
            IntentType.WRITE_FILE:     self._generate_write_file,
            IntentType.REGISTER_SKILL: self._generate_register_skill,
            IntentType.UNKNOWN:        lambda _: [],
        }

        handler = _dispatch.get(intent.intent_type, lambda _: [])
        return handler(intent)

    # ── 의도별 핸들러 ──────────────────────────────────────

    def _generate_query_state(self, intent: IntentObject) -> list[HalCommandModel]:
        """QUERY_STATE 의도 → QueryState 명령 1개 생성.

        Rust HalCommand::QueryState { resource, detailed } 에 대응.

        Args:
            intent: QUERY_STATE 타입의 IntentObject

        Returns:
            list[HalCommandModel]: [QueryState]
        """
        return [
            HalCommandModel(
                command_type=HalCommandType.QUERY_STATE,
                parameters={
                    # 조회 대상 리소스 (Rust ResourceType 값)
                    "resource": intent.resource_type.value,
                    # 상세 조회 여부 (parameters에 "detailed" 키가 있으면 사용)
                    "detailed": intent.parameters.get("detailed", False),
                },
                requires_capability=intent.resource_type,
                # 조회 명령은 최고 우선순위
                priority=0,
            )
        ]

    def _generate_allocate_memory(self, intent: IntentObject) -> list[HalCommandModel]:
        """ALLOCATE_MEMORY 의도 → [QueryState, AllocateMemory] 명령 2개 생성.

        AllocateMemory 전에 메모리 상태를 먼저 확인하여
        사용 가능한 공간이 있는지 선행 검증.

        Rust HalCommand::AllocateMemory { size_bytes, alignment, shared } 에 대응.

        Args:
            intent: ALLOCATE_MEMORY 타입의 IntentObject
                parameters 예: {"size_bytes": 4096, "alignment": 4096, "shared": false}

        Returns:
            list[HalCommandModel]: [QueryState(Memory), AllocateMemory]
        """
        size_bytes: int = intent.parameters.get("size_bytes", _DEFAULT_ALLOC_SIZE)
        alignment: int = intent.parameters.get("alignment", _DEFAULT_ALIGNMENT)
        shared: bool = intent.parameters.get("shared", False)

        return [
            # 1. 메모리 상태 선행 확인
            HalCommandModel(
                command_type=HalCommandType.QUERY_STATE,
                parameters={
                    "resource": ResourceType.MEMORY.value,
                    "detailed": True,
                },
                requires_capability=ResourceType.MEMORY,
                priority=0,
            ),
            # 2. 실제 메모리 할당 요청
            HalCommandModel(
                command_type=HalCommandType.ALLOCATE_MEMORY,
                parameters={
                    "size_bytes": size_bytes,
                    "alignment": alignment,
                    "shared": shared,
                },
                requires_capability=ResourceType.MEMORY,
                priority=1,
            ),
        ]

    def _generate_free_memory(self, intent: IntentObject) -> list[HalCommandModel]:
        """FREE_MEMORY 의도 → FreeMemory 명령 1개 생성.

        Rust HalCommand::FreeMemory { handle_id } 에 대응.

        Args:
            intent: FREE_MEMORY 타입의 IntentObject
                parameters 예: {"handle_id": 42}

        Returns:
            list[HalCommandModel]: [FreeMemory]
        """
        handle_id: int = intent.parameters.get("handle_id", 0)

        return [
            HalCommandModel(
                command_type=HalCommandType.FREE_MEMORY,
                parameters={
                    "handle_id": handle_id,
                },
                requires_capability=ResourceType.MEMORY,
                priority=0,
            )
        ]

    def _generate_cpu_hint(self, intent: IntentObject) -> list[HalCommandModel]:
        """CPU_HINT 의도 → [QueryState, CpuSchedulingHint] 명령 2개 생성.

        CPU 스케줄링 힌트를 설정하기 전에 CPU 상태를 먼저 확인.

        Rust HalCommand::CpuSchedulingHint { pid, priority, preferred_core } 에 대응.

        Args:
            intent: CPU_HINT 타입의 IntentObject
                parameters 예: {"pid": 1234, "priority": 5, "preferred_core": 2}

        Returns:
            list[HalCommandModel]: [QueryState(Cpu), CpuSchedulingHint]
        """
        pid: int = intent.parameters.get("pid", 0)
        priority: int = intent.parameters.get("priority", 0)
        preferred_core: int | None = intent.parameters.get("preferred_core", None)

        return [
            # 1. CPU 상태 선행 확인
            HalCommandModel(
                command_type=HalCommandType.QUERY_STATE,
                parameters={
                    "resource": ResourceType.CPU.value,
                    "detailed": False,
                },
                requires_capability=ResourceType.CPU,
                priority=0,
            ),
            # 2. CPU 스케줄링 힌트 설정
            HalCommandModel(
                command_type=HalCommandType.CPU_SCHEDULING_HINT,
                parameters={
                    "pid": pid,
                    "priority": priority,
                    # preferred_core가 None이면 OS가 자동 선택
                    "preferred_core": preferred_core,
                },
                requires_capability=ResourceType.CPU,
                priority=1,
            ),
        ]

    def _generate_open_file(self, intent: IntentObject) -> list[HalCommandModel]:
        """OPEN_FILE 의도 → [QueryState, OpenStorageRead] 명령 2개 생성.

        파일을 열기 전에 저장소 상태를 먼저 확인.

        Rust HalCommand::OpenStorageRead { path } 에 대응.

        Args:
            intent: OPEN_FILE 타입의 IntentObject
                parameters 예: {"path": "/home/user/doc.txt"}

        Returns:
            list[HalCommandModel]: [QueryState(Storage), OpenStorageRead]
        """
        path: str = intent.parameters.get("path", _DEFAULT_FILE_PATH)

        return [
            # 1. 저장소 상태 선행 확인
            HalCommandModel(
                command_type=HalCommandType.QUERY_STATE,
                parameters={
                    "resource": ResourceType.STORAGE.value,
                    "detailed": False,
                },
                requires_capability=ResourceType.STORAGE,
                priority=0,
            ),
            # 2. 저장소 읽기 열기 요청
            HalCommandModel(
                command_type=HalCommandType.OPEN_STORAGE_READ,
                parameters={
                    "path": path,
                },
                requires_capability=ResourceType.STORAGE,
                priority=1,
            ),
        ]

    def _generate_write_file(self, intent: IntentObject) -> list[HalCommandModel]:
        """WRITE_FILE 의도 → [QueryState, OpenStorageWrite] 명령 2개 생성.

        파일 쓰기 전에 저장소 상태를 먼저 확인.

        Rust HalCommand::OpenStorageWrite { path, create_if_missing } 에 대응.

        Args:
            intent: WRITE_FILE 타입의 IntentObject
                parameters 예: {"path": "/tmp/out.txt", "create_if_missing": true}

        Returns:
            list[HalCommandModel]: [QueryState(Storage), OpenStorageWrite]
        """
        path: str = intent.parameters.get("path", _DEFAULT_FILE_PATH)
        create_if_missing: bool = intent.parameters.get("create_if_missing", True)

        return [
            # 1. 저장소 상태 선행 확인
            HalCommandModel(
                command_type=HalCommandType.QUERY_STATE,
                parameters={
                    "resource": ResourceType.STORAGE.value,
                    "detailed": False,
                },
                requires_capability=ResourceType.STORAGE,
                priority=0,
            ),
            # 2. 저장소 쓰기 열기 요청
            HalCommandModel(
                command_type=HalCommandType.OPEN_STORAGE_WRITE,
                parameters={
                    "path": path,
                    "create_if_missing": create_if_missing,
                },
                requires_capability=ResourceType.STORAGE,
                priority=1,
            ),
        ]

    def _generate_register_skill(self, intent: IntentObject) -> list[HalCommandModel]:
        """REGISTER_SKILL 의도 → RegisterSkill 명령 1개 생성.

        Rust HalCommand::RegisterSkill { name, version, description, capabilities }
        에 대응.

        Args:
            intent: REGISTER_SKILL 타입의 IntentObject
                parameters 예: {
                    "name": "media-player",
                    "version": "1.0.0",
                    "description": "음악 재생 스킬",
                    "capabilities": ["Audio"]
                }

        Returns:
            list[HalCommandModel]: [RegisterSkill]
        """
        return [
            HalCommandModel(
                command_type=HalCommandType.REGISTER_SKILL,
                parameters={
                    "name": intent.parameters.get("name", "unnamed-skill"),
                    "version": intent.parameters.get("version", "0.0.1"),
                    "description": intent.parameters.get("description", ""),
                    # capabilities: 이 스킬이 필요로 하는 HAL 리소스 목록
                    "capabilities": intent.parameters.get(
                        "capabilities",
                        [intent.resource_type.value],
                    ),
                },
                requires_capability=intent.resource_type,
                priority=0,
            )
        ]
