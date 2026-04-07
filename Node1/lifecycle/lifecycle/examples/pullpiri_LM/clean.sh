#!/bin/bash
# ============================================================================
# Node1 Cleanup Script — Remove all setup artifacts
# ============================================================================
# This script removes all files and directories created by setup_system.sh
# Run with: sudo ./clean.sh

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

# Confirm before cleaning
confirm_clean() {
    log_warning "========================================================"
    log_warning "This will remove ALL installed binaries and artifacts!"
    log_warning "========================================================"
    echo ""
    log_info "The following will be removed:"
    echo "  - /opt/pullpiri/ (all binaries, libraries, configs)"
    echo "  - Build artifacts in pullpiri/src/target/"
    echo "  - Build artifacts in TIMPANI/timpani-o/build/"
    echo "  - Build artifacts in TIMPANI/timpani-n/build/"
    echo "  - ADAS libraries in feo/examples/rust/mini-adas/lib/"
    echo "  - ADAS build artifacts in feo/examples/rust/mini-adas/target/"
    echo "  - Lifecycle bazel-* symlinks"
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

clean_pullpiri_builds() {
    log_info "=========================================="
    log_info "Cleaning Pullpiri build artifacts"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    PULLPIRI_DIR="$( cd "$SCRIPT_DIR/../../../../pullpiri" && pwd 2>/dev/null || echo "" )"

    if [ -n "$PULLPIRI_DIR" ] && [ -d "$PULLPIRI_DIR/src/target" ]; then
        log_info "Removing Pullpiri build artifacts..."
        rm -rf "$PULLPIRI_DIR/src/target"
        log_success "Removed $PULLPIRI_DIR/src/target"
    else
        log_warning "Pullpiri build artifacts not found"
    fi
}

clean_timpani_builds() {
    log_info "=========================================="
    log_info "Cleaning Timpani build artifacts"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    TIMPANI_DIR="$( cd "$SCRIPT_DIR/../../../../TIMPANI" && pwd 2>/dev/null || echo "" )"

    if [ -n "$TIMPANI_DIR" ]; then
        # Clean timpani-o
        if [ -d "$TIMPANI_DIR/timpani-o/build" ]; then
            log_info "Removing timpani-o build directory..."
            rm -rf "$TIMPANI_DIR/timpani-o/build"
            log_success "Removed timpani-o/build"
        fi

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

clean_adas_builds() {
    log_info "=========================================="
    log_info "Cleaning ADAS build artifacts"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    FEO_DIR="$( cd "$SCRIPT_DIR/../../../../feo" && pwd 2>/dev/null || echo "" )"

    if [ -n "$FEO_DIR" ]; then
        # Clean mini-adas libraries
        MINI_ADAS_LIB="$FEO_DIR/examples/rust/mini-adas/lib"
        if [ -d "$MINI_ADAS_LIB" ]; then
            log_info "Removing ADAS libraries..."
            rm -rf "$MINI_ADAS_LIB"
            log_success "Removed $MINI_ADAS_LIB"
        fi

        # Clean mini-adas build artifacts
        MINI_ADAS_TARGET="$FEO_DIR/examples/rust/mini-adas/target"
        if [ -d "$MINI_ADAS_TARGET" ]; then
            log_info "Removing ADAS build artifacts..."
            rm -rf "$MINI_ADAS_TARGET"
            log_success "Removed $MINI_ADAS_TARGET"
        fi

        # Clean feo target directory if exists
        if [ -d "$FEO_DIR/target" ]; then
            log_info "Removing feo target directory..."
            rm -rf "$FEO_DIR/target"
            log_success "Removed $FEO_DIR/target"
        fi
    else
        log_warning "feo directory not found"
    fi
}

clean_lifecycle_builds() {
    log_info "=========================================="
    log_info "Cleaning Lifecycle build artifacts"
    log_info "=========================================="

    SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
    LIFECYCLE_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

    if [ -d "$LIFECYCLE_ROOT" ]; then
        # Clean Bazel symlinks
        log_info "Removing Bazel build symlinks..."
        rm -rf "$LIFECYCLE_ROOT/bazel-bin" 2>/dev/null || true
        rm -rf "$LIFECYCLE_ROOT/bazel-out" 2>/dev/null || true
        rm -rf "$LIFECYCLE_ROOT/bazel-lifecycle" 2>/dev/null || true
        rm -rf "$LIFECYCLE_ROOT/bazel-testlogs" 2>/dev/null || true
        log_success "Removed Bazel symlinks"

        # Clean etc directory (generated configs)
        if [ -d "$LIFECYCLE_ROOT/etc" ]; then
            log_info "Cleaning generated config files in etc/..."
            rm -f "$LIFECYCLE_ROOT/etc/lm_demo.bin" 2>/dev/null || true
            rm -f "$LIFECYCLE_ROOT/etc/hm_demo.bin" 2>/dev/null || true
            rm -f "$LIFECYCLE_ROOT/etc/hmcore.bin" 2>/dev/null || true
            rm -f "$LIFECYCLE_ROOT/etc/hmproc_adas_primary.bin" 2>/dev/null || true
            # Preserve logging config files; they may contain user customizations.
            log_info "Preserving lifecycle/etc/logging.json and lifecycle/etc/ecu_logging_config.json"
            log_success "Cleaned lifecycle/etc/"
        fi
    fi
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
    log_info "Node1 Master Node - Cleanup Script"
    log_info "========================================================"
    echo ""

    check_sudo
    confirm_clean

    # Run all cleanup steps
    clean_pullpiri_install
    echo ""

    clean_pullpiri_builds
    echo ""

    clean_timpani_builds
    echo ""

    clean_adas_builds
    echo ""

    clean_lifecycle_builds
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
    log_success "  ✓ Cleaned Pullpiri build artifacts"
    log_success "  ✓ Cleaned Timpani build artifacts"
    log_success "  ✓ Cleaned ADAS build artifacts"
    log_success "  ✓ Cleaned Lifecycle build artifacts"
    echo ""

    log_info "Manual cleanup (if needed):"
    log_info "  - Remove /etc/piccolo/settings.yaml (custom config)"
    log_info "  - Remove TIMPANI/libbpf (git clone)"
    log_info "  - Run 'bazel clean --expunge' in lifecycle directory"
    echo ""

    log_info "To rebuild, run:"
    log_info "  cd ~/s-core-poc/Node1/lifecycle/lifecycle/examples/pullpiri_LM"
    log_info "  sudo ./setup_system.sh"
    log_info "  ./build_adas_libs.sh"
    echo ""
}

# Run main function
main "$@"
