#<!--
#* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
#* SPDX-License-Identifier: MIT
#-->

# Real-time Sample Applications

This project provides sample applications for real-time system analysis. It offers periodic execution, deadline monitoring, and runtime statistics collection capabilities.

## Key Features

- **Periodic Execution**: Execute tasks at configurable periods
- **Deadline Monitoring**: Deadline violation detection and statistics
- **Runtime Measurement**: Accurate execution time measurement and statistics collection
- **Real-time Priority**: SCHED_FIFO scheduling policy support
- **Various Workloads**: Support for 8 different workload algorithms
- Newton-Raphson square root calculation (CPU-intensive)
- Fibonacci sequence calculation (sequential computation)
- Busy wait loop (pure CPU time)
- Matrix multiplication (FPU-intensive)
- Memory-intensive random access
- Cryptographic hash simulation
- Mixed workload combination
- Prime number calculation (sequential memory access)
- **Statistics Report**: Min/Max/Average execution time, deadline violation rate, etc.

## Build Instructions

```bash
git clone https://github.com/MCO-PICCOLO/TIMPANI.git
cd sample-apps
mkdir build
cd build
cmake ..
cmake --build .
```

## Usage

### Basic Syntax
```bash
sudo ./sample_apps [OPTIONS] <task_name>
```

### Options

| Option | Description | Default |
|--------|-------------|----------|
| `-p, --period PERIOD` | Period (milliseconds) | 100 |
| `-d, --deadline DEADLINE` | Deadline (milliseconds) | Same as period |
| `-r, --runtime RUNTIME` | Expected runtime (microseconds) | 50000 |
| `-P, --priority PRIORITY` | Real-time priority (1-99) | 50 |
| `-a, --algorithm ALGO` | Algorithm selection (1-8) | 1 |
| `-l, --loops LOOPS` | Loop count/parameter | 10 |
| `-s, --stats` | Enable detailed statistics | Enabled by default |
| `-t, --timer` | Timer-based periodic execution | Signal-based |
| `-h, --help` | Display help | - |

### Execution Examples

#### 1. Basic Execution
```bash
# Run with 100ms period, 50ms deadline, priority 50
sudo ./sample_apps mytask
```

## Workload Algorithms

### Available Algorithms (8 types)

| Algorithm | Number | Description | Characteristics | Suitable Use Cases |
|-----------|--------|-------------|----------|---------------|
| NSQRT | 1 | Newton-Raphson square root | CPU-intensive, floating-point operations | Numerical computation, basic performance testing |
| Fibonacci | 2 | Fibonacci sequence calculation | Sequential computation, integer operations | Algorithm performance testing |
| Busy loop | 3 | Busy wait loop | Pure CPU time consumption | Precise time control |
| Matrix | 4 | Matrix multiplication | FPU-intensive, cache effects | Numerical operations, memory hierarchy testing |
| Memory | 5 | Memory-intensive access | Random memory access, cache misses | Memory system performance testing |
| Crypto | 6 | Cryptographic hash simulation | Bit operations, integer operations | Security operation simulation |
| Mixed | 7 | Mixed workload | Various operation combinations | Real application simulation |
| Prime | 8 | Prime number calculation | Sequential memory access, conditionals | Memory bandwidth testing |

### Workload Execution Examples

#### 1. Basic Execution (Newton-Raphson)
```bash
# 100ms period, 90ms deadline, default algorithm
sudo ./sample_apps -p 100 -d 90 -a 1 -l 5 basic_task
```

#### 2. Matrix Multiplication Workload
```bash
# 200ms period, matrix multiplication, size parameter 10
sudo ./sample_apps -p 200 -d 180 -a 4 -l 10 matrix_task
```

#### 3. Memory-intensive Workload
```bash
# 500ms period, 32MB memory test
sudo ./sample_apps -p 500 -d 450 -a 5 -l 32 memory_task
```

#### 4. Cryptographic Workload
```bash
# 50ms period, cryptographic hash, 2000 rounds
sudo ./sample_apps -p 50 -d 45 -a 6 -l 20 crypto_task
```

#### 5. Mixed Workload
```bash
# 100ms period, mixed workload, intensity 8
sudo ./sample_apps -p 100 -d 90 -a 7 -l 8 mixed_task
```

#### 6. Prime Calculation Workload
```bash
# 300ms period, calculate primes up to 500,000
sudo ./sample_apps -p 300 -d 270 -a 8 -l 50 prime_task
```

#### 7. Timer-based Periodic Execution
```bash
# Automatic periodic execution using timer (no external signal required)
sudo ./sample_apps -t -p 100 timer_task
```

## Execution Modes

1. **Signal-based Mode (Default)**
   - Task execution requires external SIGRTMIN+2 signal
   - Useful when integrating with external schedulers or trigger systems

