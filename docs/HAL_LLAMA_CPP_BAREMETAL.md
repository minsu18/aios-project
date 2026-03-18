# llama.cpp Bare-Metal Integration (kernel-rpi)

Design for on-device LLM inference on Raspberry Pi without an OS.

## Prerequisites

1. **Global allocator** — llama.cpp requires malloc. kernel-rpi needs `#[global_allocator]` (bump or linked_list) with ~64–256MB heap.
2. **llama.cpp for aarch64-none** — Cross-compile with `-target aarch64-none-elf`, no libc. Use `LLAMA_NO_MALLOC=0` and provide allocator hooks if needed.
3. **Model in memory** — Embed a small quantized GGUF (e.g. 64MB) in the binary, or load from SD/MMC once a block driver exists.

## Integration Steps

1. Add bump allocator to kernel-rpi (`alloc` crate, heap in link.ld).
2. Build llama.cpp as a static lib for `aarch64-unknown-none`.
3. Add `hal-bare` feature `llama` that links `libllama.a` and exposes `inference(prompt, out_buf)`.
4. kernel-rpi `ask` command calls `hal_bare::inference`; on success, output to UART.

## Current Status

- `hal_bare::inference` is a stub; returns "LLM not available".
- `ask <q>` command exists and calls the stub.
- Allocator and full llama integration: TODO.
