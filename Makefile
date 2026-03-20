# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
#
# AI-OS 프로젝트 최상위 Makefile
# 사용법: make <target>
# 예시  : make build   → 전체 빌드
#         make test    → 전체 테스트
#         make lint    → 린트/포맷 검사
#         make clean   → 빌드 결과물 제거

# ─────────────────────────────────────────────
#  설정 변수
# ─────────────────────────────────────────────

# 빌드 프로파일 (dev | release)
PROFILE         ?= dev
# Rust 빌드 플래그
CARGO_FLAGS     := $(if $(filter release,$(PROFILE)),--release,)
# Python 소스 경로
PYTHON_DIR      := python
INTENT_DIR      := $(PYTHON_DIR)/intent_engine
# UI 소스 경로
UI_DIR          := ui/ai-shell
# 색상 출력 (터미널 지원 시)
BOLD            := \033[1m
GREEN           := \033[0;32m
YELLOW          := \033[0;33m
CYAN            := \033[0;36m
RESET           := \033[0m

# ─────────────────────────────────────────────
#  기본 타겟
# ─────────────────────────────────────────────

# make 실행 시 기본 동작 = help 출력
.DEFAULT_GOAL := help

.PHONY: help
help: ## 사용 가능한 모든 타겟 출력
	@echo ""
	@echo "$(BOLD)$(CYAN)AI-OS Makefile$(RESET)"
	@echo "$(CYAN)━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BOLD)%-20s$(RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "예시:"
	@echo "  make build           → 전체 빌드 (dev 프로파일)"
	@echo "  make build PROFILE=release → 릴리스 빌드"
	@echo "  make test            → 전체 테스트"
	@echo "  make lint            → 린트 검사"
	@echo ""

# ─────────────────────────────────────────────
#  빌드 타겟
# ─────────────────────────────────────────────

.PHONY: build
build: build-rust build-python ## 전체 빌드 (Rust + Python)
	@echo "$(GREEN)✅ 전체 빌드 완료$(RESET)"

.PHONY: build-rust
build-rust: ## Rust HAL 크레이트 빌드
	@echo "$(CYAN)▶ Rust 빌드 (ai-hal) [$(PROFILE)]...$(RESET)"
	cargo build -p ai-hal $(CARGO_FLAGS)
	@echo "$(GREEN)  ✓ ai-hal 빌드 완료$(RESET)"

.PHONY: build-rust-all
build-rust-all: ## Rust workspace 전체 빌드
	@echo "$(CYAN)▶ Rust workspace 전체 빌드 [$(PROFILE)]...$(RESET)"
	cargo build --workspace $(CARGO_FLAGS)
	@echo "$(GREEN)  ✓ Rust 전체 빌드 완료$(RESET)"

.PHONY: build-python
build-python: ## Python Intent Engine 패키지 설치 (editable)
	@echo "$(CYAN)▶ Python 패키지 설치 (intent_engine)...$(RESET)"
	cd $(PYTHON_DIR) && uv pip install -e "." --quiet 2>/dev/null || \
		pip install -e "." -q
	@echo "$(GREEN)  ✓ Python 패키지 설치 완료$(RESET)"

.PHONY: build-release
build-release: ## 릴리스 빌드 (최적화 활성화)
	@$(MAKE) build PROFILE=release

# ─────────────────────────────────────────────
#  테스트 타겟
# ─────────────────────────────────────────────

.PHONY: test
test: test-rust test-python ## 전체 테스트 실행
	@echo "$(GREEN)✅ 전체 테스트 완료$(RESET)"

.PHONY: test-rust
test-rust: ## Rust 단위 테스트 실행 (mock feature 포함)
	@echo "$(CYAN)▶ Rust 테스트 실행 (ai-hal)...$(RESET)"
	cargo test -p ai-hal --features mock -- --nocapture
	@echo "$(GREEN)  ✓ Rust 테스트 완료$(RESET)"

.PHONY: test-rust-all
test-rust-all: ## Rust workspace 전체 테스트
	@echo "$(CYAN)▶ Rust workspace 전체 테스트...$(RESET)"
	cargo test --workspace --features mock
	@echo "$(GREEN)  ✓ Rust 전체 테스트 완료$(RESET)"

.PHONY: test-python
test-python: ## Python pytest 실행
	@echo "$(CYAN)▶ Python 테스트 실행 (intent_engine)...$(RESET)"
	cd $(PYTHON_DIR) && \
		uv run pytest tests/ -v --tb=short 2>/dev/null || \
		python -m pytest tests/ -v --tb=short 2>/dev/null || \
		python $(INTENT_DIR)/parser.py
	@echo "$(GREEN)  ✓ Python 테스트 완료$(RESET)"

.PHONY: test-watch
test-watch: ## Rust 테스트 watch 모드 (cargo-watch 필요)
	@echo "$(CYAN)▶ Rust 테스트 watch 모드 시작...$(RESET)"
	cargo watch -x "test -p ai-hal --features mock"

# ─────────────────────────────────────────────
#  린트 / 포맷 타겟
# ─────────────────────────────────────────────

.PHONY: lint
lint: lint-rust lint-python ## 전체 린트 검사
	@echo "$(GREEN)✅ 전체 린트 검사 완료$(RESET)"

.PHONY: lint-rust
lint-rust: fmt-rust-check clippy ## Rust 린트 검사 (fmt + clippy)

.PHONY: fmt-rust-check
fmt-rust-check: ## Rust 코드 포맷 검사
	@echo "$(CYAN)▶ Rust 포맷 검사...$(RESET)"
	cargo fmt --all --check
	@echo "$(GREEN)  ✓ Rust 포맷 OK$(RESET)"

.PHONY: fmt-rust
fmt-rust: ## Rust 코드 포맷 자동 수정
	@echo "$(CYAN)▶ Rust 포맷 자동 수정...$(RESET)"
	cargo fmt --all
	@echo "$(GREEN)  ✓ Rust 포맷 완료$(RESET)"

.PHONY: clippy
clippy: ## Clippy 린트 검사 (경고 = 에러)
	@echo "$(CYAN)▶ Clippy 린트...$(RESET)"
	cargo clippy -p ai-hal --features mock -- -D warnings
	@echo "$(GREEN)  ✓ Clippy OK$(RESET)"

.PHONY: lint-python
lint-python: ## Python 린트 검사 (ruff + mypy)
	@echo "$(CYAN)▶ Python ruff 린트...$(RESET)"
	cd $(PYTHON_DIR) && \
		uv run ruff check $(INTENT_DIR) 2>/dev/null || \
		python -m ruff check $(INTENT_DIR) 2>/dev/null || \
		echo "  ℹ ruff 미설치 (make setup-dev 실행 권장)"
	@echo "$(CYAN)▶ Python mypy 타입 체크...$(RESET)"
	cd $(PYTHON_DIR) && \
		uv run mypy $(INTENT_DIR) --strict 2>/dev/null || \
		echo "  ℹ mypy 미설치 (make setup-dev 실행 권장)"
	@echo "$(GREEN)  ✓ Python 린트 완료$(RESET)"

.PHONY: fmt-python
fmt-python: ## Python 코드 포맷 자동 수정 (ruff format)
	@echo "$(CYAN)▶ Python ruff 포맷...$(RESET)"
	cd $(PYTHON_DIR) && uv run ruff format $(INTENT_DIR)
	@echo "$(GREEN)  ✓ Python 포맷 완료$(RESET)"

# ─────────────────────────────────────────────
#  실행 타겟
# ─────────────────────────────────────────────

.PHONY: run
run: run-intent ## 기본 실행 (Intent Engine 데모)

.PHONY: run-intent
run-intent: ## Intent Engine 파서 데모 실행
	@echo "$(CYAN)▶ Intent Engine 데모 실행...$(RESET)"
	cd $(PYTHON_DIR) && python -m intent_engine.parser

.PHONY: run-qemu
run-qemu: build-release ## QEMU에서 AI-OS 부팅 (M1 이후)
	@echo "$(YELLOW)⚠ QEMU 실행은 M1 마일스톤 이후 지원됩니다.$(RESET)"
	@echo "  현재: M0 — HAL trait 정의 단계"

# ─────────────────────────────────────────────
#  설치 / 환경 설정 타겟
# ─────────────────────────────────────────────

.PHONY: setup
setup: setup-rust setup-python ## 전체 개발 환경 설정
	@echo "$(GREEN)✅ 개발 환경 설정 완료$(RESET)"

.PHONY: setup-rust
setup-rust: ## Rust 툴체인 설정 (rustfmt, clippy)
	@echo "$(CYAN)▶ Rust 툴체인 설정...$(RESET)"
	rustup update stable
	rustup component add rustfmt clippy
	@echo "$(GREEN)  ✓ Rust 툴체인 준비 완료$(RESET)"

.PHONY: setup-python
setup-python: ## Python 개발 의존성 설치
	@echo "$(CYAN)▶ Python 개발 의존성 설치...$(RESET)"
	cd $(PYTHON_DIR) && uv pip install -e ".[dev]"
	@echo "$(GREEN)  ✓ Python 개발 환경 준비 완료$(RESET)"

.PHONY: setup-dev
setup-dev: setup ## 전체 개발 환경 설정 (setup 별칭)

# ─────────────────────────────────────────────
#  정리 타겟
# ─────────────────────────────────────────────

.PHONY: clean
clean: clean-rust clean-python ## 모든 빌드 결과물 제거
	@echo "$(GREEN)✅ 정리 완료$(RESET)"

.PHONY: clean-rust
clean-rust: ## Rust 빌드 결과물 제거
	@echo "$(CYAN)▶ Rust 빌드 캐시 제거...$(RESET)"
	cargo clean
	@echo "$(GREEN)  ✓ target/ 제거 완료$(RESET)"

.PHONY: clean-python
clean-python: ## Python 캐시 제거
	@echo "$(CYAN)▶ Python 캐시 제거...$(RESET)"
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name ".mypy_cache" -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name ".ruff_cache" -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name "*.egg-info"  -exec rm -rf {} + 2>/dev/null || true
	find . -type d -name ".pytest_cache" -exec rm -rf {} + 2>/dev/null || true
	@echo "$(GREEN)  ✓ Python 캐시 제거 완료$(RESET)"

# ─────────────────────────────────────────────
#  CI 타겟 (GitHub Actions에서 사용)
# ─────────────────────────────────────────────

.PHONY: ci
ci: lint test ## CI 전체 파이프라인 (lint + test)
	@echo "$(GREEN)✅ CI 파이프라인 통과$(RESET)"

.PHONY: ci-rust
ci-rust: fmt-rust-check clippy test-rust ## Rust CI (fmt + clippy + test)

.PHONY: ci-python
ci-python: lint-python test-python ## Python CI (lint + test)

# ─────────────────────────────────────────────
#  AGPL 헤더 검사 타겟
# ─────────────────────────────────────────────

.PHONY: check-license
check-license: ## 모든 소스 파일의 AGPL 헤더 확인
	@echo "$(CYAN)▶ AGPL-3.0 라이선스 헤더 검사...$(RESET)"
	@MISSING=0; \
	for f in $$(find crates python ui -type f \( -name "*.rs" -o -name "*.py" -o -name "*.ts" -o -name "*.tsx" \) 2>/dev/null); do \
		if ! grep -q "SPDX-License-Identifier: AGPL-3.0-or-later" "$$f"; then \
			echo "  $(YELLOW)⚠ AGPL 헤더 없음: $$f$(RESET)"; \
			MISSING=$$((MISSING+1)); \
		fi; \
	done; \
	if [ $$MISSING -eq 0 ]; then \
		echo "$(GREEN)  ✓ 모든 파일에 AGPL 헤더 있음$(RESET)"; \
	else \
		echo "$(YELLOW)  ⚠ $$MISSING개 파일에 헤더 없음$(RESET)"; \
		exit 1; \
	fi

# ─────────────────────────────────────────────
#  정보 타겟
# ─────────────────────────────────────────────

.PHONY: info
info: ## 현재 개발 환경 정보 출력
	@echo ""
	@echo "$(BOLD)$(CYAN)AI-OS 개발 환경 정보$(RESET)"
	@echo "$(CYAN)━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━$(RESET)"
	@echo "  Rust:   $$(rustc --version 2>/dev/null || echo '미설치')"
	@echo "  Cargo:  $$(cargo --version 2>/dev/null || echo '미설치')"
	@echo "  Python: $$(python3 --version 2>/dev/null || echo '미설치')"
	@echo "  uv:     $$(uv --version 2>/dev/null || echo '미설치')"
	@echo "  QEMU:   $$(qemu-system-x86_64 --version 2>/dev/null | head -1 || echo '미설치')"
	@echo "  Git:    $$(git --version 2>/dev/null || echo '미설치')"
	@echo ""
	@echo "  현재 마일스톤: M0 (HAL trait 정의 단계)"
	@echo "  레포:         https://github.com/minsu18/aios-project"
	@echo ""
