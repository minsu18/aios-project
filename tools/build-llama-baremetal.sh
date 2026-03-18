#!/bin/bash
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
# ggml-impl-cinttypes (PRI* for aarch64-none-elf)
if [[ -f "$PATCH_DIR/ggml-impl-cinttypes.patch" ]]; then
    if patch -p1 -d "$LLAMA_SRC" -s -f < "$PATCH_DIR/ggml-impl-cinttypes.patch" 2>/dev/null; then
        echo "Applied ggml-impl-cinttypes.patch"
    fi
fi
# gguf_init_from_buffer for loading model from memory (aios_llama_init_from_memory)
if python3 "$ROOT/tools/patch-gguf-init-from-buffer.py" 2>/dev/null; then
    :
fi
# ggml-cpu bare-metal: pthread stubs (single-thread only)
BAREMETAL_DEF=""
if [[ "$CC" == *none-elf* ]] && [[ "$CC" != *linux* ]]; then
    BAREMETAL_DEF="-DGGML_BARE_METAL"
    GGML_CPU="$LLAMA_SRC/ggml/src/ggml-cpu/ggml-cpu.c"
    INSERT_FILE="$ROOT/tools/ggml-cpu-baremetal-insert.txt"
    if [[ -f "$GGML_CPU" ]] && [[ -f "$INSERT_FILE" ]] && ! grep -q 'GGML_BARE_METAL.*no-op stubs' "$GGML_CPU" 2>/dev/null; then
        python3 -c "
import sys
path = sys.argv[1]
insert_path = sys.argv[2]
with open(path) as f: c = f.read()
with open(insert_path) as f: ins = f.read()
old = '#else\n\n#include <pthread.h>'
new = ins + '#include <pthread.h>'
if old in c and ins not in c:
    c = c.replace(old, new, 1)
    with open(path, 'w') as f: f.write(c)
    sys.exit(0)
sys.exit(1)
" "$GGML_CPU" "$INSERT_FILE" 2>/dev/null && echo "Applied ggml-cpu bare-metal pthread stubs" || true
    fi
    # Stub dlfcn.h for ggml-backend-dl (no dynamic loading on bare-metal)
    DL_H="$LLAMA_SRC/ggml/src/ggml-backend-dl.h"
    if [[ -f "$DL_H" ]] && ! grep -q 'GGML_BARE_METAL' "$DL_H" 2>/dev/null; then
        python3 -c "
import sys
path = sys.argv[1]
with open(path) as f: c = f.read()
old = '#else\n#    include <dlfcn.h>\n#    include <unistd.h>'
new = '''#elif defined(GGML_BARE_METAL)
#    include \"dlfcn-baremetal.h\"
#else
#    include <dlfcn.h>
#    include <unistd.h>'''
if old in c and 'GGML_BARE_METAL' not in c:
    c = c.replace(old, new, 1)
    with open(path, 'w') as f: f.write(c)
    sys.exit(0)
sys.exit(1)
" "$DL_H" 2>/dev/null && echo "Applied ggml-backend-dl bare-metal stub" || true
    fi
    # Same for ggml-backend-reg.cpp (direct dlfcn.h include)
    REG_CPP="$LLAMA_SRC/ggml/src/ggml-backend-reg.cpp"
    if [[ -f "$REG_CPP" ]] && ! grep -q 'GGML_BARE_METAL' "$REG_CPP" 2>/dev/null; then
        python3 -c "
import sys
path = sys.argv[1]
with open(path) as f: c = f.read()
old = '#elif defined(__APPLE__)\n#    include <mach-o/dyld.h>\n#    include <dlfcn.h>\n#else\n#    include <dlfcn.h>\n#    include <unistd.h>'
new = '''#elif defined(GGML_BARE_METAL)
#    include \"dlfcn-baremetal.h\"
#elif defined(__APPLE__)
#    include <mach-o/dyld.h>
#    include <dlfcn.h>
#else
#    include <dlfcn.h>
#    include <unistd.h>'''
if old in c and 'GGML_BARE_METAL' not in c:
    c = c.replace(old, new, 1)
    with open(path, 'w') as f: f.write(c)
    sys.exit(0)
