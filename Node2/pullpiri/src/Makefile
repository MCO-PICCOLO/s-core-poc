# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

.PHONY: default build
build:
	cargo build

.PHONY: clean
clean:
	cargo clean
	cargo clean --manifest-path=agent/nodeagent/Cargo.toml
	cargo clean --manifest-path=server/rocksdbservice/Cargo.toml
	cargo clean --manifest-path=tools/Cargo.toml

.PHONY: tool
tool:
	cargo build --manifest-path=tools/Cargo.toml

.PHONY: fmt
fmt:
	cargo fmt
	cd agent/nodeagent && cargo fmt
	cd server/rocksdbservice && cargo fmt
	cd tools && cargo fmt

.PHONY: clippy
clippy:
	cargo clippy
	cd agent/nodeagent && cargo clippy
	cd server/rocksdbservice && cargo clippy
	cd tools && cargo clippy
