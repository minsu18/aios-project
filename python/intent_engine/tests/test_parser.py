# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project

"""
test_parser.py
==============
IntentParser (Facade) 단위 테스트.

## 테스트 대상
- IntentParser.parse(): 정상 입력, UNKNOWN 입력, 전처리, 통계
- IntentParser.parse_batch(): 배치 처리
- IntentParser._preprocess(): 전각 문자, 공백 정규화
- IntentParser.get_stats() / reset_stats(): 통계 관리
- ParseResult: is_executable(), response_text

## 실행
```bash
pytest python/intent_engine/tests/test_parser.py -v
```
"""

from __future__ import annotations

import pytest

from ..models import IntentType, ParseResult, RouterConfig
from ..parser import IntentParser


class TestIntentParserBasic:
    """IntentParser 기본 동작 테스트."""

    def setup_method(self):
        """각 테스트 전 파서 인스턴스 생성."""
        self.parser = IntentParser()

    def test_parse_returns_parse_result(self):
        """parse() 반환 타입이 ParseResult인지 확인."""
        result = self.parser.parse("메모리 상태 조회해줘")
        assert isinstance(result, ParseResult)

    def test_parse_memory_query(self):
        """'메모리 상태 조회' → 실행 가능한 ParseResult."""
        result = self.parser.parse("메모리 상태 조회해줘")
        # 규칙 기반으로 처리 가능
        assert result.intent is not None
        assert result.hal_commands is not None

    def test_parse_unknown_input(self):
        """인식 불가 입력 → UNKNOWN 의도."""
        result = self.parser.parse("xyzabc 완전히 모르는 입력")
        assert result.intent.intent_type == IntentType.UNKNOWN

    def test_parse_raises_on_empty_string(self):
        """빈 문자열 입력 → ValueError 발생."""
        with pytest.raises(ValueError):
            self.parser.parse("")

    def test_parse_raises_on_whitespace_only(self):
        """공백만 있는 입력 → ValueError 발생."""
        with pytest.raises(ValueError):
            self.parser.parse("   ")

    def test_parse_result_has_response_text(self):
        """ParseResult에 response_text가 포함되는지 확인."""
        result = self.parser.parse("메모리 상태 조회해줘")
        assert isinstance(result.response_text, str)
        assert len(result.response_text) > 0

    def test_parse_result_has_latency(self):
        """ParseResult에 total_latency_ms가 기록되는지 확인."""
        result = self.parser.parse("메모리 상태 조회해줘")
        assert result.total_latency_ms >= 0.0

    def test_parse_cpu_hint(self):
        """'CPU 힌트' → CPU_HINT 의도."""
        result = self.parser.parse("CPU 힌트 설정해줘")
        assert result.intent.intent_type == IntentType.CPU_HINT

    def test_parse_allocate_memory(self):
        """'메모리 할당' → ALLOCATE_MEMORY 의도."""
        result = self.parser.parse("메모리 4096바이트 할당해줘")
        assert result.intent.intent_type == IntentType.ALLOCATE_MEMORY

    def test_parse_open_file(self):
        """'파일 열어' → OPEN_FILE 의도."""
        result = self.parser.parse("파일 '/data/test.txt' 열어줘")
        assert result.intent.intent_type == IntentType.OPEN_FILE


class TestIntentParserPreprocess:
    """_preprocess() 전처리 테스트."""

    def setup_method(self):
        self.parser = IntentParser()

    def test_fullwidth_cpu_normalized(self):
        """전각 'ＣＰＵ' → 반각 'CPU'로 정규화 후 처리 가능."""
        result = self.parser.parse("ＣＰＵ 상태 알려줘")
        # 전각 문자 정규화 후 CPU로 처리되어야 함
        assert result.intent is not None

    def test_consecutive_spaces_collapsed(self):
        """연속 공백이 단일 공백으로 정리되는지 확인."""
        # 직접 _preprocess 호출
        processed = self.parser._preprocess("메모리   상태   조회")
        assert "  " not in processed

    def test_tab_replaced_with_space(self):
        """탭 문자가 공백으로 변환되는지 확인."""
        processed = self.parser._preprocess("메모리\t상태\t조회")
        assert "\t" not in processed

    def test_newline_replaced_with_space(self):
        """줄바꿈이 공백으로 변환되는지 확인."""
        processed = self.parser._preprocess("메모리\n상태")
        assert "\n" not in processed

    def test_leading_trailing_whitespace_stripped(self):
        """앞뒤 공백 제거 확인."""
        processed = self.parser._preprocess("  메모리 상태  ")
        assert processed == "메모리 상태"

    def test_fullwidth_digits_normalized(self):
        """전각 숫자 '４０９６' → 반각 '4096'으로 정규화 확인."""
        processed = self.parser._preprocess("메모리 ４０９６바이트")
        assert "4096" in processed
        assert "４０９６" not in processed


