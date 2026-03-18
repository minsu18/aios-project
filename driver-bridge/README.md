# aios-driver-bridge

CLI bridge that exposes aios-drivers (camera, audio) to the Node prototype.

**Requires Linux** (V4L2, ALSA). On macOS, prints usage and exits.

## Build

```bash
cargo build -p aios-driver-bridge --release
```

On Linux, also needs `libv4l-dev`, `libasound2-dev`.

## Usage

```bash
# Capture image from /dev/video0, output JSON with base64 JPEG
aios-driver-bridge camera capture [/dev/video0]

# Capture audio (48k samples default), output JSON with base64 PCM
aios-driver-bridge audio capture [samples]
```

The prototype uses this when you run `voice capture` or `image capture` (Linux only).
