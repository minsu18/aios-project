//! Comms driver stub (WiFi, cellular, BT)
//!
//! Phase 3: Implement network send/receive via hardware bindings.

use super::{Driver, DriverError};

/// Placeholder comms driver
pub struct CommsDriver {
    initialized: bool,
}

impl CommsDriver {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for CommsDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl Driver for CommsDriver {
    fn name(&self) -> &'static str {
        "comms"
    }

    fn init(&mut self) -> Result<(), DriverError> {
        // TODO: Initialize WiFi/cellular/BT stack
        self.initialized = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.initialized
    }
}
