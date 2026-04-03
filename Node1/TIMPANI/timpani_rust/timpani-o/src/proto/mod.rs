/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

/// Proto-generated modules.
///
/// `tonic::include_proto!` expands to an `include!` of the file that
/// prost/tonic-build wrote into `OUT_DIR` during the build script.
pub mod schedinfo_v1 {
    // Package name declared in schedinfo.proto is `schedinfo.v1`.
    // tonic-build turns the dots into underscores for the file name, so the
    // generated file is `schedinfo.v1.rs` → referenced as "schedinfo.v1".
    tonic::include_proto!("schedinfo.v1");
}
