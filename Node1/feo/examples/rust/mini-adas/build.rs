/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::env;
use std::path::PathBuf;

// Relative path to the feo repository root directory
static PATH_TO_REPO_ROOT: &str = "../../../";

fn main() {
    // Only apply lifecycle library linking when the feature is enabled
    #[cfg(feature = "lifecycle")]
    {
        let lib_dir = env::current_dir()
            .expect("Failed to get current directory")
            .join("lib");

        println!("cargo:rustc-link-search=native={}", lib_dir.display());

        // Link libraries with --no-as-needed to force inclusion of all dependencies
        // Order matters: dependencies should come before things that depend on them
        println!("cargo:rustc-link-arg=-Wl,--push-state,--no-as-needed");
        println!("cargo:rustc-link-arg=-Wl,-lcommon");
        println!("cargo:rustc-link-arg=-Wl,-losal");
        println!("cargo:rustc-link-arg=-Wl,-lidentifier_hash");
        println!("cargo:rustc-link-arg=-Wl,-lprocess_state_client");
        println!("cargo:rustc-link-arg=-Wl,-llifecycle_client");
        println!("cargo:rustc-link-arg=-Wl,-lphm_logging");
        println!("cargo:rustc-link-arg=-Wl,-ltimers");
        println!("cargo:rustc-link-arg=-Wl,-l:libhm-lib.a");
        println!("cargo:rustc-link-arg=-Wl,--pop-state");
        println!("cargo:rustc-link-lib=dylib=stdc++");

        // Set rpath so the binary can find the .so files at runtime
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../../../examples/rust/mini-adas/lib");
    }
}
