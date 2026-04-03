#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

# Enable JSON test output even on stable Rust
export RUSTC_BOOTSTRAP=1

echo "🛠️ Updating package lists..."
sudo apt-get update -y

echo "📦 Installing common development packages..."
common_packages=(
  libdbus-1-dev
  git
  make
  gcc
  protobuf-compiler
  build-essential
  pkg-config
  curl
  libssl-dev
  nodejs
  jq
  npm
)
DEBIAN_FRONTEND=noninteractive sudo apt-get install -y "${common_packages[@]}"
echo "✅ Base packages installed successfully"

# ----------------------------------------
# 🦀 Install rustup, Clippy, Rustfmt, and cargo-deny
# ----------------------------------------
echo "🦀 Installing Rust toolchain..."
if ! command -v rustup &>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

# Ensure PATH is correctly set
export PATH="$HOME/.cargo/bin:$PATH"

# Install required Rust components
echo "🔧 Installing Clippy and Rustfmt..."
rustup component add clippy
rustup component add rustfmt

# Install cargo-deny
if ! command -v cargo-deny &>/dev/null; then
  echo "🔐 Installing cargo-deny..."
  cargo install cargo-deny
fi

# Install cargo2junit
if ! command -v cargo2junit &>/dev/null; then
  echo "🔐 Installing cargo2junit..."
  cargo install cargo2junit
fi

# Show installed versions
echo "📌 Installed Rust toolchain versions:"
cargo --version
cargo clippy --version
cargo fmt --version
cargo deny --version
echo "✅ Rust toolchain installed successfully."

# ----------------------------------------
# 🗄️ RocksDB Information
# ----------------------------------------

echo ""
echo "ℹ️  RocksDB Storage Backend Information:"
echo "   RocksDB runs as a containerized gRPC service (no manual installation needed)"
echo "   Container: ghcr.io/mco-piccolo/pullpiri-rocksdb:v11.18.0"
echo "   gRPC Port: 47007"
echo "   Storage Path: /tmp/pullpiri_shared_rocksdb"
echo "   In CI: RocksDB container is started automatically in the workflow"
echo ""

# ----------------------------------------
# 🐳 Install Docker and Docker Compose
# ----------------------------------------

echo "🐳 Installing Docker CLI and Docker Compose..."

# Install Docker dependencies
sudo apt-get update -y
sudo apt-get install -y \
    ca-certificates \
    curl \
    gnupg \
    lsb-release

# Add Docker’s official GPG key
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

# Set up Docker stable repository for Ubuntu Jammy
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
  https://download.docker.com/linux/ubuntu jammy stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Update and install Docker packages
sudo apt-get update -y
sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Verify installation
docker --version
docker compose version

echo "✅ Docker and Docker Compose installed."

echo ""
echo "🎉 All dependencies installed successfully!"
echo ""
echo "📝 Next Steps for RocksDB:"
echo "   1. Build project: make build"
echo "   2. Setup RocksDB storage: make setup-shared-rocksdb"
echo "   3. Build container images: make image"
echo "   4. Start services: make install"
