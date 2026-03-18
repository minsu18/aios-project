/* AIOS HAL bare-metal llama inference shim.
 * Provides a minimal C ABI for FFI from Rust. Stub implementation until
 * llama.cpp is built for aarch64-none. Replace this with real bindings
 * when libllama.a is available.
 *
 * Build: see hal-bare/build.rs
 */

#include <stddef.h>

/* aios_llama_inference: run inference. Returns bytes written or -1 on error.
 * Stub: returns -1 so caller falls back to host bridge (Ollama via serial).
 * Real: replace with llama_load/decode; return output length. */
int aios_llama_inference(const char *prompt, char *out, size_t out_len) {
    (void)prompt;
    (void)out;
    (void)out_len;
    return -1;  /* Stub: no on-device LLM. Use bridge (simulate-rpi-bridge.sh) for Ollama. */
}
