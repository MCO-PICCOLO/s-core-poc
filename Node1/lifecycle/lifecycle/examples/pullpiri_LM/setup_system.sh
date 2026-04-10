#!/bin/bash
# ============================================================================
# New System Setup Script — Pullpiri LM Demo
# ============================================================================
# This script automates the setup steps from SETUP.md (sections 1-3)
# Run with: sudo ./setup_system.sh

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running with sudo
check_sudo() {
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run with sudo privileges"
        exit 1
    fi
}

# Get the actual user (not root when using sudo)
ACTUAL_USER=${SUDO_USER:-$USER}
ACTUAL_HOME=$(eval echo ~$ACTUAL_USER)

log_info "Running as: $ACTUAL_USER (home: $ACTUAL_HOME)"

# ============================================================================
# Section 1: System Prerequisites
# ============================================================================

install_prerequisites() {
    log_info "============================================"
    log_info "Section 1: Installing System Prerequisites"
    log_info "============================================"

    # Update package lists
    log_info "Updating package lists..."
    apt update

    # Check and install Java 17
    log_info "Checking for Java 17..."
    if command -v java &> /dev/null; then
        JAVA_VERSION=$(java -version 2>&1 | head -n 1 | awk -F '"' '{print $2}' | awk -F '.' '{print $1}')
        if [ "$JAVA_VERSION" = "17" ]; then
            log_success "Java 17 already installed"
            java -version 2>&1 | head -n 1
        else
            log_warning "Java version $JAVA_VERSION found, but version 17 is required. Installing OpenJDK 17..."
            apt install -y openjdk-17-jdk
            log_success "Java 17 installed"
        fi
    else
        log_info "Java not found. Installing OpenJDK 17..."
        apt install -y openjdk-17-jdk
        log_success "Java 17 installed"
    fi

    # Check and install Timpani dependencies
    log_info "Checking Timpani dependencies..."
    MISSING_DEPS=()

    if ! dpkg -l | grep -q "libgrpc++-dev"; then
        MISSING_DEPS+=("libgrpc++-dev")
    fi
    if ! dpkg -l | grep -q "libprotobuf-dev"; then
        MISSING_DEPS+=("libprotobuf-dev")
    fi
    if ! dpkg -l | grep -q "protobuf-compiler-grpc"; then
        MISSING_DEPS+=("protobuf-compiler-grpc")
    fi
    if ! dpkg -l | grep -q "libsystemd-dev"; then
        MISSING_DEPS+=("libsystemd-dev")
    fi

    if ! dpkg -l | grep -q "^ii  cmake "; then
        MISSING_DEPS+=("cmake")
    fi
    if ! dpkg -l | grep -q "^ii  build-essential "; then
        MISSING_DEPS+=("build-essential")
    fi

    if [ ${#MISSING_DEPS[@]} -eq 0 ]; then
        log_success "All Timpani dependencies already installed"
    else
        log_info "Installing missing Timpani dependencies: ${MISSING_DEPS[*]}"
        apt install -y "${MISSING_DEPS[@]}"
        log_success "Timpani dependencies installed"
    fi

    # Check and install Bazelisk
    log_info "Checking for Bazel/Bazelisk..."
    if command -v bazel &> /dev/null; then
        BAZEL_VERSION=$(bazel --version 2>&1 | grep -oP 'bazel \K[0-9.]+' || echo "unknown")
        log_success "Bazel already installed (version: $BAZEL_VERSION)"
        if [[ "$BAZEL_VERSION" != "8.4.2" ]] && [[ "$BAZEL_VERSION" != "unknown" ]]; then
            log_warning "Expected version 8.4.2, but found $BAZEL_VERSION"
            log_info "Bazelisk will auto-download the correct version based on .bazelversion file"
        fi
    else
        log_info "Bazel not found. Installing Bazelisk..."
        wget -O /usr/local/bin/bazel \
            https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64
        chmod +x /usr/local/bin/bazel
        log_success "Bazelisk installed"
    fi

    # Check and install Rust toolchain (as actual user, not root)
    log_info "Checking for Rust toolchain 1.90.0..."
    if command -v rustup &> /dev/null; then
        if su - $ACTUAL_USER -c "rustup toolchain list" | grep -q "1.90.0"; then
            log_success "Rust toolchain 1.90.0 already installed"
        else
            log_info "Rust toolchain 1.90.0 not found. Installing..."
            su - $ACTUAL_USER -c "rustup toolchain install 1.90.0"
            log_success "Rust toolchain 1.90.0 installed"
        fi
    else
        log_warning "Rustup not found. Installing rustup and toolchain 1.90.0..."
        su - $ACTUAL_USER -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
        su - $ACTUAL_USER -c "source $ACTUAL_HOME/.cargo/env && rustup toolchain install 1.90.0"
        log_success "Rustup and Rust toolchain 1.90.0 installed"
    fi
}

# ============================================================================
# Section 2: One-Time Directory Setup
# ============================================================================

setup_directories() {
    log_info "=========================================="
    log_info "Section 2: Creating Directory Structure"
    log_info "=========================================="

    log_info "Creating /opt/pullpiri directory structure..."
    mkdir -p /opt/pullpiri/bin
    mkdir -p /opt/pullpiri/bin/etc
    mkdir -p /opt/pullpiri/lib
    mkdir -p /opt/pullpiri/etc

    log_success "Directory structure created:"
    log_success "  /opt/pullpiri/bin       - service binaries"
    log_success "  /opt/pullpiri/bin/etc   - hm_config.json"
    log_success "  /opt/pullpiri/lib       - shared .so files"
    log_success "  /opt/pullpiri/etc       - hmproc_adas_primary.bin"
}

# ============================================================================
# Section 3: Install Pullpiri Service Binaries
# ============================================================================

build_pullpiri_binaries() {
    log_info "================================================"
    log_info "Section 3: Building Pullpiri Service Binaries"
    log_info "================================================"

    # Detect the pullpiri directory and resolve to absolute path
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    PULLPIRI_DIR="$( cd "$SCRIPT_DIR/../../../../pullpiri" && pwd )"

    if [ ! -d "$PULLPIRI_DIR" ]; then
        log_error "Pullpiri directory not found at: $PULLPIRI_DIR"
        log_error "Expected location: s-core-poc/Node1/pullpiri"
        exit 1
    fi

    log_info "Found pullpiri at: $PULLPIRI_DIR"

    # List of package names to build
    declare -a packages=(
        "persistency-service"
        "apiserver"
        "monitoringserver"
        "policymanager"
        "actioncontroller"
        "filtergateway"
        "statemanager"
    )

    # Build all packages from the workspace root
    log_info "Building all Pullpiri components from workspace root..."
    for package in "${packages[@]}"; do
        log_info "Building package: $package..."
        su - $ACTUAL_USER -c "cd '$PULLPIRI_DIR/src' && source $ACTUAL_HOME/.cargo/env && cargo build --release -p $package"
        if [ $? -eq 0 ]; then
            log_success "Built $package"
        else
            log_error "Failed to build $package"
            exit 1
        fi
    done

    # Define target directory (Cargo workspace is in pullpiri/src/)
    TARGET_DIR="$PULLPIRI_DIR/src/target/release"

    # Check if target directory exists
    if [ ! -d "$TARGET_DIR" ]; then
        log_error "Target directory not found: $TARGET_DIR"
        log_error "Cargo build may have failed or used a different target directory"
        exit 1
    fi

    # Copy binaries to /opt/pullpiri/bin/
    log_info "Copying binaries from $TARGET_DIR to /opt/pullpiri/bin/..."

    declare -a binaries=(
        "persistency-service"
        "apiserver"
        "monitoringserver"
        "statemanager"
        "filtergateway"
        "actioncontroller"
        "policymanager"
    )

    COPIED_COUNT=0
    for binary in "${binaries[@]}"; do
        if [ -f "$TARGET_DIR/$binary" ]; then
            cp -f "$TARGET_DIR/$binary" /opt/pullpiri/bin/
            log_success "Copied $binary"
            COPIED_COUNT=$((COPIED_COUNT + 1))
        else
            log_warning "Binary not found: $TARGET_DIR/$binary"
        fi
    done

    if [ $COPIED_COUNT -eq 0 ]; then
        log_error "No binaries were copied. Build may have failed."
        exit 1
    fi

    chmod +x /opt/pullpiri/bin/* 2>/dev/null || true

    log_success "Pullpiri binaries installed to /opt/pullpiri/bin/ ($COPIED_COUNT binaries)"
}

build_timpani_binaries() {
    log_info "=========================================="
    log_info "Building Timpani Binaries"
    log_info "=========================================="

    # Detect the TIMPANI directory and resolve to absolute path
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    TIMPANI_DIR="$( cd "$SCRIPT_DIR/../../../../TIMPANI/timpani-o" && pwd 2>/dev/null || echo "" )"

    if [ -z "$TIMPANI_DIR" ] || [ ! -d "$TIMPANI_DIR" ]; then
        log_error "Timpani directory not found at: $SCRIPT_DIR/../../../../TIMPANI/timpani-o"
        log_error "Expected location: s-core-poc/Node1/TIMPANI/timpani-o"
        exit 1
    fi

    log_info "Found timpani-o at: $TIMPANI_DIR"

    # Clean previous build if it exists
    if [ -d "$TIMPANI_DIR/build" ]; then
        log_info "Cleaning previous build directory..."
        rm -rf "$TIMPANI_DIR/build"
        log_success "Build directory cleaned"
    fi

    # Create build directory as actual user if it doesn't exist
    su - $ACTUAL_USER -c "mkdir -p '$TIMPANI_DIR/build'"

    log_info "Running cmake..."
    su - $ACTUAL_USER -c "cd '$TIMPANI_DIR/build' && cmake -DCMAKE_C_FLAGS='-include stddef.h' .."

    log_info "Building timpani-o..."
    su - $ACTUAL_USER -c "cd '$TIMPANI_DIR/build' && make"

    # Copy binary to /opt/pullpiri/bin/
    if [ -f "$TIMPANI_DIR/build/timpani-o" ]; then
        log_info "Copying timpani-o to /opt/pullpiri/bin/..."
        cp -f "$TIMPANI_DIR/build/timpani-o" /opt/pullpiri/bin/
        chmod +x /opt/pullpiri/bin/timpani-o
        log_success "Timpani binaries installed to /opt/pullpiri/bin/"
    else
        log_error "timpani-o binary not found at: $TIMPANI_DIR/build/timpani-o"
        log_error "Build may have failed"
        exit 1
    fi
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    log_info "========================================================"
    log_info "Pullpiri LM Demo - System Setup Script"
    log_info "========================================================"
    echo ""

    check_sudo

    # Run all setup steps
    install_prerequisites
    echo ""

    setup_directories
    echo ""

    build_pullpiri_binaries
    echo ""

    build_timpani_binaries
    echo ""

    # Final summary
    log_info "========================================================"
    log_success "Setup Complete!"
    log_info "========================================================"
    echo ""

    # Verify installations
    log_info "Verifying installed binaries in /opt/pullpiri/bin/:"
    if [ -d "/opt/pullpiri/bin" ]; then
        FOUND_COUNT=$(ls -1 /opt/pullpiri/bin/ 2>/dev/null | wc -l)
        if [ "$FOUND_COUNT" -gt 0 ]; then
            ls -lh /opt/pullpiri/bin/ | grep -v "^d" | grep -v "^total" | awk '{print "  "$9" ("$5")"}'
            log_success "Found $FOUND_COUNT binaries"
        else
            log_warning "No binaries found in /opt/pullpiri/bin/ - build may have issues"
        fi
    fi

    echo ""
    log_info "Before starting the demo (optional automated step):"
    log_info "  Run Node1/pullpiri/scripts/update_demo_config.sh to apply automated config changes"
    log_info "  Example: sudo ./pullpiri/scripts/update_demo_config.sh --ip <NODE1_IP> --hostname-node2 <NODE2_HOSTNAME>"
    echo ""
    log_info "Next steps:"
    log_info "  1. Run build_adas_libs.sh to build C++ shared libraries"
    log_info "  2. Run ./run.sh to start the demo"
    echo ""
}

# Run main function
main "$@"
