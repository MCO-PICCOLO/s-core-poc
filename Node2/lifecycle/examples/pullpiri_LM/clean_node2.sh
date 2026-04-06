#!/bin/bash
# ============================================================================
# Node2 Cleanup Script — Remove all setup artifacts
# ============================================================================
# This script removes all files and directories created by setup_node2.sh
# Run with: sudo ./clean_node2.sh

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Detect actual user (even when running with sudo)
ACTUAL_USER="${SUDO_USER:-$USER}"

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

# Confirm before cleaning
confirm_clean() {
    log_warning "========================================================"
    log_warning "This will remove ALL installed binaries and artifacts!"
    log_warning "========================================================"
    echo ""
    log_info "The following will be removed:"
    echo "  - /opt/pullpiri/ (nodeagent, timpani-n, configs)"
    echo "  - Build artifacts in pullpiri/src/target/"
    echo "  - Build artifacts in TIMPANI/timpani-n/build/"
    echo "  - sea-app container image (if exists)"
    echo "  - sea-app build artifacts"
    echo "  - Lifecycle WORKSPACE file"
    echo ""

    read -p "Are you sure you want to continue? (yes/no): " -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        log_info "Cleanup cancelled."
        exit 0
    fi
}

# ============================================================================
# Cleanup Functions
# ============================================================================

clean_pullpiri_install() {
    log_info "=========================================="
    log_info "Cleaning /opt/pullpiri installation"
    log_info "=========================================="

    if [ -d "/opt/pullpiri" ]; then
        log_info "Removing /opt/pullpiri directory..."
        rm -rf /opt/pullpiri
        log_success "Removed /opt/pullpiri"
    else
        log_warning "/opt/pullpiri not found (already clean)"
    fi
}

clean_nodeagent_builds() {
    log_info "=========================================="
    log_info "Cleaning NodeAgent build artifacts"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    PULLPIRI_DIR="$( cd "$SCRIPT_DIR/../../../pullpiri" && pwd 2>/dev/null || echo "" )"

    NODEAGENT_TARGET="$PULLPIRI_DIR/src/agent/nodeagent/target"
    if [ -n "$PULLPIRI_DIR" ] && [ -d "$NODEAGENT_TARGET" ]; then
        log_info "Removing NodeAgent build artifacts..."
        rm -rf "$NODEAGENT_TARGET"
        log_success "Removed $NODEAGENT_TARGET"
    else
        log_warning "NodeAgent build artifacts not found"
    fi
}

clean_timpani_builds() {
    log_info "=========================================="
    log_info "Cleaning Timpani-n build artifacts"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    TIMPANI_DIR="$( cd "$SCRIPT_DIR/../../../TIMPANI" && pwd 2>/dev/null || echo "" )"

    if [ -n "$TIMPANI_DIR" ]; then
        # Clean timpani-n
        if [ -d "$TIMPANI_DIR/timpani-n/build" ]; then
            log_info "Removing timpani-n build directory..."
            rm -rf "$TIMPANI_DIR/timpani-n/build"
            log_success "Removed timpani-n/build"
        fi

        # Note: Keep libbpf as it's cloned from git, user can remove manually if needed
        if [ -d "$TIMPANI_DIR/libbpf" ]; then
            log_info "Note: libbpf directory kept (git clone). Remove manually if needed."
        fi
    else
        log_warning "TIMPANI directory not found"
    fi
}

