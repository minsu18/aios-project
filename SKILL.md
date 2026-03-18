---
name: example
description: Example skill for AIOS kernel (RPi, QEMU)
version: 0.1.0
tools: [
  {"name":"get_time","description":"Get current local time","inputSchema":{"type":"object","properties":{"timezone":{"type":"string","description":"IANA timezone"}},"required":[]}},
  {"name":"echo","description":"Echo back the input text","inputSchema":{"type":"object","properties":{"text":{"type":"string","description":"Text to echo"}},"required":["text"]}}
]
---

# Example Skill

This skill provides `get_time` and `echo` tools. Use `load` to register, then `example.get_time` or `example echo hi`.
