# llama.cpp Bare-Metal Integration (kernel-rpi)

Design for on-device LLM inference on Raspberry Pi without an OS.

## Prerequisites

1. **Global allocator** — kernel-rpi has bump allocator (128KB). llama.cpp would need ~64–256MB heap for real models.
2. **Toolchain** — `aarch64-elf-gcc` (Homebrew: `brew install aarch64-elf-gcc`) or `aarch64-none-elf-gcc` (ARM GNU Toolchain).
3. **llama.cpp** — Requires newlib (stdio, malloc). Homebrew aarch64-elf has no newlib; use ARM toolchain for full build.
4. **Model** — Embed small GGUF or load from SD via `kernel-rpi/src/block.rs`.

## Integration Steps

1. ~~Add bump allocator~~ — Done. `kernel-rpi/src/allocator.rs`
2. ~~FFI scaffold~~ — `hal-bare/c/llama_shim.c` provides `aios_llama_inference()`. Built when aarch64-elf-gcc or aarch64-none-elf-gcc available.
3. ~~Full llama.cpp + kernel link~~ — `tools/build-llama-baremetal.sh` builds libllama.a + ggml libs. `--features llama` links them into the kernel; sbrk.c and syscall_stubs.c provide newlib stubs.
4. ~~**Real shim (inference loop)**~~ — Implemented: `aios_llama_inference` does tokenize → decode loop → greedy sample → detokenize. `aios_llama_init_from_file(path)` loads from path. `aios_llama_init_from_memory(buf, len)` loads GGUF from memory via `gguf_init_from_buffer` (patched in gguf.cpp). Kernel `load_model` command reads MODEL.GGUF from SD root into heap and calls `init_from_memory`. SD works on both real RPi 4 (EMMC2) and QEMU (bcm2835-sdhost via `--sd`).

## aarch64-none-elf on macOS

1. Download from https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads
2. Choose: **AArch64 bare-metal target (aarch64-none-elf)**, macOS
3. Install (default: `/Applications/ArmGNUToolchain/.../aarch64-none-elf/bin/`)
4. Add to PATH, or `build-llama-baremetal.sh` auto-adds if not found:
   ```bash
   export PATH="/Applications/ArmGNUToolchain/15.2.rel1/aarch64-none-elf/bin:$PATH"
   ```

## Current Status

- **FFI wired**: `hal-bare` builds C shim; `inference()` calls it. `ask` without bridge prints stub message.
- **Serial Bridge**: `./tools/simulate-rpi-bridge.sh` for host Ollama. `ask` works with real LLM.
- **Heap**: `--features llama` expands to 64MB (for future libllama).
- **Patches**: `tools/patches/ggml-impl-cinttypes.patch` adds `#include <cinttypes>` to ggml-impl.h for PRI* macros.
- **Stubs**: `tools/llama-baremetal-stubs.h` provides PRId64/PRIi64/PRIu64 for aarch64 LP64, and `__throw_*` for -fno-exceptions.

### Full libllama.a Build — Status

**Resolved** (via `tools/llama-baremetal-stubs.h` and build script):

- `__throw_domain_error`, `__throw_bad_array_new_length` — added stubs
- `std::strtol`, `std::strtod`, etc. — `using ::strtol` etc. in `std` (newlib quirk)
- `clock_gettime`, `CLOCK_MONOTONIC` — C stub in header
- `std::mutex` in ggml-threading.cpp — bare-metal no-op (`#else` branch)
- **ggml-cpu** — `tools/ggml-cpu-baremetal-insert.txt` pthread stubs; `n_threads=1` for bare-metal
- **ggml-backend-dl/reg** — `tools/dlfcn-baremetal.h` replaces `<dlfcn.h>`
- **llama-quant.cpp** — `#if !GGML_BARE_METAL` around std::thread blocks; single-thread path when bare-metal
- **llama-model-loader.cpp** — sync validation instead of `std::async` when `GGML_BARE_METAL`

**libllama.a** — builds successfully for aarch64-none-elf (ARM GNU Toolchain + newlib). Output: `target/llama-build/libllama.a` + `libggml.a`, `libggml-cpu.a`, `libggml-base.a`.

### Kernel Link (--features llama)

The kernel links libllama when:

1. `target/llama-build/libllama.a` (+ ggml libs) exists (from `tools/build-llama-baremetal.sh`)
2. `cargo build -p aios-kernel-rpi --target aarch64-unknown-none --features llama`

**Components**:

- `hal-bare/c/llama_shim.c` — FFI to llama API (stub or full when linked)
- `hal-bare/c/sbrk.c` — newlib `_sbrk` (32MB C heap)
- `hal-bare/c/syscall_stubs.c` — newlib syscall stubs (`_write`, `posix_memalign`, `sysconf`, `lroundf`, etc.)

### Alternative: RPi OS (aarch64-linux-gnu)

Building for Linux (Raspberry Pi OS) instead of bare-metal avoids the above:

```bash
# On Ubuntu/Debian: sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
# Script will prefer aarch64-linux-gnu-gcc when available
./tools/build-llama-baremetal.sh
```

Produces `libllama.a` for RPi under Linux; kernel-rpi (bare-metal) would need a different integration path.
