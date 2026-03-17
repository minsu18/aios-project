//! AIOS Drivers
//!
//! Hardware driver modules. Each driver implements the `Driver` trait
//! and is invoked by the HAL when the kernel has initialized the hardware.
//!
//! Phase 2–3: Stub implementations. Real drivers require bare-metal or OS bindings.

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
