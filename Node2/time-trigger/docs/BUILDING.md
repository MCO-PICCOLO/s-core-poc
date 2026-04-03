<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
-->

# Building from Source

This document covers advanced build options and packaging.

## Table of Contents

- [Basic Build](#basic-build)
- [Build Options](#build-options)
- [Packaging](#packaging)
- [Development Build](#development-build)

---

## Basic Build

```bash
git clone http://mod.lge.com/hub/timpani/time-trigger.git
cd time-trigger
git submodule update --init --recursive
mkdir build && cd build
cmake ..
make -j$(nproc)
```

---

## Build Options

### CMake Options

| Option | Default | Description |
|--------|---------|-------------|
| `CONFIG_TRACE_BPF` | ON | Enable eBPF-based deadline miss detection |
| `CONFIG_TRACE_BPF_EVENT` | OFF | Enable eBPF-based sched event tracing |
| `BUILD_TESTS` | OFF | Build test programs |
| `CMAKE_BUILD_TYPE` | Release | Build type (Release/Debug) |

### Examples

```bash
# Disable eBPF support
cmake -DCONFIG_TRACE_BPF=OFF ..

# Enable eBPF event tracing (CPU time, scheduling latency)
cmake -DCONFIG_TRACE_BPF_EVENT=ON ..

# Debug build
cmake -DCMAKE_BUILD_TYPE=Debug ..

# Build with tests
cmake -DBUILD_TESTS=ON ..
make
make test
```

### Feature Descriptions

#### CONFIG_TRACE_BPF (Default: ON)

Enables eBPF-based sigwait tracing for deadline miss detection.

- Attaches to `sys_enter_rt_sigtimedwait` and `sys_exit_rt_sigtimedwait` tracepoints
- Detects when time-triggered tasks miss their deadlines
- Requires: Linux 5.15+, BTF, CAP_BPF

#### CONFIG_TRACE_BPF_EVENT (Default: OFF)

Enables additional eBPF-based event tracing.

- Tracks `sched_switch` and `sched_waking` events
- Calculates on-CPU time and scheduling latency
- Higher overhead than CONFIG_TRACE_BPF alone
- Useful for detailed performance analysis

---

## Packaging

**For automated package building with proper architecture detection, use:**
```bash
./scripts/build-packages.sh
```

See [DEPLOYMENT.md](DEPLOYMENT.md) for detailed package building and distribution.

### Manual Package Creation (DEB)

```bash
cd build
cpack -G DEB
```

Output: `timpani-n_2026.3.0_amd64.deb`

### RPM Package (RHEL/CentOS/Fedora)

```bash
cd build
cpack -G RPM
```

Output: `timpani-n-2026.3.0.x86_64.rpm`

### Tarball

```bash
cd build
cpack -G TGZ
```

Output: `timpani-n-2026.3.0-Linux.tar.gz`

### Package Contents

```
/usr/bin/timpani-n                          # Main binary
/usr/share/timpani-n/check-requirements.sh  # Requirements checker
/usr/share/doc/timpani-n/                   # Documentation
```

### Package Dependencies

The packages declare runtime dependencies:

**DEB:**
```
Depends: libelf1, zlib1g, libsystemd0
```

**RPM:**
```
Requires: elfutils-libelf, zlib, systemd-libs
```

---

## Development Build

### Debug Build with Symbols

```bash
cmake -DCMAKE_BUILD_TYPE=Debug ..
make -j$(nproc)
```

### AddressSanitizer

```bash
cmake -DCMAKE_BUILD_TYPE=Debug \
      -DCMAKE_C_FLAGS="-fsanitize=address" \
      -DCMAKE_CXX_FLAGS="-fsanitize=address" ..
make -j$(nproc)
```

### Verbose Build

```bash
make VERBOSE=1
```

### Clean Build

```bash
rm -rf build
mkdir build && cd build
cmake ..
make -j$(nproc)
```

---

## Build Troubleshooting

### "Cannot find libbpf"

```bash
# Ensure submodules are initialized
git submodule update --init --recursive
```

### "bpftool not found"

```bash
# Ubuntu 22.04+
sudo apt install -y linux-tools-$(uname -r)

# CentOS/RHEL
sudo dnf install -y bpftool

# Verify
which bpftool
bpftool version
```

### "clang: error: unknown target CPU"

Clang version is too old for eBPF.

```bash
# Check clang version (need 12+)
clang --version

# Install newer clang
sudo apt install -y clang-17
```

### "fatal error: 'bpf/bpf_helpers.h' not found"

libbpf submodule not properly initialized.

```bash
git submodule update --init --recursive
rm -rf build && mkdir build && cd build
cmake ..
```

---

## Next Steps

- [REQUIREMENTS.md](REQUIREMENTS.md) - System requirements
- [USAGE.md](USAGE.md) - Usage examples
- [../INSTALL.md](../INSTALL.md) - Installation guide
