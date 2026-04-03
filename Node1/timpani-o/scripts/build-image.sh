#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Image configuration
REGISTRY="sdv.lge.com"
IMAGE_NAME="timpani/timpani-o"
VERSION="${1:-latest}"

# Container runtime selection
# Use CONTAINER_RUNTIME env var, or auto-detect (prefer podman over docker)
if [ -n "$CONTAINER_RUNTIME" ]; then
    RUNTIME="$CONTAINER_RUNTIME"
elif command -v podman &> /dev/null; then
    RUNTIME="podman"
elif command -v docker &> /dev/null; then
    RUNTIME="docker"
else
    echo "Error: Neither docker nor podman found"
    echo "Please install Docker or Podman:"
    echo "  - Docker: https://docs.docker.com/engine/install/"
    echo "  - Podman: https://podman.io/getting-started/installation"
    exit 1
fi

echo "==================================="
echo "TIMPANI-O Container Image Builder"
echo "==================================="
echo "Container runtime: $RUNTIME"
echo "Image: ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
echo ""

cd "$PROJECT_DIR"

# Ensure submodule is initialized
if [ ! -f "external/libtrpc/CMakeLists.txt" ]; then
    echo "Initializing libtrpc submodule..."
    git submodule update --init --recursive
fi

# Build image
echo "Building image..."
$RUNTIME build \
    -t "${REGISTRY}/${IMAGE_NAME}:${VERSION}" \
    -t "${REGISTRY}/${IMAGE_NAME}:latest" \
    -f Dockerfile \
    .

echo ""
echo "==================================="
echo "Build complete!"
echo "==================================="
echo "Image: ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
echo ""
echo "To run locally:"
echo "  $RUNTIME run -p 50052:50052 -p 7777:7777 ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
echo ""
echo "To push to registry:"
echo "  $RUNTIME login ${REGISTRY}"
echo "  $RUNTIME push ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
