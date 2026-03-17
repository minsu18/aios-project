#!/usr/bin/env bash
# AIOS QEMU launcher
# Builds disk image and boots in QEMU.
# Requires: Rust nightly, qemu-system-x86_64

set -e
cd "$(dirname "$0")/.."
cargo run -p aios-boot --release
