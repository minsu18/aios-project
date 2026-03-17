//! Network abstraction — send, receive

/// Send data over network (placeholder)
pub fn net_send(_data: &[u8], _addr: &str) -> Result<usize, &'static str> {
    Err("not implemented")
}

/// Receive data from network (placeholder)
pub fn net_recv(_buf: &mut [u8]) -> Result<usize, &'static str> {
    Err("not implemented")
}
