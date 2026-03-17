# Creating AIOS Skills

This guide explains how to create and publish skills for AIOS.

## Skill Structure

A skill is a directory with at least `SKILL.md`:

```
my-skill/
├── SKILL.md        # Required: metadata and tool definitions
└── handlers.js     # Optional: tool implementations (or use built-in)
```

## SKILL.md Format

```markdown
---
name: my-skill
description: What your skill does
version: 0.1.0
author: Your Name
category: utility | productivity | fun
permissions: ["network", "filesystem", "env"]   # Optional
tools: [
  {
    "name": "tool_name",
    "description": "What the tool does",
    "inputSchema": {
      "type": "object",
      "properties": {
        "param": { "type": "string", "description": "Param desc" }
      },
      "required": ["param"]
    }
  }
]
---

# My Skill

Instructions for the AI...
```

### Metadata

| Field | Required | Description |
|-------|----------|-------------|
| name | Yes | Unique skill ID (lowercase, hyphenated) |
| description | Yes | Short description |
| version | Yes | Semver (e.g. 0.1.0) |
| author | No | Author name |
| category | No | utility, productivity, fun, etc. |
| permissions | No | Array: network, filesystem, env |
| tools | No | MCP-compatible tool definitions |

### Permissions

Declare what your skill needs. Future versions will enforce sandboxing:

- **network** — HTTP requests, APIs
- **filesystem** — Read/write files
- **env** — Access environment variables

### Tools (MCP-Compatible)

Each tool needs:
- `name` — Unique within the skill (becomes `skill_name.tool_name`)
- `description` — For the AI to decide when to call
- `inputSchema` — JSON Schema with `type: "object"`, `properties`, `required`

## Implementations

### 1. Built-in Handlers

Some tools have built-in handlers in the runtime (e.g. `example.get_time`, `weather.get_weather`, `calculator.evaluate`). No `handlers.js` needed.

### 2. handlers.js

Export functions matching tool names:

```javascript
// handlers.js
export function my_tool(args) {
  const x = args.param;
  return `Result: ${x}`;
}
```

Or default export:

```javascript
export default {
  my_tool(args) { ... }
};
```

## Registry

To add your skill to the AIOS registry:

1. Fork the [aios-project](https://github.com/minsu18/aios-project) repo
2. Add your skill to `registry/skills.json`:

```json
{
  "name": "my-skill",
  "description": "Description",
  "version": "0.1.0",
  "author": "You",
  "category": "utility",
  "source": "git:https://github.com/you/my-skill.git"
}
```

3. For `git:` source: repo must have `SKILL.md` at root
4. For `local:` source: path relative to project (for bundled skills)
5. Open a pull request

## Testing Locally

```bash
# Install from path
cd prototype && node dist/index.js install /path/to/my-skill

# Or copy to ~/.aios/skills/my-skill

# Test
node dist/index.js "trigger your tool"
```

## Categories

Suggested categories: `utility`, `productivity`, `fun`, `data`, `automation`.
