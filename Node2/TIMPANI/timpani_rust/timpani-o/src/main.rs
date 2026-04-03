/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use clap::Parser;
use tonic::transport::Server;
use tracing::{error, info, warn};

use timpani_o::config::NodeConfigManager;
use timpani_o::fault::{FaultClient, FaultNotification};
use timpani_o::grpc::{
    new_workload_store,
    node_service::{NodeServiceImpl, DEFAULT_SYNC_TIMEOUT_SECS},
    schedinfo_service::SchedInfoServiceImpl,
};
use timpani_o::proto::schedinfo_v1::{
    node_service_server::NodeServiceServer, sched_info_service_server::SchedInfoServiceServer,
    FaultType,
};

// ── CLI argument definition ───────────────────────────────────────────────────

/// Timpani-O global scheduler (Rust implementation).
///
/// Example:
///   timpani-o -s 50052 -f localhost -p 50053 -d 50054 \
///             --nodeconfig examples/node_configurations.yaml
#[derive(Debug, Parser)]
#[command(
    name = "timpani-o",
    about = "Timpani-O global scheduler – Rust implementation",
    long_about = None,
)]
struct Cli {
    /// Port for the upstream SchedInfoService gRPC server (receives workloads from Pullpiri).
    #[arg(short = 's', long = "sinfoport", default_value_t = 50052)]
    sinfo_port: u16,

    /// FaultService host address (Pullpiri gRPC endpoint).
    #[arg(short = 'f', long = "faulthost", default_value = "localhost")]
    fault_host: String,

    /// Port for the FaultService gRPC client (Pullpiri endpoint).
    #[arg(short = 'p', long = "faultport", default_value_t = 50053)]
    fault_port: u16,

    /// Port for the downstream node gRPC service (Timpani-N endpoint).
    #[arg(short = 'd', long = "nodeport", default_value_t = 50054)]
    node_port: u16,

    /// Enable the NotifyFault demo (sends one fault notification then clears).
    #[arg(short = 'n', long = "notifyfault", default_value_t = false)]
    notify_fault: bool,

    /// Timeout (seconds) for the SyncTimer barrier.
    ///
    /// If not all active nodes call SyncTimer within this window, the barrier
    /// is cancelled and all waiting nodes receive DEADLINE_EXCEEDED.  Set to 0
    /// to use the built-in default of 30 seconds.
    #[arg(short = 't', long = "sync-timeout-secs", default_value_t = DEFAULT_SYNC_TIMEOUT_SECS)]
    sync_timeout_secs: u64,

