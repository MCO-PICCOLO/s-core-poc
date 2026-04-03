#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
# SPDX-License-Identifier: MIT

set -e

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
    exit 1
fi

FULL_IMAGE="${REGISTRY}/${IMAGE_NAME}:${VERSION}"
LATEST_IMAGE="${REGISTRY}/${IMAGE_NAME}:latest"

echo "==================================="
echo "TIMPANI-O Image Push"
echo "==================================="
echo "Container runtime: $RUNTIME"
echo "Image: ${FULL_IMAGE}"
echo ""

# Check if image exists locally
if ! $RUNTIME image inspect "${FULL_IMAGE}" &> /dev/null; then
    echo "Error: Image '${FULL_IMAGE}' not found locally"
    echo ""
    echo "Build the image first:"
    echo "  ./scripts/build-image.sh ${VERSION}"
    exit 1
fi

# Prompt for login if needed
echo "Ensure you are logged in to the registry:"
echo "  $RUNTIME login ${REGISTRY}"
echo ""
read -p "Continue with push? [y/N] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Push cancelled"
    exit 0
fi

# Push image
echo "Pushing ${FULL_IMAGE}..."
$RUNTIME push "${FULL_IMAGE}"

# Also push latest if version is specified
if [ "$VERSION" != "latest" ]; then
    echo "Pushing ${LATEST_IMAGE}..."
    $RUNTIME push "${LATEST_IMAGE}"
fi

echo ""
echo "==================================="
echo "Push complete!"
echo "==================================="
echo ""
echo "To pull this image on another machine:"
echo "  $RUNTIME pull ${FULL_IMAGE}"
