//! AIOS Drivers
//!
//! Hardware driver modules. Each driver implements the `Driver` trait
//! and is invoked by the HAL when the kernel has initialized the hardware.
//!
//! - Default: stubs for bare-metal
//! - `host` feature: real Linux bindings (V4L2 camera, ALSA audio, std::net comms)

#![cfg_attr(not(feature = "host"), no_std)]

#[cfg(not(feature = "host"))]
extern crate alloc;

use core::fmt;

/// Common driver error
#[derive(Debug, Clone)]
pub struct DriverError(&'static str);

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Driver interface. Implementations are registered with the HAL.
pub trait Driver {
    /// Driver name (e.g. "camera", "audio")
    fn name(&self) -> &'static str;

    /// Initialize the driver. Called once at boot.
    fn init(&mut self) -> Result<(), DriverError>;

    /// Check if driver is ready for use
    fn ready(&self) -> bool;
}

pub mod camera;
pub mod audio;
pub mod comms;

pub use camera::capture_image;
pub use audio::{play, capture};
pub use comms::{udp_send, udp_recv};