```bash
# Terminal 1: Run the application
sudo ./sample_apps -p 50 -d 45 signal_task

# Terminal 2: Send periodic signals
while true; do
    killall -s $(($(kill -l SIGRTMIN) + 2)) sample_apps
    sleep 0.05  # 50ms period
done
```

2. **Timer-based Mode**
   - Uses internal timer for automatic periodic execution
   - Suitable for independent real-time tasks

```bash
sudo ./sample_apps -t -p 50 -d 45 timer_task
```

## Statistics Information

The application provides the following runtime statistics:

- **Min/Max/Average Runtime**: In microseconds
- **Deadline Violation Count and Rate**: Percentage of total executions
- **Total Execution Count**: Number of completed periods
- **Last Runtime**: Duration of the most recent execution

### Output Example
```text
=== Runtime Statistics for mytask ===
Iterations:      1000
Min runtime:     45230 us
Max runtime:     52100 us
Avg runtime:     48765 us
Last runtime:    49120 us
Deadline misses: 12 (1.20%)
Period:          50 ms
Deadline:        45 ms
Expected runtime: 50000 us
=====================================
```

## Container Execution

### Docker Build
```bash
# Build Ubuntu-based image
docker build -t ubuntu_latest:sample_apps_v3 -f ./Dockerfile.ubuntu ./

# Build CentOS-based image
docker build -t centos_latest:sample_apps_v3 -f ./Dockerfile.centos ./
```

### Docker Run
```bash
# Run basic workload (scheduling parameters managed by pullpiri and timpani-o/timpani-n)
docker run -it --rm -d \
    --ulimit rtprio=99 \
    --cap-add=sys_nice \
    --privileged \
    --name basic_task \
    ubuntu_latest:sample_apps_v3 \
    sample_apps basic_task

# Run specific algorithm workload (specify only algorithm and loop parameters)
docker run -it --rm -d \
    --ulimit rtprio=99 \
    --cap-add=sys_nice \
    --privileged \
    --name matrix_task \
    ubuntu_latest:sample_apps_v3 \
    sample_apps -a 4 -l 10 matrix_task
```

## Real-time Performance Tuning

### 1. System Configuration
```bash
# Set CPU governor to performance
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Disable IRQ balancing
sudo systemctl stop irqbalance

# Isolate specific CPU cores (kernel parameter at boot)
# isolcpus=2,3 nohz_full=2,3 rcu_nocbs=2,3
```

### 2. Priority Setting Guide
- **1-33**: Low priority (general real-time tasks)
- **34-66**: Medium priority (important real-time tasks)
- **67-99**: High priority (critical system tasks)

### 3. Period and Deadline Setting Recommendations
- **Deadline < Period**: Typical real-time system
- **Deadline = Period**: Default setting for periodic tasks
- **Short Period (< 10ms)**: High-frequency control systems
- **Long Period (> 100ms)**: Monitoring and logging tasks

## Workload-based Runtime Measurement

For detailed workload analysis and runtime measurement methods, refer to `WORKLOAD_GUIDE.md`.

### Quick Measurement Guide

1. **Baseline Measurement**: Measure base runtime for each workload with minimum parameters
2. **Gradual Increase**: Adjust parameters until target runtime is reached
3. **Period Setting**: `Period = Target_Runtime + Safety_Margin + System_Overhead`
4. **Deadline Setting**: `Deadline = Period × (0.8 ~ 0.95)` (depending on strictness)

### Expected Runtime Reference Table by Workload

| Algorithm | Parameter Example | Expected Runtime | Recommended Period/Deadline |
|----------|---------------|---------------|---------------------|
| NSQRT | `-l 5` | ~100μs | 50ms/45ms |
| Matrix | `-l 5` | ~1.5ms | 100ms/90ms |
| Memory | `-l 8` | ~52ms | 200ms/180ms |
| Crypto | `-l 5` | ~185μs | 50ms/45ms |
| Mixed | `-l 3` | ~5.7ms | 100ms/90ms |
| Prime | `-l 10` | ~few ms | 100ms/90ms |

## Troubleshooting

### Permission Issues
```bash
# sudo privileges required for real-time priority setting
sudo ./sample_apps ...

# Or grant real-time priority permissions to user
echo "username - rtprio 99" | sudo tee -a /etc/security/limits.conf
```

### Performance Issues
- If CPU usage is near 100%, increase the period or reduce loop count
- If deadline violations occur frequently, increase the deadline or reduce workload
- Stop unnecessary services to reduce system noise

### Signal Issues
- If signals are not delivered in signal-based mode, check the process name and PID
- If multiple instances are running, send signals to a specific PID

## License

This project is part of LG Electronics' Timpani Project.