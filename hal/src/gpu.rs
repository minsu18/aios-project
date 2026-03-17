//! GPU abstraction — inference, render

/// Run on-device inference (placeholder)
pub fn inference(_model_id: &str, _input: &[u8]) -> Result<Vec<u8>, &'static str> {
    Err("not implemented")
}

/// Render to display (placeholder)
pub fn render(_framebuffer: &[u8]) -> Result<(), &'static str> {
    Err("not implemented")
}
