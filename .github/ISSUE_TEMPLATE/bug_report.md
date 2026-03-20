---
name: 🐛 버그 리포트
about: 버그를 발견하셨나요? 상세히 알려주세요.
title: "[BUG] "
labels: ["bug", "triage"]
assignees: minsu18
---

## 🐛 버그 설명

발생한 버그를 명확하게 설명해주세요.

## 재현 방법

버그를 재현하기 위한 최소한의 단계를 작성해주세요.

```
1.
2.
3.
4. 에러 발생
```

## 예상 동작

어떻게 동작해야 했는지 설명해주세요.

## 실제 동작

실제로 어떻게 동작했는지 설명해주세요. 에러 메시지나 로그를 포함해주세요.

```
에러 메시지 / 로그를 여기에 붙여넣으세요
```

## 관련 레이어

- [ ] HAL (`crates/ai-hal`)
- [ ] AI Core Bridge (`crates/ai-core-bridge`)
- [ ] Intent Engine (`python/intent_engine`)
- [ ] Skill Runtime (`crates/skill-runtime`)
- [ ] UI (`ui/ai-shell`)
- [ ] CI/CD (`.github/workflows`)
- [ ] 기타:

## 환경

| 항목 | 버전 |
|------|------|
| OS | |
| Rust | `rustc --version` 출력 |
| Python | `python --version` 출력 |
| 커밋 해시 | `git rev-parse HEAD` 출력 |

## 최소 재현 코드 (있다면)

```rust
// Rust 코드
```

```python
# Python 코드
```

## 추가 컨텍스트

스크린샷, 관련 이슈 링크 등 추가 정보를 여기에 작성해주세요.
