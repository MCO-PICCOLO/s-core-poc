#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT
#
# Release management script for timpani-n
#
# This script automates the release process:
# 1. Version validation
# 2. Build packages
# 3. Create git tag
# 4. Update CHANGELOG
#
# Usage:
#   ./scripts/release.sh [VERSION]
#
# Example:
#   ./scripts/release.sh 2026.03.1

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Check if version is provided
if [ $# -eq 0 ]; then
    print_error "Version not specified"
    echo "Usage: $0 VERSION"
    echo "Example: $0 2026.03.1"
    exit 1
fi

NEW_VERSION="$1"

# Validate version format (YYYY.MM.PATCH)
if ! [[ "$NEW_VERSION" =~ ^[0-9]{4}\.[0-9]{1,2}\.[0-9]+$ ]]; then
    print_error "Invalid version format: $NEW_VERSION"
    print_info "Expected format: YYYY.MM.PATCH (e.g., 2026.03.1)"
    exit 1
fi

print_info "Starting release process for version $NEW_VERSION"

# Check if working tree is clean
if ! git diff-index --quiet HEAD --; then
    print_error "Working tree is not clean. Commit or stash changes first."
    git status --short
    exit 1
fi

# Check if VERSION file exists
VERSION_FILE="$PROJECT_ROOT/VERSION"
if [ ! -f "$VERSION_FILE" ]; then
    print_error "VERSION file not found: $VERSION_FILE"
    exit 1
fi

CURRENT_VERSION=$(cat "$VERSION_FILE" | tr -d '\n')
print_info "Current version: $CURRENT_VERSION"
print_info "New version: $NEW_VERSION"

# Confirm release
echo ""
print_warn "This will:"
echo "  1. Update VERSION file"
echo "  2. Build DEB and RPM packages"
echo "  3. Create git tag v$NEW_VERSION"
echo "  4. Commit changes"
echo ""
read -p "Continue with release? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_info "Release cancelled"
    exit 0
fi

# Step 1: Update VERSION file
print_step "Updating VERSION file..."
echo "$NEW_VERSION" > "$VERSION_FILE"
print_info "VERSION file updated to $NEW_VERSION"

# Step 2: Build packages
print_step "Building packages..."
"$SCRIPT_DIR/build-packages.sh" --clean

if [ $? -ne 0 ]; then
    print_error "Package build failed"
    # Restore VERSION file
    echo "$CURRENT_VERSION" > "$VERSION_FILE"
    exit 1
fi

# Step 3: Commit VERSION change
print_step "Committing VERSION change..."
git add "$VERSION_FILE"
git commit -m "chore: bump version to $NEW_VERSION"

# Step 4: Create git tag
print_step "Creating git tag v$NEW_VERSION..."
TAG_MESSAGE="Release v$NEW_VERSION

Packages built:
- timpani-n_${NEW_VERSION}_amd64.deb
- timpani-n-${NEW_VERSION}.x86_64.rpm
- timpani-n-${NEW_VERSION}-Linux.tar.gz

See CHANGELOG.md for details."

git tag -a "v$NEW_VERSION" -m "$TAG_MESSAGE"

print_info "======================================"
print_info "Release v$NEW_VERSION created!"
print_info "======================================"
print_info ""
print_info "Packages location: $PROJECT_ROOT/packages/"
print_info ""
print_info "Next steps:"
print_info "  1. Review the packages in ./packages/"
print_info "  2. Update CHANGELOG.md if needed"
print_info "  3. Push commit and tag:"
print_info "     git push origin main"
print_info "     git push origin v$NEW_VERSION"
print_info "  4. Upload packages to release server"
print_info ""
print_warn "Note: Commit and tag have been created locally but not pushed yet."
