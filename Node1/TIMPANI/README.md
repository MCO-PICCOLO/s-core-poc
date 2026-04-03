<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

# TIMPANI

Distributed real-time scheduling system with time-triggered execution capabilities. TIMPANI provides both C and Rust implementations of node executors and schedulers for deterministic real-time applications.

This repository contains both original C implementations and modern Rust ports with enhanced type safety and memory safety.

## Architecture

- **TIMPANI-N (Node Executor)**: Executes time-triggered tasks on individual nodes
- **TIMPANI-O (Node Scheduler)**: Orchestrates and schedules tasks across distributed nodes
- **Sample Applications**: Real-time test applications for system validation

## Getting Started

### Clone the Repository

```bash
git clone --recurse-submodules https://github.com/MCO-PICCOLO/TIMPANI.git
cd TIMPANI
```

> **Note:** Use `--recurse-submodules` to automatically clone the required submodules (libbpf, etc.).

Refer to the individual component READMEs below for specific build and setup instructions.

## Components

### [Sample Applications](sample-apps/README.md)
Real-time sample applications for real-time system analysis. Provides periodic execution, deadline monitoring, and runtime statistics collection capabilities.

**Quick Build:**
```bash
cd sample-apps
mkdir build && cd build
cmake ..
make
```
*For detailed setup and usage → [Full Documentation](sample-apps/README.md)*

### [TIMPANI-N (Node Executor)](timpani-n/README.md)
C implementation of the time-triggered node executor component.

**Quick Build:**
```bash
cd timpani-n
mkdir build && cd build
cmake ..
make
```

- [CentOS Setup](timpani-n/README.CentOS.md)
- [Ubuntu 20 Setup](timpani-n/README.Ubuntu20.md)

*For detailed setup, dependencies, and usage → [Full Documentation](timpani-n/README.md)*

### [TIMPANI-O (Node Scheduler)](timpani-o/README.md)
C implementation of the orchestrator component with gRPC & protobuf support for distributed scheduling.

**Quick Build:**
```bash
cd timpani-o
mkdir build && cd build
cmake ..
make
```
*For detailed setup, protobuf configuration, and usage → [Full Documentation](timpani-o/README.md)*

### [TIMPANI Rust Components](timpani_rust/README.md)
Rust ports of TIMPANI components with enhanced type safety and memory safety.

#### [Rust TIMPANI-N (Node Executor)](timpani_rust/timpani-n/README.md)
Rust implementation of the node executor with comprehensive CLI interface, configuration validation, and structured logging. **Status**: Configuration parsing complete, runtime features in development.

**Quick Build:**
```bash
cd timpani_rust/timpani-n
cargo build --release
cargo test  # Run tests
```
*For detailed setup, usage examples, and current status → [Full Documentation](timpani_rust/timpani-n/README.md)*

#### [Rust TIMPANI-O (Node Scheduler)](timpani_rust/timpani-o/)
Rust implementation of the global scheduler component. **Status**: In development.

**Quick Build:**
```bash
cd timpani_rust/timpani-o
cargo build --release
cargo test  # Run tests
```
*For detailed setup and current development status → [Full Documentation](timpani_rust/timpani-o/README.md)*

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📖 Documentation Structure

```
TIMPANI/
├── README.md                    # This file - main project overview
├── sample-apps/
│   ├── README.md               # Sample applications documentation
├── timpani-n/
│   ├── README.md               # C implementation: Node executor
│   ├── README.CentOS.md       # CentOS setup guide
│   └── README.Ubuntu20.md     # Ubuntu setup guide
├── timpani-o/
│   └── README.md               # C implementation: Node scheduler
└── timpani_rust/
    ├── README.md               # Rust components overview
    ├── timpani-n/
    │   └── README.md           # Rust node executor (config parsing complete)
    └── timpani-o/
        └── README.md           # Rust node scheduler (in development)
```



---

**Navigation:** [Sample Apps](sample-apps/) | [TIMPANI-N (C)](timpani-n/) | [TIMPANI-O (C)](timpani-o/) | [Rust Components](timpani_rust/) | [Rust TIMPANI-N](timpani_rust/timpani-n/)