sys.exit(1)
" "$REG_CPP" 2>/dev/null && echo "Applied ggml-backend-reg bare-metal stub" || true
    fi
    if [[ -f "$GGML_CPU" ]] && ! grep -q 'Bare-metal: no pthread, single-thread only' "$GGML_CPU" 2>/dev/null; then
        python3 -c "
import sys, re
path = sys.argv[1]
with open(path) as f: c = f.read()
pat = r'(#if defined\\(__EMSCRIPTEN__\\) && !defined\\(__EMSCRIPTEN_PTHREADS__\\)\s+// Emscripten[^\n]*\n\s+n_threads = 1;)\s+(#endif)'
add = r'''\1
#elif defined(GGML_BARE_METAL)
    // Bare-metal: no pthread, single-thread only
    n_threads = 1;
\2'''
if re.search(pat, c) and 'Bare-metal: no pthread, single-thread only' not in c:
    c = re.sub(pat, add, c, count=1)
    with open(path, 'w') as f: f.write(c)
    sys.exit(0)
sys.exit(1)
" "$GGML_CPU" 2>/dev/null && echo "Applied ggml-cpu n_threads=1 for bare-metal" || true
    fi
    # llama-quant.cpp: wrap std::thread blocks in #if !GGML_BARE_METAL (bare-metal has no std::thread)
    QUANT_CPP="$LLAMA_SRC/src/llama-quant.cpp"
    if [[ -f "$QUANT_CPP" ]] && ! grep -q '#if !defined(GGML_BARE_METAL).*block_size' "$QUANT_CPP" 2>/dev/null; then
        python3 -c "
import sys
path = sys.argv[1]
with open(path) as f: c = f.read()
# 1) First function: add #if at start of multi-thread block (after return;)
old1 = '''        return;
    }

    size_t block_size;
    if (tensor->type == GGML_TYPE_F16 ||'''
new1 = '''        return;
    }

#if !defined(GGML_BARE_METAL)
    size_t block_size;
    if (tensor->type == GGML_TYPE_F16 ||'''
# 2) Add #endif before the closing } of the function (before 'do we allow this tensor')
old2 = '''    for (auto & w : workers) { w.join(); }
    workers.clear();
}

//
// do we allow this tensor to be quantized?'''
new2 = '''    for (auto & w : workers) { w.join(); }
    workers.clear();
#endif
}

//
// do we allow this tensor to be quantized?'''
# 3) Force nthread<2 when GGML_BARE_METAL so we take the early return
old3 = '    if (nthread < 2) {'
new3 = '''    if (nthread < 2
#if defined(GGML_BARE_METAL)
        || true  /* bare-metal: no std::thread */
#endif
    ) {'''
if old1 in c and '#if !defined(GGML_BARE_METAL)' not in c:
    c = c.replace(old1, new1, 1)
if old2 in c:
    c = c.replace(old2, new2, 1)
if old3 in c and c.count('GGML_BARE_METAL') < 2:
    c = c.replace(old3, new3, 1)
with open(path, 'w') as f: f.write(c)
sys.exit(0)
" "$QUANT_CPP" 2>/dev/null && echo "Applied llama-quant bare-metal (dequant multi-thread wrap)" || true
    fi
    # 4) Second function (llama_tensor_quantize_impl): wrap mutex/workers block
    if [[ -f "$QUANT_CPP" ]] && ! grep -q '#if !defined(GGML_BARE_METAL).*std::mutex' "$QUANT_CPP" 2>/dev/null; then
        python3 -c "
