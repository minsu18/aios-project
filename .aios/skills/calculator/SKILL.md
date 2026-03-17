---
name: calculator
description: Safe arithmetic calculator (+, -, *, /, ^)
version: 0.1.0
author: AIOS
category: utility
tools: [
  {
    "name": "evaluate",
    "description": "Evaluate a simple math expression (numbers, +, -, *, /, ^)",
    "inputSchema": {
      "type": "object",
      "properties": {
        "expression": { "type": "string", "description": "Math expression e.g. 2+3*4" }
      },
      "required": ["expression"]
    }
  }
]
---

# Calculator Skill

Evaluates arithmetic expressions safely. Supports +, -, *, /, ^ (power). No eval() — parser-only for security.
