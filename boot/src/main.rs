//! AIOS Boot — Disk image and QEMU launcher
//!
//! Run with: cargo run -p aios-boot
//!
//! VM specs via env:
//!   AIOS_VM_CPUS=2   (default: 1)
//!   AIOS_VM_MEM=512 (default: 256, in MB)

use std::env;
use std::process::Command;

fn main() {
    let bios_path = env!("BIOS_PATH");

    let cpus = env::var("AIOS_VM_CPUS").unwrap_or_else(|_| "1".into());
    let mem_mb = env::var("AIOS_VM_MEM").unwrap_or_else(|_| "256".into());

    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-serial").arg("mon:stdio");
    cmd.arg("-drive").arg(format!("format=raw,file={bios_path}"));
    cmd.arg("-m").arg(format!("{mem_mb}M"));
    cmd.arg("-smp").arg(&cpus);
    cmd.arg("-display").arg("none"); // No graphical window; serial only

    let status = cmd.status().expect("failed to run QEMU");
    std::process::exit(status.code().unwrap_or(1));
}
