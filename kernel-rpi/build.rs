use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let link_ld = manifest.join("link.ld");
    println!("cargo:rustc-link-arg=-T{}", link_ld.display());

    let boot_s = manifest.join("src").join("boot.S");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let boot_o = out_dir.join("boot.o");

    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("aarch64") {
        let mut status = Command::new("aarch64-elf-as")
            .args(["-c", boot_s.to_str().unwrap(), "-o", boot_o.to_str().unwrap()])
            .status();
        if status.as_ref().map(|s| !s.success()).unwrap_or(true) {
            status = Command::new("aarch64-none-elf-as")
                .args(["-c", boot_s.to_str().unwrap(), "-o", boot_o.to_str().unwrap()])
                .status();
        }
        if status.is_err() || !status.as_ref().unwrap().success() {
            // Fallback: try clang
            let status = Command::new("clang")
                .args([
                    "--target=aarch64-none-elf",
                    "-c",
                    boot_s.to_str().unwrap(),
                    "-o",
                    boot_o.to_str().unwrap(),
                ])
                .status();
            if status.is_err() || !status.as_ref().unwrap().success() {
                println!("cargo:warning=Could not compile boot.S. Install aarch64-none-elf toolchain.");
                return;
            }
        }

        println!("cargo:rustc-link-arg={}", boot_o.display());
    }
}