clean_sea_app() {
    log_info "=========================================="
    log_info "Cleaning sea-app artifacts"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    SEA_APP_DIR="$( cd "$SCRIPT_DIR/../../../sea_app" && pwd 2>/dev/null || echo "" )"

    # Remove container images (both possible tags) - run as the actual user
    if command -v podman &> /dev/null; then
        # Check for containers in actual user's context (containers are built by the user, not root)
        if su - $ACTUAL_USER -c "podman images | grep -q 'sdv.lge.com/demo/sea_app'" 2>/dev/null; then
            log_info "Removing sea-app container image (sdv.lge.com/demo/sea_app:1.0) from user $ACTUAL_USER's podman..."
            su - $ACTUAL_USER -c "podman rmi sdv.lge.com/demo/sea_app:1.0" 2>/dev/null || true
            log_success "Removed sea-app container image"
        elif su - $ACTUAL_USER -c "podman images | grep -q 'localhost/sea-app'" 2>/dev/null; then
            log_info "Removing sea-app container image (localhost/sea-app:latest) from user $ACTUAL_USER's podman..."
            su - $ACTUAL_USER -c "podman rmi localhost/sea-app:latest" 2>/dev/null || true
            log_success "Removed localhost/sea-app container image"
        else
            log_warning "No sea-app container images found for user $ACTUAL_USER (already clean)"
        fi
    fi

    # Remove build artifacts
    if [ -n "$SEA_APP_DIR" ]; then
        if [ -d "$SEA_APP_DIR/target" ]; then
            log_info "Removing sea-app build artifacts..."
            rm -rf "$SEA_APP_DIR/target"
            log_success "Removed $SEA_APP_DIR/target"
        fi
    else
        log_warning "sea_app directory not found"
    fi
}

clean_lifecycle_workspace() {
    log_info "=========================================="
    log_info "Cleaning Lifecycle WORKSPACE"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    LIFECYCLE_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

    if [ -f "$LIFECYCLE_ROOT/WORKSPACE" ]; then
        log_info "Removing WORKSPACE file..."
        rm -f "$LIFECYCLE_ROOT/WORKSPACE"
        log_success "Removed WORKSPACE file"
    else
        log_warning "WORKSPACE file not found"
    fi

    # Clean Bazel symlinks if they exist
    log_info "Checking for Bazel symlinks..."
    rm -rf "$LIFECYCLE_ROOT/bazel-bin" 2>/dev/null || true
    rm -rf "$LIFECYCLE_ROOT/bazel-out" 2>/dev/null || true
    rm -rf "$LIFECYCLE_ROOT/bazel-lifecycle" 2>/dev/null || true
    rm -rf "$LIFECYCLE_ROOT/bazel-testlogs" 2>/dev/null || true
    log_success "Cleaned Bazel symlinks (if any)"
}

clean_config_files() {
    log_info "=========================================="
    log_info "Cleaning configuration files"
    log_info "=========================================="

    # Note: We don't remove /etc/piccolo/settings.yaml as user may have customized it
    if [ -f "/etc/piccolo/settings.yaml" ]; then
        log_info "Note: /etc/piccolo/settings.yaml kept (contains custom IP config)"
        log_info "      Remove manually if needed: sudo rm /etc/piccolo/settings.yaml"
    fi
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    log_info "========================================================"
    log_info "Node2 Worker Node - Cleanup Script"
    log_info "========================================================"
    echo ""

    check_sudo
    confirm_clean

    # Run all cleanup steps
    clean_pullpiri_install
    echo ""

    clean_nodeagent_builds
    echo ""

    clean_timpani_builds
    echo ""

    clean_sea_app
    echo ""

    clean_lifecycle_workspace
    echo ""

    clean_config_files
    echo ""

    # Final summary
    log_info "========================================================"
    log_success "Cleanup Complete!"
    log_info "========================================================"
    echo ""

    log_info "Summary:"
    log_success "  ✓ Removed /opt/pullpiri installation"
    log_success "  ✓ Cleaned NodeAgent build artifacts"
    log_success "  ✓ Cleaned Timpani-n build artifacts"
    log_success "  ✓ Cleaned sea-app artifacts and container image"
    log_success "  ✓ Removed Lifecycle WORKSPACE file"
    echo ""

    log_info "Manual cleanup (if needed):"
    log_info "  - Remove /etc/piccolo/settings.yaml (custom config)"
    log_info "  - Remove TIMPANI/libbpf (git clone)"
    log_info "  - Remove other podman images: podman rmi <image>"
    echo ""

    log_info "To rebuild, run:"
    log_info "  cd ~/s-core-poc/Node2/lifecycle/examples/pullpiri_LM"
    log_info "  sudo ./setup_node2.sh"
    echo ""
}

# Run main function
main "$@"
