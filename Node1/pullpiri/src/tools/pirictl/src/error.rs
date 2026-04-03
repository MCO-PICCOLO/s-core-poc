/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Error handling for pirictl

use std::fmt;

/// Custom error type for CLI operations
#[derive(Debug)]
pub enum CliError {
    /// HTTP client errors
    Http(reqwest::Error),
    /// JSON parsing errors
    Json(serde_json::Error),
    /// IO errors
    Io(std::io::Error),
    /// Custom error messages
    Custom(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Http(e) => write!(f, "HTTP error: {}", e),
            CliError::Json(e) => write!(f, "JSON error: {}", e),
            CliError::Io(e) => write!(f, "IO error: {}", e),
            CliError::Custom(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for CliError {}

impl From<reqwest::Error> for CliError {
    fn from(err: reqwest::Error) -> Self {
        CliError::Http(err)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::Json(err)
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::Io(err)
    }
}

/// Result type for CLI operations
pub type Result<T> = std::result::Result<T, CliError>;
