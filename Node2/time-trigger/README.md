<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
-->

# Timpani-N (Time Trigger)

[![Version](https://img.shields.io/badge/version-2026.03-blue.svg)](CHANGELOG.md)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**Timpani-N** is a time-triggered task scheduler and monitor for real-time Linux systems. It uses eBPF-based instrumentation to detect deadline misses and provides fine-grained control over task scheduling.

## Key Features

- **Real-time Task Scheduling**: SCHED_FIFO, SCHED_RR, SCHED_DEADLINE support
- **eBPF-based Monitoring**: Kernel-level deadline miss detection via tracepoints
- **CPU Affinity Control**: Per-task CPU pinning and isolation
- **D-Bus Integration**: Communication with Timpani-O orchestrator via libtrpc

## Quick Start

### 1. Check System Requirements

```bash
# Run the requirements checker first
./scripts/check-requirements.sh
```

See [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) for detailed system requirements.

### 2. Install

**From Package (Recommended):**
```bash
# Ubuntu/Debian
sudo dpkg -i timpani-n_*.deb

# RHEL/CentOS
sudo rpm -i timpani-n-*.rpm
```

**From Source:**
```bash
git clone http://mod.lge.com/hub/timpani/time-trigger.git
cd time-trigger
git submodule update --init --recursive
mkdir build && cd build
cmake .. && make
sudo make install
```

See [INSTALL.md](INSTALL.md) for detailed installation instructions.

### 3. Run

```bash
# Run with root privileges
sudo timpani-n

# Or with capabilities (after package install)
timpani-n
```

See [docs/USAGE.md](docs/USAGE.md) for detailed usage instructions.

## Documentation

| Document | Description |
|----------|-------------|
| [INSTALL.md](INSTALL.md) | Installation guide for all platforms |
| [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) | System requirements and compatibility |
| [docs/BUILDING.md](docs/BUILDING.md) | Advanced build options and configurations |
| [docs/USAGE.md](docs/USAGE.md) | Usage examples and configuration |
| [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) | Package deployment and release management |

## Compatibility Matrix

| OS | Version | Kernel | Status |
|----|---------|--------|--------|
| Ubuntu | 24.04 LTS | 6.8+ | ✅ Supported (non-RT) |
| Ubuntu | 22.04 LTS | 5.15+ | ✅ Supported (non-RT) |
| Ubuntu | 20.04 LTS | 5.15+ (HWE) | ⚠️ Requires HWE kernel |
| RHEL/CentOS | 9 | 5.14+ | ✅ Supported (non-RT) |
| Fedora | 40+ | 6.12+ | ✅ RT-ready |

See [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) for full compatibility details.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    User Space                            │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │  Timpani-N  │◄──►│   libtrpc   │◄──►│  Timpani-O  │  │
│  │  (Monitor)  │    │   (D-Bus)   │    │(Orchestrator)│ │
│  └──────┬──────┘    └─────────────┘    └─────────────┘  │
│         │                                                │
├─────────┼────────────────────────────────────────────────┤
│         ▼           Kernel Space                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │    eBPF     │    │  Scheduler  │    │   ftrace    │  │
│  │  (sigwait)  │    │ FIFO/RR/DL  │    │  (events)   │  │
│  └─────────────┘    └─────────────┘    └─────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.
