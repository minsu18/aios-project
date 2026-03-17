//! Audio driver stub
//!
//! Phase 3: Implement speaker output and microphone capture via hardware bindings.

use super::{Driver, DriverError};

/// Placeholder audio driver
pub struct AudioDriver {
    initialized: bool,
}

impl AudioDriver {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for AudioDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for AudioDriver {
    fn name(&self) -> &'static str {
        "audio"
    }

    fn init(&mut self) -> Result<(), DriverError> {
        // TODO: Initialize audio hardware (speaker, microphone)
        self.initialized = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.initialized
    }
}
