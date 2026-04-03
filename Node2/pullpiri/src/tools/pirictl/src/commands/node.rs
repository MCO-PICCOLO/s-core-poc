/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Node command implementation

use crate::commands::format::{format_bytes, format_memory};
use crate::commands::{print_error, print_info, print_json, print_success, print_table_header};
use crate::{Result, SettingsClient};
use clap::Subcommand;
use colored::Colorize;

#[derive(Subcommand)]
pub enum NodeAction {
    /// Get all nodes
    Get,
    /// Describe specific node by ID
    Describe {
        /// Node ID
        id: String,
    },
    /// Get node information in raw JSON format
    Raw {
        /// Node ID (optional)
        id: Option<String>,
    },
}

/// Handle node commands
pub async fn handle(client: &SettingsClient, action: NodeAction) -> Result<()> {
    match action {
        NodeAction::Get => get_nodes(client).await,
        NodeAction::Describe { id } => describe_node(client, &id).await,
        NodeAction::Raw { id } => {
            if let Some(node_id) = id {
                get_node_raw(client, &node_id).await
            } else {
                get_nodes_raw(client).await
            }
        }
    }
}

/// Get all nodes
async fn get_nodes(client: &SettingsClient) -> Result<()> {
    print_info("Fetching nodes list...");

    match client.get("/api/v1/nodes").await {
        Ok(nodes) => {
            print_table_header("Nodes", &[
                ("NAME", 24),
                ("IP", 18),
                ("OS", 22),
                ("ARCH", 10),
            ]);

            // Look for "nodes" array in the response
            if let Some(nodes_array) = nodes.get("nodes").and_then(|n| n.as_array()) {
                if nodes_array.is_empty() {
                    println!("No nodes found.");
                } else {
                    // Print each node
                    for node in nodes_array.iter() {
                        let name = node.get("node_name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                        let ip = node.get("ip").and_then(|i| i.as_str()).unwrap_or("N/A");
                        let os = node.get("os").and_then(|o| o.as_str()).unwrap_or("Unknown");
                        let arch = node.get("arch").and_then(|a| a.as_str()).unwrap_or("Unknown");

                        println!(
                            "{:<24} {:<18} {:<22} {:<10}",
                            name, ip, os, arch
                        );
                    }
                }
            } else if let Some(name) = nodes.get("node_name") {
                // Single node response
                let ip = nodes.get("ip").and_then(|i| i.as_str()).unwrap_or("N/A");
                let os = nodes.get("os").and_then(|o| o.as_str()).unwrap_or("Unknown");
                let arch = nodes.get("arch").and_then(|a| a.as_str()).unwrap_or("Unknown");
                println!(
                    "{:<24} {:<18} {:<22} {:<10}",
                    name.as_str().unwrap_or("Unknown"), ip, os, arch
                );
            } else {
                println!("No nodes found.");
            }

            println!();
            print_success("Nodes list retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch nodes: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Describe specific node information
async fn describe_node(client: &SettingsClient, node_id: &str) -> Result<()> {
    print_info(&format!("Fetching node information for ID: {}", node_id));

    let endpoint = format!("/api/v1/nodes/{}", node_id);
    match client.get(&endpoint).await {
        Ok(node) => {
            // Node name
            let node_name = node
                .get("node_name")
                .and_then(|n| n.as_str())
                .unwrap_or(node_id);
            println!("\n{:<24}{}", format!("{}:", "Name".bold()), node_name);

            // System Info
            println!("{}", "System Info:".bold());
            if let Some(os) = node.get("os").and_then(|o| o.as_str()) {
                println!("  {:<22}{}", "OS Image:", os);
            }
            if let Some(arch) = node.get("arch").and_then(|a| a.as_str()) {
                println!("  {:<22}{}", "Architecture:", arch);
            }
            // Container runtime is not provided by API, using default
            println!("  {:<22}Podman", "Container Runtime:");
            if let Some(ip) = node.get("ip").and_then(|i| i.as_str()) {
                println!("  {:<22}{}", "Internal IP:", ip);
            }

            // Capacity
            println!("{}", "Capacity:".bold());
            if let Some(cpu_count) = node.get("cpu_count").and_then(|c| c.as_u64()) {
                println!("  {:<22}{}", "cpu:", cpu_count);
            }
            if let Some(gpu_count) = node.get("gpu_count").and_then(|g| g.as_u64()) {
                println!("  {:<22}{}", "gpu:", gpu_count);
            }
            if let Some(total_memory) = node.get("total_memory").and_then(|m| m.as_u64()) {
                println!("  {:<22}{}", "memory:", format_memory(total_memory));
            }

            // Allocatable (Current Usage)
            println!("{}", "Allocatable:".bold());
            if let (Some(cpu_usage), Some(cpu_count)) = (
                node.get("cpu_usage").and_then(|u| u.as_f64()),
                node.get("cpu_count").and_then(|c| c.as_u64()),
            ) {
                println!("  {:<22}{} ({:.2}% used)", "cpu:", cpu_count, cpu_usage);
            }
            if let (Some(_used_memory), Some(total_memory), Some(mem_usage)) = (
                node.get("used_memory").and_then(|m| m.as_u64()),
                node.get("total_memory").and_then(|m| m.as_u64()),
                node.get("mem_usage").and_then(|u| u.as_f64()),
            ) {
                println!("  {:<22}{} ({:.2}% used)", "memory:", format_memory(total_memory), mem_usage);
            }

            // Network I/O
            println!("{}", "Network I/O:".bold());
            if let Some(rx_bytes) = node.get("rx_bytes").and_then(|r| r.as_u64()) {
                println!("  {:<22}{}", "RX:", format_bytes(rx_bytes));
            }
            if let Some(tx_bytes) = node.get("tx_bytes").and_then(|t| t.as_u64()) {
                println!("  {:<22}{}", "TX:", format_bytes(tx_bytes));
            }

            // Disk I/O
            println!("{}", "Disk I/O:".bold());
            if let Some(read_bytes) = node.get("read_bytes").and_then(|r| r.as_u64()) {
                println!("  {:<22}{}", "Read:", format_bytes(read_bytes));
            }
            if let Some(write_bytes) = node.get("write_bytes").and_then(|w| w.as_u64()) {
                println!("  {:<22}{}", "Write:", format_bytes(write_bytes));
            }

            print_success("Node information retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch node {}: {}", node_id, e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get nodes in raw JSON format
async fn get_nodes_raw(client: &SettingsClient) -> Result<()> {
    print_info("Fetching raw nodes data...");

    match client.get("/api/v1/nodes").await {
        Ok(nodes) => {
            print_json(&nodes)?;
            print_success("Raw nodes data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to fetch raw nodes data: {}", e));
            return Err(e);
        }
    }

    Ok(())
}

/// Get specific node in raw JSON format
async fn get_node_raw(client: &SettingsClient, node_id: &str) -> Result<()> {
    print_info(&format!("Fetching raw node data for ID: {}", node_id));

    let endpoint = format!("/api/v1/nodes/{}", node_id);
    match client.get(&endpoint).await {
        Ok(node) => {
            print_json(&node)?;
            print_success("Raw node data retrieved successfully");
        }
        Err(e) => {
            print_error(&format!(
                "Failed to fetch raw node data for {}: {}",
                node_id, e
            ));
            return Err(e);
        }
    }

    Ok(())
}
