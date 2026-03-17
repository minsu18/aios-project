//! Audio abstraction - speaker, microphone

/// Play audio to speaker (placeholder)
pub fn play(_data: &[u8]) -> Result<(), &'static str> {
    // TODO: HAL implementation
    Err("not implemented")
}

/// Capture audio from microphone (placeholder)
pub fn capture() -> Result<Vec<u8>, &'static str> {
    // TODO: HAL implementation
    Err("not implemented")
}
