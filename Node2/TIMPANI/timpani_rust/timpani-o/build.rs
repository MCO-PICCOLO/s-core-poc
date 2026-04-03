/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

/// Build script – compiles protobuf definitions into Rust source code.
///
/// tonic-build wraps prost-build and additionally generates tonic server/client
/// stubs.  The generated files are written to `OUT_DIR` (managed by Cargo) and
/// pulled into the crate via `tonic::include_proto!` in `src/proto/mod.rs`.
///
/// Prerequisites
/// -------------
/// `protoc` (the protobuf compiler) must be available on `$PATH`, or its path
/// must be set in the `PROTOC` environment variable before running `cargo build`.
/// Install on Ubuntu/Debian: `sudo apt install -y protobuf-compiler`
/// Install on macOS:          `brew install protobuf`
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Path to the proto source relative to this crate's root.
    // Both proto files now live inside the Rust project itself so that the
    // crate can be built without the C++ tree present alongside it.
    let proto_root = "./proto";

    // All proto files to compile.
    //   schedinfo.proto    — SchedInfoService (Pullpiri → Timpani-O) + FaultService
    //   node_service.proto — NodeService (Timpani-N → Timpani-O)
    let proto_files = [
        format!("{}/schedinfo.proto", proto_root),
        format!("{}/node_service.proto", proto_root),
    ];

    // Tell Cargo to re-run this build script when any proto file changes.
    for f in &proto_files {
        println!("cargo:rerun-if-changed={}", f);
    }

    let proto_refs: Vec<&str> = proto_files.iter().map(String::as_str).collect();

    tonic_build::configure()
        // Generate both server and client stubs for every service.
        // Servers: SchedInfoService, NodeService (Timpani-O serves these).
        // Client:  FaultService (Timpani-O calls Pullpiri).
        .build_server(true)
        .build_client(true)
        // Derive serde Serialize/Deserialize on every generated message so we can
        // (de)serialise them easily in tests and logging.
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &proto_refs,   // proto files to compile
            &[proto_root], // directories to search for imports
        )?;

    Ok(())
}
