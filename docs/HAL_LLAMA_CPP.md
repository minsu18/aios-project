# HAL + llama.cpp Integration

This document describes how AIOS HAL's `gpu.inference` connects to llama.cpp for on-device inference.

## Overview

- **hal/src/gpu.rs** exposes `inference(model_id, input) -> Result<Vec<u8>, &str>`.
- When built with `--features llama`, uses the **llama-cpp-2** crate (Rust bindings to llama.cpp).
- Models: GGUF format (e.g. TinyLlama, Phi-2, Qwen2-0.5B quantized).

## Build and Run

### 1. Build HAL with llama feature

```bash
cargo build -p aios-hal --features llama
```

The **llama-cpp-2** crate vendored/builds llama.cpp automatically.

### 2. Run inference example

Download a GGUF model:

```bash
curl -L -o tinyllama.gguf https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf
```

Run:

```bash
cargo run -p aios-hal --features llama --example llama_inference -- ./tinyllama.gguf "What is AI?"
```

## Target Considerations

| Target    | Notes                                                |
|----------|------------------------------------------------------|
| x86_64   | AVX2/AVX-512 if available; CPU-only is fine for dev |
| aarch64  | RPi 4: use quantized Q4_K_M; ~2GB RAM for 1B model   |
| Bare-metal | No OS; needs custom allocator, no pthread. hal-bare::inference is stub until linked |

**kernel-rpi** calls `hal_bare::inference::inference(prompt)` via the `ask <q>` command. Currently returns "LLM not available" until llama.cpp is integrated. See [HAL_LLAMA_CPP_BAREMETAL.md](HAL_LLAMA_CPP_BAREMETAL.md) for bare-metal integration design.

## References

- [llama.cpp](https://github.com/ggerganov/llama.cpp)
- [GGUF format](https://github.com/ggerganov/ggml/blob/master/docs/gguf.md)
- HAL: `hal/src/gpu.rs`
