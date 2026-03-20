# AI-OS 기여 가이드 (Contributing Guide)

> SPDX-License-Identifier: AGPL-3.0-or-later
> Copyright (C) 2025 minsu18 <https://github.com/minsu18>
> Project : AI-OS — https://github.com/minsu18/aios-project

AI-OS 프로젝트에 기여해 주셔서 감사합니다!
이 문서는 코드 기여, 버그 리포트, 기능 제안을 위한 가이드입니다.

---

## 목차

1. [시작 전 읽어주세요](#1-시작-전-읽어주세요)
2. [개발 환경 설정](#2-개발-환경-설정)
3. [브랜치 전략](#3-브랜치-전략)
4. [커밋 메시지 규칙](#4-커밋-메시지-규칙)
5. [코드 스타일](#5-코드-스타일)
6. [PR 제출 절차](#6-pr-제출-절차)
7. [라이선스 동의](#7-라이선스-동의)

---

## 1. 시작 전 읽어주세요

- 모든 기여는 **AGPL-3.0-or-later** 라이선스에 동의함을 의미합니다.
- 큰 변경사항 전에 반드시 **Issue를 먼저 열어** 논의해주세요.
- 모든 소스 파일 상단에 **AGPL-3.0 라이선스 헤더**를 포함해야 합니다.
- 코드 주석은 **한국어**로 작성합니다.

---

## 2. 개발 환경 설정

### 사전 요구사항

| 도구 | 버전 | 용도 |
|------|------|------|
| Rust | 1.75+ (stable) | HAL / Kernel 개발 |
| Python | 3.11+ | Intent Engine 개발 |
| [uv](https://github.com/astral-sh/uv) | latest | Python 패키지 관리 |
| QEMU | 8.x | 커널 테스트 환경 |
| Git | 2.x | 버전 관리 |

### 설치

```bash
# 1. 레포 포크 후 클론
git clone https://github.com/<YOUR_USERNAME>/aios-project.git
cd aios-project

# 2. upstream 리모트 추가
git remote add upstream https://github.com/minsu18/aios-project.git

# 3. Rust 툴체인 설치
rustup update stable
rustup component add rustfmt clippy

# 4. Python 환경 설정
cd python
uv pip install -e ".[dev]"
cd ..

# 5. 빌드 확인
make build
make test
```

---

## 3. 브랜치 전략

```
main          ← 안정 릴리스 (직접 push 금지)
  └── develop ← 통합 개발 브랜치
        ├── feature/hal-memory-impl     ← 기능 개발
        ├── fix/intent-parser-edge-case ← 버그 수정
        └── docs/update-architecture    ← 문서 수정
```

| 브랜치 패턴 | 용도 | 머지 대상 |
|------------|------|---------|
| `feature/<scope>-<desc>` | 새 기능 개발 | `develop` |
| `fix/<scope>-<desc>` | 버그 수정 | `develop` |
| `docs/<desc>` | 문서 수정 | `develop` |
| `refactor/<scope>` | 리팩토링 | `develop` |
| `chore/<desc>` | 빌드/툴 변경 | `develop` |

### 브랜치 생성 예시

```bash
# develop 최신화
git fetch upstream
git checkout develop
git merge upstream/develop

# 새 브랜치 생성
git checkout -b feature/hal-memory-linux
```

---

## 4. 커밋 메시지 규칙

[Conventional Commits](https://www.conventionalcommits.org/) 형식을 따릅니다.

### 형식

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### 타입 목록

| 타입 | 설명 | 예시 |
|------|------|------|
| `feat` | 새 기능 추가 | `feat(hal): add MemoryHal Linux implementation` |
| `fix` | 버그 수정 | `fix(intent): resolve parser edge case for empty input` |
| `docs` | 문서 변경 | `docs: update HAL API reference` |
| `refactor` | 기능 변경 없는 코드 개선 | `refactor(hal): extract capability check into helper` |
| `test` | 테스트 추가/수정 | `test(hal): add MockHal OOM edge case test` |
| `chore` | 빌드/설정 변경 | `chore: update ci-rust.yml MSRV to 1.76` |
| `perf` | 성능 개선 | `perf(intent): cache keyword lookup table` |

### 스코프 목록

| 스코프 | 대상 |
|--------|------|
| `hal` | `crates/ai-hal` |
| `bridge` | `crates/ai-core-bridge` |
| `skill-rt` | `crates/skill-runtime` |
| `intent` | `python/intent_engine` |
| `ui` | `ui/ai-shell` |
| `ci` | `.github/workflows` |
| `docs` | `docs/` |

### 예시

```
feat(hal): add MemoryHal Linux implementation

/proc/meminfo 파싱 기반의 실제 메모리 상태 조회 구현.
MockHal과 동일한 AiHalInterface 구현체로 M1 마일스톤 달성.

Closes #42
```

---

## 5. 코드 스타일

### 5-1. 공통 규칙

- **모든 소스 파일 상단**에 AGPL-3.0 헤더 포함 (아래 참조)
- **코드 주석은 한국어**로 작성
- **영어**는 공개 API 문서 (`///` doc comment), 커밋 메시지, PR 제목에 사용

### 5-2. AGPL-3.0 헤더 (필수)

**Rust 파일 (`.rs`)**
```rust
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
```

**Python 파일 (`.py`)**
```python
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2025 minsu18 <https://github.com/minsu18>
# Project : AI-OS — https://github.com/minsu18/aios-project
```

**TypeScript 파일 (`.ts`, `.tsx`)**
```typescript
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 minsu18 <https://github.com/minsu18>
// Project : AI-OS — https://github.com/minsu18/aios-project
```

### 5-3. Rust 코드 스타일

```bash
# 포맷 검사 (CI에서 실행)
cargo fmt --check

# 포맷 자동 수정
cargo fmt

# Clippy 린트 (경고 = 에러)
cargo clippy -- -D warnings
```

- `rustfmt.toml` 설정을 따름
- `#[allow(...)]` 사용 시 반드시 한국어 이유 주석 추가
- `unsafe` 블록 사용 시 반드시 // SAFETY: 주석 필수

### 5-4. Python 코드 스타일

```bash
# 린트
uv run ruff check python/

# 자동 수정
uv run ruff check --fix python/

# 타입 체크
uv run mypy python/ --strict
```

- 타입 힌트 **필수** (mypy strict 통과해야 함)
- docstring: Google 스타일
- 모든 public 함수/클래스에 docstring 작성

### 5-5. 주석 작성 예시

```rust
/// HAL 명령 실행 결과 — 공개 API 문서 (영어)
///
/// Returns both the execution result and an audit entry.
pub struct HalResult {
    // 감사 엔트리: 보안 감사 및 디버깅에 사용 (한국어 주석)
    pub audit: AuditEntry,
    // 실제 실행 결과
    pub outcome: Result<HalResponse, HalError>,
}
```

---

## 6. PR 제출 절차

### PR 체크리스트

PR 제출 전 아래 항목을 모두 확인해주세요:

```
[ ] AGPL-3.0 라이선스 헤더가 모든 새 파일에 포함됨
[ ] 코드 주석이 한국어로 작성됨
[ ] cargo fmt --check 통과
[ ] cargo clippy -- -D warnings 통과
[ ] cargo test -p <crate> --features mock 통과
[ ] Python: ruff check 통과
[ ] Python: mypy --strict 통과
[ ] 새 기능에 대한 단위 테스트 작성됨
[ ] 관련 문서(docs/) 업데이트됨
[ ] Conventional Commits 형식 사용됨
```

### PR 제목 형식

```
feat(hal): add Linux MemoryHal implementation
fix(intent): handle empty string input in RuleBasedBackend
```

### PR 본문 템플릿

```markdown
## 변경 내용
이 PR에서 무엇을 변경했는지 설명해주세요.

## 관련 이슈
Closes #<issue_number>

## 테스트
어떻게 테스트했는지 설명해주세요.

## 체크리스트
- [ ] AGPL 헤더 포함
- [ ] 한국어 주석
- [ ] 테스트 통과
- [ ] 문서 업데이트
```

---

## 7. 라이선스 동의

이 프로젝트에 기여함으로써 귀하는 기여 내용이
**GNU Affero General Public License v3.0 이상**으로 라이선스됨에 동의합니다.

자세한 내용은 [LICENSE](LICENSE) 파일을 참조하세요.

---

질문이 있으시면 [Issues](https://github.com/minsu18/aios-project/issues)를 통해 문의해주세요.
