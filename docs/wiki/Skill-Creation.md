# Creating Skills

## Structure

```
my-skill/
├── SKILL.md        # Required: metadata + tools
└── handlers.js     # Optional: implementations
```

## SKILL.md Example

```markdown
---
name: my-skill
description: What your skill does
version: 0.1.0
author: Your Name
category: utility
permissions: ["network"]
tools: [
  {
    "name": "tool_name",
    "description": "What it does",
    "inputSchema": {
      "type": "object",
      "properties": { "param": { "type": "string" } },
      "required": ["param"]
    }
  }
]
---

# My Skill

Instructions for the AI...
```

## Registry

Add to `registry/skills.json` in a PR:

```json
{
  "name": "my-skill",
  "description": "Description",
  "version": "0.1.0",
  "author": "You",
  "category": "utility",
  "source": "git:https://github.com/you/repo.git"
}
```
