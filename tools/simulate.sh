#!/usr/bin/env bash
# AIOS VM Simulation — Boot with configurable specs, then run AI prototype
#
# Usage: ./tools/simulate.sh [--cpus N] [--memory MB] [--vm-only]
#
#   --cpus N     VM CPU count (default: 2)
#   --memory M   VM RAM in MB (default: 512)
#   --vm-only    Run QEMU only, no prototype (Ctrl+A then X to exit QEMU)
#   --no-vm      Skip QEMU, run prototype only (for environments without QEMU)
#
# Requirements: Rust nightly, qemu-system-x86_64 (unless --no-vm)

set -e

CPUS=2
MEMORY=512
VM_ONLY=false
NO_VM=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --cpus) CPUS="$2"; shift 2 ;;
    --memory) MEMORY="$2"; shift 2 ;;
    --vm-only) VM_ONLY=true; shift ;;
    --no-vm) NO_VM=true; shift ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$NO_VM" == false ]]; then
  echo "=== AIOS VM Simulation ==="
  echo "  CPUs: $CPUS"
  echo "  Memory: ${MEMORY} MB"
  echo ""
  echo "Booting virtual machine..."
  echo "  (Press Ctrl+A then X to exit QEMU if using --vm-only)"
  echo ""

  export AIOS_VM_CPUS=$CPUS
  export AIOS_VM_MEM=$MEMORY

  if [[ "$VM_ONLY" == true ]]; then
    cargo run -p aios-boot
    exit 0
  fi

  # Run QEMU for ~6 seconds to show boot, then kill (cross-platform)
  cargo run -p aios-boot &
  QEMU_PID=$!
  sleep 6
  kill $QEMU_PID 2>/dev/null || true
  wait $QEMU_PID 2>/dev/null || true
  echo ""
  echo ">>> Booting AI layer (prototype)..."
  echo ""
fi

cd "$ROOT/prototype"
npm run build 2>/dev/null || true
exec node dist/index.js interactive
