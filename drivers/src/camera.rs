//! Camera driver stub
//!
//! Phase 3: Implement capture_image, capture_video_frame via hardware bindings.

use super::{Driver, DriverError};

/// Placeholder camera driver
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
        // TODO: Initialize camera hardware
        self.initialized = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.initialized
    }
}
