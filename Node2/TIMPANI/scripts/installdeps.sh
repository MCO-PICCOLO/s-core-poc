#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

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

echo "🦀 Installing Rust toolchain..."
if ! command -v rustup &>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

export PATH="$HOME/.cargo/bin:$PATH"

echo "🔧 Installing Clippy and Rustfmt..."
rustup component add clippy
rustup component add rustfmt

if ! command -v cargo-deny &>/dev/null; then
  echo "🔐 Installing cargo-deny..."
  cargo install cargo-deny
fi

if ! command -v cargo2junit &>/dev/null; then
  echo "🔐 Installing cargo2junit..."
  cargo install cargo2junit
fi

echo "📌 Installed Rust toolchain versions:"
cargo --version
cargo clippy --version
cargo fmt --version
cargo deny --version
echo "✅ Rust toolchain installed successfully."
