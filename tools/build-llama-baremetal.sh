#!/usr/bin/env bash
# Build llama.cpp for aarch64 bare-metal (kernel-rpi).
#
# Phase 1: Build minimal libllama.a with newlib (malloc, memcpy, etc.).
# Phase 2: Wire into hal-bare; provide sbrk and file I/O stubs in kernel.
#
# Requirements:
#   - aarch64-elf-gcc (brew install aarch64-elf-gcc) or aarch64-none-elf-gcc
#   - CMake, Git
#
# Usage: ./tools/build-llama-baremetal.sh

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT/target/llama-build"
LLAMA_SRC="$BUILD_DIR/llama.cpp"

# Prefer aarch64-none-elf (ARM GNU + newlib); fallback aarch64-elf. aarch64-linux-gnu for RPi OS.
# Auto-add ARM toolchain to PATH if installed in default location
if ! command -v aarch64-none-elf-gcc &>/dev/null; then
    for dir in /Applications/ArmGNUToolchain/*/aarch64-none-elf/bin; do
        if [[ -d $dir ]] && [[ -x $dir/aarch64-none-elf-gcc ]]; then
            export PATH="$dir:$PATH"
            break
        fi
    done
fi
CC=""
for c in aarch64-none-elf-gcc aarch64-linux-gnu-gcc aarch64-elf-gcc; do
    if command -v "$c" &>/dev/null; then
        CC="$c"
        break
    fi
done
if [[ -z "$CC" ]]; then
    echo "ERROR: Install aarch64-elf-gcc: brew install aarch64-elf-gcc"
    exit 1
fi
CXX="${CC/gcc/g++}"
echo "Using toolchain: $CC, $CXX"

echo ""
echo "=== AIOS llama.cpp bare-metal build ==="
echo ""

mkdir -p "$BUILD_DIR"
if [[ ! -d "$LLAMA_SRC" ]]; then
    echo "Cloning llama.cpp..."
    git clone --depth 1 https://github.com/ggerganov/llama.cpp.git "$LLAMA_SRC"
fi

# Apply bare-metal patches (PRI* macros for aarch64-none-elf)
PATCH_DIR="$ROOT/tools/patches"
if [[ -f "$PATCH_DIR/ggml-impl-cinttypes.patch" ]]; then
    if patch -p1 -d "$LLAMA_SRC" -s -f < "$PATCH_DIR/ggml-impl-cinttypes.patch" 2>/dev/null; then
        echo "Applied ggml-impl-cinttypes.patch"
    fi
fi

cd "$LLAMA_SRC"

# Toolchain: aarch64 bare-metal (Generic) or Linux if aarch64-linux-gnu available
TOOLCHAIN="$BUILD_DIR/aarch64-baremetal.cmake"
SYS_NAME="Generic"
[[ "$CC" == *linux* ]] && SYS_NAME="Linux"
cat > "$TOOLCHAIN" << EOF
set(CMAKE_SYSTEM_NAME ${SYS_NAME})
set(CMAKE_SYSTEM_PROCESSOR aarch64)
set(CMAKE_C_COMPILER   ${CC})
set(CMAKE_CXX_COMPILER ${CXX})
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)
set(CMAKE_C_FLAGS   "-O2" CACHE STRING "")
set(CMAKE_CXX_FLAGS "-O2 -include ${ROOT}/tools/llama-baremetal-stubs.h" CACHE STRING "")
EOF

echo "Configuring llama.cpp (minimal lib only)..."
if cmake -B build -DCMAKE_TOOLCHAIN_FILE="$TOOLCHAIN" \
    -DGGML_NATIVE=OFF \
    -DLLAMA_BUILD_TESTS=OFF \
    -DLLAMA_BUILD_EXAMPLES=OFF \
    -DLLAMA_BUILD_TOOLS=OFF \
    -DLLAMA_BUILD_SERVER=OFF \
    -DLLAMA_BUILD_COMMON=OFF \
    -DLLAMA_OPENSSL=OFF \
    -DBUILD_SHARED_LIBS=OFF \
    -DCMAKE_BUILD_TYPE=Release \
    2>&1; then
    echo ""
    echo "Building libllama.a..."
    cmake --build build -j --target llama 2>&1 || true
fi

if [[ -f "$LLAMA_SRC/build/bin/libllama.a" ]]; then
    cp "$LLAMA_SRC/build/bin/libllama.a" "$BUILD_DIR/"
    echo ""
    echo "Built: $BUILD_DIR/libllama.a"
    echo "Next: Implement hal-bare/c/llama_shim.c to call llama API; add sbrk/model-load stubs."
elif [[ -f "$LLAMA_SRC/build/libllama.a" ]]; then
    cp "$LLAMA_SRC/build/libllama.a" "$BUILD_DIR/"
    echo ""
    echo "Built: $BUILD_DIR/libllama.a"
else
    echo ""
    echo "Full libllama.a build not yet achieved."
    echo "Blocker: ggml/llama.cpp require stdio.h, malloc (newlib)."
    echo "  - Homebrew aarch64-elf-gcc is freestanding-only (no newlib)"
    echo "  - Use ARM GNU Toolchain (aarch64-none-elf) with newlib:"
    echo "    https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads"
    echo ""
    echo "Current: hal-bare FFI stub works. Use simulate-rpi-bridge.sh for 'ask' with Ollama."
fi
