#!/bin/bash
# Quick launcher for adas_secondary

cd "$(dirname "$0")"

AGENT_ID="${1:-1}"

echo "=========================================="
echo "Starting adas_secondary (Agent ID: $AGENT_ID)"
echo "=========================================="

cargo run --release --bin adas_secondary \
  --features signalling_relayed_tcp -- "$AGENT_ID"
