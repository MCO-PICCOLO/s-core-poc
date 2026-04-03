#!/bin/bash
# Run mini-adas with Launch Manager daemon

LM_DIR="/tmp/mini_adas_lm"

if [ ! -d "$LM_DIR" ]; then
    echo "❌ LM daemon not set up yet!"
    echo ""
    echo "Run setup first:"
    echo "  ./setup_lm_daemon.sh"
    echo ""
    exit 1
fi

if [ ! -f "$LM_DIR/launch_manager" ]; then
    echo "❌ launch_manager binary not found in $LM_DIR"
    exit 1
fi

echo "=============================================="
echo "Starting Launch Manager Daemon"
echo "=============================================="
echo ""
echo "📂 Working directory: $LM_DIR"
echo "📚 Library path: $LM_DIR/lib"
echo "⚙️  Config: etc/lm_demo.bin"
echo ""
echo "LM will automatically start:"
echo "  • mini_adas_primary (with HM integration)"
echo "  • mini_adas_secondary_1"
echo "  • mini_adas_secondary_2"
echo ""
echo "Press Ctrl+C to stop"
echo "=============================================="
echo ""

cd "$LM_DIR"
export LD_LIBRARY_PATH="$LM_DIR/lib:$LD_LIBRARY_PATH"

# Clean stale processes from prior interrupted runs to avoid IPC/topic collisions.
pkill -f "$LM_DIR/launch_manager" 2>/dev/null || true
pkill -f "$LM_DIR/mini_adas/adas_primary" 2>/dev/null || true
pkill -f "$LM_DIR/mini_adas/adas_secondary" 2>/dev/null || true

./launch_manager &
LM_PID=$!

cleanup() {
    kill "$LM_PID" 2>/dev/null || true
    pkill -f "$LM_DIR/mini_adas/adas_primary" 2>/dev/null || true
    pkill -f "$LM_DIR/mini_adas/adas_secondary" 2>/dev/null || true
}

trap cleanup INT TERM EXIT
wait "$LM_PID"
