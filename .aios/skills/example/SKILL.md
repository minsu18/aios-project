---
name: example
description: Example skill for AIOS prototype
version: 0.1.0
tools: [
  {"name":"get_time","description":"Get current local time","inputSchema":{"type":"object","properties":{"timezone":{"type":"string","description":"IANA timezone"}},"required":[]}},
  {"name":"echo","description":"Echo back the input text","inputSchema":{"type":"object","properties":{"text":{"type":"string","description":"Text to echo"}},"required":["text"]}}
]
---

# Example Skill

This skill demonstrates MCP-compatible tool definitions in SKILL.md.

## Tools

- **get_time**: Returns current local time (optional timezone)
- **echo**: Echoes input text (for testing)

## Usage

The AI Core invokes these tools when the user intent matches. Tool names are prefixed with skill name: `example.get_time`, `example.echo`.
