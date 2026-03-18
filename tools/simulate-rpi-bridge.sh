#!/usr/bin/env bash
# AIOS RPi Simulation with Serial Bridge — Host inference via Ollama
#
# Usage: ./tools/simulate-rpi-bridge.sh
#
# Builds kernel, then runs QEMU with serial bridge. "ask <q>" is forwarded
# to host Ollama. Requires: ollama serve, ollama pull llama3.2
#
# Exit: Ctrl+A then X (or Ctrl+C)
# Env: AIOS_OLLAMA_HOST, AIOS_OLLAMA_MODEL, AIOS_SERIAL_PIPE

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

./tools/build-rpi.sh
echo ""
echo "Starting with Serial Bridge (host Ollama for 'ask')..."
echo ""

exec node "$ROOT/tools/serial-bridge.mjs"
