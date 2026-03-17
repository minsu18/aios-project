//! AIOS AI Core
//!
//! - Multimodal input: text, voice, image, video
//! - Intent understanding and task decomposition
//! - Routing: on-device vs cloud
//! - Skill/MCP orchestration

/// Input modalities
#[derive(Debug, Clone)]
pub enum Input {
    Text(String),
    Voice(Vec<u8>),
    Image(Vec<u8>),
    Video(Vec<u8>),
}

/// Inference target
#[derive(Debug, Clone)]
pub enum InferenceTarget {
    OnDevice,
    Cloud,
}

/// Route input to appropriate inference (placeholder)
pub fn route(input: &Input) -> InferenceTarget {
    match input {
        Input::Text(t) if t.len() < 100 => InferenceTarget::OnDevice,
        _ => InferenceTarget::Cloud,
    }
}
