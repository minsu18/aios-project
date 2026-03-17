---
name: weather
description: Get weather for any location (mock data in prototype)
version: 0.1.0
author: AIOS
category: productivity
permissions: ["network"]
tools: [
  {
    "name": "get_weather",
    "description": "Get current weather for a location",
    "inputSchema": {
      "type": "object",
      "properties": {
        "location": { "type": "string", "description": "City or location name" },
        "units": { "type": "string", "description": "celsius or fahrenheit" }
      },
      "required": ["location"]
    }
  }
]
---

# Weather Skill

Returns current weather. In the prototype, returns mock data. With network permission, could call a real API.
