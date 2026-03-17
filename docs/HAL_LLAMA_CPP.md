# HAL + llama.cpp Integration

This document describes how to connect AIOS HAL's `gpu.inference` to llama.cpp for bare-metal on-device inference.

## Overview

- **hal/src/gpu.rs** exposes `inference(model_id, input) -> Result<Vec<u8>, &str>`.
- When built with `--features llama`, the implementation will call into llama.cpp via C FFI.
- Models: GGUF format (e.g. TinyLlama, Phi-2, Qwen2-0.5B quantized).

## Build Steps (Future)

### 1. Build llama.cpp for target

```bash
git clone https://github.com/ggerganov/llama.cpp
cd llama.cpp
mkdir build && cd build

# For host (x86_64)
cmake .. -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF -DBUILD_SHARED_LIBS=OFF
cmake --build . --config Release

# For Raspberry Pi (aarch64)
cmake .. -DCMAKE_SYSTEM_PROCESSOR=aarch64 -DLLAMA_BUILD_TESTS=OFF -DBUILD_SHARED_LIBS=OFF
cmake --build . --config Release
```

Output: `libllama.a` (static) or `libllama.so` (shared).

### 2. Configure aios-hal

Add to `hal/build.rs` (when implementing):

```rust
fn main() {
    let llama_path = env::var("LLAMA_CPP_PATH").unwrap_or_else(|_| "path/to/llama.cpp".into());
    println!("cargo:rustc-link-search=native={}/build", llama_path);
    println!("cargo:rustc-link-lib=static=llama");
    println!("cargo:rerun-if-changed=build.rs");
}
```

### 3. FFI bindings

Create `hal/src/llama_ffi.rs` with `#[repr(C)]` structs and `extern "C"` declarations matching `llama.h`:

- `llama_backend_init()`
- `llama_load_model_from_file()`
- `llama_new_context_with_model()`
- `llama_decode()`
- `llama_sampling_*` for token selection
- `llama_backend_free()`

### 4. Wire gpu::inference

In `gpu.rs` (with `#[cfg(feature = "llama")]`):

1. Parse `model_id` as path (or resolve from `AIOS_MODEL_PATH`).
2. Load model and context.
3. Tokenize input (use llama.cpp tokenizer or integrate sentencepiece).
4. Run decode loop until EOS or max_tokens.
5. Decode output tokens to UTF-8.

## Target Considerations

| Target    | Notes                                                |
|----------|------------------------------------------------------|
| x86_64   | AVX2/AVX-512 if available; CPU-only is fine for dev |
| aarch64  | RPi 4: use quantized Q4_K_M; ~2GB RAM for 1B model   |
| Bare-metal | No OS; needs custom allocator, no pthread          |

## References

- [llama.cpp](https://github.com/ggerganov/llama.cpp)
- [GGUF format](https://github.com/ggerganov/ggml/blob/master/docs/gguf.md)
- HAL: `hal/src/gpu.rs`