import sys
path = sys.argv[1]
with open(path) as f: c = f.read()
old = '''        return new_size;
    }

    std::mutex mutex;
    int64_t counter = 0;'''
new = '''        return new_size;
    }

#if !defined(GGML_BARE_METAL)
    std::mutex mutex;
    int64_t counter = 0;'''
old2 = '''    if (nthread < 2) {
        // single-thread'''
new2 = '''    if (nthread < 2
#if defined(GGML_BARE_METAL)
        || true  /* bare-metal: no std::thread */
#endif
    ) {
        // single-thread'''
old3 = '''    workers.clear();
    if (!valid) {
        throw std::runtime_error(\"quantized data validation failed\");
    }
    return new_size;
}

//
// imatrix requirement check'''
new3 = '''    workers.clear();
    if (!valid) {
        throw std::runtime_error(\"quantized data validation failed\");
    }
    return new_size;
#endif
    return 0; /* unreachable when GGML_BARE_METAL */
}

//
// imatrix requirement check'''
if old in c and '#if !defined(GGML_BARE_METAL)' not in c.split('llama_tensor_quantize_impl')[1][:500]:
    c = c.replace(old, new, 1)
if old2 in c and c.count('GGML_BARE_METAL') < 3:
    c = c.replace(old2, new2, 1)
if old3 in c:
    c = c.replace(old3, new3, 1)
with open(path, 'w') as f: f.write(c)
sys.exit(0)
" "$QUANT_CPP" 2>/dev/null && echo "Applied llama-quant bare-metal (quantize multi-thread wrap)" || true
    fi
    # llama-model-loader.cpp: replace std::async validation with sync for bare-metal
    LOADER_CPP="$LLAMA_SRC/src/llama-model-loader.cpp"
    if [[ -f "$LOADER_CPP" ]] && ! grep -q 'validation_result.emplace_back(cur,' "$LOADER_CPP" 2>/dev/null; then
        python3 -c "
import sys
path = sys.argv[1]
with open(path) as f: c = f.read()
old1 = 'std::vector<std::future<std::pair<ggml_tensor *, bool>>> validation_result;'
new1 = '''#if defined(GGML_BARE_METAL)
    std::vector<std::pair<ggml_tensor *, bool>> validation_result;
#else
    std::vector<std::future<std::pair<ggml_tensor *, bool>>> validation_result;
#endif'''
old2 = '''            if (check_tensors) {
                validation_result.emplace_back(std::async(std::launch::async, [cur, data, n_size] {
                    return std::make_pair(cur, ggml_validate_row_data(cur->type, data, n_size));
                }));
            }'''
new2 = '''            if (check_tensors) {
#if defined(GGML_BARE_METAL)
                validation_result.emplace_back(cur, ggml_validate_row_data(cur->type, data, n_size));
#else
                validation_result.emplace_back(std::async(std::launch::async, [cur, data, n_size] {
                    return std::make_pair(cur, ggml_validate_row_data(cur->type, data, n_size));
                }));
#endif
            }'''
old3 = '''                if (check_tensors) {
                    validation_result.emplace_back(std::async(std::launch::async, [cur, n_size] {
                        return std::make_pair(cur, ggml_validate_row_data(cur->type, cur->data, n_size));
                    }));
                }'''
new3 = '''                if (check_tensors) {
#if defined(GGML_BARE_METAL)
                    validation_result.emplace_back(cur, ggml_validate_row_data(cur->type, cur->data, n_size));
#else
                    validation_result.emplace_back(std::async(std::launch::async, [cur, n_size] {
                        return std::make_pair(cur, ggml_validate_row_data(cur->type, cur->data, n_size));
                    }));
#endif
                }'''
old4 = '''    for (auto & future : validation_result) {
        auto result = future.get();'''
new4 = '''    for (auto & future : validation_result) {
#if defined(GGML_BARE_METAL)
        auto result = future;
#else
        auto result = future.get();
#endif'''
ok = False
if old1 in c and 'GGML_BARE_METAL' not in c[:c.find(old1)+500]:
    c = c.replace(old1, new1, 1); ok = True
