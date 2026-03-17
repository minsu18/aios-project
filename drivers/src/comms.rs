//! Comms driver (WiFi, cellular, BT abstraction)
//!
//! - Default: stub for bare-metal
//! - `host` feature: UDP/TCP via std::net

#[cfg(not(feature = "host"))]
use alloc::vec::Vec;

use super::{Driver, DriverError};

/// Comms driver
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
        self.initialized = true;
        Ok(())
    }

    fn ready(&self) -> bool {
        self.initialized
    }
}

/// Send data over UDP to addr (e.g. "192.168.1.1:8080").
pub fn udp_send(data: &[u8], addr: &str) -> Result<usize, DriverError> {
    #[cfg(feature = "host")]
    {
        udp_send_host(data, addr)
    }
    #[cfg(not(feature = "host"))]
    {
        let _ = (data, addr);
        Err(DriverError("comms: build with --features host for network"))
    }
}

/// Receive UDP data. Binds to port, blocks until a packet arrives.
pub fn udp_recv(buf: &mut [u8], port: u16) -> Result<usize, DriverError> {
    #[cfg(feature = "host")]
    {
        udp_recv_host(buf, port)
    }
    #[cfg(not(feature = "host"))]
    {
        let _ = (buf, port);
        Err(DriverError("comms: build with --features host for network"))
    }
}

#[cfg(feature = "host")]
fn udp_send_host(data: &[u8], addr: &str) -> Result<usize, DriverError> {
    use std::net::UdpSocket;

    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|_| DriverError("comms: bind failed"))?;
    let n = socket.send_to(data, addr).map_err(|_| DriverError("comms: send_to failed"))?;
    Ok(n)
}

#[cfg(feature = "host")]
fn udp_recv_host(buf: &mut [u8], port: u16) -> Result<usize, DriverError> {
    use std::net::UdpSocket;

    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).map_err(|_| DriverError("comms: bind failed"))?;
    socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();
    let (n, _) = socket.recv_from(buf).map_err(|_| DriverError("comms: recv_from failed"))?;
    Ok(n)
}
