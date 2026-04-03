#!/usr/bin/env bash
# build_adas_libs.sh
#
# Builds the C++ shared libraries required by adas_primary (lifecycle feature)
# from the S-CORE Lifecycle Bazel build and copies them into the mini-adas lib/.
#
# Usage:
#   cd lifecycle/lifecycle/examples/pullpiri_LM
#   chmod +x build_adas_libs.sh
#   ./build_adas_libs.sh
#
# Optional overrides (export before running):
#   LIFECYCLE_ROOT   — path to lifecycle/lifecycle   (default: auto-detected)
#   MINI_ADAS_LIB    — destination lib/ directory    (default: auto-detected)

set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve paths
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIFECYCLE_ROOT="${LIFECYCLE_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
MINI_ADAS_LIB="${MINI_ADAS_LIB:-$(cd "$LIFECYCLE_ROOT/../../feo/examples/rust/mini-adas/lib" && pwd)}"
BAZEL_BIN="$LIFECYCLE_ROOT/bazel-bin"

echo "=== adas_primary libs builder ==="
echo "  LIFECYCLE_ROOT : $LIFECYCLE_ROOT"
echo "  MINI_ADAS_LIB  : $MINI_ADAS_LIB"
echo ""

# ---------------------------------------------------------------------------
# Step 1 — Bazel build
# ---------------------------------------------------------------------------
echo "[1/3] Building C++ shared libraries with Bazel..."
cd "$LIFECYCLE_ROOT"

bazel build --config=x86_64-linux \
  //src/launch_manager_daemon/common:all \
  //src/launch_manager_daemon/health_monitor_lib:phm_logging \
  //src/launch_manager_daemon/health_monitor_lib:timers \
  //src/launch_manager_daemon/health_monitor_lib:hm_shared_lib \
  //src/launch_manager_daemon/process_state_client_lib:process_state_client \
  //src/launch_manager_daemon/lifecycle_client_lib:lifecycle_client

echo ""
echo "[1/3] Build complete."

# ---------------------------------------------------------------------------
# Step 2 — Copy .so files
# ---------------------------------------------------------------------------
echo ""
echo "[2/3] Copying .so files to $MINI_ADAS_LIB ..."

mkdir -p "$MINI_ADAS_LIB"

copy_so() {
    local src="$1"
    if [[ -f "$src" ]]; then
        cp -v "$src" "$MINI_ADAS_LIB/"
    else
        # bazel-bin is a symlink; try to find the file under the real execroot
        local found
        found="$(find "$BAZEL_BIN" -name "$(basename "$src")" 2>/dev/null | head -1)"
        if [[ -n "$found" ]]; then
            cp -v "$found" "$MINI_ADAS_LIB/"
        else
            echo "WARNING: $(basename "$src") not found in bazel-bin — skipping."
        fi
    fi
}

copy_so "$BAZEL_BIN/src/launch_manager_daemon/common/libcommon.so"
copy_so "$BAZEL_BIN/src/launch_manager_daemon/common/libosal.so"
copy_so "$BAZEL_BIN/src/launch_manager_daemon/common/libidentifier_hash.so"
copy_so "$BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libphm_logging.so"
copy_so "$BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libtimers.so"
copy_so "$BAZEL_BIN/src/launch_manager_daemon/process_state_client_lib/libprocess_state_client.so"
copy_so "$BAZEL_BIN/src/launch_manager_daemon/lifecycle_client_lib/liblifecycle_client.so"

# ---------------------------------------------------------------------------
# Step 3 — Verify
# ---------------------------------------------------------------------------
echo ""
echo "[3/3] Verifying $MINI_ADAS_LIB ..."
ls -lh "$MINI_ADAS_LIB/"

echo ""
echo "=== Done. You can now build adas_primary: ==="
echo ""
echo "  cd $(dirname "$MINI_ADAS_LIB")"
echo "  RUSTFLAGS=\"-L \$(pwd)/lib\" cargo build --release \\"
echo "    --bin adas_primary \\"
echo "    --features signalling_relayed_tcp,lifecycle"
