/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! node-sim — manual test simulator for multiple Timpani-N nodes.
//!
//! Spawns one Tokio task per node ID and exercises the full node protocol:
//!
//!   1. `GetSchedInfo(node_id)` — fetches and prints the assigned schedule
//!   2. `SyncTimer(node_id)`    — blocks until all nodes have called this,
//!      printing the shared start time on ACK
//!   3. `ReportDMiss(node_id, task)` — optional, triggered by `--dmiss`
//!
//! # Usage
//! ```text
//! # After pullpiri-sim has sent AddSchedInfo:
//! cargo run --bin node-sim -- \
//!     --nodes node01,node02,node03 \
//!     --dmiss node01:task_safety \
//!     --dmiss-delay-ms 2000
//! ```
//!
//! This concurrently fires the sync barrier so Timpani-O dispatches start_time
//! to all three nodes at once.

use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use tracing::{error, info, warn};

use timpani_o::proto::schedinfo_v1::{
    node_service_client::NodeServiceClient, DeadlineMissInfo, NodeSchedRequest, SyncRequest,
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "node-sim",
    about = "Timpani-N node simulator: GetSchedInfo → SyncTimer → ReportDMiss"
)]
struct Cli {
    /// Host where Timpani-O's node-facing service is listening.
    #[arg(long, default_value = "localhost")]
    node_host: String,

    /// Port for Timpani-O's node-facing service.
    #[arg(long, default_value_t = 50054)]
    node_port: u16,

    /// Comma-separated list of node IDs to simulate concurrently.
    /// All listed nodes will call SyncTimer at the same time to fire the
    /// barrier.  This list must match the nodes that received tasks from
    /// AddSchedInfo, otherwise the barrier will never release.
    #[arg(long, default_value = "node01,node02,node03")]
    nodes: String,

    /// Send a deadline-miss report from one node, given as "node_id:task_name".
    /// Example: --dmiss node01:task_safety
    #[arg(long)]
    dmiss: Option<String>,

    /// Milliseconds to wait after SyncTimer ACK before sending ReportDMiss.
    /// Simulates the task running for a while before missing its deadline.
    #[arg(long, default_value_t = 2000)]
    dmiss_delay_ms: u64,
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let node_ids: Vec<String> = cli
        .nodes
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if node_ids.is_empty() {
        anyhow::bail!("--nodes must contain at least one node ID");
    }

    let addr = format!("http://{}:{}", cli.node_host, cli.node_port);

    // Parse --dmiss "node_id:task_name"
    let dmiss_target: Option<(String, String)> = match &cli.dmiss {
        None => None,
        Some(s) => {
            let mut parts = s.splitn(2, ':');
            match (parts.next(), parts.next()) {
                (Some(n), Some(t)) if !n.is_empty() && !t.is_empty() => {
                    Some((n.to_string(), t.to_string()))
                }
                _ => {
                    anyhow::bail!("--dmiss format must be 'node_id:task_name', got: '{s}'");
                }
            }
        }
    };

    info!("Simulating {} node(s) against {addr}", node_ids.len());
    info!("Nodes: {:?}", node_ids);
    if let Some((n, t)) = &dmiss_target {
        info!(
            "After barrier: node '{}' will report deadline miss on task '{}' \
             after {} ms",
            n, t, cli.dmiss_delay_ms
        );
    }

    // ── Spawn one Tokio task per node concurrently ────────────────────────────
    let mut handles = Vec::with_capacity(node_ids.len());

    for node_id in &node_ids {
        let node_id = node_id.clone();
        let addr = addr.clone();
        let dmiss = dmiss_target.clone();
        let dmiss_delay = cli.dmiss_delay_ms;

        let handle = tokio::spawn(async move {
            if let Err(e) = simulate_node(&node_id, &addr, dmiss, dmiss_delay).await {
                error!("[{node_id}] simulation error: {e}");
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.await.ok();
    }

    info!("All node simulations complete.");
    Ok(())
}

// ── per-node protocol sequence ────────────────────────────────────────────────

async fn simulate_node(
    node_id: &str,
    addr: &str,
    dmiss: Option<(String, String)>,
    dmiss_delay_ms: u64,
) -> Result<()> {
    let mut client = NodeServiceClient::connect(addr.to_string())
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "[{node_id}] cannot connect to NodeService at {addr}: {e}\n\
                 → Is Timpani-O running and has AddSchedInfo been sent?",
                node_id = node_id,
                addr = addr
            )
        })?;

