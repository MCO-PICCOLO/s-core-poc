# Timpani-N Node Executor

> **⚠️ Development Status**: This is a **work-in-progress** Rust port of the C implementation. Core configuration and CLI are complete, but runtime features are still being developed. See [Current Implementation Status](#current-implementation-status) for details.

Timpani-N is a Rust implementation of the Timpani node executor, providing time-triggered scheduling capabilities for distributed real-time systems. This is a complete port from the original C implementation with enhanced type safety, memory safety, and modern Rust features.

## Overview

Timpani-N acts as a **node executor** in the Timpani distributed real-time system architecture:
- **Timpani-N (Node Executor)**: Executes scheduled tasks on individual nodes
- **Timpani-O (Node Scheduler)**: Orchestrates and schedules tasks across the distributed system

## Features

- 🔧 **Real-time scheduling** with configurable RT priority (1-99) *(TBD - config parsing implemented, runtime RT scheduling TBD)*
- 🔧 **CPU affinity control** for deterministic execution *(TBD - config parsing implemented, runtime CPU binding TBD)*
- 📋 **Distributed synchronization** across multiple nodes *(TBD - network communication not yet implemented)*
- 📋 **BPF-based plotting** for performance analysis *(TBD - BPF integration not yet implemented)*
- 📋 **Apex.OS test mode** compatibility *(TBD - Apex.OS integration not yet implemented)*
- ✅ **Configurable logging** with multiple levels
- ✅ **Type-safe configuration** with comprehensive validation
- ✅ **Memory-safe** Rust implementation

## Current Implementation Status

This is a **work-in-progress** Rust port of the C implementation. Here's what's currently available:

### ✅ **Fully Implemented**
- ✅ **Configuration parsing** and validation
- ✅ **Command-line interface** with clap
- ✅ **Logging system** with tracing
- ✅ **Error handling** with comprehensive error types
- ✅ **Type safety** and memory safety
- ✅ **Unit and integration tests**
- ✅ **Build system** with Cargo

### 🔧 **Partially Implemented**
- 🔧 **Basic application structure** (config → initialize → run → cleanup)
- 🔧 **Context management** (data structures defined, initialization TBD)

### 📋 **To Be Developed (TBD)**
- 📋 **Real-time scheduling** (RT priority setting)
- 📋 **CPU affinity control** (actual CPU binding)
- 📋 **Network communication** (scheduler connection)
- 📋 **Multi-node synchronization**
- 📋 **BPF integration** (performance plotting)
- 📋 **Apex.OS integration**
- 📋 **Time-triggered task execution**
- 📋 **Hyperperiod management**
- 📋 **Signal handling**
- 📋 **D-Bus communication**

### 💡 **Current Behavior**
When you run `timpani-n` now, it will:
1. ✅ Parse and validate your configuration
2. ✅ Initialize logging
3. ✅ Display configuration settings
4. ⚠️ Print "Runtime loop not yet implemented"
5. ✅ Exit cleanly

## Prerequisites

### System Requirements
- **Rust**: 1.70+ (with `rustc` and `cargo`)
- **OS**: Linux (Ubuntu 20.04+, CentOS 7+, or compatible)
- **Architecture**: x86_64, aarch64
- **Permissions**: Root privileges may be required for RT scheduling

### Dependencies
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install build-essential git

# CentOS/RHEL/Fedora
sudo yum groupinstall "Development Tools"
sudo yum install git

# Or using dnf on newer versions
sudo dnf groupinstall "Development Tools"
sudo dnf install git
```

### Install Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version  # Verify installation
```

## Build Instructions

### 1. Clone the Repository
```bash
git clone <repository-url>
cd TIMPANI/timpani_rust/timpani-n
```

### 2. Build the Project
```bash
# Development build (debug mode)
cargo build

# Release build (optimized)
cargo build --release

# Build output location:
# - Debug: ./target/debug/timpani-n
# - Release: ./target/release/timpani-n
```

### 3. Run Tests
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_config_validation

# Test coverage (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

### 4. Install Binary
```bash
# Install to ~/.cargo/bin (must be in PATH)
cargo install --path .

# Or copy manually
sudo cp target/release/timpani-n /usr/local/bin/

# Verify installation
timpani-n --help
```

## Usage

### Basic Syntax
```bash
timpani-n [OPTIONS] [HOST]
```

### Command-Line Options

| Option | Short | Description | Default | Example |
|--------|-------|-------------|---------|---------|
| `--cpu <CPU_NUM>` | `-c` | CPU affinity for time trigger | No affinity | `-c 2` |
| `--prio <PRIO>` | `-P` | RT priority (1-99) | Default scheduler | `-P 50` |
| `--port <PORT>` | `-p` | Connection port | 7777 | `-p 8080` |
| `--node-id <NODE_ID>` | `-n` | Node identifier | "1" | `-n node-01` |
| `--log-level <LEVEL>` | `-l` | Log verbosity (0-5) | 3 (info) | `-l 4` |
| `--enable-sync` | `-s` | Enable multi-node sync | Disabled | `-s` |
| `--enable-plot` | `-g` | Enable BPF plotting | Disabled | `-g` |
| `--enable-apex` | `-a` | Apex.OS test mode | Disabled | `-a` |
| `--help` | `-h` | Show help message | - | `-h` |

### Log Levels
- **0 (Silent)**: No output
- **1 (Error)**: Error messages only
- **2 (Warning)**: Warnings and errors
- **3 (Info)**: General information (default)
- **4 (Debug)**: Detailed debugging
- **5 (Verbose)**: Maximum verbosity

## Usage Examples

### 1. Basic Usage (Default Configuration)
```bash
# Run with all defaults
timpani-n

# Current output:
# Configuration:
#   CPU affinity: -1
#   Priority: -1
#   Server: 127.0.0.1:7777
#   Node ID: 1
#   Log level: Info
# Runtime loop not yet implemented
```

### 2. Connect to Remote Server *(TBD)*
```bash
# Connect to scheduler at 192.168.1.100:8080 (TBD - network communication not implemented)
timpani-n --port 8080 192.168.1.100

# Or using short options
timpani-n -p 8080 192.168.1.100
```
**Note**: Network communication with remote scheduler is not yet implemented.

### 3. High-Priority Real-Time Configuration *(TBD)*
```bash
# RT priority 80, CPU core 2, verbose logging (TBD - RT scheduling not implemented)
sudo timpani-n --cpu 2 --prio 80 --log-level 5 --node-id rt-node-01

# Expected behavior (when implemented):
# - Binds to CPU core 2 (TBD)
# - Uses RT priority 80 (TBD - requires root)
# - Maximum logging verbosity ✅
# - Node identifier: rt-node-01 ✅
```
**Note**: Real-time priority setting and CPU affinity binding are not yet implemented.

### 4. Development/Testing Setup *(TBD)*
```bash
# Debug mode with sync and plotting enabled (TBD - features not implemented)
timpani-n --enable-sync --enable-plot --log-level 4 --node-id dev-node

# Expected output (when implemented):
# - Synchronization status (TBD)
# - BPF plot data (.gpdata files) (TBD)
# - Debug-level logging ✅
```
**Note**: Multi-node synchronization and BPF plotting are not yet implemented.

### 5. Apex.OS Compatibility Mode *(TBD)*
```bash
# Test mode without TT schedule info (TBD - Apex.OS integration not implemented)
timpani-n --enable-apex --node-id apex-test --log-level 2

# Use when (future implementation):
# - Testing without full scheduler (TBD)
# - Apex.OS integration testing (TBD)
# - Simplified deployment scenarios (TBD)
```
**Note**: Apex.OS integration is not yet implemented.

### 6. Multi-Node Synchronized Setup *(TBD)*

**Node 1:**
```bash
# TBD - Multi-node synchronization and network communication not implemented
sudo timpani-n --cpu 0 --prio 60 --enable-sync --node-id node-01 --port 7777 scheduler.example.com
```

**Node 2:**
```bash
# TBD - Multi-node synchronization and network communication not implemented
sudo timpani-n --cpu 1 --prio 60 --enable-sync --node-id node-02 --port 7777 scheduler.example.com
```

**Node 3:**
```bash
# TBD - Multi-node synchronization and network communication not implemented
sudo timpani-n --cpu 2 --prio 60 --enable-sync --node-id node-03 --port 7777 scheduler.example.com
```

**Note**: Multi-node synchronization, network communication, RT priority setting, and CPU affinity are not yet implemented.

### 7. Performance Analysis Setup *(TBD)*
```bash
# Enable plotting for performance analysis (TBD - BPF plotting not implemented)
timpani-n --enable-plot --enable-sync --cpu 3 --prio 70 --node-id perf-node

# Expected output (when implemented):
# - perf-node.gpdata file for analysis (TBD)
# - Use with gnuplot or similar tools for visualization (TBD)
```
**Note**: BPF-based performance plotting is not yet implemented.

## Configuration Validation

The system performs comprehensive configuration parsing and validation:

### Currently Implemented ✅
```bash
timpani-n --log-level 0            # Silent mode - ✅
timpani-n --log-level 5            # Verbose mode - ✅
timpani-n --port 1 --node-id test  # Port validation - ✅
timpani-n --port 65535             # Maximum port - ✅
timpani-n --node-id "custom-node"   # Node ID validation - ✅
```

### Runtime Features (TBD) 🔧
```bash
timpani-n --cpu 0 --prio 1        # Config parsing ✅, RT setting TBD
timpani-n --cpu 15 --prio 99       # Config parsing ✅, RT setting TBD
timpani-n --enable-sync            # Config parsing ✅, sync implementation TBD
timpani-n --enable-plot            # Config parsing ✅, BPF plotting TBD
timpani-n --enable-apex            # Config parsing ✅, Apex.OS integration TBD
```

### Invalid Configurations ❌
```bash
timpani-n --prio 100              # Error: Priority must be 1-99 or -1
timpani-n --prio 0                # Error: Priority must be 1-99 or -1
timpani-n --cpu 1025              # Error: CPU number too high
timpani-n --cpu -2                # Error: CPU must be -1 or >= 0
timpani-n --port 0                # Error: Port must be 1-65535
timpani-n --log-level 6           # Error: Log level must be 0-5
timpani-n --node-id ""            # Error: Node ID cannot be empty
```

## Environment Variables

The application respects standard environment variables:

```bash
# Rust logging (if tracing not initialized)
export RUST_LOG=timpani_n=debug
timpani-n

# Rust backtrace for debugging
export RUST_BACKTRACE=1
timpani-n

# Custom configuration via environment
export TIMPANI_DEFAULT_PORT=8080
export TIMPANI_LOG_LEVEL=4
```

## Integration Examples

### Docker Deployment
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/timpani-n /usr/local/bin/
ENTRYPOINT ["timpani-n"]
CMD ["--help"]
```

```bash
# Build and run container
docker build -t timpani-n .
docker run --rm timpani-n --node-id docker-node --log-level 3 scheduler.local
```

### Systemd Service
```ini
# /etc/systemd/system/timpani-n.service
[Unit]
Description=Timpani-N Node Executor
After=network.target

[Service]
Type=simple
User=timpani
Group=timpani
ExecStart=/usr/local/bin/timpani-n --cpu 2 --prio 50 --enable-sync --node-id %H scheduler.internal
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start service
sudo systemctl enable timpani-n
sudo systemctl start timpani-n
sudo systemctl status timpani-n
```

## Troubleshooting

### Common Issues

#### 1. Permission Denied for RT Priority *(TBD)*
```bash
# Problem (when RT scheduling is implemented):
sudo timpani-n --prio 50
# Error: Operation not permitted (TBD - RT scheduling not yet implemented)

# Future solution: Check RT limits
ulimit -r
# If output is 0, increase RT priority limit:
echo "timpani soft rtprio 99" | sudo tee -a /etc/security/limits.conf
echo "timpani hard rtprio 99" | sudo tee -a /etc/security/limits.conf
```
**Note**: Real-time priority setting is not yet implemented.

#### 2. CPU Affinity Not Working *(TBD)*
```bash
# Problem (when CPU affinity is implemented):
timpani-n --cpu 8
# Currently: Config validation works, runtime CPU binding TBD

# For future implementation: Check available CPUs
nproc                    # Number of CPUs
cat /proc/cpuinfo       # CPU details
# Use CPU number within range (0 to nproc-1)
```
**Note**: CPU affinity binding is not yet implemented.

#### 3. Connection Issues *(TBD)*
```bash
# Problem (when network communication is implemented):
timpani-n --port 7777 scheduler.example.com
# Currently: Config parsing works, network communication TBD

# For future debugging:
ping scheduler.example.com                    # Check connectivity
telnet scheduler.example.com 7777           # Test port access
nslookup scheduler.example.com              # Verify DNS resolution
```
**Note**: Network communication with scheduler is not yet implemented.

#### 4. Port Already in Use *(TBD)*
```bash
# Problem (when network communication is implemented):
timpani-n --port 7777
# Currently: Config parsing works, network binding TBD

# For future debugging:
sudo netstat -tulpn | grep 7777
# Kill conflicting process or use different port
timpani-n --port 7778
```
**Note**: Network port binding is not yet implemented.

### Debugging Tips

#### 1. Increase Logging Verbosity
```bash
# Maximum debug output
timpani-n --log-level 5 > debug.log 2>&1

# Or use Rust environment logging
RUST_LOG=timpani_n=trace timpani-n
```

#### 2. Validate Configuration
```bash
# Test configuration without running
timpani-n --cpu 2 --prio 50 --port 8080 --help
# If help displays, configuration parsing succeeded
```

#### 3. Check System Resources *(TBD)*
```bash
# Monitor CPU usage (when runtime is implemented)
top -p $(pgrep timpani-n)

# Check memory usage
ps aux | grep timpani-n

# Monitor network connections (when networking is implemented)
ss -tulpn | grep timpani-n
```
**Note**: Full runtime implementation is TBD.

## Development

### Building from Source
```bash
# Clone repository
git clone <repository-url>
cd TIMPANI/timpani_rust/timpani-n

# Development workflow
cargo check          # Fast syntax checking
cargo clippy          # Linting
cargo fmt             # Code formatting
cargo test            # Run tests
cargo bench           # Benchmarks (if available)
```

### Code Structure
```
src/
├── main.rs           # Application entry point
├── lib.rs            # Library interface
├── config.rs         # Configuration management
├── context.rs        # Runtime context
└── error.rs          # Error handling

tests/
└── integration_tests.rs  # Integration tests
```

### Running Tests
```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# Specific test
cargo test test_config_validation

# With coverage
cargo tarpaulin --out html --output-dir coverage
```

### Contributing
1. Follow Rust coding conventions
2. Add tests for new features
3. Update documentation
4. Run `cargo clippy` and `cargo fmt`
5. Ensure all tests pass

## Performance Considerations

### Current Implementation
- **Memory**: Release builds use ~50% less memory than debug builds ✅
- **Logging**: Lower log levels (0-2) reduce overhead in production ✅
- **Configuration**: Fast parsing and validation with zero-copy where possible ✅

### Future Implementation (TBD)
- **RT Priority**: Higher values (80-99) will provide better real-time guarantees *(TBD)*
- **CPU Affinity**: Binding to specific cores will reduce scheduling overhead *(TBD)*
- **Network**: Connection pooling and efficient serialization *(TBD)*
- **BPF**: Low-overhead performance monitoring *(TBD)*

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Support

For issues, feature requests, or questions:

### Current Development Phase
1. **Configuration Issues**: Fully supported - report any CLI parsing or validation problems
2. **Build Issues**: Fully supported - report compilation or test failures
3. **Runtime Features**: Most are TBD - see [Current Implementation Status](#current-implementation-status)

### Reporting Issues
1. Check this documentation and current implementation status
2. Review existing issues in the repository
3. Create a new issue with detailed information:
   - **For implemented features**: Include logs with `--log-level 5`
   - **For TBD features**: Reference this README's implementation status
   - **For new features**: Propose additions to the implementation roadmap

### Contributing
Contributions welcome, especially for TBD features! See the [Development](#development) section.

---

**Note**: This is a Rust port of the original C implementation. For the C version documentation, see the main project README.
