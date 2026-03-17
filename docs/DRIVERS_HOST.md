# Host Drivers (Linux)

When built with `--features host`, the drivers crate uses real Linux hardware:

| Driver  | Binding    | Notes                              |
|---------|------------|------------------------------------|
| Camera  | V4L2/rscam | `/dev/video0`, MJPG 640x480       |
| Audio   | ALSA       | `default` device, s16le 44.1kHz    |
| Comms   | std::net   | UDP send/recv                     |

## Build

```bash
cargo build -p aios-drivers --features host
```

## Run Demo

```bash
cargo run -p aios-drivers --features host --example host_demo
```

Requires:

- Linux with V4L2 (camera)
- ALSA (audio)
- No extra deps for UDP

## API

```rust
use aios_drivers::{capture_image, play, capture, udp_send, udp_recv};

// Camera
let frame = capture_image("/dev/video0")?;

// Audio
play(&pcm_bytes)?;
let samples = capture(44100 * 2)?;  // 1 sec stereo

// Comms
udp_send(b"data", "192.168.1.1:8080")?;
let n = udp_recv(&mut buf, 3456)?;
```
