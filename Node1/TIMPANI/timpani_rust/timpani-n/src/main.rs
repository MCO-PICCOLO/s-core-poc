/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

use timpani_n::{
    config::{exit_codes, Config},
    init_logging, run_app,
};
use tracing::error;

fn main() -> anyhow::Result<()> {
    // Parse configuration from command-line arguments
    let config = match Config::from_args() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(exit_codes::FAILURE);
        }
    };

    // Initialize tracing/logging with the configured log level
    init_logging(config.log_level);

    // Run the main application logic
    if let Err(e) = run_app(config) {
        error!("Application error: {}", e);
        std::process::exit(exit_codes::FAILURE);
    }

    Ok(())
}
