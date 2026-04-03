<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
-->

# Package Deployment Guide

This document describes how to build, manage, and deploy timpani-n packages.

## Table of Contents

- [Overview](#overview)
- [Building Packages](#building-packages)
- [Release Process](#release-process)
- [Package Repository Setup](#package-repository-setup)
- [CI/CD Integration](#cicd-integration)
- [Distribution](#distribution)

---

## Overview

Timpani-n supports package-based deployment for:
- **DEB packages**: Ubuntu, Debian
- **RPM packages**: RHEL, CentOS, Fedora
- **Tarballs**: Generic Linux

### Package Naming Convention

```
timpani-n_<VERSION>_<ARCH>.deb       # Debian/Ubuntu
timpani-n-<VERSION>.<ARCH>.rpm       # RHEL/CentOS
timpani-n-<VERSION>-Linux.tar.gz     # Tarball
```

Example:
```
timpani-n_2026.03.0_amd64.deb
timpani-n-2026.03.0.x86_64.rpm
timpani-n-2026.03.0-Linux.tar.gz
```

---

## Building Packages

### Prerequisites

**Ubuntu/Debian:**
```bash
sudo apt-get install -y cmake build-essential \
    dpkg-dev rpm \
    libelf-dev zlib1g-dev \
    clang llvm bpftool \
    libsystemd-dev
```

**RHEL/CentOS:**
```bash
sudo dnf install -y cmake gcc gcc-c++ \
    rpm-build dpkg \
    elfutils-libelf-devel zlib-devel \
    clang llvm bpftool \
    systemd-devel
```

### Build All Packages

```bash
./scripts/build-packages.sh
```

This will create (architecture depends on build host):
- `packages/timpani-n_<VERSION>_amd64.deb` (on x86_64)
- `packages/timpani-n_<VERSION>_arm64.deb` (on aarch64)
- `packages/timpani-n-<VERSION>.x86_64.rpm` (on x86_64)
- `packages/timpani-n-<VERSION>-Linux.tar.gz`

**Note:** Cross-compilation is not supported. Build packages on the target architecture.

### Build Options

```bash
# Build DEB package only
./scripts/build-packages.sh --deb-only

# Build RPM package only
./scripts/build-packages.sh --rpm-only

# Clean build
./scripts/build-packages.sh --clean

# Specify output directory
./scripts/build-packages.sh --output /path/to/packages
```

### Building on ARM (Raspberry Pi)

**Raspberry Pi 4/5 (Ubuntu/Debian ARM64):**

Install dependencies:
```bash
sudo apt-get update
sudo apt-get install -y cmake build-essential \
    dpkg-dev \
    libelf-dev zlib1g-dev \
    clang llvm \
    libsystemd-dev

# Install bpftool for your kernel version
sudo apt-get install -y linux-tools-$(uname -r) || \
sudo apt-get install -y linux-tools-generic
```

Build DEB package:
```bash
./scripts/build-packages.sh --deb-only --clean
```

This will create:
- `packages/timpani-n_<VERSION>_arm64.deb`

**Note:**
- ARM64/aarch64 architecture is automatically detected and mapped to `arm64` for Debian packages
- Ensure your kernel supports eBPF (5.15+ recommended, check with `uname -r`)
- BTF support is required (check with `ls /sys/kernel/btf/vmlinux`)
- For optimal performance, use a real-time kernel (PREEMPT_RT)

**Troubleshooting on Raspberry Pi:**

If bpftool is not found:
```bash
# Find available bpftool
find /usr -name bpftool 2>/dev/null

# Add to PATH temporarily
export PATH="/usr/lib/linux-tools/$(uname -r):$PATH"
```

If BTF is not available:
```bash
# Check kernel config
zgrep CONFIG_DEBUG_INFO_BTF /proc/config.gz

# You may need to upgrade kernel or use Ubuntu 22.04+ which has BTF enabled
```

---

## Release Process

### Automated Release

Use the release script to automate version bump, build, and tagging:

```bash
# Create new release
./scripts/release.sh 2026.03.1
```

This will:
1. Validate version format (YYYY.MM.PATCH)
2. Update `VERSION` file
3. Build DEB and RPM packages
4. Commit VERSION change
5. Create git tag `v2026.03.1`

### Manual Release Steps

If you prefer manual control:

#### 1. Update VERSION file

```bash
echo "2026.03.1" > VERSION
```

#### 2. Update CHANGELOG.md

Add release notes:

```markdown
## [2026.03.1] - 2026-03-15

### Fixed
- Bug fix description

### Added
- New feature description
```

#### 3. Build packages

```bash
./scripts/build-packages.sh --clean
```

#### 4. Commit and tag

```bash
git add VERSION CHANGELOG.md
git commit -m "chore: release v2026.03.1"
git tag -a v2026.03.1 -m "Release v2026.03.1"
```

#### 5. Push to remote

```bash
git push origin main
git push origin v2026.03.1
```

---

## Package Repository Setup

### Option 1: Simple HTTP Server

For small teams, serve packages via HTTP:

```bash
# Copy packages to web server
scp packages/timpani-n_*.deb user@server:/var/www/releases/timpani-n/

# Users download via wget
wget http://server/releases/timpani-n/timpani-n_2026.03.0_amd64.deb
```

### Option 2: APT Repository (Debian/Ubuntu)

#### Setup APT Repository

```bash
# Install reprepro
sudo apt-get install reprepro

# Create repository structure
mkdir -p /var/www/apt/conf

# Configure reprepro
cat > /var/www/apt/conf/distributions <<EOF
Origin: LG Electronics
Label: timpani-n
Codename: focal
Architectures: amd64 arm64
Components: main
Description: Timpani-N Repository
EOF

# Add package
reprepro -b /var/www/apt includedeb focal timpani-n_2026.03.0_amd64.deb
```

#### Client Configuration

```bash
# Add repository
echo "deb [trusted=yes] http://server/apt focal main" | \
    sudo tee /etc/apt/sources.list.d/timpani-n.list

# Install
sudo apt-get update
sudo apt-get install timpani-n
```

### Option 3: YUM/DNF Repository (RHEL/CentOS)

#### Setup YUM Repository

```bash
# Create repository structure
mkdir -p /var/www/yum/timpani-n/el9/x86_64

# Copy RPM
cp timpani-n-2026.03.0.x86_64.rpm /var/www/yum/timpani-n/el9/x86_64/

# Create repository metadata
createrepo /var/www/yum/timpani-n/el9/x86_64/
```

#### Client Configuration

```bash
# Add repository
cat > /etc/yum.repos.d/timpani-n.repo <<EOF
[timpani-n]
name=Timpani-N Repository
baseurl=http://server/yum/timpani-n/el9/x86_64
enabled=1
gpgcheck=0
EOF

# Install
sudo dnf install timpani-n
```

---

## CI/CD Integration

### GitLab CI Example

Create `.gitlab-ci.yml`:

```yaml
stages:
  - build
  - package
  - release

variables:
  VERSION: $(cat VERSION)

build:
  stage: build
  image: ubuntu:24.04
  before_script:
    - apt-get update
    - apt-get install -y cmake build-essential libelf-dev zlib1g-dev clang llvm bpftool libsystemd-dev
  script:
    - mkdir build && cd build
    - cmake .. -DCMAKE_BUILD_TYPE=Release
    - make -j$(nproc)
  artifacts:
    paths:
      - build/timpani-n
    expire_in: 1 hour

package:deb:
  stage: package
  image: ubuntu:24.04
  dependencies:
    - build
  script:
    - ./scripts/build-packages.sh --deb-only
  artifacts:
    paths:
      - packages/*.deb
    expire_in: 1 week

package:rpm:
  stage: package
  image: rockylinux:9
  dependencies:
    - build
  script:
    - ./scripts/build-packages.sh --rpm-only
  artifacts:
    paths:
      - packages/*.rpm
    expire_in: 1 week

release:
  stage: release
  only:
    - tags
  script:
    - echo "Creating release for $CI_COMMIT_TAG"
    # Upload to release server
    - scp packages/* user@server:/var/www/releases/timpani-n/
  artifacts:
    paths:
      - packages/*
```

---

## Distribution

### Internal Distribution

**For LGE internal use:**

1. Upload packages to MOD (mod.lge.com):
   ```bash
   # Upload to MOD releases
   scp packages/timpani-n_*.deb mod.lge.com:/releases/timpani/timpani-n/
   ```

2. Update internal package repository

3. Notify users via email/Slack

### External Distribution (if applicable)

**For GitHub releases:**

1. Create GitHub release:
   ```bash
   gh release create v2026.03.0 \
       packages/timpani-n_2026.03.0_amd64.deb \
       packages/timpani-n-2026.03.0.x86_64.rpm \
       --title "Release v2026.03.0" \
       --notes-file CHANGELOG.md
   ```

2. Update documentation with download links

---

## Verification

### Test Package Installation

**DEB:**
```bash
# Install
sudo dpkg -i timpani-n_2026.03.0_amd64.deb

# Verify binary and libraries
timpani-n -h
ldd /usr/bin/timpani-n  # Should show libtrpc.so.2 is found

# Verify systemd service
systemctl status timpani-n
sudo systemctl start timpani-n
sudo journalctl -u timpani-n -n 20

# Verify service management
sudo systemctl stop timpani-n
sudo systemctl restart timpani-n

# Uninstall (proper cleanup)
sudo systemctl stop timpani-n      # Stop service first
sudo systemctl disable timpani-n   # Disable auto-start
sudo dpkg -r timpani-n             # Remove package
sudo systemctl daemon-reload       # Reload systemd
systemctl status timpani-n         # Verify removal (should fail)

# Complete purge (removes config files too)
sudo dpkg -P timpani-n
```

**RPM:**
```bash
# Install
sudo rpm -i timpani-n-2026.03.0.x86_64.rpm

# Verify binary and libraries
timpani-n -h
ldd /usr/bin/timpani-n  # Should show libtrpc.so.2 is found

# Verify systemd service
systemctl status timpani-n
sudo systemctl start timpani-n
sudo journalctl -u timpani-n -n 20

# Verify service management
sudo systemctl stop timpani-n
sudo systemctl restart timpani-n

# Uninstall (proper cleanup)
sudo systemctl stop timpani-n      # Stop service first
sudo systemctl disable timpani-n   # Disable auto-start
sudo rpm -e timpani-n              # Remove package
sudo systemctl daemon-reload       # Reload systemd
systemctl status timpani-n         # Verify removal (should fail)
```

### Package Contents Verification

```bash
# DEB
dpkg-deb -c timpani-n_2026.03.0_amd64.deb

# RPM
rpm -qlp timpani-n-2026.03.0.x86_64.rpm
```

Expected contents:
```
/usr/bin/timpani-n
/usr/share/timpani-n/check-requirements.sh
/usr/share/doc/timpani-n/README.md
/usr/share/doc/timpani-n/INSTALL.md
/usr/share/doc/timpani-n/docs/...
```

---

## Troubleshooting

### Build Failures

**Missing bpftool:**
```bash
# Find available bpftool
find /usr -name bpftool 2>/dev/null

# Add to PATH
export PATH="/usr/lib/linux-tools-$(uname -r):$PATH"
```

**BTF errors:**
```bash
# Check BTF support
ls -la /sys/kernel/btf/vmlinux
```

### Package Installation Issues

**Dependency errors:**
```bash
# DEB
sudo apt-get install -f

# RPM
sudo dnf install --allowerasing
```

**Capability errors after install:**
```bash
# Verify capabilities
getcap /usr/bin/timpani-n

# Set manually if needed
sudo setcap cap_bpf,cap_perfmon,cap_sys_nice,cap_sys_ptrace+ep /usr/bin/timpani-n
```

---

## Best Practices

1. **Always test packages** in a clean environment before distribution
2. **Update CHANGELOG.md** for every release
3. **Use semantic versioning** (CalVer: YYYY.MM.PATCH)
4. **Sign packages** for production environments (GPG)
5. **Maintain multiple versions** in repository for rollback support
6. **Document breaking changes** clearly in CHANGELOG
7. **Automate CI/CD pipeline** for consistent builds
8. **Version lock dependencies** in CMakeLists.txt

---

## References

- [BUILDING.md](BUILDING.md) - Advanced build options and configurations
- [INSTALL.md](INSTALL.md) - Installation guide
- [CHANGELOG.md](../CHANGELOG.md) - Version history
- [CPack Documentation](https://cmake.org/cmake/help/latest/module/CPack.html)
