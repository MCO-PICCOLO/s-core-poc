/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Metrics command implementation

use crate::commands::{print_error, print_json, print_success};
use crate::{Result, SettingsClient};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum MetricsAction {
    /// Get metrics in raw JSON format
    Raw,
}

/// Handle metrics commands
pub async fn handle(client: &SettingsClient, action: MetricsAction) -> Result<()> {
    match action {
        MetricsAction::Raw => get_metrics_raw(client).await,
    }
}

/// Get and display raw JSON metrics
async fn get_metrics_raw(client: &SettingsClient) -> Result<()> {
    print_success("Fetching raw metrics...");

    match client.get("/api/v1/metrics").await {
        Ok(metrics) => {
            print_json(&metrics)?;
            print_success("Raw metrics retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch raw metrics: {}", e));
            return Err(e);
        }
    }

    Ok(())
}
