#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT
#
# Build DEB and RPM packages for timpani-n
#
# Usage:
#   ./scripts/build-packages.sh [OPTIONS]
#
# Options:
#   -d, --deb-only        Build DEB package only
#   -r, --rpm-only        Build RPM package only
#   -o, --output DIR      Output directory [default: ./packages]
#   -c, --clean           Clean build directory before building
#   -h, --help            Show this help message
#
# Note: This script performs native builds only. To build ARM packages,
#       run this script directly on an ARM system (e.g., Raspberry Pi).

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
OUTPUT_DIR="$PROJECT_ROOT/packages"
TARGET_ARCH=$(uname -m)
BUILD_DEB=true
BUILD_RPM=true
CLEAN_BUILD=false

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_help() {
    head -n 15 "$0" | grep "^#" | sed 's/^# //g' | sed 's/^#//g'
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--deb-only)
            BUILD_DEB=true
            BUILD_RPM=false
            shift
            ;;
        -r|--rpm-only)
            BUILD_DEB=false
            BUILD_RPM=true
            shift
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -c|--clean)
            CLEAN_BUILD=true
            shift
            ;;
        -h|--help)
            show_help
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            ;;
    esac
done

# Read version from VERSION file
if [ ! -f "$PROJECT_ROOT/VERSION" ]; then
    print_error "VERSION file not found!"
    exit 1
fi

VERSION=$(cat "$PROJECT_ROOT/VERSION" | tr -d '\n')
print_info "Building timpani-n version: $VERSION"
print_info "Target architecture: $TARGET_ARCH"

# Check dependencies
check_dependencies() {
    local missing_deps=()
    local warnings=()

    if $BUILD_DEB; then
        if ! command -v dpkg-deb &> /dev/null; then
            missing_deps+=("dpkg-deb")
        fi
    fi

    if $BUILD_RPM; then
        if ! command -v rpmbuild &> /dev/null; then
            warnings+=("rpmbuild not found - RPM package will be skipped")
            BUILD_RPM=false
        fi
    fi

    if ! command -v cmake &> /dev/null; then
        missing_deps+=("cmake")
    fi

    if [ ${#warnings[@]} -gt 0 ]; then
        for warning in "${warnings[@]}"; do
            print_warn "$warning"
        done
    fi

    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_error "Missing required dependencies: ${missing_deps[*]}"
        exit 1
    fi
}

check_dependencies

# Clean build directory
if $CLEAN_BUILD && [ -d "$BUILD_DIR" ]; then
    print_info "Cleaning build directory..."
    rm -rf "$BUILD_DIR"
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build packages
cd "$PROJECT_ROOT"

if [ ! -d "$BUILD_DIR" ]; then
    print_info "Creating build directory..."
    mkdir -p "$BUILD_DIR"
fi

cd "$BUILD_DIR"

# Configure with CMake
print_info "Configuring CMake..."
cmake .. -DCMAKE_BUILD_TYPE=Release \
         -DCONFIG_TRACE_BPF=ON \
         -DCONFIG_TRACE_BPF_EVENT=OFF

# Build
print_info "Building timpani-n..."
# Add bpftool to PATH if needed
if [ -d "/usr/lib/linux-hwe-6.17-tools-6.17.0-14" ]; then
    export PATH="/usr/lib/linux-hwe-6.17-tools-6.17.0-14:$PATH"
fi
make -j$(nproc)

# Build DEB package
if $BUILD_DEB; then
    print_info "Building DEB package..."
    cpack -G DEB

    DEB_FILE=$(ls timpani-n_*.deb 2>/dev/null | head -1)
    if [ -n "$DEB_FILE" ]; then
        mv "$DEB_FILE" "$OUTPUT_DIR/"
        print_info "DEB package created: $OUTPUT_DIR/$DEB_FILE"
    else
        print_error "Failed to create DEB package"
        exit 1
    fi
fi

# Build RPM package
if $BUILD_RPM; then
    print_info "Building RPM package..."
    cpack -G RPM

    RPM_FILE=$(ls timpani-n-*.${TARGET_ARCH}.rpm 2>/dev/null | head -1)
    if [ -z "$RPM_FILE" ]; then
        # Fallback to x86_64 if TARGET_ARCH doesn't match
        RPM_FILE=$(ls timpani-n-*.rpm 2>/dev/null | head -1)
    fi

    if [ -n "$RPM_FILE" ]; then
        mv "$RPM_FILE" "$OUTPUT_DIR/"
        print_info "RPM package created: $OUTPUT_DIR/$RPM_FILE"
    else
        print_error "Failed to create RPM package"
        exit 1
    fi
fi

# Create tarball
print_info "Creating source tarball..."
cpack -G TGZ
TARBALL=$(ls timpani-n-*.tar.gz 2>/dev/null | head -1)
if [ -n "$TARBALL" ]; then
    mv "$TARBALL" "$OUTPUT_DIR/"
    print_info "Tarball created: $OUTPUT_DIR/$TARBALL"
fi

# Summary
print_info "======================================"
print_info "Package build completed successfully!"
print_info "======================================"
print_info "Output directory: $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR"

print_info ""
print_info "To install:"
if $BUILD_DEB; then
    print_info "  Ubuntu/Debian: sudo dpkg -i $OUTPUT_DIR/timpani-n_*.deb"
fi
if $BUILD_RPM; then
    print_info "  RHEL/CentOS:   sudo rpm -i $OUTPUT_DIR/timpani-n-*.rpm"
fi
