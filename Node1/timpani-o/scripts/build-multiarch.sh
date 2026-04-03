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
PUSH_FLAG=""
PLATFORMS="linux/amd64,linux/arm64"

# Parse arguments
shift 2>/dev/null || true
while [[ $# -gt 0 ]]; do
    case $1 in
        --push)
            PUSH_FLAG="--push"
            shift
            ;;
        --platform)
            PLATFORMS="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [version] [--push] [--platform linux/amd64,linux/arm64]"
            exit 1
            ;;
    esac
done

cd "$PROJECT_DIR"

# Ensure submodule is initialized
if [ ! -f "external/libtrpc/CMakeLists.txt" ]; then
    echo "Initializing libtrpc submodule..."
    git submodule update --init --recursive
fi

echo "============================================"
echo "TIMPANI-O Multi-Architecture Image Builder"
echo "============================================"

# Container runtime selection
# Use CONTAINER_RUNTIME env var, or auto-detect (prefer podman over docker)
if [ -n "$CONTAINER_RUNTIME" ] && [ "$CONTAINER_RUNTIME" = "podman" ]; then
    echo "Using: podman (via CONTAINER_RUNTIME)"
    echo ""
    echo "For Podman multi-arch builds, run separately for each architecture:"
    echo "  podman build --platform linux/amd64 -t ${REGISTRY}/${IMAGE_NAME}:${VERSION}-amd64 ."
    echo "  podman build --platform linux/arm64 -t ${REGISTRY}/${IMAGE_NAME}:${VERSION}-arm64 ."
    echo ""
    echo "Then create a manifest:"
    echo "  podman manifest create ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
    echo "  podman manifest add ${REGISTRY}/${IMAGE_NAME}:${VERSION} ${REGISTRY}/${IMAGE_NAME}:${VERSION}-amd64"
    echo "  podman manifest add ${REGISTRY}/${IMAGE_NAME}:${VERSION} ${REGISTRY}/${IMAGE_NAME}:${VERSION}-arm64"
    echo "  podman manifest push ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
    exit 0
elif command -v podman &> /dev/null && [ -z "$CONTAINER_RUNTIME" ]; then-z "$CONTAINER_RUNTIME" ]; then
    echo "Using: podman (auto-detected, set CONTAINER_RUNTIME=docker to use Docker)"
    echo ""
    echo "For Podman multi-arch builds, run separately for each architecture:"
    echo "  podman build --platform linux/amd64 -t ${REGISTRY}/${IMAGE_NAME}:${VERSION}-amd64 ."
    echo "  podman build --platform linux/arm64 -t ${REGISTRY}/${IMAGE_NAME}:${VERSION}-arm64 ."
    echo ""
    echo "Then create a manifest:"
    echo "  podman manifest create ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
    echo "  podman manifest add ${REGISTRY}/${IMAGE_NAME}:${VERSION} ${REGISTRY}/${IMAGE_NAME}:${VERSION}-amd64"
    echo "  podman manifest add ${REGISTRY}/${IMAGE_NAME}:${VERSION} ${REGISTRY}/${IMAGE_NAME}:${VERSION}-arm64"
    echo "  podman manifest push ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
    exit 0
elif [ -n "$CONTAINER_RUNTIME" ] && [ "$CONTAINER_RUNTIME" = "docker" ] && command -v docker &> /dev/null && docker buildx version &> /dev/null; then
    echo "Using: docker buildx (via CONTAINER_RUNTIME)"
elif command -v docker &> /dev/null && docker buildx version &> /dev/null; then
    echo "Using: docker buildx (auto-detected)"
else
    echo "Error: docker buildx is required for multi-arch builds"
    echo ""
    echo "Install Docker with buildx support:"
    echo "  https://docs.docker.com/buildx/working-with-buildx/"
    echo ""
    echo "Or use Podman with manifest support (see above)"
    exit 1
fi

echo "Image: ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
echo "Platforms: ${PLATFORMS}"
if [ -n "$PUSH_FLAG" ]; then
    echo "Mode: Build and Push"
else
    echo "Mode: Build only (use --push to push to registry)"
fi
echo ""

# Create/use buildx builder
BUILDER_NAME="timpani-multiarch"
if ! docker buildx inspect "$BUILDER_NAME" &> /dev/null; then
    echo "Creating buildx builder: $BUILDER_NAME"
    docker buildx create --name "$BUILDER_NAME" --use --bootstrap
else
    docker buildx use "$BUILDER_NAME"
fi

# Build for multiple architectures
echo "Building multi-arch image..."
docker buildx build \
    --platform "${PLATFORMS}" \
    -t "${REGISTRY}/${IMAGE_NAME}:${VERSION}" \
    -t "${REGISTRY}/${IMAGE_NAME}:latest" \
    -f Dockerfile \
    $PUSH_FLAG \
    .

echo ""
echo "============================================"
echo "Multi-arch build complete!"
echo "============================================"
if [ -n "$PUSH_FLAG" ]; then
    echo "Image pushed to: ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
    echo ""
    echo "To pull on any architecture:"
    echo "  docker pull ${REGISTRY}/${IMAGE_NAME}:${VERSION}"
else
    echo "Image built locally for platforms: ${PLATFORMS}"
    echo ""
    echo "To push to registry, run with --push flag:"
    echo "  $0 ${VERSION} --push"
fi
