# AIOS Skill Runtime

Load, execute, and isolate skills/MCP-compatible modules. Skills are the installable AI add-ons surfaced through the App Store.

## Prototype

Phase 1 implementation lives in `prototype/` (TypeScript):

```bash
cd prototype && npm run skills
```

## Skill format (SKILL.md)

```markdown
---
name: my-skill
description: Does something useful
version: 0.1.0
tools: [{"name":"my_tool","description":"...","inputSchema":{...}}]
---

# My Skill

Instructions for the AI...
```

## Install paths

- User: `~/.aios/skills/`
- Project: `.aios/skills/`

## MCP compatibility

Skills expose tools in MCP format. See [docs/MCP_SPEC.md](../docs/MCP_SPEC.md) for the full spec.
