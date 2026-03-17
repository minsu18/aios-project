//! AIOS Hardware Abstraction Layer
//!
//! Abstracts: memory, CPU, GPU, network, camera, speaker, microphone.
//! The kernel calls these functions after boot; drivers provide the implementation.
//! Phase 2: Interface definitions. Phase 3: Real implementations via aios-drivers.

pub mod audio;
pub mod camera;
pub mod cpu;
pub mod gpu;
pub mod memory;
pub mod network;

#[cfg(feature = "llama")]
mod llama_inference;

pub use audio::*;
pub use camera::*;
pub use cpu::*;
pub use gpu::*;
pub use memory::*;
pub use network::*;

/// Initialize HAL. Called by kernel after early boot.
/// Phase 2: No-op. Phase 3: Register drivers, init subsystems.
pub fn init() {
    // TODO: driver registration, memory allocator setup
}
