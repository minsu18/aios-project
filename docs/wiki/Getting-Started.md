# Getting Started

## Requirements

- Node.js 18+
- (Optional) Rust nightly — kernel, HAL
- (Optional) QEMU — VM simulation

## 1. Clone and Run Prototype

```bash
git clone https://github.com/minsu18/aios-project.git
cd aios-project/prototype
npm install
npm run build
npm run demo
```

## 2. Try Different Inference Backends

| Backend | Command | Notes |
|---------|---------|------|
| placeholder (default) | `npm run demo` | Rule-based, no network |
| Ollama | `AIOS_INFERENCE=ollama npm run demo` | Run `ollama serve` first; then `ollama pull llama3.2` |
| Transformers | `AIOS_INFERENCE=transformers npm run demo` | First run downloads model |
| Offline mode | `AIOS_OFFLINE=1 npm run demo` | No cloud calls |

## 3. App Store Commands

```bash
node dist/index.js skills          # List loaded skills
node dist/index.js browse          # Browse registry
node dist/index.js install-from-registry <name>
node dist/index.js remove <name>
```

## 4. Kernel + QEMU (Optional)

```bash
cargo run -p aios-boot
```

## 5. Raspberry Pi Build

```bash
./tools/build-rpi.sh
# Copy kernel8.img to SD card boot partition
```
