//! Inference abstraction for bare-metal LLM
//!
//! Stub: returns error until llama.cpp is linked with custom allocator.
//! See docs/HAL_LLAMA_CPP.md for integration path.

/// Run on-device inference. Stub: not yet available for bare-metal.
///
/// Full integration requires: custom allocator, GGUF model in flash/RAM, llama.cpp aarch64 build.
#[inline(always)]
pub fn inference(_input: &str) -> Result<&'static str, &'static str> {
    Err("LLM not available. Requires llama.cpp + alloc for bare-metal. See docs/HAL_LLAMA_CPP.md")
}