if old2 in c: c = c.replace(old2, new2, 1); ok = True
if old3 in c: c = c.replace(old3, new3, 1); ok = True
if old4 in c: c = c.replace(old4, new4, 1); ok = True
if ok:
    with open(path, 'w') as f: f.write(c)
    sys.exit(0)
sys.exit(1)
" "$LOADER_CPP" 2>/dev/null && echo "Applied llama-model-loader bare-metal sync validation" || true
    fi
fi
# ggml-threading: bare-metal no-op (no std::mutex); applied in-place to avoid patch-fail
THREADING_CPP="$LLAMA_SRC/ggml/src/ggml-threading.cpp"
if [[ -f "$THREADING_CPP" ]] && ! grep -q 'Bare-metal / no-threads' "$THREADING_CPP" 2>/dev/null; then
    # Wrap mutex usage with #if for hosted; add no-op branch for bare-metal
    {
        echo '#include "ggml-threading.h"'
        echo '#if defined(__linux__) || defined(__APPLE__) || defined(_WIN32) || defined(__ANDROID__)'
        echo '#include <mutex>'
        echo ''
        echo 'std::mutex ggml_critical_section_mutex;'
        echo ''
        echo 'void ggml_critical_section_start() {'
        echo '    ggml_critical_section_mutex.lock();'
        echo '}'
        echo ''
        echo 'void ggml_critical_section_end(void) {'
        echo '    ggml_critical_section_mutex.unlock();'
        echo '}'
        echo '#else'
        echo '/* Bare-metal / no-threads: no-op critical section */'
        echo 'void ggml_critical_section_start(void) { (void)0; }'
        echo 'void ggml_critical_section_end(void) { (void)0; }'
        echo '#endif'
    } > "$THREADING_CPP"
    echo "Applied ggml-threading bare-metal stub"
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
set(CMAKE_C_FLAGS   "-O2 -include ${ROOT}/tools/llama-baremetal-stubs.h -I${ROOT}/tools ${BAREMETAL_DEF}" CACHE STRING "")
set(CMAKE_CXX_FLAGS "-O2 -include ${ROOT}/tools/llama-baremetal-stubs.h -I${ROOT}/tools ${BAREMETAL_DEF}" CACHE STRING "")
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

# ggml libs (llama depends on ggml; must all be linked)
GGML_DIR="$LLAMA_SRC/build/ggml/src"
copy_ggml() {
    for f in libggml.a libggml-cpu.a libggml-base.a; do
        if [[ -f "$GGML_DIR/$f" ]]; then
            cp "$GGML_DIR/$f" "$BUILD_DIR/"
        fi
    done
}

if [[ -f "$LLAMA_SRC/build/bin/libllama.a" ]]; then
    cp "$LLAMA_SRC/build/bin/libllama.a" "$BUILD_DIR/"
    copy_ggml
    echo ""
    echo "Built: $BUILD_DIR/libllama.a + ggml libs"
    echo "Next: Implement hal-bare/c/llama_shim.c to call llama API; add sbrk/model-load stubs."
elif [[ -f "$LLAMA_SRC/build/src/libllama.a" ]]; then
    cp "$LLAMA_SRC/build/src/libllama.a" "$BUILD_DIR/"
    copy_ggml
    echo ""
    echo "Built: $BUILD_DIR/libllama.a + ggml libs"
    echo "Next: Implement hal-bare/c/llama_shim.c to call llama API; add sbrk/model-load stubs."
elif [[ -f "$LLAMA_SRC/build/libllama.a" ]]; then
    cp "$LLAMA_SRC/build/libllama.a" "$BUILD_DIR/"
    copy_ggml
    echo ""
    echo "Built: $BUILD_DIR/libllama.a + ggml libs"
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
