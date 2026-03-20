# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project

"""
intent_engine 패키지

AI-OS의 자연어 의도 분석 엔진.
사용자 입력을 HAL 명령으로 변환하는 파이프라인.
"""

from .parser import (
    ActionType,
    ClaudeApiBackend,
    InferenceBackend,
    InferenceBackendInterface,
    IntentObject,
    IntentParser,
    IntentType,
    OnDeviceBackend,
    ResourceTarget,
    RuleBasedBackend,
)

__all__ = [
    "IntentParser",
    "IntentObject",
    "IntentType",
    "ActionType",
    "ResourceTarget",
    "InferenceBackend",
    "InferenceBackendInterface",
    "RuleBasedBackend",
    "OnDeviceBackend",
    "ClaudeApiBackend",
]
