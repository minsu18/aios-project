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

/// llama.cpp-backed inference (stub for when feature is enabled)
#[cfg(feature = "llama")]
pub fn inference(model_id: &str, input: &[u8]) -> Result<Vec<u8>, &'static str> {
    // TODO: FFI to llama.cpp
    // - llama_backend_init()
    // - llama_load_model_from_file()
    // - llama_new_context_with_model()
    // - llama_decode() + llama_sampling in a loop
    // - llama_backend_free()
    let _ = (model_id, input);
    Err("llama feature enabled but FFI not yet implemented")
}

/// Render framebuffer to display (placeholder)
pub fn render(_framebuffer: &[u8]) -> Result<(), &'static str> {
    Err("not implemented")
}
