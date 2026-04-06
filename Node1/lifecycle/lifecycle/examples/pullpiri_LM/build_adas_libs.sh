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
# NOTE: Do NOT use 'cd path && pwd' for MINI_ADAS_LIB — the directory may not
# exist yet. Build the path as a plain string; mkdir -p creates it below.
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIFECYCLE_ROOT="${LIFECYCLE_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
MINI_ADAS_LIB="${MINI_ADAS_LIB:-$LIFECYCLE_ROOT/../../feo/examples/rust/mini-adas/lib}"
# Resolve any ../.. in the path without requiring the dir to exist
MINI_ADAS_LIB="$(realpath -m "$MINI_ADAS_LIB")"
BAZEL_BIN="$LIFECYCLE_ROOT/bazel-bin"

echo "=== adas_primary libs builder ==="
echo "  LIFECYCLE_ROOT : $LIFECYCLE_ROOT"
echo "  MINI_ADAS_LIB  : $MINI_ADAS_LIB"
echo "  BAZEL_BIN      : $BAZEL_BIN"
echo ""
echo "  To copy to a different destination, export MINI_ADAS_LIB before running."
echo "  Example: MINI_ADAS_LIB=/opt/pullpiri/lib ./build_adas_libs.sh"
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
  //src/launch_manager_daemon/health_monitor_lib:hm-lib \
  //src/launch_manager_daemon/health_monitor_lib:hm_shared_lib \
  //src/launch_manager_daemon/process_state_client_lib:process_state_client \
  //src/launch_manager_daemon/lifecycle_client_lib:lifecycle_client

echo ""
echo "[1/3] Build complete."

# ---------------------------------------------------------------------------
# Step 2 — Copy .so and .a files
# ---------------------------------------------------------------------------
echo ""
echo "[2/3] Copying libs to $MINI_ADAS_LIB ..."

mkdir -p "$MINI_ADAS_LIB"

copy_lib() {
    local src="$1"
    local filename
    filename="$(basename "$src")"
    local dest="$MINI_ADAS_LIB/$filename"

    # Remove destination if it exists but is not writable (e.g. root-owned from a prior run)
    if [[ -f "$dest" ]] && [[ ! -w "$dest" ]]; then
        sudo rm -f "$dest"
    fi

    if [[ -f "$src" ]]; then
        cp -v "$src" "$dest"
    else
        # bazel-bin may be root-owned; use sudo find if available
        local found
        if [[ $EUID -eq 0 ]]; then
            found="$(find "$BAZEL_BIN" -name "$filename" 2>/dev/null | grep -v '_objs' | head -1)"
        else
            found="$(sudo find "$BAZEL_BIN" -name "$filename" 2>/dev/null | grep -v '_objs' | head -1)"
        fi

        if [[ -n "$found" ]]; then
            cp -v "$found" "$dest"
        else
            echo "ERROR: $filename not found anywhere under $BAZEL_BIN"
            echo "       Run 'sudo find $BAZEL_BIN -name $filename' to investigate."
            exit 1
        fi
    fi
}

# Shared libraries (.so)
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/common/libcommon.so"
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/common/libosal.so"
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/common/libidentifier_hash.so"
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libphm_logging.so"
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libtimers.so"
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/process_state_client_lib/libprocess_state_client.so"
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/lifecycle_client_lib/liblifecycle_client.so"

# Static libraries (.a) — required by build.rs for adas_primary
copy_lib "$BAZEL_BIN/src/launch_manager_daemon/health_monitor_lib/libhm-lib.a"

# ---------------------------------------------------------------------------
# Step 3 — Verify
# ---------------------------------------------------------------------------
echo ""
echo "[3/3] Verifying $MINI_ADAS_LIB ..."
ls -lh "$MINI_ADAS_LIB/"

# Expected files
EXPECTED=(libcommon.so libosal.so libidentifier_hash.so libphm_logging.so
          libtimers.so libprocess_state_client.so liblifecycle_client.so libhm-lib.a)
MISSING=()
for f in "${EXPECTED[@]}"; do
    [[ -f "$MINI_ADAS_LIB/$f" ]] || MISSING+=("$f")
done

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo ""
    echo "ERROR: The following files are still missing:"
    for f in "${MISSING[@]}"; do echo "  - $f"; done
    exit 1
fi

echo ""
echo "=== All 8 libs present. You can now build adas_primary: ==="
echo ""
echo "  cd <feo/examples/rust/mini-adas>"
echo "  RUSTFLAGS=\"-L \$(pwd)/lib\" cargo build --release \\"
echo "    --bin adas_primary \\"
echo "    --features signalling_relayed_tcp,lifecycle"
