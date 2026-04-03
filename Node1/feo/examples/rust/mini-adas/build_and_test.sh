#!/bin/bash
# Quick build and test script for mini-adas with Health Monitor integration

set -e

echo "=================================="
echo "Mini-ADAS Health Monitor Integration"
echo "=================================="

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Run this script from feo/examples/rust/mini-adas directory"
    exit 1
fi

echo ""
echo "Step 1: Building adas_primary with Health Monitor..."
cargo build --release --bin adas_primary --features signalling_relayed_tcp

echo ""
echo "Step 2: Building adas_secondary..."
cargo build --release --bin adas_secondary --features signalling_relayed_tcp

echo ""
echo "=================================="
echo "Build Complete!"
echo "=================================="
echo ""
echo "Binaries created:"
echo "  - target/release/adas_primary"
echo "  - target/release/adas_secondary"
echo ""
echo "Configuration:"
echo "  - Health Monitor config: etc/hm_config.json"
echo "  - Launch Manager config: ../../../lifecycle/examples/config/mini_adas_lifecycle.json"
echo ""
echo "To run standalone (without Launch Manager):"
echo "  Terminal 1: ./target/release/adas_primary 400"
echo "  Terminal 2: ./target/release/adas_secondary 1"
echo "  Terminal 3: ./target/release/adas_secondary 2"
echo ""
echo "For full Launch Manager integration, see HEALTH_MONITOR_INTEGRATION.md"
echo ""
