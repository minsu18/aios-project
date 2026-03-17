# AIOS Skill Runtime

스킬/MCP 호환 모듈의 로드, 실행, 격리.

## 스킬 형식 (SKILL.md)

```markdown
---
name: my-skill
description: Does something useful
version: 0.1.0
---

# My Skill

Instructions for the AI...
```

## 설치 위치

- 사용자: `~/.aios/skills/`
- 프로젝트: `.aios/skills/`

## MCP 호환

스킬은 MCP(Model Context Protocol) 도구 정의를 포함할 수 있으며,
AI Core가 해당 도구를 동적 호출한다.
