# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.

"""
parser.py
=========
AI-OS Intent Engine 퍼사드(Facade) 파서 모듈.

## 아키텍처 위치
```
User Input (자연어)
    │
    ▼
IntentParser.parse(text)          ← 이 모듈
    │
    ├─ 1. _preprocess(text)       전처리 (정규화)
    ├─ 2. InferenceRouter.route() 추론 라우팅 (규칙/온디바이스/클라우드)
    ├─ 3. HalCommandGenerator     HAL 명령 시퀀스 생성
    └─ 4. ParseResult 조립        최종 결과 패키징
    │
    ▼
ParseResult → ai-core-bridge → Rust ai-hal
```

## 설계 패턴
- **Facade** (GoF): 복잡한 서브시스템(Router, Codegen)을 단일 인터페이스로 노출
- **Chain of Responsibility**: InferenceRouter 내부에서 백엔드 체인 처리

## 참조
- Facade Pattern: GoF Design Patterns (1994), p.185
- Phi-4-mini: https://huggingface.co/microsoft/phi-4-mini-instruct
"""

from __future__ import annotations

import time
import unicodedata
from typing import Any

from .hal_codegen import HalCommandGenerator
from .inference_router import InferenceRouter
from .models import (
    IntentObject,
    IntentType,
    ParseResult,
    RouterConfig,
)


