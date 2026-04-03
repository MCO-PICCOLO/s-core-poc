#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
LOG_FILE="$PROJECT_ROOT/deny_results.log"
TMP_FILE="$PROJECT_ROOT/deny_output.txt"
mkdir -p "$PROJECT_ROOT/dist/reports/deny"
REPORT_FILE="$PROJECT_ROOT/dist/reports/deny/deny_summary.md"

rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "Running cargo-deny checks..." | tee -a "$LOG_FILE"

cd "$PROJECT_ROOT/timpani_rust"

if ! command -v cargo-deny &>/dev/null; then
  echo "📦 Installing cargo-deny..." | tee -a "$LOG_FILE"
  cargo install cargo-deny
fi

if cargo deny check | tee "$TMP_FILE"; then
  echo "✅ cargo-deny checks passed clean." | tee -a "$LOG_FILE"
  echo "✅ cargo-deny: **PASSED**" >> "$REPORT_FILE"
else
  echo "::error ::❌ cargo-deny failed! Found warnings/errors." | tee -a "$LOG_FILE"
  echo "❌ cargo-deny: **FAILED**" >> "$REPORT_FILE"
  exit 1
fi
