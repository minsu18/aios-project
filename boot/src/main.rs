//! AIOS Boot — Disk image and QEMU launcher
//!
//! Run with: cargo run -p aios-boot [-- bios]

use std::env;
use std::process::Command;

fn main() {
    let bios_path = env!("BIOS_PATH");

    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-serial").arg("mon:stdio");
    cmd.arg("-drive")
        .arg(format!("format=raw,file={bios_path}"));

    let status = cmd.status().expect("failed to run QEMU");
    std::process::exit(status.code().unwrap_or(1));
}
