use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    // Build kernel for x86_64-unknown-none (manual; artifact deps need nightly)
    let kernel_path = build_kernel();
    if kernel_path.is_none() {
        println!("cargo:warning=Kernel build skipped (run: cargo build -p aios-kernel --target x86_64-unknown-none)");
        // Set placeholder so main.rs compiles; binary will fail at runtime if run
        println!("cargo:rustc-env=BIOS_PATH={}/bios.img", out_dir.display());
        return;
    }
    let kernel = kernel_path.unwrap();

    let bios_path = out_dir.join("bios.img");
    bootloader::BiosBoot::new(&kernel)
        .create_disk_image(&bios_path)
        .expect("failed to create BIOS disk image");

    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}

fn build_kernel() -> Option<PathBuf> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")?;
    let workspace_root = PathBuf::from(&manifest_dir).join("..");

    let status = std::process::Command::new("cargo")
        .current_dir(&workspace_root)
        .args([
            "build",
            "-p",
            "aios-kernel",
            "--target",
            "x86_64-unknown-none",
            "--release",
        ])
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }
    let target = workspace_root.join("target/x86_64-unknown-none/release/kernel");
    if target.exists() {
        Some(target)
    } else {
        None
    }
}
