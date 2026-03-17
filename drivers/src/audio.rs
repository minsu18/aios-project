//! Audio driver
//!
//! - Default: stub for bare-metal
//! - `host` feature: ALSA for playback and capture

#[cfg(not(feature = "host"))]
use alloc::vec::Vec;

use super::{Driver, DriverError};

/// Audio driver
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
        self.initialized = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.initialized
    }
}

/// Play PCM audio (s16le, 16kHz stereo). Returns on completion.
pub fn play(data: &[u8]) -> Result<(), DriverError> {
    #[cfg(feature = "host")]
    {
        play_host(data)
    }
    #[cfg(not(feature = "host"))]
    {
        let _ = data;
        Err(DriverError("audio: build with --features host for ALSA"))
    }
}

/// Capture audio from microphone. Returns PCM s16le bytes.
pub fn capture(samples: usize) -> Result<Vec<u8>, DriverError> {
    #[cfg(feature = "host")]
    {
        capture_host(samples)
    }
    #[cfg(not(feature = "host"))]
    {
        let _ = samples;
        Err(DriverError("audio: build with --features host for ALSA"))
    }
}

#[cfg(feature = "host")]
fn play_host(data: &[u8]) -> Result<(), DriverError> {
    use alsa::pcm::{PCM, HwParams, Format, Access};
    use alsa::{Direction, ValueOr};

    let pcm = PCM::new("default", Direction::Playback, false).map_err(|_| DriverError("audio: failed to open PCM"))?;
    {
        let hwp = HwParams::any(&pcm).map_err(|_| DriverError("audio: HwParams any"))?;
        hwp.set_channels(2).map_err(|_| DriverError("audio: set channels"))?;
        hwp.set_rate(44100, ValueOr::Nearest).map_err(|_| DriverError("audio: set rate"))?;
        hwp.set_format(Format::s16()).map_err(|_| DriverError("audio: set format"))?;
        hwp.set_access(Access::RWInterleaved).map_err(|_| DriverError("audio: set access"))?;
        pcm.hw_params(&hwp).map_err(|_| DriverError("audio: hw_params"))?;
    }
    let io = pcm.io_i16().map_err(|_| DriverError("audio: io_i16"))?;
    let buf: Vec<i16> = data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
    io.writei(&buf).map_err(|_| DriverError("audio: write failed"))?;
    pcm.drain().map_err(|_| DriverError("audio: drain failed"))?;
    Ok(())
}

#[cfg(feature = "host")]
fn capture_host(samples: usize) -> Result<Vec<u8>, DriverError> {
    use alsa::pcm::{PCM, HwParams, Format, Access};
    use alsa::{Direction, ValueOr};

    let pcm = PCM::new("default", Direction::Capture, false).map_err(|_| DriverError("audio: failed to open PCM"))?;
    {
        let hwp = HwParams::any(&pcm).map_err(|_| DriverError("audio: HwParams any"))?;
        hwp.set_channels(2).map_err(|_| DriverError("audio: set channels"))?;
        hwp.set_rate(44100, ValueOr::Nearest).map_err(|_| DriverError("audio: set rate"))?;
        hwp.set_format(Format::s16()).map_err(|_| DriverError("audio: set format"))?;
        hwp.set_access(Access::RWInterleaved).map_err(|_| DriverError("audio: set access"))?;
        pcm.hw_params(&hwp).map_err(|_| DriverError("audio: hw_params"))?;
    }
    let io = pcm.io_i16().map_err(|_| DriverError("audio: io_i16"))?;
    let mut buf = vec![0i16; samples];
    io.readi(&mut buf).map_err(|_| DriverError("audio: read failed"))?;
    let out: Vec<u8> = buf.iter().flat_map(|s| s.to_le_bytes()).collect();
    Ok(out)
}
