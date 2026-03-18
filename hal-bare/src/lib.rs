//! AIOS HAL — no_std bare-metal subset
//!
//! Minimal HAL interface for kernel integration. Used by kernel-rpi.
//! Full aios-hal will be linked when it supports no_std.

#![no_std]

pub mod timer;

/// Initialize HAL. Called by kernel after early boot.
pub fn init() {
    // Stub: timer and UART are initialized by kernel
}