    /// Path to the YAML node configuration file.
    #[arg(short = 'c', long = "nodeconfig")]
    node_config: Option<PathBuf>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // Initialise structured logging.
    // Level is controlled by the RUST_LOG env-var (e.g. RUST_LOG=debug).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug")),
        )
        .init();

    info!("Timpani-O starting up...");

    // ── Parse CLI arguments ───────────────────────────────────────────────────
    let cli = Cli::parse();

    info!(
        sinfo_port        = cli.sinfo_port,
        fault_host        = %cli.fault_host,
        fault_port        = cli.fault_port,
        node_port         = cli.node_port,
        notify_fault      = cli.notify_fault,
        sync_timeout_secs = cli.sync_timeout_secs,
        node_config       = ?cli.node_config,
        "Configuration"
    );

    // ── Load node configuration ───────────────────────────────────────────────
    let mut node_config_manager = NodeConfigManager::new();

    match &cli.node_config {
        Some(path) => {
            info!("Loading node configuration from: {}", path.display());
            if let Err(e) = node_config_manager.load_from_file(path) {
                error!("Failed to load node configuration: {:#}", e);
                process::exit(1);
            }
        }
        None => {
            warn!("No node configuration file provided, using default node settings");
        }
    }

    // ── Print loaded nodes ────────────────────────────────────────────────────
    if node_config_manager.is_loaded() {
        let nodes = node_config_manager.get_all_nodes();
        info!("Loaded {} node(s):", nodes.len());
        // Sort by name for deterministic output
        let mut sorted: Vec<_> = nodes.values().collect();
        sorted.sort_by_key(|n| &n.name);
        for node in sorted {
            info!(
                "  [{name}]  cpus={cpus:?}  memory={mem}MB  arch={arch}  location={loc}",
                name = node.name,
                cpus = node.available_cpus,
                mem = node.max_memory_mb,
                arch = node.architecture,
                loc = node.location,
            );
        }
    }

    // ── Shared state ──────────────────────────────────────────────────────────
    let node_config_manager = Arc::new(node_config_manager);
    let workload_store = new_workload_store();

    // ── Fault client (lazy — connects to Pullpiri on first RPC call) ──────────
    let pullpiri_addr = format!("http://{}:{}", cli.fault_host, cli.fault_port);
    let fault_notifier = match FaultClient::connect_lazy(pullpiri_addr.clone()) {
        Ok(n) => n,
        Err(e) => {
            error!("Failed to build FaultClient for {pullpiri_addr}: {e}");
            process::exit(1);
        }
    };
    info!(addr = %pullpiri_addr, "FaultClient ready (lazy connect)");

    // ── gRPC service instances ────────────────────────────────────────────────
    let sched_info_svc = SchedInfoServiceImpl::new(
        Arc::clone(&node_config_manager),
        Arc::clone(&workload_store),
        Arc::clone(&fault_notifier),
    );
    let node_svc = NodeServiceImpl::new(
        Arc::clone(&workload_store),
        Arc::clone(&fault_notifier),
        std::time::Duration::from_secs(cli.sync_timeout_secs),
    );

    // ── Server addresses ──────────────────────────────────────────────────────
    let sinfo_addr = format!("0.0.0.0:{}", cli.sinfo_port)
        .parse()
        .expect("invalid sinfo_port");
    let node_addr = format!("0.0.0.0:{}", cli.node_port)
        .parse()
        .expect("invalid node_port");

    info!(addr = %sinfo_addr, "SchedInfoService starting (upstream — Pullpiri)");
    info!(addr = %node_addr,  "NodeService starting      (downstream — Timpani-N)");

    // ── Graceful shutdown — shared watch channel ──────────────────────────────
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let mut shutdown_rx_node = shutdown_rx.clone();

    // Signal handler task: watches for Ctrl-C or SIGTERM.
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv()          => {}
            }
        }
        #[cfg(not(unix))]
        tokio::signal::ctrl_c().await.ok();

        info!("Shutdown signal received — stopping servers");
        let _ = shutdown_tx.send(true);
    });

    // Shutdown futures: each server gets its own receiver clone.
    let sinfo_shutdown = {
        let mut rx = shutdown_rx.clone();
        async move {
            while !*rx.borrow() {
                rx.changed().await.ok();
            }
        }
    };
    let node_shutdown = async move {
        while !*shutdown_rx_node.borrow() {
            shutdown_rx_node.changed().await.ok();
        }
    };

    // ── Optional NotifyFault demo ─────────────────────────────────────────────
    //
    // Matches C++ NotifyFaultDemo(): sends one synthetic fault to Pullpiri after
    // a short startup delay to verify the FaultService connection is reachable.
    // Useful when you want to confirm the Pullpiri side is listening without
    // needing a real deadline miss event.
    if cli.notify_fault {
        let notifier = Arc::clone(&fault_notifier);
        tokio::spawn(async move {
            // Give the servers a moment to bind before attempting the outbound call.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            info!("--notifyfault: sending synthetic fault notification to Pullpiri");
            let result = notifier
                .notify_fault(FaultNotification {
                    workload_id: "workload_demo".into(),
                    node_id: "node_demo".into(),
                    task_name: "task_demo".into(),
                    fault_type: FaultType::Dmiss,
                })
                .await;
            match result {
                Ok(()) => info!("--notifyfault: synthetic fault delivered successfully"),
                Err(e) => warn!("--notifyfault: fault notification failed: {e}"),
            }
        });
    }

    // ── Start both servers concurrently ──────────────────────────────────────
    let sinfo_server = Server::builder()
        .add_service(SchedInfoServiceServer::new(sched_info_svc))
        .serve_with_shutdown(sinfo_addr, sinfo_shutdown);

    let node_server = Server::builder()
        .add_service(NodeServiceServer::new(node_svc))
        .serve_with_shutdown(node_addr, node_shutdown);

    match tokio::try_join!(sinfo_server, node_server) {
        Ok(_) => info!("Servers stopped cleanly"),
        Err(e) => {
            error!("Server error: {e}");
            process::exit(1);
        }
    }
}
