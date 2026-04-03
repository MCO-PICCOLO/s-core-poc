#!/usr/bin/env bash
# stress_app_cpus.sh — Stress the CPUs assigned to a named running process
#
# Usage:
#   ./stress_app_cpus.sh <app_name> [duration_sec] [load_percent]
#
#   app_name      Name as it appears in /proc/<pid>/comm  (required)
#   duration_sec  Stress duration in seconds              (default: 30)
#   load_percent  Target CPU load 1-100                   (default: 95)
#
# Examples:
#   ./stress_app_cpus.sh sea_app
#   ./stress_app_cpus.sh sea_app 60 80
#   ./stress_app_cpus.sh task_cpumask 20 100
#   ./stress_app_cpus.sh sea_app 30 100   # saturate — guarantee deadline misses
# ---------------------------------------------------------------------------
set -euo pipefail

APP_NAME="${1:-}"
DURATION="${2:-30}"
LOAD="${3:-95}"

if [[ -z "$APP_NAME" ]]; then
    echo "Usage: $0 <app_name> [duration_sec] [load_percent]"
    echo ""
    echo "  app_name      process name (as shown in 'ps -e' COMM column)"
    echo "  duration_sec  how long to stress (default 30)"
    echo "  load_percent  CPU load 1-100 (default 95, use 100 for max pressure)"
    exit 1
fi

if [[ "$LOAD" -lt 1 || "$LOAD" -gt 100 ]]; then
    echo "[ERROR] load_percent must be between 1 and 100"
    exit 1
fi

# ── Find PID by process name ────────────────────────────────────────────────
find_pid() {
    local name="$1"
    # Primary: search whole-process comm (/proc/PID/comm)
    for f in /proc/[0-9]*/comm; do
        [[ -r "$f" ]] || continue
        if [[ "$(< "$f")" == "$name" ]]; then
            echo "${f%/comm}" | grep -oP '\d+'
            return 0
        fi
    done
    # Fallback: search per-thread comm, return the process PID (not TID)
    for f in /proc/[0-9]*/task/[0-9]*/comm; do
        [[ -r "$f" ]] || continue
        if [[ "$(< "$f")" == "$name" ]]; then
            local proc_pid="${f#/proc/}"; proc_pid="${proc_pid%%/*}"
            echo "$proc_pid"
            return 0
        fi
    done
    return 1
}

PID=$(find_pid "$APP_NAME") || {
    echo "[ERROR] Process '$APP_NAME' not found. Is it running?"
    exit 1
}
echo "[INFO] Found '$APP_NAME' → PID $PID"

# ── Get CPU affinity of the target process ──────────────────────────────────
CPUS_ALLOWED_LIST=$(grep -i "^Cpus_allowed_list:" /proc/"$PID"/status 2>/dev/null | awk '{print $2}')
if [[ -z "$CPUS_ALLOWED_LIST" ]]; then
    echo "[ERROR] Cannot read CPU affinity for PID $PID (process may have died)"
    exit 1
fi

# Expand kernel cpu list format: "2", "2,5", "2-4", "0,2-4,8"
read -ra TARGET_CPUS <<< "$(python3 -c "
import sys
s = '$CPUS_ALLOWED_LIST'
cpus = []
for part in s.split(','):
    if '-' in part:
        lo, hi = part.split('-')
        cpus.extend(range(int(lo), int(hi)+1))
    else:
        cpus.append(int(part))
print(' '.join(map(str, cpus)))
")"

NUM_CPUS="${#TARGET_CPUS[@]}"

echo "[INFO] CPU affinity of '$APP_NAME': [$CPUS_ALLOWED_LIST] → CPUs: ${TARGET_CPUS[*]}"
echo "[INFO] Stressing $NUM_CPUS CPU(s) at ${LOAD}% load for ${DURATION}s"
echo ""

# ── Python worker script (heredoc, passed via stdin to python3) ────────────
# Uses time.perf_counter() in a tight spin loop: no subprocess overhead.
# Duty cycle: spin for (load/100)*period, sleep for remainder. Period = 5ms.
PYWORKER='
import time, sys
duration = float(sys.argv[1])
load     = float(sys.argv[2]) / 100.0
period   = 0.005
deadline = time.monotonic() + duration
while time.monotonic() < deadline:
    spin_end = time.perf_counter() + period * load
    while time.perf_counter() < spin_end:
        pass
    slack = period * (1.0 - load)
    if slack > 0.0001:
        time.sleep(slack)
'

# ── Launch one worker per CPU ──────────────────────────────────────────────
WORKER_PIDS=()
for cpu in "${TARGET_CPUS[@]}"; do
    taskset -c "$cpu" python3 -c "$PYWORKER" "$DURATION" "$LOAD" &
    wpid=$!
    WORKER_PIDS+=("$wpid")
    echo "[INFO] Stress worker PID=$wpid pinned to CPU $cpu"
done

echo ""

# ── Cleanup on exit ────────────────────────────────────────────────────────
cleanup() {
    echo ""
    echo "[INFO] Stopping ${#WORKER_PIDS[@]} stress worker(s)..."
    for wpid in "${WORKER_PIDS[@]}"; do
        kill "$wpid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
    echo "[INFO] Stress complete."
}
trap cleanup EXIT INT TERM

echo "[INFO] Press Ctrl+C to stop early."
echo ""

# ── Live per-CPU utilization display ──────────────────────────────────────
# Read initial /proc/stat snapshot
declare -A PREV_IDLE PREV_TOTAL
for cpu in "${TARGET_CPUS[@]}"; do
    read -ra f <<< "$(grep "^cpu${cpu} " /proc/stat)"
    PREV_IDLE[$cpu]=${f[4]}
    tot=0; for ((i=1;i<${#f[@]};i++)); do tot=$((tot+f[i])); done
    PREV_TOTAL[$cpu]=$tot
done

for ((elapsed=1; elapsed<=DURATION; elapsed++)); do
    sleep 1

    # Check target process is still alive
    if ! kill -0 "$PID" 2>/dev/null; then
        echo ""
        echo "[WARN] '$APP_NAME' (PID $PID) has exited!"
        break
    fi

    util_str=""
    for cpu in "${TARGET_CPUS[@]}"; do
        read -ra f <<< "$(grep "^cpu${cpu} " /proc/stat)"
        idle=${f[4]}
        tot=0; for ((i=1;i<${#f[@]};i++)); do tot=$((tot+f[i])); done

        d_idle=$(( idle - PREV_IDLE[$cpu] ))
        d_tot=$(( tot  - PREV_TOTAL[$cpu] ))
        [[ $d_tot -gt 0 ]] && util=$(( 100 - d_idle * 100 / d_tot )) || util=0

        PREV_IDLE[$cpu]=$idle
        PREV_TOTAL[$cpu]=$tot
        util_str+="  CPU${cpu}=${util}%"
    done

    printf "[%3ds/%ds]%s\n" "$elapsed" "$DURATION" "$util_str"
done

echo ""
echo "[INFO] Waiting for workers to finish..."
