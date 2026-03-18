//! AIOS Driver Bridge — CLI to expose camera/audio drivers for prototype
//!
//! Usage:
//!   aios-driver-bridge camera capture [/dev/video0]  # JPEG base64 to stdout
//!   aios-driver-bridge audio capture [samples]        # PCM base64 to stdout (default 48000)
//!
//! Requires Linux (V4L2, ALSA). On macOS, prints usage and exits.

use std::env;
use std::io::Write;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let arg = args.get(3);

    #[cfg(target_os = "linux")]
    {
        runner(cmd, sub, arg);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (cmd, sub, arg);
        let _ = writeln!(
            std::io::stderr(),
            "aios-driver-bridge: Requires Linux (V4L2, ALSA). Use voice <file> or image <file> on macOS."
        );
        let _ = writeln!(
            std::io::stderr(),
            "Usage: aios-driver-bridge camera capture [/dev/video0] | audio capture [samples]"
        );
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
fn runner(cmd: &str, sub: &str, arg: Option<&String>) {
    use aios_drivers::{capture as audio_capture, capture_image};
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;

    match (cmd, sub) {
        ("camera", "capture") => {
            let device = arg.map(|s| s.as_str()).unwrap_or("/dev/video0");
            match capture_image(device) {
                Ok(data) => {
                    let b64 = BASE64.encode(&data);
                    println!("{{\"ok\":true,\"format\":\"jpeg\",\"data\":\"{b64}\"}}");
                }
                Err(e) => {
                    eprintln!("{{\"ok\":false,\"error\":\"{}\"}}", e);
                    std::process::exit(1);
                }
            }
        }
        ("audio", "capture") => {
            let samples: usize = arg.and_then(|s| s.parse().ok()).unwrap_or(48000);
            match audio_capture(samples) {
                Ok(data) => {
                    let b64 = BASE64.encode(&data);
                    println!(
                        "{{\"ok\":true,\"format\":\"pcm_s16le\",\"sample_rate\":44100,\"channels\":2,\"data\":\"{b64}\"}}"
                    );
                }
                Err(e) => {
                    eprintln!("{{\"ok\":false,\"error\":\"{}\"}}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            let _ = writeln!(
                std::io::stderr(),
                "Usage: aios-driver-bridge camera capture [/dev/video0] | audio capture [samples]"
            );
            std::process::exit(1);
        }
    }
}
