//! AIOS Hardware Abstraction Layer
//!
//! Abstracts: memory, CPU, GPU, network, camera, speaker, microphone

pub mod audio;
pub mod camera;
pub mod cpu;
pub mod gpu;
pub mod memory;
pub mod network;

pub use audio::*;
pub use camera::*;
pub use cpu::*;
pub use gpu::*;
pub use memory::*;
pub use network::*;
