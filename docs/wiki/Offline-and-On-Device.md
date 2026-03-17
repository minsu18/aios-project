# Offline & On-Device Inference

AIOS works **without network**. Built-in on-device AI is always available.

## Offline Mode

```bash
AIOS_OFFLINE=1 npm run demo
```

- All inference stays on-device
- Built-in skills (time, weather, calculator, echo) work offline
- Complex queries use fallback message when cloud is disabled

## On-Device LLM Options

| Option | Prototype | Bare-metal |
|--------|-----------|------------|
| Rule-based | ✅ placeholder | — |
| Ollama | ✅ AIOS_INFERENCE=ollama | — |
| Transformers.js | ✅ AIOS_INFERENCE=transformers | — |
| llama.cpp | — | HAL + docs/HAL_LLAMA_CPP.md |

## Fallback Order

(First tried → last resort)

1. **Skill tool match** — time, weather, calc, echo
2. **On-device LLM** — if model loaded (Ollama, Transformers)
3. **Rule-based** — keyword response
4. **Cloud** — only when online and enabled
