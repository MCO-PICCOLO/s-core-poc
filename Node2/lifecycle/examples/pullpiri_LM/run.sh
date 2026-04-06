#!/bin/bash
# *******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
# *******************************************************************************
#
# run.sh - Launch the S-CORE Lifecycle Launch Manager with Pullpiri components
#
# This script builds the Pullpiri lifecycle configuration, copies the compiled
# Flatbuffer binaries and logging configs into the correct locations, and then
# starts the Launch Manager daemon.
#
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine workspace root
if [ -n "$BUILD_WORKSPACE_DIRECTORY" ]; then
    WORKSPACE_ROOT="$BUILD_WORKSPACE_DIRECTORY"
else
    # Assume we are inside examples/pullpiri_LM/
    WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
fi

# If the script is invoked via sudo, run Bazel as the original user to avoid
# creating root-owned output files that the sandbox compiler cannot read.
if [ -n "$SUDO_USER" ]; then
    BAZEL_CMD="sudo -u $SUDO_USER bazel"
else
    BAZEL_CMD="bazel"
fi

BAZEL_BIN="$WORKSPACE_ROOT/bazel-bin"
LM_BINARY="$BAZEL_BIN/src/launch_manager_daemon/launch_manager"
CFG_DIR="$BAZEL_BIN/examples/pullpiri_LM/flatbuffer_out"
ETC_DIR="$WORKSPACE_ROOT/etc"

echo "============================================="
echo "  Pullpiri Lifecycle - Build & Run"
echo "============================================="

# ---- Step 1: Build ----
echo ""
echo "[1/4] Building S-CORE Lifecycle with Pullpiri config..."
cd "$WORKSPACE_ROOT"
$BAZEL_CMD build --config=x86_64-linux \
    --strategy=LifecycleJsonConfigGeneration=local \
    --strategy=LaunchManagerFlatbufferConfigGeneration=local \
    --check_direct_dependencies=off \
    //examples/pullpiri_LM:pullpiri_config //src/launch_manager_daemon:launch_manager

# ---- Step 2: Copy flatbuffer configs ----
echo ""
echo "[2/4] Copying compiled Flatbuffer configs to $ETC_DIR ..."
mkdir -p "$ETC_DIR"
sudo cp "$CFG_DIR/lm_demo.bin"  "$ETC_DIR/lm_demo.bin"
sudo cp "$CFG_DIR/hm_demo.bin"  "$ETC_DIR/hm_demo.bin"
sudo cp "$CFG_DIR/hmcore.bin"   "$ETC_DIR/hmcore.bin"

# ---- Step 3: Copy logging configs ----
echo ""
echo "[3/4] Copying LM and HM logging configs to $ETC_DIR ..."
sudo cp "$SCRIPT_DIR/config/lm_logging.json"         "$ETC_DIR/logging.json"
sudo cp "$SCRIPT_DIR/config/ecu_logging_config.json"  "$ETC_DIR/ecu_logging_config.json"

# ---- Step 5: Launch ----
echo ""
echo "[4/4] Starting Launch Manager daemon..."
echo "      Press Ctrl+C to stop."
echo "============================================="
echo ""

exec "$LM_BINARY"
