<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
-->

# System Requirements

*Last updated: 2026.03*

This document details the system requirements for running Timpani-N.

> **Quick Check:** Run `./scripts/check-requirements.sh` to automatically verify your system.

## Table of Contents

- [Kernel Requirements](#kernel-requirements)
- [Supported Distributions](#supported-distributions)
- [Runtime Dependencies](#runtime-dependencies)
- [Permissions](#permissions)
- [Hardware Considerations](#hardware-considerations)

---

## Kernel Requirements

### Minimum Version

| Requirement | Minimum | RT/Production |
|-------------|---------|---------------|
| **Kernel Version** | 5.15+ | 6.12+ |
| **Use Case** | Development, non-RT | PREEMPT_RT, sched-ext |

> **⚠️ Important:** For **PREEMPT_RT kernels**, version **6.12+** is **strongly recommended** due to eBPF ring buffer spinlock deadlock issues in earlier RT kernels.

### Required Kernel Options

The following kernel options must be enabled (`=y`):

| Option | Description | Required |
|--------|-------------|----------|
| `CONFIG_BPF` | Enable BPF subsystem | ✅ Yes |
| `CONFIG_BPF_SYSCALL` | BPF system call | ✅ Yes |
| `CONFIG_BPF_JIT` | BPF JIT compiler | ⚠️ Recommended |
| `CONFIG_DEBUG_INFO_BTF` | BTF type information | ✅ Yes |
| `CONFIG_FTRACE` | Function tracer | ⚠️ Recommended |
| `CONFIG_TRACEPOINTS` | Tracepoints support | ✅ Yes |

#### Verify Kernel Configuration

```bash
# Method 1: /proc/config.gz
zcat /proc/config.gz | grep -E "CONFIG_BPF|CONFIG_DEBUG_INFO_BTF"

# Method 2: /boot/config-*
grep -E "CONFIG_BPF|CONFIG_DEBUG_INFO_BTF" /boot/config-$(uname -r)
```

### BTF Support

BTF (BPF Type Format) is **required** for eBPF CO-RE (Compile Once, Run Everywhere).

```bash
# Verify BTF is available
ls -la /sys/kernel/btf/vmlinux
```

If missing, your kernel needs to be rebuilt with `CONFIG_DEBUG_INFO_BTF=y` or upgraded to a distribution kernel that includes BTF.

### RT Kernel Considerations

For **PREEMPT_RT** kernels:

| Kernel Version | eBPF Compatibility |
|----------------|-------------------|
| < 6.12 | ❌ **Not recommended** - eBPF ring buffer spinlock can cause deadlocks |
| **6.12+** | ✅ RT-safe eBPF ring buffer |

> **⚠️ Critical:** eBPF ring buffer uses spinlocks internally. On PREEMPT_RT kernels < 6.12, this can cause priority inversion and deadlocks when RT tasks interact with BPF subsystem.

**Recommendation:** For RT/production environments, use kernel **6.12+** only.

---

## Supported Distributions

### Fully Supported ✅

| Distribution | Version | Default Kernel | Notes |
|--------------|---------|----------------|-------|
| Ubuntu | 24.04 LTS | 6.8 | Best support |
| Ubuntu | 22.04 LTS | 5.15 | Fully functional |
| Fedora | 40+ | 6.8+ | Latest features |
| CentOS Stream | 10 | 6.x | Recommended |
| RHEL | 9.4+ | 5.14 | With BTF backports |

### Supported with Caveats ⚠️

| Distribution | Version | Issue | Workaround |
|--------------|---------|-------|------------|
| Ubuntu | 20.04 LTS | Kernel 5.4 too old | Install HWE kernel |
| Ubuntu | 20.04 LTS | clang 10 too old | Install clang 17+ |
| Ubuntu | 20.04 LTS | bpftool 5.4 too old | Build from source |
| CentOS Stream | 9 | May need EPEL | Enable EPEL repo |
| RHEL | 8 | Kernel 4.18, no BTF | ❌ Not supported |

### Not Supported ❌

| Distribution | Version | Reason |
|--------------|---------|--------|
| Ubuntu | 18.04 | Kernel 4.15, no BTF |
| CentOS | 7 | Kernel 3.10, no BPF |
| RHEL | 7 | Kernel 3.10, no BPF |
| RHEL | 8 | Kernel 4.18, no BTF |
| Debian | 10 | Kernel 4.19, no BTF |

---

## Runtime Dependencies

### Required Libraries

| Library | Ubuntu/Debian | RHEL/CentOS | Purpose |
|---------|---------------|-------------|---------|
| libelf | `libelf1` | `elfutils-libelf` | eBPF loading |
| zlib | `zlib1g` | `zlib` | Compression |
| libsystemd | `libsystemd0` | `systemd-libs` | D-Bus (libtrpc) |

### Verify Libraries

```bash
# Check if libraries are installed
ldconfig -p | grep -E "libelf|libz|libsystemd"
```

### Build Dependencies

See [INSTALL.md](../INSTALL.md) for build-time dependencies.

---

## Permissions

Timpani-N requires elevated privileges for:

1. **Loading eBPF programs** into the kernel
2. **Attaching to tracepoints** for monitoring
3. **Setting real-time scheduling** for processes
4. **Reading process information** via `/proc`

### Option 1: Run as Root

```bash
sudo ./timpani-n
```

### Option 2: Linux Capabilities

```bash
# Set capabilities on the binary
sudo setcap cap_bpf,cap_perfmon,cap_sys_nice,cap_sys_ptrace+ep ./timpani-n

# Verify
getcap ./timpani-n
```

| Capability | Purpose |
|------------|---------|
| `CAP_BPF` | Load and attach eBPF programs |
| `CAP_PERFMON` | Access perf events and tracepoints |
| `CAP_SYS_NICE` | Set real-time scheduling (SCHED_FIFO/RR/DEADLINE) |
| `CAP_SYS_PTRACE` | Access other process info via /proc |

> **Note:** Packages (DEB/RPM) automatically set capabilities during installation.

---

## Hardware Considerations

### CPU

- **Minimum:** 2 cores (1 for timpani-n, 1+ for monitored tasks)
- **Recommended:** 4+ cores for meaningful CPU isolation

### Memory

- **Minimum:** 256 MB available
- **eBPF maps:** ~64KB per monitored process

### Architecture

| Architecture | Status |
|--------------|--------|
| x86_64 (amd64) | ✅ Fully supported |
| ARM64 (aarch64) | ✅ Fully supported |
| ARM32 | ❌ Not supported |
| RISC-V | ⚠️ Experimental |

---

## Verification Script

Run the automated requirements checker:

```bash
# Basic check
./scripts/check-requirements.sh

# Include build tool checks
./scripts/check-requirements.sh --build
```

Example output:
```
===========================================
  Timpani-N System Requirements Checker
===========================================

── Kernel Requirements ──
[  OK  ] Kernel version: 6.8.0-40-generic (recommended)
[  OK  ] BTF support available (/sys/kernel/btf/vmlinux)

── Kernel Configuration ──
[  OK  ] CONFIG_BPF=y
[  OK  ] CONFIG_BPF_SYSCALL=y
[  OK  ] CONFIG_DEBUG_INFO_BTF=y

── Runtime Libraries ──
[  OK  ] libelf found
[  OK  ] zlib found
[  OK  ] libsystemd found

── Permissions ──
[ WARN ] Not running as root
         └─ timpani-n requires root or CAP_BPF,CAP_PERFMON,CAP_SYS_NICE,CAP_SYS_PTRACE

===========================================
  Summary
===========================================
⚠ 1 warning(s) found. System may work but check warnings.
```

---

## Next Steps

- [INSTALL.md](../INSTALL.md) - Installation instructions
- [BUILDING.md](BUILDING.md) - Building from source
- [USAGE.md](USAGE.md) - Usage examples
