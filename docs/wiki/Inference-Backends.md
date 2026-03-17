# Inference Backends

Set the `AIOS_INFERENCE` environment variable to switch backends.

| Value | Description |
|-------|-------------|
| placeholder | Rule-based, no network (default) |
| openai | Cloud; needs OPENAI_API_KEY |
| anthropic | Cloud; needs ANTHROPIC_API_KEY |
| ollama | Local; run `ollama serve` and `ollama pull <model>` |
| transformers | Local; @huggingface/transformers |

## Ollama

```bash
ollama serve
ollama pull llama3.2
AIOS_INFERENCE=ollama AIOS_OLLAMA_MODEL=llama3.2 npm run demo
```

## Transformers.js

First run downloads the model (~300MB).

```bash
AIOS_INFERENCE=transformers npm run demo
# Optional: AIOS_TRANSFORMERS_MODEL=Xenova/LaMini-Flan-T5-783M
```

## Offline Mode

```bash
AIOS_OFFLINE=1 npm run demo
```

Forces all inference on-device; no cloud calls.
