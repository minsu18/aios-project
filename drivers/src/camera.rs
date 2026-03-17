//! Camera driver
//!
//! - Default: stub for bare-metal
//! - `host` feature: V4L2 via rscam (e.g. /dev/video0)

#[cfg(not(feature = "host"))]
use alloc::vec::Vec;

use super::{Driver, DriverError};

/// Camera driver
pub struct CameraDriver {
    initialized: bool,
}

impl CameraDriver {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for CameraDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for CameraDriver {
    fn name(&self) -> &'static str {
        "camera"
    }

    fn init(&mut self) -> Result<(), DriverError> {
        self.initialized = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.initialized
    }
}

/// Capture a single image frame. Returns JPEG or raw bytes.
pub fn capture_image(device: &str) -> Result<Vec<u8>, DriverError> {
    #[cfg(feature = "host")]
    {
        capture_image_host(device)
    }
    #[cfg(not(feature = "host"))]
    {
        let _ = device;
        Err(DriverError("camera: build with --features host for V4L2"))
    }
}

#[cfg(feature = "host")]
fn capture_image_host(device: &str) -> Result<Vec<u8>, DriverError> {
    use rscam::{Camera, Config};

    let mut camera = Camera::new(device).map_err(|_| DriverError("camera: failed to open device"))?;
    camera
        .start(&Config {
            interval: (1, 30),
            resolution: (640, 480),
            format: b"MJPG",
            ..Default::default()
        })
        .map_err(|_| DriverError("camera: failed to start"))?;

    let frame = camera.capture().map_err(|_| DriverError("camera: capture failed"))?;
    Ok(frame.to_vec())
}
