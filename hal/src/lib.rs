//! AIOS Hardware Abstraction Layer
//!
//! Abstracts: memory, CPU, GPU, network, camera, speaker, microphone

pub mod memory;
pub mod audio;
pub mod camera;

pub use memory::*;
pub use audio::*;
pub use camera::*;
