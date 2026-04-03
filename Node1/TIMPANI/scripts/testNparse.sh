#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

PROJECT_ROOT=${GITHUB_WORKSPACE:-$(pwd)}
LOG_FILE="$PROJECT_ROOT/test_results.log"
TMP_FILE="$PROJECT_ROOT/test_output.json"
mkdir -p "$PROJECT_ROOT/dist/tests" "$PROJECT_ROOT/target"
REPORT_FILE="$PROJECT_ROOT/dist/tests/test_summary.xml"

rm -f "$LOG_FILE" "$TMP_FILE" "$REPORT_FILE"

echo "Running Cargo Tests..." | tee -a "$LOG_FILE"

cd "$PROJECT_ROOT/timpani_rust"

echo "🔍 Debug: Running in $(pwd), RUSTC_BOOTSTRAP=${RUSTC_BOOTSTRAP:-1}" | tee -a "$LOG_FILE"

if RUSTC_BOOTSTRAP=1 cargo test --workspace -- -Z unstable-options --format json > "$TMP_FILE" 2>>"$LOG_FILE"; then
  echo "✅ Tests passed" | tee -a "$LOG_FILE"
else
  echo "::error ::❌ Tests failed!" | tee -a "$LOG_FILE"
fi

echo "🔍 Debug: Test output file size: $(wc -l < "$TMP_FILE" 2>/dev/null || echo 0) lines" | tee -a "$LOG_FILE"

if [[ -f "$TMP_FILE" ]]; then
  if command -v jq &>/dev/null; then
    # Filter out non-JSON lines (like log messages) before parsing
    # Add error handling for CI environments
    if json_lines=$(grep '^{' "$TMP_FILE" 2>/dev/null); then
      passed=$(echo "$json_lines" | jq -r 'select(.type == "test" and .event == "ok") | .name' 2>/dev/null | wc -l || echo "0")
      failed=$(echo "$json_lines" | jq -r 'select(.type == "test" and .event == "failed") | .name' 2>/dev/null | wc -l || echo "0")
      echo "ℹ️ Passed: $passed, Failed: $failed" | tee -a "$LOG_FILE"
    else
      echo "⚠️ Warning: No JSON test results found, parsing raw output" | tee -a "$LOG_FILE"
      # Fallback: count test results by parsing cargo output directly
      passed=$(grep -c "test result: ok" "$TMP_FILE" 2>/dev/null || echo "0")
      echo "ℹ️ Estimated passed tests: $passed" | tee -a "$LOG_FILE"
    fi
  fi

  if command -v cargo2junit &>/dev/null; then
    # Filter JSON lines for cargo2junit with error handling
    if grep '^{' "$TMP_FILE" >/dev/null 2>&1; then
      grep '^{' "$TMP_FILE" | cargo2junit > "$REPORT_FILE" 2>/dev/null || {
        echo "⚠️ Warning: cargo2junit failed, skipping XML report" | tee -a "$LOG_FILE"
      }
    fi
  fi
fi

# Only exit with error if actual tests failed, not JSON parsing issues
if grep -q "::error ::" "$LOG_FILE"; then
  echo "❌ Exiting due to test failures" | tee -a "$LOG_FILE"
  exit 1
fi
echo "🎉 All tests passed!" | tee -a "$LOG_FILE"