class IntentParser:
    """AI-OS Intent 파서 퍼사드(Facade).

    자연어 입력을 받아 ParseResult(의도 + HAL 명령 목록)를 반환하는
    단일 공개 인터페이스. 내부 복잡도(라우팅, 코드젠)는 완전히 감춤.

    ## 파이프라인
    ```
    parse(text)
        │
        ├─ _preprocess()          전각문자 정규화, 공백 정리
        ├─ InferenceRouter.route() 추론 백엔드 선택 및 실행
        ├─ HalCommandGenerator    IntentObject → HAL 명령 목록
        └─ ParseResult            의도 + 명령 + 응답 텍스트 조립
    ```

    ## 사용 예시
    ```python
    parser = IntentParser()
    result = parser.parse("메모리 상태 조회해줘")
    if result.is_executable():
        bridge.execute(result.hal_commands)
    ```
    """

    def __init__(self, config: RouterConfig | None = None) -> None:
        """IntentParser 초기화.

        Args:
            config: 라우터 설정. None이면 RouterConfig 기본값 사용.
        """
        # 라우터 설정 (기본값 또는 주입된 설정)
        self._config: RouterConfig = config or RouterConfig()

        # 추론 라우터: 규칙 기반 → 온디바이스 → 클라우드 순서로 시도
        self._router: InferenceRouter = InferenceRouter(self._config)

        # HAL 명령 생성기: IntentObject → List[HalCommandModel]
        self._codegen: HalCommandGenerator = HalCommandGenerator()

        # 파이프라인 통계 (디버깅 및 모니터링용)
        self._stats: dict[str, int | float] = {
            "total_parsed": 0,      # 총 파싱 횟수
            "executable": 0,         # 실행 가능 결과 수
            "unknown": 0,            # UNKNOWN 결과 수
            "total_latency_ms": 0.0, # 누적 총 지연 시간
        }

    # ── 공개 메서드 ────────────────────────────────────────

    def parse(self, text: str) -> ParseResult:
        """자연어 텍스트를 ParseResult로 변환.

        4단계 파이프라인:
            1. 전처리: 전각 문자 정규화, 공백 정리
            2. 추론 라우팅: 규칙 기반 → 온디바이스 → 클라우드
            3. HAL 명령 생성: IntentObject → List[HalCommandModel]
            4. 결과 조립: ParseResult 패키징

        Args:
            text: 사용자 자연어 입력 문자열

        Returns:
            ParseResult: 의도 + HAL 명령 목록 + 응답 텍스트

        Raises:
            ValueError: text가 None이거나 빈 문자열인 경우
        """
        # 입력 검증
        if not text or not text.strip():
            raise ValueError("입력 텍스트가 비어 있습니다.")

        pipeline_start = time.perf_counter()

        # ── 1단계: 전처리 ──────────────────────────────────
        processed = self._preprocess(text)

        # ── 2단계: 추론 라우팅 ─────────────────────────────
        # InferenceRouter가 최적의 백엔드를 선택하여 IntentObject 반환
        intent: IntentObject = self._router.route(processed)

        # ── 3단계: HAL 명령 시퀀스 생성 ─────────────────────
        # UNKNOWN 의도는 명령 없음 (빈 목록 반환)
        hal_commands = self._codegen.generate(intent)

        # ── 4단계: ParseResult 조립 ─────────────────────────
        pipeline_end = time.perf_counter()
        total_ms = (pipeline_end - pipeline_start) * 1000.0

        result = ParseResult(
            intent=intent,
            hal_commands=hal_commands,
            total_latency_ms=total_ms,
            response_text=self._build_response_text(intent, len(hal_commands)),
        )

        # 통계 업데이트
        self._stats["total_parsed"] = int(self._stats["total_parsed"]) + 1
        self._stats["total_latency_ms"] = float(self._stats["total_latency_ms"]) + total_ms
        if result.is_executable():
            self._stats["executable"] = int(self._stats["executable"]) + 1
        if intent.intent_type == IntentType.UNKNOWN:
            self._stats["unknown"] = int(self._stats["unknown"]) + 1

        return result

    def parse_batch(self, texts: list[str]) -> list[ParseResult]:
        """여러 텍스트를 순차 파싱하여 결과 목록 반환.

        M1 구현 예정: 온디바이스 배치 추론으로 최적화.
        현재는 단순 순차 처리.

        Args:
            texts: 파싱할 자연어 입력 문자열 목록

        Returns:
            list[ParseResult]: 각 입력에 대한 ParseResult 목록
        """
        return [self.parse(t) for t in texts]

    def get_stats(self) -> dict[str, Any]:
        """파이프라인 통계 반환.

        Returns:
            dict: total_parsed, executable, unknown, total_latency_ms,
                  avg_latency_ms, router_stats 포함
        """
        total = int(self._stats["total_parsed"])
        total_ms = float(self._stats["total_latency_ms"])

        return {
            # 전체 파싱 횟수
            "total_parsed": total,
            # 실행 가능 결과 수 (HAL 명령 포함 + 신뢰도 충분)
            "executable": self._stats["executable"],
            # UNKNOWN 처리 수
            "unknown": self._stats["unknown"],
            # 누적 지연 시간 (ms)
            "total_latency_ms": total_ms,
            # 평균 지연 시간 (ms)
            "avg_latency_ms": (total_ms / total) if total > 0 else 0.0,
            # 라우터 내부 통계 (백엔드별 히트 수)
            "router_stats": self._router.get_stats(),
        }

    def reset_stats(self) -> None:
        """파이프라인 통계 초기화."""
        self._stats = {
            "total_parsed": 0,
            "executable": 0,
            "unknown": 0,
            "total_latency_ms": 0.0,
        }
        self._router.reset_stats()

    # ── 비공개 메서드 ──────────────────────────────────────

    def _preprocess(self, text: str) -> str:
        """입력 텍스트 전처리.

        수행 작업:
            1. 전각 문자(Fullwidth) → 반각 문자(ASCII) 변환
               예: "ＣＰＵ" → "CPU", "１２３" → "123"
            2. 연속 공백 → 단일 공백
            3. 탭·줄바꿈 → 공백
            4. 앞뒤 공백 제거

        유니코드 정규화 참조:
            https://unicode.org/reports/tr15/ (NFKC Normalization)

        Args:
            text: 원본 사용자 입력

        Returns:
            str: 정규화된 텍스트
        """
        # NFKC 정규화: 전각/반각 통일, 합성 문자 분해 후 재결합
        normalized = unicodedata.normalize("NFKC", text)

        # 탭·줄바꿈 → 공백
        normalized = normalized.replace("\t", " ").replace("\n", " ").replace("\r", " ")

        # 연속 공백 → 단일 공백
        while "  " in normalized:
            normalized = normalized.replace("  ", " ")

        return normalized.strip()

    def _build_response_text(self, intent: IntentObject, cmd_count: int) -> str:
        """의도와 명령 수에 따른 사용자 응답 텍스트 생성.

        사용자에게 보여줄 짧은 확인 메시지를 생성.
        UNKNOWN 또는 저신뢰도일 경우 안내 메시지 반환.

        Args:
            intent:    추론된 의도 객체
            cmd_count: 생성된 HAL 명령 수

        Returns:
            str: 사용자 표시용 응답 텍스트
        """
        # 의도 인식 불가 처리
        if intent.intent_type == IntentType.UNKNOWN:
            return (
                f"'{intent.raw_input[:40]}' 명령을 인식하지 못했습니다. "
                "다시 말씀해 주시겠어요?"
            )

        # 저신뢰도 경고
        if not intent.is_confident():
            return (
                f"명령을 이해했지만 확신도가 낮습니다 "
                f"(신뢰도: {intent.confidence:.0%}). "
                "계속 진행할까요?"
            )

        # 의도별 응답 템플릿
        templates: dict[IntentType, str] = {
            IntentType.QUERY_STATE: (
                f"{intent.resource_type.value} 상태를 조회합니다. "
                f"(명령 {cmd_count}개)"
            ),
            IntentType.ALLOCATE_MEMORY: (
                f"메모리 할당을 요청합니다. "
                f"(명령 {cmd_count}개)"
            ),
            IntentType.FREE_MEMORY: (
                f"메모리를 해제합니다. "
                f"(명령 {cmd_count}개)"
            ),
            IntentType.CPU_HINT: (
                f"CPU 스케줄링 힌트를 설정합니다. "
                f"(명령 {cmd_count}개)"
            ),
            IntentType.OPEN_FILE: (
                f"파일을 열기 위한 저장소 접근을 요청합니다. "
                f"(명령 {cmd_count}개)"
            ),
            IntentType.WRITE_FILE: (
                f"파일 쓰기를 위한 저장소 접근을 요청합니다. "
                f"(명령 {cmd_count}개)"
            ),
            IntentType.REGISTER_SKILL: (
                f"새 스킬을 등록합니다. "
                f"(명령 {cmd_count}개)"
            ),
        }

        return templates.get(
            intent.intent_type,
            f"명령을 처리합니다. (명령 {cmd_count}개)",
        )


