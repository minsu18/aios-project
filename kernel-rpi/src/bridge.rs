//! Host inference bridge — UART protocol for delegating "ask" to host Ollama.
//!
//! Protocol: kernel sends "AIOS_BRIDGE_ASK:<prompt>\n", blocks until it receives
//! "AIOS_BRIDGE_REPLY:<response>\n" on UART. Host bridge (tools/serial-bridge) intercepts.
//! With no bridge (e.g. simulate-rpi.sh), times out after ~3s and returns an error.

#![allow(static_mut_refs)]

use crate::uart_write;

const PREFIX_ASK: &[u8] = b"AIOS_BRIDGE_ASK:";
const PREFIX_REPLY: &[u8] = b"AIOS_BRIDGE_REPLY:";
const REPLY_BUF_LEN: usize = 512;
const TIMEOUT_SEC: u64 = 60; // Ollama can take 10–30s for first response

static mut REPLY_BUF: [u8; REPLY_BUF_LEN] = [0; REPLY_BUF_LEN];

/// Send ask request to host and block until reply. Returns Ok(response) or Err on timeout/invalid.
pub fn ask_host(prompt: &str) -> Result<&'static str, &'static str> {
    uart_write(b"\n");
    uart_write(PREFIX_ASK);
    uart_write(prompt.as_bytes());
    uart_write(b"\n");

    let reply = read_bridge_reply()?;
    Ok(reply)
}

/// Read until "AIOS_BRIDGE_REPLY:" then collect until '\n'.
/// Times out if no reply within TIMEOUT_SEC (e.g. when simulate-rpi.sh without bridge).
fn read_bridge_reply() -> Result<&'static str, &'static str> {
    let (start_ticks, freq) = aios_hal_bare::timer::read();
    let mut matched = 0usize;
    let mut payload_len = 0usize;

    loop {
        let b = match unsafe { crate::uart_try_read_byte() } {
            Some(byte) => byte,
            None => {
                let (now, _) = aios_hal_bare::timer::read();
                if freq > 0 && now > start_ticks {
                    let elapsed = (now - start_ticks) / freq;
                    if elapsed >= TIMEOUT_SEC {
                        return Err("Bridge timeout. Use simulate-rpi-bridge.sh for 'ask'.");
                    }
                }
                continue;
            }
        };

        if matched < PREFIX_REPLY.len() {
            if b == PREFIX_REPLY[matched] {
                matched += 1;
            } else {
                matched = 0;
            }
            continue;
        }

        if b == b'\n' || b == b'\r' {
            let bytes: &'static [u8] =
                unsafe { core::slice::from_raw_parts(REPLY_BUF.as_ptr(), payload_len) };
            return core::str::from_utf8(bytes).map_err(|_| "invalid UTF-8");
        }

        if payload_len < REPLY_BUF_LEN {
            unsafe {
                *REPLY_BUF.as_mut_ptr().add(payload_len) = b;
            }
            payload_len += 1;
        }
    }
}
