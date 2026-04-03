#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
LOG_FILE="$PROJECT_ROOT/clippy_results.log"
TMP_FILE="$PROJECT_ROOT/clippy_output.txt"
mkdir -p "$PROJECT_ROOT/dist/reports/clippy"
REPORT_FILE="$PROJECT_ROOT/dist/reports/clippy/clippy_summary.md"

rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "Running Cargo clippy..." | tee -a "$LOG_FILE"

cd "$PROJECT_ROOT/timpani_rust"

if cargo clippy --workspace --all-targets --all-features | tee "$TMP_FILE"; then
  echo "✅ Clippy passed clean." | tee -a "$LOG_FILE"
  echo "✅ Clippy: **PASSED**" >> "$REPORT_FILE"
else
  echo "::error ::❌ Clippy failed! Found warnings/errors." | tee -a "$LOG_FILE"
  echo "❌ Clippy: **FAILED**" >> "$REPORT_FILE"
  exit 1
fi
