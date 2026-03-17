//! GPU abstraction — inference, render
//!
//! On-device inference is designed to integrate with llama.cpp for bare-metal.
//! See docs/HAL_LLAMA_CPP.md for build and integration instructions.

/// Run on-device inference via llama.cpp (when linked).
///
/// # Design
///
/// - `model_id`: Path to GGUF model or model name (e.g. "tinyllama").
/// - `input`: Prompt bytes (UTF-8). For chat, use format expected by the model.
/// - Returns generated text as UTF-8 bytes.
///
/// # llama.cpp integration (future)
///
/// When `aios-hal` is built with feature `llama`, this calls into llama.cpp:
///
/// ```c
/// // Pseudo-interface (llama.cpp C API)
/// struct llama_context *ctx = llama_new_from_file(model_path, params);
/// llama_decode(ctx, ...);
/// // ... token generation loop ...
/// ```
///
/// Build: `cargo build -p aios-hal --features llama`
/// Requires: libllama.a or libllama.so, built for target (e.g. aarch64 for RPi).
#[cfg(not(feature = "llama"))]
pub fn inference(_model_id: &str, _input: &[u8]) -> Result<Vec<u8>, &'static str> {
    Err("llama.cpp not linked. Build with --features llama. See docs/HAL_LLAMA_CPP.md")
}

/// llama.cpp-backed inference via llama-cpp-2
#[cfg(feature = "llama")]
pub fn inference(model_id: &str, input: &[u8]) -> Result<Vec<u8>, &'static str> {
    let prompt = std::str::from_utf8(input).map_err(|_| "input is not valid UTF-8")?;
    super::llama_inference::run_inference(model_id, prompt)
        .map_err(|_| "llama inference failed; check model path and logs")
}

/// Render framebuffer to display (placeholder)
pub fn render(_framebuffer: &[u8]) -> Result<(), &'static str> {
    Err("not implemented")
}
