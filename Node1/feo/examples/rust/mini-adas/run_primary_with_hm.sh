#!/bin/bash
# Quick launcher for adas_primary with Health Monitor

cd "$(dirname "$0")"

export LD_LIBRARY_PATH=$(pwd)/lib:$LD_LIBRARY_PATH
export RUSTFLAGS="-L $(pwd)/lib"
export RUST_LOG=info,health_monitoring_lib=debug
export IDENTIFIER="mini_adas_primary"
export CONFIG_PATH="$(pwd)/etc/hmproc_adas_primary.bin"

echo "=========================================="
echo "Starting adas_primary with Health Monitor"
echo "=========================================="
echo "Library path: $(pwd)/lib"
echo "Cycle time: ${1:-400}ms"
echo ""
echo "✅ LD_LIBRARY_PATH set (runtime linking)"
echo "✅ RUSTFLAGS set (build-time linking)"
echo "✅ RUST_LOG configured for HM debugging"
echo "✅ CONFIG_PATH set: $(pwd)/etc/hmproc_adas_primary.bin"
echo ""
echo "Expected startup logs:"
echo "  - Loaded HM config: deadline window [200, 600] ms"
echo "  - Health Monitor initialized"
echo "  - Health Monitor started"
echo "  - Starting deadline monitoring for FEO execution"
echo ""
echo "=========================================="
echo ""

cargo run --release --bin adas_primary \
  --features signalling_relayed_tcp,lifecycle -- "${1:-400}"
