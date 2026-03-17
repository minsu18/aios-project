//! Demo of host drivers (camera, audio, comms).
//! Run: cargo run -p aios-drivers --features host --example host_demo

use aios_drivers::{audio, camera, comms};

fn main() {
    println!("AIOS Drivers Host Demo (Linux)");
    println!("---");

    // Camera: capture one frame from /dev/video0
    match camera::capture_image("/dev/video0") {
        Ok(buf) => println!("Camera: captured {} bytes", buf.len()),
        Err(e) => println!("Camera: {} (no camera?)", e),
    }

    // Comms: send UDP packet
    let msg = b"hello from aios";
    match comms::udp_send(msg, "127.0.0.1:3457") {
        Ok(n) => println!("Comms send: {} bytes sent", n),
        Err(e) => println!("Comms send: {}", e),
    }

    // Audio: play 1s of silence (PCM s16le 44.1kHz stereo)
    let samples = 44100 * 2 * 2; // 1 sec stereo
    let silent: Vec<u8> = vec![0; samples];
    match audio::play(&silent) {
        Ok(()) => println!("Audio: played 1s silence"),
        Err(e) => println!("Audio play: {} (no ALSA?)", e),
    }

    println!("--- done");
}
