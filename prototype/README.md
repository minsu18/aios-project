# AIOS Phase 1 Prototype

TypeScript prototype for AI core and skill runtime. Runs on host for validation before bare-metal deployment.

## Features

- **AI Core**: Text input → intent inference → routing (on-device vs cloud)
- **Skill Tool Invocation**: Invokes MCP tools (e.g. example.get_time, example.echo)
- **Pluggable Inference**: `AIOS_INFERENCE=placeholder|openai|anthropic` (openai/anthropic hooks ready)
- **Multimodal API**: `processMultimodal()` accepts text/voice/image/video (voice/image/video placeholders)
- **Skill Runtime**: Load SKILL.md from `~/.aios/skills/` or `.aios/skills/`
- **App Store CLI**: Install and remove skills

## Usage

```bash
npm install
npm run build

# Demo prompts (time, greeting, echo, weather, complex query)
npm run demo

# List loaded skills and tools
npm run skills

# Install skill from path
node dist/index.js install /path/to/skill-dir

# Remove installed skill
node dist/index.js remove <skill-name>

# Single prompt (JSON output)
node dist/index.js "What time is it?"
node dist/index.js "echo Hello from skill!"
```

## Example Skill

See `.aios/skills/example/SKILL.md` for a sample skill with MCP tool definitions.
