/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Command implementations for pirictl

pub mod board;
pub mod container;
pub mod format;
pub mod metrics;
pub mod node;
pub mod soc;
pub mod top;
pub mod yaml;

use crate::Result;
use colored::Colorize;
use serde_json::Value;

/// Helper function to pretty print JSON output
pub fn print_json(value: &Value) -> Result<()> {
    let pretty = serde_json::to_string_pretty(value)?;
    println!("{}", pretty);
    Ok(())
}

/// Helper function to print success messages
pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Helper function to print error messages
pub fn print_error(message: &str) {
    println!("{} {}", "✗".red().bold(), message);
}

/// Helper function to print info messages
pub fn print_info(message: &str) {
    println!("{} {}", "ℹ".blue().bold(), message);
}

/// Print table header with title and columns
/// 
/// # Arguments
/// * `title` - Table title (e.g., "Boards", "Nodes")
/// * `columns` - Array of (column_name, width) tuples
pub fn print_table_header(_title: &str, columns: &[(&str, usize)]) {
    // Print column headers only (kubectl style - simple and clean)
    println!();
    for (name, width) in columns {
        print!("{:<width$} ", name, width = width);
    }
    println!();
}
