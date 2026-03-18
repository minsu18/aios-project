//! Build llama inference C shim for aarch64 bare-metal.
//! Links libllama_shim.a so that inference() can call into C.
//! See docs/HAL_LLAMA_CPP_BAREMETAL.md for full llama.cpp integration.
//!
//! With feature "llama" + libllama.a: compiles with newlib, links libllama.
//! Without: compiles minimal stub with -nostdlib.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(llama_shim)");
    println!("cargo:rustc-check-cfg=cfg(llama_linked)");
    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("aarch64") {
        return;
    }

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.parent().unwrap();
    let llama_build = workspace.join("target").join("llama-build");
    let libllama = llama_build.join("libllama.a");
    let llama_include = llama_build.join("llama.cpp").join("include");
    let ggml_include = llama_build.join("llama.cpp").join("ggml").join("include");
    let shim_c = manifest.join("c").join("llama_shim.c");
    let sbrk_c = manifest.join("c").join("sbrk.c");
    let syscall_stubs_c = manifest.join("c").join("syscall_stubs.c");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lib = out_dir.join("libllama_shim.a");

    let want_llama = env::var("CARGO_FEATURE_LLAMA").is_ok();
    let have_libllama = libllama.is_file() && llama_include.is_dir();
    let link_llama = want_llama && have_libllama;

    let (obj_paths, success) = if link_llama {
        // Build with libllama: newlib, sbrk, llama API
        let cc_opt = if Command::new("aarch64-none-elf-gcc").arg("--version").status().is_ok() {
            Some("aarch64-none-elf-gcc")
        } else if Command::new("aarch64-elf-gcc").arg("--version").status().is_ok() {
            Some("aarch64-elf-gcc")
        } else {
            println!("cargo:warning=Need aarch64-none-elf-gcc for libllama. Run tools/build-llama-baremetal.sh first.");
            None
        };

        if let Some(cc) = cc_opt {
            let mut objs = Vec::new();
            let stubs_h = workspace.join("tools").join("llama-baremetal-stubs.h");
            let base_args = [
                "-c",
                "-O2",
                "-DAIOS_LLAMA_LINKED",
                "-I",
                llama_include.to_str().unwrap(),
                "-I",
                ggml_include.to_str().unwrap(),
                "-include",
                stubs_h.to_str().unwrap(),
            ];
            let shim_o = out_dir.join("llama_shim.o");
            let sbrk_o = out_dir.join("sbrk.o");
            let stubs_o = out_dir.join("syscall_stubs.o");
            let s1 = Command::new(cc)
                .args(&base_args)
                .args(["-o", shim_o.to_str().unwrap(), shim_c.to_str().unwrap()])
                .status();
            let s2 = Command::new(cc)
                .args(["-c", "-O2", "-o", sbrk_o.to_str().unwrap(), sbrk_c.to_str().unwrap()])
                .status();
            let s3 = Command::new(cc)
                .args(["-c", "-O2", "-o", stubs_o.to_str().unwrap(), syscall_stubs_c.to_str().unwrap()])
                .status();
            if s1.as_ref().map(|x| x.success()).unwrap_or(false)
                && s2.as_ref().map(|x| x.success()).unwrap_or(false)
                && s3.as_ref().map(|x| x.success()).unwrap_or(false)
            {
                objs.push(shim_o);
                objs.push(sbrk_o);
                objs.push(stubs_o);
                (objs, true)
            } else {
                println!("cargo:warning=Failed to compile llama shim with libllama. Check include paths.");
                (Vec::new(), false)
            }
        } else {
            (Vec::new(), false)
        }
    } else {
        // Stub: minimal, no stdlib
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
        if status.as_ref().map(|s| !s.success()).unwrap_or(true) {
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
            (
                if s.as_ref().map(|x| x.success()).unwrap_or(false) {
                    vec![o]
                } else {
                    Vec::new()
                },
                s.as_ref().map(|x| x.success()).unwrap_or(false),
            )
        } else {
            (vec![out_dir.join("llama_shim.o")], true)
        }
    };

    if !success || obj_paths.is_empty() {
        if link_llama {
            println!("cargo:warning=Could not build llama shim with libllama. inference will use stub.");
        } else {
            println!("cargo:warning=Could not compile llama_shim.c. inference will use Rust stub.");
        }
        return;
    }

    // Create static lib
    let ar_success = {
        let mut ok = false;
        for ar in ["llvm-ar", "aarch64-none-elf-ar", "aarch64-elf-ar"] {
            let mut c = Command::new(ar);
            c.args(["rcs", lib.to_str().unwrap()]);
            for o in &obj_paths {
                c.arg(o.to_str().unwrap());
            }
            if c.status().as_ref().map(|s| s.success()).unwrap_or(false) {
                ok = true;
                break;
            }
        }
        ok
    };

    if ar_success {
        println!("cargo:rustc-link-lib=static=llama_shim");
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        println!("cargo:rustc-cfg=llama_shim");
        if link_llama {
            println!("cargo:rustc-link-search=native={}", llama_build.display());
            // llama depends on ggml; link in resolution order
            println!("cargo:rustc-link-lib=static=llama");
            println!("cargo:rustc-link-lib=static=ggml");
            println!("cargo:rustc-link-lib=static=ggml-cpu");
            println!("cargo:rustc-link-lib=static=ggml-base");
            println!("cargo:rustc-link-lib=static=stdc++");
            println!("cargo:rustc-link-lib=static=gcc");
            println!("cargo:rustc-link-lib=static=c");  /* newlib for abort, strcmp, etc. */
            // ARM toolchain lib paths for libstdc++.a and libgcc.a
            for gcc in ["aarch64-none-elf-gcc", "aarch64-elf-gcc"] {
                if let Ok(out) = Command::new(gcc).arg("-print-sysroot").output() {
                    if out.status.success() {
                        let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !sysroot.is_empty() {
                            let lib_path = PathBuf::from(&sysroot).join("lib");
                            if lib_path.is_dir() {
                                println!("cargo:rustc-link-search=native={}", lib_path.display());
                            }
                        }
                    }
                }
                if let Ok(out) = Command::new(gcc).arg("-print-file-name=libgcc.a").output() {
                    if out.status.success() {
                        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !path.is_empty() {
                            if let Some(parent) = PathBuf::from(&path).parent() {
                                if parent.is_dir() {
                                    println!("cargo:rustc-link-search=native={}", parent.display());
                                }
                            }
                        }
                    }
                }
            }
            println!("cargo:rustc-cfg=llama_linked");
        }
    }
}