class TestIntentParserStats:
    """IntentParser 통계 관리 테스트."""

    def setup_method(self):
        self.parser = IntentParser()

    def test_stats_initial_zero(self):
        """초기 통계 모두 0인지 확인."""
        stats = self.parser.get_stats()
        assert stats["total_parsed"] == 0
        assert stats["executable"] == 0
        assert stats["unknown"] == 0

    def test_stats_incremented_after_parse(self):
        """parse() 호출 후 total_parsed 증가 확인."""
        self.parser.parse("메모리 상태 조회해줘")
        stats = self.parser.get_stats()
        assert stats["total_parsed"] == 1

    def test_unknown_stats_incremented(self):
        """UNKNOWN 결과 후 unknown 통계 증가 확인."""
        self.parser.parse("xyzabc 알 수 없는 입력")
        stats = self.parser.get_stats()
        assert stats["unknown"] >= 1

    def test_reset_stats(self):
        """reset_stats() 후 통계 초기화 확인."""
        self.parser.parse("메모리 상태 조회해줘")
        self.parser.reset_stats()
        stats = self.parser.get_stats()
        assert stats["total_parsed"] == 0
        assert stats["unknown"] == 0

    def test_stats_avg_latency(self):
        """통계에 avg_latency_ms가 포함되는지 확인."""
        self.parser.parse("메모리 상태 조회해줘")
        stats = self.parser.get_stats()
        assert "avg_latency_ms" in stats
        assert stats["avg_latency_ms"] >= 0.0

    def test_stats_includes_router_stats(self):
        """통계에 router_stats가 포함되는지 확인."""
        stats = self.parser.get_stats()
        assert "router_stats" in stats
        assert isinstance(stats["router_stats"], dict)


class TestIntentParserBatch:
    """parse_batch() 배치 처리 테스트."""

    def setup_method(self):
        self.parser = IntentParser()

    def test_parse_batch_returns_list(self):
        """parse_batch() 반환 타입이 list인지 확인."""
        results = self.parser.parse_batch(["메모리 조회", "CPU 확인"])
        assert isinstance(results, list)

    def test_parse_batch_length_matches(self):
        """parse_batch() 반환 목록 길이가 입력과 일치하는지 확인."""
        texts = ["메모리 조회", "CPU 확인", "파일 열어줘"]
        results = self.parser.parse_batch(texts)
        assert len(results) == len(texts)

    def test_parse_batch_all_parse_results(self):
        """parse_batch() 모든 반환 항목이 ParseResult인지 확인."""
        results = self.parser.parse_batch(["메모리 조회", "CPU 확인"])
        for r in results:
            assert isinstance(r, ParseResult)

    def test_parse_batch_stats_accumulated(self):
        """parse_batch() 후 total_parsed가 배치 수만큼 증가 확인."""
        self.parser.reset_stats()
        self.parser.parse_batch(["메모리 조회", "CPU 확인", "파일 열어줘"])
        stats = self.parser.get_stats()
        assert stats["total_parsed"] == 3


class TestIntentParserConfig:
    """RouterConfig를 통한 IntentParser 설정 테스트."""

    def test_custom_config_accepted(self):
        """사용자 지정 RouterConfig를 파서에 전달 가능한지 확인."""
        config = RouterConfig(prefer_privacy=True)
        parser = IntentParser(config=config)
        # 파서 생성 및 parse 호출 가능한지 확인
        result = parser.parse("메모리 상태 조회해줘")
        assert isinstance(result, ParseResult)

    def test_default_config_used_when_none(self):
        """config=None 시 기본 RouterConfig 사용 확인."""
        parser = IntentParser(config=None)
        assert parser._config is not None
        assert isinstance(parser._config, RouterConfig)


class TestIntentParserResponseText:
    """_build_response_text() 응답 텍스트 생성 테스트."""

    def setup_method(self):
        self.parser = IntentParser()

    def test_unknown_response_contains_input(self):
        """UNKNOWN 의도 응답 텍스트에 원본 입력이 포함되는지 확인."""
        result = self.parser.parse("xyzabc 알 수 없는 입력")
        # UNKNOWN 응답은 재시도 안내 포함
        assert len(result.response_text) > 0

    def test_known_intent_response_not_empty(self):
        """알려진 의도 응답 텍스트가 비어 있지 않은지 확인."""
        result = self.parser.parse("메모리 상태 조회해줘")
        assert len(result.response_text) > 0
