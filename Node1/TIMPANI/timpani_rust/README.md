<!--
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: MIT
-->

# TIMPANI Rust

Rust ports of the TIMPANI global scheduler and node executor.
The crates mirror the C++ implementation in [`timpani-o/`](../timpani-o) and the C implementation in [`timpani-n/`](../timpani-n).

## Crates

| Crate | Description |
|-------|-------------|
| `timpani-o` | Global scheduler — task admission, hyperperiod calculation, gRPC service |
| `timpani-n` | Node executor — executes scheduled tasks on individual nodes |

## Prerequisites

- Rust stable toolchain (`rustup` recommended)
- `protoc` — Protocol Buffers compiler
- [`just`](https://github.com/casey/just) — optional, for the dev workflow shortcuts
- [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) — optional, for licence/dependency checks

```bash
# Install optional tools
cargo install just cargo-deny
```

## Build

```bash
cd timpani_rust
cargo build
```

## Test

```bash
cargo test
```

## Run

```bash
cargo run -- --help
```

## Dev Workflow (Justfile)

`just check` mirrors the full CI pipeline locally:

```
just check     # fmt + clippy + deny + build + test (all in one)
just fmt       # format
just fix       # auto-fix clippy suggestions
just setup     # install the pre-push git hook
```

## Configuration

The scheduler reads a YAML file describing the available nodes and their CPUs.
See [`timpani-o/examples/node_configurations.yaml`](../timpani-o/examples/node_configurations.yaml) for an example.
