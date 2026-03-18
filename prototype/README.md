# AIOS Phase 1 Prototype

TypeScript prototype for AI core and skill runtime. Runs on host for validation before bare-metal deployment.

## Features

- **AI Core**: Text input → intent inference → routing (on-device vs cloud)
- **Skill Tool Invocation**: Invokes MCP tools (e.g. example.get_time, example.echo)
- **Pluggable Inference**: `AIOS_INFERENCE=placeholder|openai|anthropic|ollama|transformers`
  - `ollama`: Local LLM via Ollama (run `ollama serve`, `ollama pull llama3.2`). Optional: `AIOS_OLLAMA_MODEL`, `AIOS_OLLAMA_HOST`
  - `transformers`: Local LLM via @huggingface/transformers (first run downloads model). Optional: `AIOS_TRANSFORMERS_MODEL`
- **Offline-first**: `AIOS_OFFLINE=1` forces all inference on-device (no cloud calls). Built-in skills (time, weather, calculator, echo) always work without network.
- **Multimodal I/O**: Voice (Whisper STT), image (Vision API) — `voice <file>`, `voice capture` (Linux/mic), `image <file> [prompt]`, `image capture [prompt]` (Linux/camera)
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

# Browse registry (local or AIOS_REGISTRY_URL, defaults to GitHub raw)
node dist/index.js browse
node dist/index.js install-from-registry <skill-name>

# Update skill(s) to latest from registry
node dist/index.js update [skill-name]

# Remove installed skill
node dist/index.js remove <skill-name>

# Multimodal: voice or image (file path, or capture from hardware on Linux)
node dist/index.js voice path/to/audio.wav
node dist/index.js voice capture   # Linux: mic via driver-bridge
node dist/index.js image path/to/image.png "What's in this image?"
node dist/index.js image capture "Describe"   # Linux: camera via driver-bridge

# Single prompt (JSON output)
node dist/index.js "What time is it?"
node dist/index.js "echo Hello from skill!"
```

## Example Skill

See `.aios/skills/example/SKILL.md` for a sample skill with MCP tool definitions.