# ─────────────────────────────────────────────
#  모듈 직접 실행 시 동작 확인
# ─────────────────────────────────────────────

if __name__ == "__main__":
    """모듈 직접 실행 시 기본 동작 확인.

    실제 단위 테스트는 tests/test_parser.py에서 pytest로 실행.
    """
    import json

    print("=" * 60)
    print("AI-OS IntentParser (Facade) 동작 확인")
    print("=" * 60)

    parser = IntentParser()

    test_cases = [
        "메모리 상태 조회해줘",
        "메모리 4096바이트 할당해줘",
        "CPU 코어 2번 힌트 설정해줘",
        "/data/log.txt 파일 열어줘",
        "/tmp/output.txt 파일 써줘",
        "알 수 없는 이상한 입력 xyz",
        # 전각 문자 테스트
        "ＣＰＵ 상태 알려줘",
    ]

    for text in test_cases:
        result = parser.parse(text)
        print(f"\n입력: '{text}'")
        print(f"  → 의도: {result.intent.intent_type.value}")
        print(f"  → 신뢰도: {result.intent.confidence:.2f}")
        print(f"  → 백엔드: {result.intent.backend_used.value}")
        print(f"  → HAL 명령 수: {len(result.hal_commands)}")
        print(f"  → 실행 가능: {result.is_executable()}")
        print(f"  → 응답: {result.response_text}")

    print(f"\n통계: {json.dumps(parser.get_stats(), ensure_ascii=False, indent=2)}")
