# AIOS Phase 1 Prototype

TypeScript prototype for AI core and skill runtime. Runs on host for validation before bare-metal deployment.

## Features

- **AI Core**: Text input → intent inference → routing (on-device vs cloud)
- **Skill Tool Invocation**: Invokes MCP tools (e.g. example.get_time, example.echo)
- **Pluggable Inference**: `AIOS_INFERENCE=placeholder|openai|anthropic` (requires OPENAI_API_KEY / ANTHROPIC_API_KEY)
- **Multimodal I/O**: Voice (Whisper STT), image (Vision API) — `voice <file>`, `image <file> [prompt]`
- **Skill Runtime**: Load SKILL.md from `~/.aios/skills/` or `.aios/skills/`
- **App Store CLI**: Install, remove, browse registry, install-from-registry

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

# Browse registry and install from registry
node dist/index.js browse
node dist/index.js install-from-registry <skill-name>

# Remove installed skill
node dist/index.js remove <skill-name>

# Multimodal: voice (WAV/MP3) or image (requires API key)
node dist/index.js voice path/to/audio.wav
node dist/index.js image path/to/image.png "What's in this image?"

# Single prompt (JSON output)
node dist/index.js "What time is it?"
node dist/index.js "echo Hello from skill!"
```

## Example Skill

See `.aios/skills/example/SKILL.md` for a sample skill with MCP tool definitions.