    info!("[{node_id}] connected to NodeService at {addr}");

    // ── 1. GetSchedInfo ───────────────────────────────────────────────────────
    info!("[{node_id}] → GetSchedInfo");

    let resp = client
        .get_sched_info(NodeSchedRequest {
            node_id: node_id.to_string(),
        })
        .await
        .map_err(|s| anyhow::anyhow!("[{node_id}] GetSchedInfo failed: {s}", node_id = node_id))?
        .into_inner();

    info!(
        "[{node_id}] ← GetSchedInfo: workload='{}' hyperperiod={}µs tasks={}",
        resp.workload_id,
        resp.hyperperiod_us,
        resp.tasks.len()
    );

    for t in &resp.tasks {
        let cpu = if t.cpu_affinity != 0 {
            t.cpu_affinity.trailing_zeros()
        } else {
            u32::MAX // "any"
        };
        let policy = match t.sched_policy {
            0 => "NORMAL",
            1 => "FIFO",
            2 => "RR",
            _ => "?",
        };
        let cpu_str = if cpu == u32::MAX {
            "any".to_string()
        } else {
            format!("CPU{cpu}")
        };
        info!(
            "[{node_id}]    ▸ name={:<20} {:<6} prio={:>3}  policy={:<6}  \
             period={:>7}µs  runtime={:>6}µs  deadline={:>7}µs  max_dmiss={}",
            t.name,
            cpu_str,
            t.sched_priority,
            policy,
            t.period_us,
            t.runtime_us,
            t.deadline_us,
            t.max_dmiss
        );
    }

    if resp.tasks.is_empty() {
        warn!(
            "[{node_id}] no tasks assigned to this node — \
             skipping SyncTimer (it would block the barrier)"
        );
        return Ok(());
    }

    // ── 2. SyncTimer — blocks until ALL nodes have called it ─────────────────
    info!("[{node_id}] → SyncTimer  (waiting for all nodes to synchronise…)");

    let sync = client
        .sync_timer(SyncRequest {
            node_id: node_id.to_string(),
        })
        .await
        .map_err(|s| anyhow::anyhow!("[{node_id}] SyncTimer failed: {s}", node_id = node_id))?
        .into_inner();

    if sync.ack {
        info!(
            "[{node_id}] ✅  Barrier released — start at {}.{:09}s (CLOCK_REALTIME)",
            sync.start_time_sec, sync.start_time_nsec
        );
    } else {
        warn!("[{node_id}] ⚠️  SyncTimer returned ack=false — aborting");
        return Ok(());
    }

    // ── 3. Optional deadline miss report ─────────────────────────────────────
    if let Some((dmiss_node, dmiss_task)) = &dmiss {
        if dmiss_node.as_str() == node_id {
            info!(
                "[{node_id}] ⏱  Simulating task run — waiting {dmiss_delay_ms} ms \
                 before reporting deadline miss on '{dmiss_task}'"
            );
            tokio::time::sleep(Duration::from_millis(dmiss_delay_ms)).await;

            info!("[{node_id}] → ReportDMiss  task='{dmiss_task}'");
            let dr = client
                .report_d_miss(DeadlineMissInfo {
                    node_id: node_id.to_string(),
                    task_name: dmiss_task.clone(),
                })
                .await
                .map_err(|s| {
                    anyhow::anyhow!("[{node_id}] ReportDMiss failed: {s}", node_id = node_id)
                })?
                .into_inner();

            if dr.status == 0 {
                info!(
                    "[{node_id}] ✅  ReportDMiss acknowledged — \
                       Timpani-O will forward to Pullpiri's FaultService"
                );
            } else {
                warn!(
                    "[{node_id}] ⚠️  ReportDMiss status={} msg='{}'",
                    dr.status, dr.error_message
                );
            }
        }
    }

    Ok(())
}
