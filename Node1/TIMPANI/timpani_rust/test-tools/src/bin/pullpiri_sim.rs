/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! pullpiri-sim — manual test simulator for Pullpiri's two roles:
//!
//! **Role 1 — Client**: reads a workload YAML, sends `AddSchedInfo` to
//! Timpani-O's upstream gRPC port.
//!
//! **Role 2 — Server**: serves `FaultService` so Timpani-O can forward
//! deadline-miss notifications here.
//!
//! # Usage
//! ```text
//! # Terminal 1: start Timpani-O first
//! cargo run --bin timpani-o -- --nodeconfig ../../timpani-o/examples/node_configurations.yaml
//!
//! # Terminal 2: start pullpiri-sim
//! cargo run --bin pullpiri-sim -- --workload test-tools/workloads/example_workload.yaml
//!
//! # Terminal 3: start node-sim (fires the sync barrier, then sends a deadline miss)
//! cargo run --bin node-sim -- --nodes node01,node02,node03 --dmiss node01:task_safety
//! ```
//!
//! pullpiri-sim will then print:
//!   ✅  AddSchedInfo succeeded
//!   🔔  FAULT: workload=test_workload node=node01 task=task_safety type=DMISS

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing::{error, info};

use timpani_o::proto::schedinfo_v1::{
    fault_service_server::{FaultService, FaultServiceServer},
    sched_info_service_client::SchedInfoServiceClient,
    FaultInfo, Response as ProtoResponse, SchedInfo,
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "pullpiri-sim",
    about = "Pullpiri simulator: sends AddSchedInfo and receives fault notifications"
)]
struct Cli {
    /// Host where Timpani-O SchedInfoService is listening.
    #[arg(long, default_value = "localhost")]
    sinfo_host: String,

    /// Port for Timpani-O's upstream SchedInfoService.
    #[arg(long, default_value_t = 50052)]
    sinfo_port: u16,

    /// Port this simulator listens on for Timpani-O's fault notifications.
    /// Must match --faultport used when starting timpani-o.
    #[arg(long, default_value_t = 50053)]
    fault_port: u16,

    /// Path to the workload YAML file to send.
    #[arg(long, short = 'w')]
    workload: PathBuf,
}

// ── FaultService implementation ───────────────────────────────────────────────

/// Receives `NotifyFault` calls from Timpani-O and logs them.
struct PullpiriFaultService;

#[tonic::async_trait]
impl FaultService for PullpiriFaultService {
    async fn notify_fault(
        &self,
        request: Request<FaultInfo>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let info = request.into_inner();
        let fault_type = match info.r#type {
            0 => "UNKNOWN",
            1 => "DMISS",
            _ => "INVALID",
        };
        // Use eprintln directly so the notification stands out regardless of
        // log level.
        eprintln!(
            "\n🔔  FAULT NOTIFICATION\n\
             \tworkload  : {}\n\
             \tnode      : {}\n\
             \ttask      : {}\n\
             \ttype      : {}\n",
            info.workload_id, info.node_id, info.task_name, fault_type
        );
        info!(
            workload = %info.workload_id,
            node     = %info.node_id,
            task     = %info.task_name,
            fault    = %fault_type,
            "FaultService: NotifyFault received"
        );
        Ok(Response::new(ProtoResponse { status: 0 }))
    }
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

    // ── Step 1: start the FaultService server ─────────────────────────────────
    let fault_addr: std::net::SocketAddr = format!("0.0.0.0:{}", cli.fault_port).parse()?;

    info!("Starting FaultService on {fault_addr}");

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let fault_server = {
        let mut rx = shutdown_rx.clone();
        Server::builder()
            .add_service(FaultServiceServer::new(PullpiriFaultService))
            .serve_with_shutdown(fault_addr, async move {
                while !*rx.borrow() {
                    rx.changed().await.ok();
                }
            })
    };
    tokio::spawn(fault_server);

    // Brief pause to let the fault server port become available.
    tokio::time::sleep(Duration::from_millis(200)).await;
    info!(
        "FaultService ready — Timpani-O can now send fault notifications to :{}",
        cli.fault_port
    );

    // ── Step 2: read workload YAML ────────────────────────────────────────────
    info!("Reading workload from: {}", cli.workload.display());
    let file = std::fs::File::open(&cli.workload)
        .map_err(|e| anyhow::anyhow!("cannot open workload file: {e}"))?;
    let sched_info: SchedInfo = serde_yaml::from_reader(file)
        .map_err(|e| anyhow::anyhow!("failed to parse workload YAML: {e}"))?;

    info!(
        workload_id = %sched_info.workload_id,
        task_count  = sched_info.tasks.len(),
        "Workload loaded"
    );
    for (i, t) in sched_info.tasks.iter().enumerate() {
        let policy_str = match t.policy {
            0 => "NORMAL",
            1 => "FIFO",
            2 => "RR",
            _ => "?",
        };
        info!(
            "  [{i}] name={} node={} prio={} policy={} period={}us runtime={}us",
            t.name, t.node_id, t.priority, policy_str, t.period, t.runtime
        );
    }

    // ── Step 3: send AddSchedInfo to Timpani-O ────────────────────────────────
    let sinfo_addr = format!("http://{}:{}", cli.sinfo_host, cli.sinfo_port);
    info!("Connecting to Timpani-O SchedInfoService at {sinfo_addr}");

    let mut client = SchedInfoServiceClient::connect(sinfo_addr)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "cannot connect to Timpani-O: {e}\n\
            → Is Timpani-O running on {}:{}?",
                cli.sinfo_host,
                cli.sinfo_port
            )
        })?;

    let response = client
        .add_sched_info(Request::new(sched_info))
        .await
        .map_err(|s| anyhow::anyhow!("AddSchedInfo RPC failed: {s}"))?
        .into_inner();

    if response.status == 0 {
        info!("✅  AddSchedInfo succeeded (status=0)");
        info!("Timing-O should now be waiting for all nodes to call SyncTimer.");
        info!("→ Start node-sim in another terminal.");
    } else {
        error!("❌  AddSchedInfo failed (status={})", response.status);
    }

    // ── Step 4: keep running to receive fault notifications ───────────────────
    info!("Waiting for fault notifications from Timpani-O (Ctrl-C to stop)...");

    tokio::signal::ctrl_c().await?;

    info!("Shutdown signal received — stopping pullpiri-sim");
    let _ = shutdown_tx.send(true);

    Ok(())
}
