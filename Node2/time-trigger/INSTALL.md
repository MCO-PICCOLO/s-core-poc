<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
-->

# Installation Guide

This guide covers installation of Timpani-N on various Linux distributions.

> **Important:** Before installation, run the requirements checker:
> ```bash
> ./scripts/check-requirements.sh
> ```

## Table of Contents

- [Package Installation (Recommended)](#package-installation-recommended)
- [Building from Source](#building-from-source)
  - [Ubuntu 22.04 / 24.04](#ubuntu-2204--2404)
  - [Ubuntu 20.04](#ubuntu-2004)
  - [RHEL / CentOS Stream 9/10](#rhel--centos-stream-910)
  - [Fedora](#fedora)
- [Post-Installation](#post-installation)
- [Troubleshooting](#troubleshooting)

---

## Package Installation (Recommended)

### Ubuntu / Debian

```bash
# Download the package
wget https://releases.example.com/timpani-n_2026.3.0_amd64.deb

# Install
sudo dpkg -i timpani-n_2026.3.0_amd64.deb

# Fix dependencies if needed
sudo apt-get install -f
```

### RHEL / CentOS / Fedora

```bash
# Download the package
wget https://releases.example.com/timpani-n-2026.3.0.x86_64.rpm

# Install
sudo rpm -i timpani-n-2026.3.0.x86_64.rpm
# or
sudo dnf install timpani-n-2026.3.0.x86_64.rpm
```

---

## Building from Source

### Ubuntu 22.04 / 24.04

#### Install Dependencies

```bash
# Basic build tools
sudo apt install -y build-essential cmake pkg-config git

# eBPF/libbpf dependencies
sudo apt install -y libelf-dev zlib1g-dev

# BPF compiler and tools
sudo apt install -y clang llvm
sudo apt install -y linux-tools-$(uname -r)  # bpftool

# libtrpc dependencies (D-Bus communication)
sudo apt install -y libsystemd-dev

# (Optional) For test programs
sudo apt install -y libyaml-dev
```

#### Build

```bash
git clone http://mod.lge.com/hub/timpani/time-trigger.git
cd time-trigger
git submodule update --init --recursive
mkdir build && cd build
cmake ..
make -j$(nproc)
```

---

### Ubuntu 20.04

Ubuntu 20.04 requires newer versions of clang and bpftool.

#### Upgrade Kernel (Required)

```bash
# Ubuntu 20.04 ships with kernel 5.4, upgrade to HWE kernel (5.15+)
sudo apt install -y linux-image-generic-hwe-20.04 linux-headers-generic-hwe-20.04
sudo reboot
```

#### Install Dependencies

```bash
# Basic build tools and libraries
sudo apt install -y build-essential cmake pkg-config git
sudo apt install -y libelf-dev zlib1g-dev libsystemd-dev

# (Optional) For test programs
sudo apt install -y libyaml-dev
```

#### Install Latest Clang

```bash
# Ubuntu 20.04's default clang (v10) is too old
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 17

# Create symlinks
sudo ln -sf /usr/bin/clang-17 /usr/local/bin/clang
sudo ln -sf /usr/bin/llc-17 /usr/local/bin/llc
```

#### Install Latest bpftool

```bash
# Ubuntu 20.04's default bpftool (v5.4) lacks BTF support
sudo apt install -y libcap-dev

git clone --recurse-submodules -b v7.4.0 https://github.com/libbpf/bpftool.git
cd bpftool/src
make
sudo cp bpftool /usr/local/bin/
```

#### Build

```bash
git clone http://mod.lge.com/hub/timpani/time-trigger.git
cd time-trigger
git submodule update --init --recursive
mkdir build && cd build
cmake ..
make -j$(nproc)
```

---

### RHEL / CentOS Stream 9/10

#### Install Dependencies

```bash
# Development tools
sudo dnf group install -y "Development Tools"
sudo dnf install -y cmake pkg-config git

# eBPF/libbpf dependencies
sudo dnf install -y elfutils-libelf-devel zlib-devel

# BPF compiler and tools
sudo dnf install -y clang llvm bpftool

# libtrpc dependencies
sudo dnf install -y systemd-devel

# (Optional) For test programs
sudo dnf config-manager --set-enabled crb  # CentOS only
sudo dnf install -y libyaml-devel
```

#### Build

```bash
git clone http://mod.lge.com/hub/timpani/time-trigger.git
cd time-trigger
git submodule update --init --recursive
mkdir build && cd build
cmake ..
make -j$(nproc)
```

---

### Fedora

#### Install Dependencies

```bash
# All dependencies
sudo dnf install -y @development-tools cmake pkg-config git
sudo dnf install -y elfutils-libelf-devel zlib-devel
sudo dnf install -y clang llvm bpftool
sudo dnf install -y systemd-devel libyaml-devel
```

#### Build

```bash
git clone http://mod.lge.com/hub/timpani/time-trigger.git
cd time-trigger
git submodule update --init --recursive
mkdir build && cd build
cmake ..
make -j$(nproc)
```

---

## Creating Packages

**Note:** Package creation requires native builds. Run on the target architecture.

```bash
# Use the automated script (recommended)
./scripts/build-packages.sh --clean

# Or manually with CPack
cd build

# Debian package (.deb)
cpack -G DEB

# RPM package (.rpm)
cpack -G RPM

# Tarball (.tar.gz)
cpack -G TGZ
```

See [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md) for detailed package building instructions.

---

## Post-Installation

### Verify Installation

```bash
# Check binary (installed as /usr/bin/timpani-n)
timpani-n -h

# Run requirements check
/usr/share/timpani-n/check-requirements.sh
```

### Set Capabilities (Non-root execution)

If installed via package, capabilities are set automatically. For manual builds:

```bash
# Binary name is 'timpani-n'
sudo setcap cap_bpf,cap_perfmon,cap_sys_nice,cap_sys_ptrace+ep /usr/local/bin/timpani-n
```

### Configure systemd Service (Recommended)

When installed via package, timpani-n includes a systemd service file. You can manage it as a system service:

```bash
# Enable automatic startup on boot
sudo systemctl enable timpani-n

# Start the service
sudo systemctl start timpani-n

# Check service status
systemctl status timpani-n

# View service logs
sudo journalctl -u timpani-n -f
```

**Service Management:**

```bash
# Stop the service
sudo systemctl stop timpani-n

# Restart the service
sudo systemctl restart timpani-n

# Disable auto-start
sudo systemctl disable timpani-n
```

**Customizing Service Options:**

To modify service startup options (e.g., add configuration file):

```bash
# Create service override
sudo systemctl edit timpani-n
```

Add your customizations:
```ini
[Service]
ExecStart=
ExecStart=/usr/bin/timpani-n -P 25 -c 0 -l 3
```

Then reload and restart:
```bash
sudo systemctl daemon-reload
sudo systemctl restart timpani-n
```

For detailed usage instructions, see [docs/USAGE.md](docs/USAGE.md).

---

## Uninstallation

### Package-Based Installation

**Ubuntu/Debian (DEB):**

```bash
# Stop and disable the service first
sudo systemctl stop timpani-n
sudo systemctl disable timpani-n

# Remove the package (keeps configuration files)
sudo dpkg -r timpani-n

# Or purge completely (removes configuration files too)
sudo dpkg -P timpani-n

# Verify removal
systemctl status timpani-n  # Should show "Unit could not be found"
which timpani-n             # Should return nothing
```

**RHEL/CentOS (RPM):**

```bash
# Stop and disable the service first
sudo systemctl stop timpani-n
sudo systemctl disable timpani-n

# Remove the package
sudo rpm -e timpani-n

# Verify removal
systemctl status timpani-n  # Should show "Unit could not be found"
which timpani-n             # Should return nothing
```

**Manual cleanup (if needed):**

```bash
# Reload systemd after package removal
sudo systemctl daemon-reload

# Reset any failed states
sudo systemctl reset-failed

# Remove service override files (if you created any)
sudo rm -rf /etc/systemd/system/timpani-n.service.d/

# Remove configuration directory (if it exists)
sudo rm -rf /etc/timpani-n/

# Remove log files (if any)
sudo rm -rf /var/log/timpani-n/
```

### Source-Based Installation

```bash
# Stop the service if running
sudo systemctl stop timpani-n
sudo systemctl disable timpani-n

# Remove binary
sudo rm /usr/local/bin/timpani-n

# Remove systemd service file (if you installed one)
sudo rm /etc/systemd/system/timpani-n.service
sudo systemctl daemon-reload

# Remove build directory
cd /path/to/time-trigger
rm -rf build/
```

---

## Troubleshooting

### "BTF not available"

Your kernel was not built with `CONFIG_DEBUG_INFO_BTF=y`.

```bash
# Check BTF support
ls -la /sys/kernel/btf/vmlinux

# If missing, upgrade kernel or rebuild with BTF enabled
```

### "Operation not permitted" when loading BPF

Missing capabilities or not running as root.

```bash
# Run as root
sudo ./timpani-n

# Or set capabilities
sudo setcap cap_bpf,cap_perfmon,cap_sys_nice,cap_sys_ptrace+ep ./timpani-n
```

### "libbpf: failed to find BTF for extern"

Kernel version is too old or BTF is incomplete.

```bash
# Check kernel version (5.15+ required)
uname -r

# Upgrade kernel if needed
```

### bpftool not found

```bash
# Ubuntu 22.04+
sudo apt install -y linux-tools-$(uname -r)

# CentOS/RHEL
sudo dnf install -y bpftool
```

---

## Next Steps

- See [docs/USAGE.md](docs/USAGE.md) for usage examples
- See [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) for system requirements
- See [docs/BUILDING.md](docs/BUILDING.md) for advanced build options
