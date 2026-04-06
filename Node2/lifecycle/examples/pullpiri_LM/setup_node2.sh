#!/bin/bash
# ============================================================================
# Node2 System Setup Script — Pullpiri Worker Node
# ============================================================================
# This script automates the setup steps for Node2 (Worker Node)
# Run with: sudo ./setup_node2.sh

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
# Section 1: Install Prerequisites
# ============================================================================

install_prerequisites() {
    log_info "=========================================="
    log_info "Section 1: Installing Prerequisites"
    log_info "=========================================="

    # Update package lists
    log_info "Updating package lists..."
    apt update

    # Check and install Rust
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

    # Check and install Bazelisk
    log_info "Checking for Bazel/Bazelisk..."
    if command -v bazel &> /dev/null; then
        BAZEL_VERSION=$(bazel --version 2>&1 | grep -oP 'bazel \K[0-9.]+' || echo "unknown")
        log_success "Bazel already installed (version: $BAZEL_VERSION)"
    else
        log_info "Bazel not found. Installing Bazelisk..."
        wget -O /usr/local/bin/bazel \
            https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-amd64
        chmod +x /usr/local/bin/bazel
        log_success "Bazelisk installed"
    fi

    # Check and install Java 17
    log_info "Checking for Java 17..."
    if command -v java &> /dev/null; then
        JAVA_VERSION=$(java -version 2>&1 | head -n 1 | awk -F '"' '{print $2}' | awk -F '.' '{print $1}')
        if [ "$JAVA_VERSION" = "17" ]; then
            log_success "Java 17 already installed"
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

    # Install build tools
    log_info "Installing build tools..."
    MISSING_DEPS=()

    if ! dpkg -l | grep -q "^ii  cmake "; then
        MISSING_DEPS+=("cmake")
    fi
    if ! dpkg -l | grep -q "^ii  build-essential "; then
        MISSING_DEPS+=("build-essential")
    fi
    if ! dpkg -l | grep -q "^ii  git "; then
        MISSING_DEPS+=("git")
    fi

    if [ ${#MISSING_DEPS[@]} -eq 0 ]; then
        log_success "All build tools already installed"
    else
        log_info "Installing missing build tools: ${MISSING_DEPS[*]}"
        apt install -y "${MISSING_DEPS[@]}"
        log_success "Build tools installed"
    fi

    # Install Podman
    log_info "Checking for Podman..."
    if command -v podman &> /dev/null; then
        log_success "Podman already installed"
    else
        log_info "Installing Podman..."
        apt install -y podman
        log_success "Podman installed"
    fi

    # Install Timpani dependencies
    log_info "Checking Timpani dependencies..."
    if dpkg -l | grep -q "libsystemd-dev"; then
        log_success "libsystemd-dev already installed"
    else
        log_info "Installing libsystemd-dev..."
        apt install -y libsystemd-dev
        log_success "libsystemd-dev installed"
    fi
}

# ============================================================================
# Section 2: Create Directory Structure
# ============================================================================

setup_directories() {
    log_info "=========================================="
    log_info "Section 2: Creating Directory Structure"
    log_info "=========================================="

    log_info "Creating /opt/pullpiri directory structure..."
    mkdir -p /opt/pullpiri/bin
    mkdir -p /opt/pullpiri/lib

    log_success "Directory structure created:"
    log_success "  /opt/pullpiri/bin       - service binaries"
    log_success "  /opt/pullpiri/lib       - shared .so files"
}

# ============================================================================
# Section 3: Build NodeAgent Binary
# ============================================================================

build_nodeagent() {
    log_info "=========================================="
    log_info "Section 3: Building NodeAgent Binary"
    log_info "=========================================="

    # Detect the pullpiri directory
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    NODEAGENT_DIR="$( cd "$SCRIPT_DIR/../../../pullpiri/src/agent/nodeagent" && pwd )"

    if [ ! -d "$NODEAGENT_DIR" ]; then
        log_error "NodeAgent directory not found at: $NODEAGENT_DIR"
        log_error "Expected location: s-core-poc/Node2/pullpiri/src/agent/nodeagent"
        exit 1
    fi

    log_info "Found nodeagent at: $NODEAGENT_DIR"

    # Build nodeagent
    log_info "Building nodeagent binary..."
    su - $ACTUAL_USER -c "cd '$NODEAGENT_DIR' && source $ACTUAL_HOME/.cargo/env && cargo build --release"

    if [ $? -eq 0 ]; then
        log_success "Built nodeagent"
    else
        log_error "Failed to build nodeagent"
        exit 1
    fi

    # Copy binary to /opt/pullpiri/bin/
    TARGET_DIR="$NODEAGENT_DIR/target/release"
    if [ -f "$TARGET_DIR/nodeagent" ]; then
        log_info "Copying nodeagent to /opt/pullpiri/bin/..."
        cp -f "$TARGET_DIR/nodeagent" /opt/pullpiri/bin/

        # Set ownership and permissions
        chown root:root /opt/pullpiri/bin/* || true
        chmod +x /opt/pullpiri/bin/* || true

        log_success "NodeAgent binary installed to /opt/pullpiri/bin/"
    else
        log_error "nodeagent binary not found at: $TARGET_DIR/nodeagent"
        exit 1
    fi
}

# ============================================================================
# Section 4: Build Timpani-n
# ============================================================================

build_timpani() {
    log_info "=========================================="
    log_info "Section 4: Building Timpani-n"
    log_info "=========================================="

    # Detect the TIMPANI directory
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    TIMPANI_DIR="$( cd "$SCRIPT_DIR/../../../TIMPANI" && pwd 2>/dev/null || echo "" )"

    if [ -z "$TIMPANI_DIR" ] || [ ! -d "$TIMPANI_DIR" ]; then
        log_error "Timpani directory not found at: $SCRIPT_DIR/../../../TIMPANI"
        log_error "Expected location: s-core-poc/Node2/TIMPANI"
        exit 1
    fi

    log_info "Found TIMPANI at: $TIMPANI_DIR"

    # Clone libbpf if needed
    if [ ! -d "$TIMPANI_DIR/libbpf/src" ]; then
        log_info "Cloning libbpf..."
        su - $ACTUAL_USER -c "cd '$TIMPANI_DIR' && git clone https://github.com/libbpf/libbpf.git"
        log_success "libbpf cloned"
    else
        log_success "libbpf already present"
    fi

    # Clean previous build if it exists
    if [ -d "$TIMPANI_DIR/timpani-n/build" ]; then
        log_info "Cleaning previous build directory..."
        rm -rf "$TIMPANI_DIR/timpani-n/build"
        log_success "Build directory cleaned"
    fi

    # Create build directory
    su - $ACTUAL_USER -c "mkdir -p '$TIMPANI_DIR/timpani-n/build'"

    log_info "Running cmake..."
    su - $ACTUAL_USER -c "cd '$TIMPANI_DIR/timpani-n/build' && cmake .."

    log_info "Building timpani-n..."
    su - $ACTUAL_USER -c "cd '$TIMPANI_DIR/timpani-n/build' && make"

    # Copy binary to /opt/pullpiri/bin/
    if [ -f "$TIMPANI_DIR/timpani-n/build/timpani-n" ]; then
        log_info "Copying timpani-n to /opt/pullpiri/bin/..."
        cp -f "$TIMPANI_DIR/timpani-n/build/timpani-n" /opt/pullpiri/bin/

        # Set ownership and permissions
        chown root:root /opt/pullpiri/bin/* || true
        chmod +x /opt/pullpiri/bin/* || true

        log_success "Timpani-n binary installed to /opt/pullpiri/bin/"
    else
        log_error "timpani-n binary not found at: $TIMPANI_DIR/timpani-n/build/timpani-n"
        log_error "Build may have failed"
        exit 1
    fi
}

# ============================================================================
# Section 5: Build sea-app Container
# ============================================================================

build_sea_app() {
    log_info "=========================================="
    log_info "Section 5: Building sea-app Container"
    log_info "=========================================="

    # Detect the sea_app directory
    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    SEA_APP_DIR="$( cd "$SCRIPT_DIR/../../../sea_app" && pwd )"

    if [ ! -d "$SEA_APP_DIR" ]; then
        log_error "sea_app directory not found at: $SEA_APP_DIR"
        log_error "Expected location: s-core-poc/Node2/sea_app"
        exit 1
    fi

    log_info "Found sea_app at: $SEA_APP_DIR"

    # Build the Rust binary
    log_info "Building sea_app Rust binary..."
    su - $ACTUAL_USER -c "cd '$SEA_APP_DIR' && source $ACTUAL_HOME/.cargo/env && cargo build --release"

    if [ $? -eq 0 ]; then
        log_success "Built sea_app binary"
    else
        log_error "Failed to build sea_app"
        exit 1
    fi

    # Build Podman container image
    log_info "Building Podman container image..."
    su - $ACTUAL_USER -c "cd '$SEA_APP_DIR' && podman build -t sdv.lge.com/demo/sea_app:1.0 ."

    if [ $? -eq 0 ]; then
        log_success "sea-app container image built: sdv.lge.com/demo/sea_app:1.0"
    else
        log_error "Failed to build sea-app container image"
        exit 1
    fi
}

# ============================================================================
# Section 6: Create WORKSPACE File
# ============================================================================

create_workspace() {
    log_info "=========================================="
    log_info "Section 6: Creating WORKSPACE File"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    LIFECYCLE_DIR="$( cd "$SCRIPT_DIR/../.." && pwd )"

    if [ ! -f "$LIFECYCLE_DIR/WORKSPACE" ]; then
        log_info "Creating WORKSPACE file..."
        su - $ACTUAL_USER -c "touch '$LIFECYCLE_DIR/WORKSPACE'"
        log_success "WORKSPACE file created at: $LIFECYCLE_DIR/WORKSPACE"
    else
        log_success "WORKSPACE file already exists"
    fi
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    log_info "========================================================"
    log_info "Node2 Worker Node - System Setup Script"
    log_info "========================================================"
    echo ""

    check_sudo

    # Run all setup steps


    setup_directories
    echo ""

    build_nodeagent
    echo ""

    build_timpani
    echo ""

    build_sea_app
    echo ""

    create_workspace
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
    log_info "Verifying container images:"
    su - $ACTUAL_USER -c "podman images | grep sea_app" || log_warning "sea-app image not found"

    echo ""
    log_info "Next steps:"
    log_info "  1. Configure /etc/piccolo/nodeagent.yaml with Node1 and Node2 IPs"
    log_info "  2. Update lifecycle/examples/pullpiri_LM/config/pullpiri_lm_config.json"
    log_info "  3. Run ./run.sh to start the Launch Manager"
    echo ""
}

# Run main function
main "$@"
