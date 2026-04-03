/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Timpani-O – global scheduler (Rust port)
//!
//! Module layout (filled in as the migration progresses):
//!
//! ```text
//! lib.rs
//! ├── proto/          – generated gRPC/protobuf types & stubs
//! ├── config/         – YAML node configuration
//! ├── scheduler/      – three scheduling algorithms
//! ├── hyperperiod/    – LCM / GCD helpers
//! ├── grpc/           – gRPC server + client wiring
//! └── fault/          – fault reporting to Pullpiri
//! ```

pub mod config;
pub mod fault;
pub mod grpc;
pub mod hyperperiod;
pub mod proto;
pub mod scheduler;
pub mod task;
