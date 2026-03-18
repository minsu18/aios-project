//! Build llama inference C shim for aarch64 bare-metal.
//! Links libllama_shim.a so that inference() can call into C.
//! See docs/HAL_LLAMA_CPP_BAREMETAL.md for full llama.cpp integration.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(llama_shim)");
    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("aarch64") {
        return;
    }

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let shim_c = manifest.join("c").join("llama_shim.c");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lib = out_dir.join("libllama_shim.a");

    let mut status = Command::new("aarch64-elf-gcc")
        .args([
            "-c",
            "-o",
            out_dir.join("llama_shim.o").to_str().unwrap(),
            "-ffreestanding",
            "-nostdlib",
            "-O2",
            shim_c.to_str().unwrap(),
        ])
        .status();
    if status.as_ref().map(|s| !s.success()).unwrap_or(true) {
        status = Command::new("aarch64-none-elf-gcc")
            .args([
                "-c",
                "-o",
                out_dir.join("llama_shim.o").to_str().unwrap(),
                "-ffreestanding",
                "-nostdlib",
                "-O2",
                shim_c.to_str().unwrap(),
            ])
            .status();
    }

    let (obj_path, success) = if status.as_ref().map(|s| s.success()).unwrap_or(false) {
        (out_dir.join("llama_shim.o"), true)
    } else {
        // Fallback: clang
        let o = out_dir.join("llama_shim.o");
        let s = Command::new("clang")
            .args([
                "--target=aarch64-none-elf",
                "-c",
                "-o",
                o.to_str().unwrap(),
                "-ffreestanding",
                "-nostdlib",
                "-O2",
                shim_c.to_str().unwrap(),
            ])
            .status();
        (o, s.map(|x| x.success()).unwrap_or(false))
    };

    if !success {
        println!("cargo:warning=Could not compile llama_shim.c. inference will use Rust stub.");
        return;
    }

    // Create static lib
    let ar_status = Command::new("llvm-ar")
        .args(["rcs", lib.to_str().unwrap(), obj_path.to_str().unwrap()])
        .status();

    let linked = if ar_status.as_ref().map(|s| s.success()).unwrap_or(false) {
        println!("cargo:rustc-link-lib=static=llama_shim");
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        true
    } else {
        let ar_status = Command::new("aarch64-elf-ar")
            .args(["rcs", lib.to_str().unwrap(), obj_path.to_str().unwrap()])
            .status();
        if ar_status.as_ref().map(|s| s.success()).unwrap_or(false) {
            println!("cargo:rustc-link-lib=static=llama_shim");
            println!("cargo:rustc-link-search=native={}", out_dir.display());
            true
        } else {
            let ar_status = Command::new("aarch64-none-elf-ar")
                .args(["rcs", lib.to_str().unwrap(), obj_path.to_str().unwrap()])
                .status();
            if ar_status.as_ref().map(|s| s.success()).unwrap_or(false) {
                println!("cargo:rustc-link-lib=static=llama_shim");
                println!("cargo:rustc-link-search=native={}", out_dir.display());
                true
            } else {
                false
            }
        }
    };

    if linked {
        println!("cargo:rustc-cfg=llama_shim");
    }
}
