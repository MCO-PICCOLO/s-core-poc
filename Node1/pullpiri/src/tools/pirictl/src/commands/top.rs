/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Top command implementation

use crate::commands::format::{format_bytes, format_memory};
use crate::commands::{print_error, print_info};
use crate::{Result, SettingsClient};
use clap::Subcommand;
use colored::Colorize;

#[derive(Subcommand)]
pub enum TopResource {
    /// Display live system metrics
    Metrics,
}

/// Handle top commands
pub async fn handle(client: &SettingsClient, resource: TopResource) -> Result<()> {
    match resource {
        TopResource::Metrics => top_metrics(client).await,
    }
}

/// Display and continuously update formatted metrics
async fn top_metrics(client: &SettingsClient) -> Result<()> {
    print_info("Fetching system metrics...");

    // Fetch all resource metrics
    let boards_result = client.get("/api/v1/metrics/boards").await;
    let socs_result = client.get("/api/v1/metrics/socs").await;
    let nodes_result = client.get("/api/v1/metrics/nodes").await;

    // Collect metrics data
    let mut metrics_data = Vec::new();

    // Process boards
    if let Ok(boards) = boards_result {
        if let Some(array) = boards.as_array() {
            for board in array {
                metrics_data.push(MetricRow::from_board(board));
            }
        }
    }

    // Process SoCs
    if let Ok(socs) = socs_result {
        if let Some(array) = socs.as_array() {
            for soc in array {
                metrics_data.push(MetricRow::from_soc(soc));
            }
        }
    }

    // Process nodes
    if let Ok(nodes) = nodes_result {
        if let Some(array) = nodes.as_array() {
            for node in array {
                metrics_data.push(MetricRow::from_node(node));
            }
        }
    }

    if metrics_data.is_empty() {
        print_error("No metrics data available");
        return Ok(());
    }

    // Print table header
    println!();
    println!(
        "{:<6} {:<15} {:<3} {:<5} {:<19} {:<5} {:<3} {:<7} {:<17} {:<21}",
        "LEVEL".bold(),
        "NAME/ID".bold(),
        "CPU".bold(),
        "CPU%".bold(),
        "MEMORY".bold(),
        "MEM%".bold(),
        "GPU".bold(),
        "ARCH".bold(),
        "NET(RX/TX)".bold(),
        "DISK(R/W)".bold()
    );

    // Print each metric row
    for metric in metrics_data {
        println!(
            "{:<6} {:<15} {:<3} {:<5} {:<19} {:<5} {:<3} {:<7} {:<17} {:<21}",
            metric.level,
            metric.name,
            metric.cpu,
            metric.cpu_percent,
            metric.memory,
            metric.mem_percent,
            metric.gpu,
            metric.arch,
            metric.network,
            metric.disk
        );
    }

    println!();
    Ok(())
}

/// Struct to hold a single metric row
struct MetricRow {
    level: String,
    name: String,
    cpu: String,
    cpu_percent: String,
    memory: String,
    mem_percent: String,
    gpu: String,
    arch: String,
    network: String,
    disk: String,
}

impl MetricRow {
    fn from_board(board: &serde_json::Value) -> Self {
        let board_id = board
            .get("board_id")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let cpu_count = board
            .get("total_cpu_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cpu_usage = board
            .get("total_cpu_usage")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let total_memory = board
            .get("total_memory")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let used_memory = board
            .get("total_used_memory")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let mem_usage = board
            .get("total_mem_usage")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let gpu_count = board
            .get("total_gpu_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let network_rx = board
            .get("total_rx_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let network_tx = board
            .get("total_tx_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let disk_read = board
            .get("total_read_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let disk_write = board
            .get("total_write_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Self {
            level: "Board".to_string(),
            name: board_id,
            cpu: cpu_count.to_string(),
            cpu_percent: format!("{:.1}%", cpu_usage),
            memory: format!(
                "{} / {}",
                format_memory(used_memory),
                format_memory(total_memory)
            ),
            mem_percent: format!("{:.1}%", mem_usage),
            gpu: gpu_count.to_string(),
            arch: "-".to_string(),
            network: format!("{}/{}", format_bytes(network_rx), format_bytes(network_tx)),
            disk: format!("{}/{}", format_bytes(disk_read), format_bytes(disk_write)),
        }
    }

    fn from_soc(soc: &serde_json::Value) -> Self {
        let soc_id = soc
            .get("soc_id")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let cpu_count = soc
            .get("total_cpu_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cpu_usage = soc
            .get("total_cpu_usage")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let total_memory = soc
            .get("total_memory")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let used_memory = soc
            .get("total_used_memory")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let mem_usage = soc
            .get("total_mem_usage")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let gpu_count = soc
            .get("total_gpu_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let network_rx = soc
            .get("total_rx_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let network_tx = soc
            .get("total_tx_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let disk_read = soc
            .get("total_read_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let disk_write = soc
            .get("total_write_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Self {
            level: "SoC".to_string(),
            name: soc_id,
            cpu: cpu_count.to_string(),
            cpu_percent: format!("{:.1}%", cpu_usage),
            memory: format!(
                "{} / {}",
                format_memory(used_memory),
                format_memory(total_memory)
            ),
            mem_percent: format!("{:.1}%", mem_usage),
            gpu: gpu_count.to_string(),
            arch: "-".to_string(),
            network: format!("{}/{}", format_bytes(network_rx), format_bytes(network_tx)),
            disk: format!("{}/{}", format_bytes(disk_read), format_bytes(disk_write)),
        }
    }

    fn from_node(node: &serde_json::Value) -> Self {
        let node_name = node
            .get("node_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let cpu_count = node
            .get("cpu_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cpu_usage = node
            .get("cpu_usage")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let total_memory = node
            .get("total_memory")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let used_memory = node
            .get("used_memory")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let mem_usage = node
            .get("mem_usage")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let gpu_count = node.get("gpu_count").and_then(|v| v.as_u64()).unwrap_or(0);

        let arch = node
            .get("arch")
            .and_then(|v| v.as_str())
            .unwrap_or("-")
            .to_string();

        let network_rx = node
            .get("rx_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let network_tx = node
            .get("tx_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let disk_read = node
            .get("read_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let disk_write = node
            .get("write_bytes")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Self {
            level: "Node".to_string(),
            name: node_name,
            cpu: cpu_count.to_string(),
            cpu_percent: format!("{:.1}%", cpu_usage),
            memory: format!(
                "{} / {}",
                format_memory(used_memory),
                format_memory(total_memory)
            ),
            mem_percent: format!("{:.1}%", mem_usage),
            gpu: gpu_count.to_string(),
            arch,
            network: format!("{}/{}", format_bytes(network_rx), format_bytes(network_tx)),
            disk: format!("{}/{}", format_bytes(disk_read), format_bytes(disk_write)),
        }
    }
}
