/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! PiriCTL Library
//!
//! This library provides the core functionality for the pirictl tool,
//! which communicates with the Pullpiri SettingsService via REST APIs.

pub mod client;
pub mod commands;
pub mod error;

pub use client::SettingsClient;
pub use error::{CliError, Result};
