<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
-->

# Usage Guide

This document covers how to use Timpani-N for time-triggered task scheduling and monitoring.

## Table of Contents

- [Basic Usage](#basic-usage)
  - [Running as a systemd Service](#running-as-a-systemd-service)
  - [Running Manually](#running-manually)
- [Command Line Options](#command-line-options)
- [Configuration](#configuration)
- [Example Scenarios](#example-scenarios)
- [Integration with Timpani-O](#integration-with-timpani-o)
- [Monitoring and Debugging](#monitoring-and-debugging)

---

## Basic Usage

### Running as a systemd Service

After installing timpani-n via package (DEB/RPM), you can manage it as a systemd service:

**Check service status:**
```bash
systemctl status timpani-n
```

**Start the service:**
```bash
sudo systemctl start timpani-n
```

**Stop the service:**
```bash
sudo systemctl stop timpani-n
```

**Restart the service:**
```bash
sudo systemctl restart timpani-n
```

**Enable auto-start on boot:**
```bash
sudo systemctl enable timpani-n
```

**Disable auto-start:**
```bash
sudo systemctl disable timpani-n
```

**View service logs:**
```bash
# Follow logs in real-time
sudo journalctl -u timpani-n -f

# View recent logs
sudo journalctl -u timpani-n -n 100

# View logs since last boot
sudo journalctl -u timpani-n -b
```

**Service configuration:**

The systemd service file is located at `/usr/lib/systemd/system/timpani-n.service`.

To customize the service (e.g., add configuration file or change options):

```bash
# Create override file
sudo systemctl edit timpani-n
```

Add your customizations:
```ini
[Service]
ExecStart=
ExecStart=/usr/bin/timpani-n -P 25 -c 0 -l 3
```

Save and reload:
```bash
sudo systemctl daemon-reload
sudo systemctl restart timpani-n
```

### Running Manually

For development or testing, you can run timpani-n directly:

```bash
# Run with root privileges
sudo timpani-n

# Or if capabilities are set
timpani-n

# From build directory
cd build
sudo ./timpani-n
```

### Quick Test with Sample Processes

Open multiple terminals and run:

**Terminal 1 - Sample process (10ms period):**
```bash
cd build
sudo ./exprocs wakee1 10000
```

**Terminal 2 - Sample process (50ms period):**
```bash
cd build
sudo ./exprocs wakee2 50000
```

**Terminal 3 - Sample process (20ms period):**
```bash
cd build
sudo ./exprocs wakee3 20000
```

**Terminal 4 - Dummy SchedInfo server:**
```bash
cd build
./dummy_server
```

**Terminal 5 - Timpani-N:**
```bash
cd build
sudo ./timpetrigger
```

---

## Command Line Options

```bash
timpani-n [OPTIONS]

Options:
  -c <cpu_num>   CPU affinity for timpani-n
  -P <prio>      RT priority (1~99) for timpani-n
  -p <port>      Port to connect to
  -n <node id>   Node ID (default: hostname)
  -l <level>     Log level (0=silent, 1=error, 2=warning, 3=info, 4=debug, 5=verbose)
  -s             Enable timer synchronization across multiple nodes
  -g             Enable saving plot data file by using BPF (<node id>.gpdata)
  -a             Enable Apex.OS test mode which works without TT schedule info
  -h             Show this help
```

---

## Configuration

### schedinfo.yaml

Timpani-N reads task information from `schedinfo.yaml`:

```yaml
# Example schedinfo.yaml
tasks:
  - name: wakee1
    pid: 0           # Auto-detect by name
    period_us: 10000
    deadline_us: 8000
    priority: 90
    cpu: 1
    policy: SCHED_FIFO

  - name: wakee2
    pid: 0
    period_us: 50000
    deadline_us: 45000
    priority: 80
    cpu: 2
    policy: SCHED_FIFO

  - name: wakee3
    pid: 0
    period_us: 20000
    deadline_us: 18000
    priority: 85
    cpu: 3
    policy: SCHED_RR
```

### Configuration Fields

| Field | Description | Values |
|-------|-------------|--------|
| `name` | Process name | String |
| `pid` | Process ID (0 = auto-detect) | Integer |
| `period_us` | Task period in microseconds | Integer |
| `deadline_us` | Task deadline in microseconds | Integer |
| `priority` | RT priority (1-99) | Integer |
| `cpu` | CPU affinity | Integer or list |
| `policy` | Scheduling policy | SCHED_FIFO, SCHED_RR, SCHED_DEADLINE |

---

## Example Scenarios

### Scenario 1: Basic Deadline Monitoring

Monitor tasks for deadline misses:

```bash
# Start timetrigger with BPF tracing
sudo ./timetrigger

# Output shows deadline miss events
[INFO] Task wakee1 (PID 1234) deadline miss detected
[INFO]   Expected: 10000 us, Actual: 12500 us
```

### Scenario 2: CPU Isolation

Isolate real-time tasks on specific CPUs:

1. Boot with CPU isolation:
```bash
# Add to kernel command line
isolcpus=2,3 nohz_full=2,3 rcu_nocbs=2,3
```

2. Configure tasks to use isolated CPUs:
```yaml
tasks:
  - name: critical_task
    cpu: 2
    priority: 99
    policy: SCHED_FIFO
```

### Scenario 3: SCHED_DEADLINE

Use Linux's deadline scheduler:

```yaml
tasks:
  - name: deadline_task
    policy: SCHED_DEADLINE
    period_us: 10000
    deadline_us: 8000
    runtime_us: 2000    # Required for SCHED_DEADLINE
```

---

## Integration with Timpani-O

Timpani-N communicates with Timpani-O (orchestrator) via libtrpc (D-Bus).

### Architecture

```
┌─────────────┐         ┌─────────────┐
│  Timpani-O  │◄──────►│  Timpani-N  │
│ (Central)   │ libtrpc │  (Node)    │
│ Port 7777   │  D-Bus  │            │
└─────────────┘         └─────────────┘
```

### Connection

Timpani-N automatically connects to Timpani-O on startup:

```bash
# Timpani-O must be running first
# On orchestrator node:
./timpani-o -d 7777

# On worker node:
sudo ./timetrigger
```

### Messages

| Direction | Message | Description |
|-----------|---------|-------------|
| N → O | `TaskRegistered` | Task registered for monitoring |
| N → O | `DeadlineMiss` | Deadline miss detected |
| N → O | `SchedulingStats` | Periodic statistics |
| O → N | `SetScheduling` | Set task scheduling parameters |
| O → N | `SetAffinity` | Set task CPU affinity |

---

## Monitoring and Debugging

### Debug Mode

```bash
sudo ./timpani-n -l 4
```

### BPF Program Status

```bash
# List loaded BPF programs
sudo bpftool prog list

# Show BPF maps
sudo bpftool map list
```

### Tracing Output

```bash
# View ftrace buffer
sudo cat /sys/kernel/debug/tracing/trace

# Clear buffer
sudo echo > /sys/kernel/debug/tracing/trace
```

### Performance Monitoring

```bash
# Monitor scheduling latencies
sudo perf sched latency

# Record scheduling events
sudo perf sched record -- sleep 10
sudo perf sched latency
```

### Deadline Miss Analysis

When a deadline miss occurs, Timpani-N logs:

```
[DEADLINE_MISS] Task: wakee1 (PID: 1234)
  Period:    10000 us
  Deadline:   8000 us
  Actual:    12500 us
  Overrun:    4500 us (56.25%)
  CPU:       2
  Timestamp: 1234567890.123456
```

---

## Troubleshooting

### "Failed to attach BPF program"

```bash
# Check if tracepoints exist
sudo cat /sys/kernel/debug/tracing/available_events | grep sigtimedwait

# Verify BTF
ls -la /sys/kernel/btf/vmlinux
```

### "Permission denied"

```bash
# Run as root
sudo ./timetrigger

# Or set capabilities
sudo setcap cap_bpf,cap_perfmon,cap_sys_nice,cap_sys_ptrace+ep ./timetrigger
```

### "Cannot connect to Timpani-O"

```bash
# Check if Timpani-O is running
ps aux | grep timpani-o

# Check D-Bus port
ss -tlnp | grep 7777
```

### High CPU usage

eBPF event tracing can have overhead. Disable if not needed:

```bash
# Build without BPF event tracing
cmake -DCONFIG_TRACE_BPF_EVENT=OFF ..
```

---

## Next Steps

- [REQUIREMENTS.md](REQUIREMENTS.md) - System requirements
- [BUILDING.md](BUILDING.md) - Build options
- [../INSTALL.md](../INSTALL.md) - Installation guide
