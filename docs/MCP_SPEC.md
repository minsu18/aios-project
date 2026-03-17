# AIOS MCP-Compatible Spec

AIOS skills expose tools in MCP (Model Context Protocol) format. Skills are the installable AI add-ons distributed via the App Store. This document describes how skills define tools and how the AI Core discovers and invokes them.

## Reference

- [MCP Specification — Tools](https://modelcontextprotocol.io/specification/2024-11-05/server/tools)
- [MCP Specification — 2024-11-05](https://modelcontextprotocol.io/specification/2024-11-05/)

## Skill Format (SKILL.md)

Each skill lives in its own directory with a `SKILL.md` file:

```
~/.aios/skills/my-skill/
├── SKILL.md          # Required: metadata + tool definitions
└── (optional files)  # Resources, config, etc.
```

### Frontmatter

YAML frontmatter at the top of SKILL.md:

```markdown
---
name: my-skill
description: Brief description of what the skill does
version: 0.1.0
tools: [...]   # Optional: MCP-compatible tool definitions (JSON)
---
```

### Tool Definition (MCP-Compatible)

Each tool in the `tools` array must conform to the MCP Tool schema:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique identifier within the skill |
| `description` | string | Yes | Human-readable description for the model |
| `inputSchema` | object | Yes | JSON Schema for parameters |

**Full tool name**: Skills prefix tool names. A tool `get_weather` in skill `weather` becomes `weather.get_weather` in the global namespace.

#### inputSchema

Must have `type: "object"`. Use `properties` and `required` for parameters:

```json
{
  "name": "get_weather",
  "description": "Get current weather for a location",
  "inputSchema": {
    "type": "object",
    "properties": {
      "location": {
        "type": "string",
        "description": "City name or zip code"
      },
      "units": {
        "type": "string",
        "description": "celsius or fahrenheit"
      }
    },
    "required": ["location"]
  }
}
```

### Example SKILL.md

```markdown
---
name: weather
description: Weather information for any location
version: 0.1.0
tools: [
  {
    "name": "get_weather",
    "description": "Get current weather for a location",
    "inputSchema": {
      "type": "object",
      "properties": {
        "location": { "type": "string", "description": "City or zip" }
      },
      "required": ["location"]
    }
  }
]
---

# Weather Skill

Instructions for the AI...
```

## Protocol Compatibility

### tools/list

The Skill Runtime aggregates tools from all loaded skills and returns them in MCP `tools/list` format:

```json
{
  "tools": [
    {
      "name": "weather.get_weather",
      "description": "Get current weather for a location",
      "inputSchema": { "type": "object", "properties": { ... } }
    }
  ]
}
```

### tools/call

Tool invocation uses the full name (`skill.tool_name`). The runtime dispatches to the correct skill implementation. Arguments must match the tool's `inputSchema`.

## Install Paths

Skills are loaded from (in order):

1. `.aios/skills/` (project directory)
2. `~/.aios/skills/` (user home)

## Isolation

Phase 1 prototype does not enforce sandboxing. Phase 2+ will add:

- Permission-based access control (e.g., `network`, `filesystem`, `camera`)
- Optional sandbox execution for untrusted skills
