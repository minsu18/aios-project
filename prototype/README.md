# AIOS Phase 1 Prototype

TypeScript prototype for AI core and skill runtime. Runs on host for validation before bare-metal deployment.

## Features

- **AI Core**: Text input → intent inference → routing (on-device vs cloud)
- **Skill Runtime**: Load SKILL.md from `~/.aios/skills/` or `.aios/skills/`
- **MCP-Compatible Tools**: Skills expose tools in MCP format; see [docs/MCP_SPEC.md](../docs/MCP_SPEC.md)

## Usage

```bash
npm install
npm run build

# Demo prompts (time, greeting, weather, complex query)
npm run demo

# List loaded skills and tools
npm run skills

# Single prompt (JSON output)
node dist/index.js "What time is it?"
```

## Example Skill

See `.aios/skills/example/SKILL.md` for a sample skill with MCP tool definitions.
