# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

"""
intent_engine
=============
AI-OS Intent Engine 패키지.

자연어 사용자 입력을 구조화된 HAL 명령 시퀀스로 변환하는 파이프라인.

## 공개 API
- `IntentParser`     : 메인 퍼사드 — 자연어 → ParseResult
- `ParseResult`      : 파싱 결과 컨테이너 (의도 + HAL 명령 + 메타데이터)
- `IntentObject`     : 구조화된 의도 표현 객체
- `HalCommandModel`  : 단일 HAL 명령 표현 모델
- `RouterConfig`     : 추론 라우터 설정 데이터클래스
- `IntentType`       : 의도 분류 열거형
- `ResourceType`     : 하드웨어 리소스 분류 열거형
- `InferenceBackend` : 추론 백엔드 종류 열거형
- `HalCommandType`   : HAL 명령 종류 열거형

## 사용 예시
```python
from intent_engine import IntentParser, RouterConfig

config = RouterConfig(prefer_privacy=True)
parser = IntentParser(config=config)

result = parser.parse("메모리 상태 조회해줘")
if result.is_executable():
    # ai-core-bridge에 전달
    bridge.execute(result.to_dict())
```
"""

from .hal_codegen import HalCommandGenerator
from .inference_router import InferenceRouter
from .models import (
    HalCommandModel,
    HalCommandType,
    InferenceBackend,
    IntentObject,
    IntentType,
    ParseResult,
    ResourceType,
    RouterConfig,
)
from .parser import IntentParser

# 패키지 공개 API 명시
__all__ = [
    # 핵심 진입점
    "IntentParser",
    # 데이터 모델
    "IntentObject",
    "HalCommandModel",
    "ParseResult",
    "RouterConfig",
    # 열거형
    "IntentType",
    "ResourceType",
    "InferenceBackend",
    "HalCommandType",
    # 내부 컴포넌트 (고급 사용자용)
    "InferenceRouter",
    "HalCommandGenerator",
]

# 패키지 버전 (M0)
__version__ = "0.1.0"
