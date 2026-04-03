#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail  # Exit immediately on error, unset variable, or pipe failure

LOG_FILE="fmt_results.log"
TMP_FILE="fmt_output.txt"

# Create directory to store formatting reports
mkdir -p dist/reports/fmt
REPORT_FILE="dist/reports/fmt/fmt_summary.md"

# Clean up old logs and reports before starting
rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "Running Cargo fmt..." | tee -a "$LOG_FILE"

# Determine project root: use GitHub workspace if set, else current dir
PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
cd "$PROJECT_ROOT"

FAILED_TOTAL=0  # Counter for failed formatting checks
PASSED_TOTAL=0  # Counter for passed formatting checks
PIDS=()        # (Unused here but declared in case of future parallel runs)

# Declare paths to Cargo.toml manifests for different crates/components
MAJOR_MANIFEST="src/Cargo.toml"
NODEAGENT_MANIFEST="src/agent/nodeagent/Cargo.toml"
ROCKSDBSERVICE_MANIFEST="src/server/rocksdbservice/Cargo.toml"
TOOLS_MANIFEST="src/tools/Cargo.toml"

# Function to run 'cargo fmt --check' on a given manifest and record results
run_fmt() {
  local manifest="$1"
  local label="$2"
  local fmt_passed=false

  echo "Running fmt for $label ($manifest)" | tee -a "$LOG_FILE"

  # Run cargo fmt in check mode; output to TMP_FILE
  if cargo fmt --manifest-path="$manifest" --all --check | tee "$TMP_FILE"; then
    echo "fmt for $label passed clean." | tee -a "$LOG_FILE"
    fmt_passed=true
  else
    echo "::error ::fmt for $label failed! Found formatting issues. Check logs." | tee -a "$LOG_FILE"
    # Optionally, you could extract and log only relevant warnings/errors from TMP_FILE
    # e.g., grep -E "warning:|error:" "$TMP_FILE" | tee -a "$LOG_FILE"
  fi

  # Append pass/fail status to markdown summary report
  if $fmt_passed; then
    echo "✅ fmt for $label: PASSED" >> "$REPORT_FILE"
  else
    echo "❌ fmt for $label: FAILED" >> "$REPORT_FILE"
    (( FAILED_TOTAL++ ))  # Increment failure count
  fi
}

# Run formatting checks for each crate manifest if the file exists
if [[ -f "$MAJOR_MANIFEST" ]]; then
  run_fmt "$MAJOR_MANIFEST" "major"
else
  echo "::warning ::$MAJOR_MANIFEST not found, skipping..."
fi

if [[ -f "$NODEAGENT_MANIFEST" ]]; then
  run_fmt "$NODEAGENT_MANIFEST" "nodeagent"
else
  echo "::warning ::$NODEAGENT_MANIFEST not found, skipping..."
fi

if [[ -f "$ROCKSDBSERVICE_MANIFEST" ]]; then
  run_fmt "$ROCKSDBSERVICE_MANIFEST" "rocksdbservice"
else
  echo "::warning ::$ROCKSDBSERVICE_MANIFEST not found, skipping..."
fi

if [[ -f "$TOOLS_MANIFEST" ]]; then
  run_fmt "$TOOLS_MANIFEST" "tools"
else
  echo "::warning ::$TOOLS_MANIFEST not found, skipping..."
fi
