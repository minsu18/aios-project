//! Inference abstraction for bare-metal LLM
//!
//! FFI path: llama_shim.c provides minimal C ABI. When built for aarch64 with
//! aarch64-none-elf-gcc or clang, inference() calls into the shim.
//! Full llama.cpp + GGUF: see docs/HAL_LLAMA_CPP_BAREMETAL.md, tools/build-llama-baremetal.sh

#[cfg(llama_shim)]
const OUT_BUF_LEN: usize = 512;

#[cfg(llama_shim)]
static mut OUT_BUF: [core::ffi::c_char; OUT_BUF_LEN] = [0; OUT_BUF_LEN];

#[cfg(llama_shim)]
extern "C" {
    fn aios_llama_inference(
        prompt: *const core::ffi::c_char,
        out: *mut core::ffi::c_char,
        out_len: usize,
    ) -> core::ffi::c_int;
}

/// Run on-device inference.
///
/// When llama_shim is built (aarch64 + ARM toolchain), calls C FFI.
/// Otherwise returns stub error. Full llama.cpp: tools/build-llama-baremetal.sh
pub fn inference(_input: &str) -> Result<&'static str, &'static str> {
    #[cfg(llama_shim)]
    {
        let prompt_ptr = _input.as_ptr() as *const core::ffi::c_char;
        let n = unsafe {
            aios_llama_inference(prompt_ptr, OUT_BUF.as_mut_ptr(), OUT_BUF_LEN)
        };
        if n >= 0 {
            let i = (n as usize).min(OUT_BUF_LEN);
            let bytes: &'static [u8] =
                unsafe { core::slice::from_raw_parts(OUT_BUF.as_ptr() as *const u8, i) };
            if let Ok(s) = core::str::from_utf8(bytes) {
                return Ok(s);
            }
        }
    }

    #[allow(unreachable_code)]
    Err("LLM not available. Requires aarch64-none-elf toolchain for FFI. See docs/HAL_LLAMA_CPP_BAREMETAL.md")
}
