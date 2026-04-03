#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT
#
# Timpani-N System Requirements Checker
# Run this script before building or installing timpani-n
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
ERRORS=0
WARNINGS=0

print_header() {
    echo ""
    echo "==========================================="
    echo "  Timpani-N System Requirements Checker"
    echo "==========================================="
    echo ""
}

print_result() {
    local status=$1
    local message=$2
    local detail=$3

    case $status in
        "OK")
            echo -e "[${GREEN}  OK  ${NC}] $message"
            ;;
        "WARN")
            echo -e "[${YELLOW} WARN ${NC}] $message"
            [ -n "$detail" ] && echo -e "         └─ $detail"
            ((WARNINGS++))
            ;;
        "FAIL")
            echo -e "[${RED} FAIL ${NC}] $message"
            [ -n "$detail" ] && echo -e "         └─ $detail"
            ((ERRORS++))
            ;;
    esac
}

# Check kernel version
check_kernel_version() {
    echo "── Kernel Requirements ──"
    
    local kernel_version=$(uname -r)
    local major=$(echo "$kernel_version" | cut -d. -f1)
    local minor=$(echo "$kernel_version" | cut -d. -f2)
    local version_num=$((major * 100 + minor))

    # Minimum: 5.15, RT/Production: 6.12+
    if [ $version_num -ge 612 ]; then
        print_result "OK" "Kernel version: $kernel_version (RT-safe, sched-ext ready)"
    elif [ $version_num -ge 515 ]; then
        print_result "OK" "Kernel version: $kernel_version (supported for non-RT)"
    else
        print_result "FAIL" "Kernel version: $kernel_version (unsupported)" \
            "Minimum required: 5.15+"
    fi

    # Check if RT kernel
    if uname -r | grep -qi "rt\|preempt"; then
        if [ $version_num -ge 612 ]; then
            print_result "OK" "RT kernel detected (6.12+ RT-safe eBPF)"
        else
            print_result "FAIL" "RT kernel detected but version < 6.12" \
                "eBPF ring buffer spinlock can cause deadlocks on RT kernels < 6.12"
        fi
    fi
}

# Check BTF support
check_btf_support() {
    echo ""
    echo "── eBPF/BTF Support ──"

    if [ -f /sys/kernel/btf/vmlinux ]; then
        print_result "OK" "BTF support available (/sys/kernel/btf/vmlinux)"
    else
        print_result "FAIL" "BTF not available" \
            "Kernel must be built with CONFIG_DEBUG_INFO_BTF=y"
    fi

    # Check BPF filesystem
    if mount | grep -q "bpf on /sys/fs/bpf"; then
        print_result "OK" "BPF filesystem mounted"
    else
        print_result "WARN" "BPF filesystem not mounted" \
            "Run: sudo mount -t bpf bpf /sys/fs/bpf"
    fi
}

# Check kernel config options
check_kernel_config() {
    echo ""
    echo "── Kernel Configuration ──"

    local config_file=""
    if [ -f /proc/config.gz ]; then
        config_file="/proc/config.gz"
    elif [ -f "/boot/config-$(uname -r)" ]; then
        config_file="/boot/config-$(uname -r)"
    fi

    if [ -z "$config_file" ]; then
        print_result "WARN" "Cannot find kernel config" \
            "Unable to verify kernel options"
        return
    fi

    local get_config
    if [ "$config_file" = "/proc/config.gz" ]; then
        get_config="zcat $config_file"
    else
        get_config="cat $config_file"
    fi

    # Required options
    for opt in CONFIG_BPF CONFIG_BPF_SYSCALL CONFIG_BPF_JIT; do
        if $get_config 2>/dev/null | grep -q "^${opt}=y"; then
            print_result "OK" "$opt=y"
        else
            print_result "FAIL" "$opt not enabled" \
                "Required for eBPF support"
        fi
    done

    # BTF (already checked above, but verify config)
    if $get_config 2>/dev/null | grep -q "^CONFIG_DEBUG_INFO_BTF=y"; then
        print_result "OK" "CONFIG_DEBUG_INFO_BTF=y"
    else
        print_result "FAIL" "CONFIG_DEBUG_INFO_BTF not enabled" \
            "Required for eBPF CO-RE"
    fi
}

# Check runtime libraries
check_libraries() {
    echo ""
    echo "── Runtime Libraries ──"

    # libelf
    if ldconfig -p 2>/dev/null | grep -q "libelf.so"; then
        print_result "OK" "libelf found"
    else
        print_result "FAIL" "libelf not found" \
            "Install: libelf1 (Debian/Ubuntu) or elfutils-libelf (RHEL/CentOS)"
    fi

    # zlib
    if ldconfig -p 2>/dev/null | grep -q "libz.so"; then
        print_result "OK" "zlib found"
    else
        print_result "FAIL" "zlib not found" \
            "Install: zlib1g (Debian/Ubuntu) or zlib (RHEL/CentOS)"
    fi

    # libsystemd (for libtrpc)
    if ldconfig -p 2>/dev/null | grep -q "libsystemd.so"; then
        print_result "OK" "libsystemd found"
    else
        print_result "FAIL" "libsystemd not found" \
            "Install: libsystemd0 (Debian/Ubuntu) or systemd-libs (RHEL/CentOS)"
    fi
}

# Check build tools (optional)
check_build_tools() {
    echo ""
    echo "── Build Tools (for source builds) ──"

    for tool in gcc cmake clang bpftool; do
        if command -v $tool &>/dev/null; then
            local version=$($tool --version 2>/dev/null | head -1)
            print_result "OK" "$tool: $version"
        else
            print_result "WARN" "$tool not found" \
                "Required for building from source"
        fi
    done
}

# Check permissions
check_permissions() {
    echo ""
    echo "── Permissions ──"

    if [ "$EUID" -eq 0 ]; then
        print_result "OK" "Running as root"
    else
        print_result "WARN" "Not running as root" \
            "timpani-n requires root or CAP_BPF,CAP_PERFMON,CAP_SYS_NICE,CAP_SYS_PTRACE"
    fi

    # Check if capabilities can be used
    if command -v setcap &>/dev/null; then
        print_result "OK" "setcap available for capability management"
    else
        print_result "WARN" "setcap not found" \
            "Install libcap2-bin (Debian/Ubuntu) or libcap (RHEL/CentOS)"
    fi
}

# Print summary
print_summary() {
    echo ""
    echo "==========================================="
    echo "  Summary"
    echo "==========================================="
    
    if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
        echo -e "${GREEN}✓ All checks passed! System is ready for timpani-n.${NC}"
    elif [ $ERRORS -eq 0 ]; then
        echo -e "${YELLOW}⚠ $WARNINGS warning(s) found. System may work but check warnings.${NC}"
    else
        echo -e "${RED}✗ $ERRORS error(s), $WARNINGS warning(s) found.${NC}"
        echo -e "${RED}  Please resolve errors before using timpani-n.${NC}"
    fi
    echo ""

    return $ERRORS
}

# Main
print_header
check_kernel_version
check_btf_support
check_kernel_config
check_libraries
check_permissions

if [ "${1:-}" = "--build" ]; then
    check_build_tools
fi

print_summary
exit $?
