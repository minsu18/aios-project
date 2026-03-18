# llama.cpp Bare-Metal Integration (kernel-rpi)

Design for on-device LLM inference on Raspberry Pi without an OS.

## Prerequisites

1. **Global allocator** — kernel-rpi has bump allocator (128KB). llama.cpp would need ~64–256MB heap for real models.
2. **Toolchain** — `aarch64-elf-gcc` (Homebrew: `brew install aarch64-elf-gcc`) or `aarch64-none-elf-gcc` (ARM GNU Toolchain).
3. **llama.cpp** — Requires newlib (stdio, malloc). Homebrew aarch64-elf has no newlib; use ARM toolchain for full build.
4. **Model** — Embed small GGUF or load from SD via `kernel-rpi/src/block.rs`.

## Integration Steps

1. ~~Add bump allocator~~ — Done. `kernel-rpi/src/allocator.rs`
2. ~~FFI scaffold~~ — `hal-bare/c/llama_shim.c` provides `aios_llama_inference()`. Built when aarch64-elf-gcc or aarch64-none-elf-gcc available. Returns stub until libllama linked.
3. **Full llama.cpp** — Run `tools/build-llama-baremetal.sh`. Requires toolchain with newlib (ARM GNU Toolchain from developer.arm.com). ggml/llama need stdio.h, malloc.
4. **Real shim** — Replace stub in `hal-bare/c/llama_shim.c` with llama_load_model_from_memory, llama_decode, etc. Provide sbrk and file I/O stubs in kernel.

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

### Full libllama.a Build — Blockers

1. **-ffreestanding**: C++ headers (`<mutex>`, `<cmath>`, `<string>`) error with "not available in freestanding mode". Removing it requires hosted libc.
2. **std::strto***: newlib's `<cstdlib>` does not put `strtol`, `strtod`, `strtof` in `std::`; `basic_string.h` (stoi/stod/stof) fails.
3. **Generic target**: `CMAKE_SYSTEM_NAME Generic` limits includes; `clock_gettime`, `CLOCK_MONOTONIC` missing for ggml.c.
4. **C++ STL**: gguf.cpp, ggml-backend.cpp use std::map, std::string, std::mutex — require full hosted C++.

### Alternative: RPi OS (aarch64-linux-gnu)

Building for Linux (Raspberry Pi OS) instead of bare-metal avoids the above:

```bash
# On Ubuntu/Debian: sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu
# Script will prefer aarch64-linux-gnu-gcc when available
./tools/build-llama-baremetal.sh
```

Produces `libllama.a` for RPi under Linux; kernel-rpi (bare-metal) would need a different integration path.
