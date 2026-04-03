#!/usr/bin/env python3
# vim: set sw=4 ts=4 et:

# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

import sys
import os
import os.path
import json
import argparse

def create_trace_event(start, end, name, resource=None, per_cpu=False):
    is_wakeup = name.endswith("_wakeup")
    base_name = name.replace("_wakeup", "") if is_wakeup else name

    # Parse CPU core information from resource (e.g., "node01-C2" -> "CPU 2")
    cpu_core = "Unknown"
    if resource and per_cpu:
        # Extract CPU core from resource string (e.g., "node01-C2" -> "C2")
        parts = resource.split('-')
        if len(parts) >= 2 and parts[1].startswith('C'):
            cpu_core = f"{parts[0]} Core {parts[1][1:]}"  # Convert "C2" to "CPU 2"

    if per_cpu:
        pid = cpu_core
        tid = base_name
    else:
        if resource:
            # Extract Node ID from resource string (e.g., "node01-C2" -> "node01")
            parts = resource.split('-')
            pid = f"Tasks on {parts[0]}"
        else:
            pid = "Tasks"
        tid = base_name

    return {
        "name": "Wakeup Latency" if is_wakeup else "Execution",
        "cat": "Task Scheduling",
        "ph": "X", # 'X' indicates a complete event
        "ts": start, # Start time (already in us unit for Chrome JSON)
        "dur": end - start, # Duration (already in us unit for Chrome JSON)
        "pid": pid,  # Process ID (CPU core when per_cpu=True)
        "tid": tid,  # Thread ID (task name)
        "args": {
            "task": base_name,
            "cpu_core": cpu_core if per_cpu else "N/A",
            "resource": resource if resource else "N/A",
            "start_time": start,
            "end_time": end,
            "duration_us": end - start,
            "type": "wakeup_latency" if is_wakeup else "execution"
        }  # Additional arguments
    }

def generate_combined_trace_json(events):
    """Generate a single trace JSON containing both per-task and per-CPU views"""
    all_trace_events = []

    # Generate per-task events
    for event in events:
        start, end, name, resource = event
        trace_event = create_trace_event(start, end, name, resource, per_cpu=False)
        all_trace_events.append(trace_event)

    # Generate per-CPU events
    for event in events:
        start, end, name, resource = event
        trace_event = create_trace_event(start, end, name, resource, per_cpu=True)
        all_trace_events.append(trace_event)

    trace_json = {
        "traceEvents": all_trace_events,
        "displayTimeUnit": "us",  # Display time in microseconds since our data is in us
    }

    return trace_json

def generate_combined_trace_file(events, filename):
    trace_json = generate_combined_trace_json(events)
    save_trace_to_file(trace_json, filename)

def save_trace_to_file(trace_json, filename):
    with open(filename, 'w') as f:
        json.dump(trace_json, f, indent=2)

def read_events_from_file(filename, include_wakeup=False):
    events = []
    with open(filename, 'r') as f:
        for line in f:
            parts = line.strip().split()
            # Format: taskname event ignored resource priority wakeuptime starttime stoptime ignored
            # We need columns: 0 (taskname), 3 (resource), 5 (wakeuptime), 6 (starttime), 7 (stoptime)
            if len(parts) >= 8:
                try:
                    name = parts[0]
                    resource = parts[3]  # Resource column (e.g., "node01-C2")
                    wakeup_time = int(parts[5])  # Already in microseconds
                    start_time = int(parts[6])   # Already in microseconds
                    stop_time = int(parts[7])    # Already in microseconds

                    events.append((start_time, stop_time, name, resource))

                    # Optionally include wakeup latency as a separate event
                    if include_wakeup and wakeup_time < start_time:
                        events.append((wakeup_time, start_time, f"{name}_wakeup", resource))

                except (ValueError, IndexError):
                    # Skip lines that don't have valid numeric data
                    continue
    return events

if __name__ == "__main__":

    parser = argparse.ArgumentParser(description="Convert scheduling log data to Chrome JSON trace format")
    parser.add_argument("-i", "--input", type=str, nargs='+', required=True, help="input data file")
    parser.add_argument("-o", "--output", type=str, required=True, help="output JSON file")

    args = parser.parse_args()

    output_file = args.output

    all_events = []

    for input_file in args.input:
        print(f"Processing input file: {input_file}")
        all_events += read_events_from_file(input_file, include_wakeup=True)

    print(f"Generating Chrome JSON trace file: {output_file}")
    generate_combined_trace_file(all_events, output_file)

    sys.exit(0)
