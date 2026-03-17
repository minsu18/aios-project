# On-Device Inference Design

## Goal

AIOS must work entirely on the terminal/device with **built-in AI that runs without network**. When connectivity is unavailable, the system still operates using local inference.

## Current State

| Component          | Offline-Capable | Notes                                        |
|--------------------|-----------------|----------------------------------------------|
| Placeholder backend| Yes             | Rule-based intent; no real LLM               |
| Skill tools        | Yes             | time, weather, calculator — no network       |
| OpenAI/Anthropic   | No              | Cloud APIs only                              |
| Multimodal (STT)   | No*             | Whisper API; on-device Whisper possible      |
| Multimodal (Vision)| No*             | Vision API; local vision models possible     |

\* Can be replaced with local models (see Options below).

## Design Choices

### 1. Default Behavior

- **Offline detected** → Use placeholder or local LLM; never call cloud APIs.
- **Online** → Optionally use cloud for complex queries; fall back to on-device on failure.

### 2. On-Device AI Options

| Option               | Runtime        | Pros                         | Cons                      |
|----------------------|----------------|------------------------------|---------------------------|
| Rule-based (current) | JS/TS, Rust    | No deps, instant, tiny       | Limited to known intents  |
| transformer.js       | Node.js        | Real LLM, runs in JS         | Model size, CPU cost      |
| llama.cpp / Ollama   | Native binary  | Fast, GGUF models            | External process or build |
| HAL `gpu.inference`  | Bare-metal     | Fits kernel stack            | Needs driver + model      |

### 3. Layered Fallback

```
User input
    │
    ▼
┌─────────────────────────────────────────────────────┐
│ 1. Skill tool match (time, weather, calc, echo…)   │ ← Always offline
└─────────────────────────────────────────────────────┘
    │ no match
    ▼
┌─────────────────────────────────────────────────────┐
│ 2. On-device LLM (if available and model loaded)   │ ← Offline
└─────────────────────────────────────────────────────┘
    │ no model or fail
    ▼
┌─────────────────────────────────────────────────────┐
│ 3. Rule-based / keyword response                   │ ← Offline fallback
└─────────────────────────────────────────────────────┘
    │
    ▼ (only when online and explicitly enabled)
┌─────────────────────────────────────────────────────┐
│ 4. Cloud API (OpenAI, Anthropic)                   │ ← Optional
└─────────────────────────────────────────────────────┘
```

## Implementation Checklist

- [x] **Offline mode** — `AIOS_OFFLINE=1`; skip cloud entirely
- [x] **On-device LLM backend (prototype)** — `AIOS_INFERENCE=ollama` (Ollama) or `transformers` (@huggingface/transformers)
- [x] **HAL gpu.inference** — Interface + design for llama.cpp; stub with `--features llama` (FFI TODO)
- [ ] **HAL FFI** — Wire `gpu::inference` to llama.cpp; see docs/HAL_LLAMA_CPP.md

## References

- Prototype inference: `prototype/src/inference.ts`
- HAL GPU abstraction: `hal/src/gpu.rs`
- Routing logic: `prototype/src/ai-core.ts`
