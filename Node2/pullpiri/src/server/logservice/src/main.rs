// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! A Tokio-based log aggregator. It receives protobuf
//! logs from a Unix datagram socket and streams them to
//! stdout, while simultaneously opening SSE-based HTTP
//! endpoints (/, /logs) so that the same log stream
//! can be viewed in real time in a browser.

mod log_entry;
mod receiver;
mod web;

use std::{collections::VecDeque, sync::Arc};

use log_entry::LogEvent;
use receiver::{bind_sock, cleanup_socket, run as run_receiver};
use tokio::signal;
use tokio::sync::{broadcast, Mutex};
use web::{default_http_addr, run_http_server, WebState};

const BROADCAST_CAPACITY: usize = 1024;
pub const LOG_HISTORY_CAPACITY: usize = 2000;

/// Entry point: Open a Unix socket and run the receiving task
/// and HTTP server task in parallel. Pressing Ctrl+C stops
/// both tasks and cleans up the socket file.
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let logd_path = common::logd::LOGD_SOCKET_PATH;
    let logd = bind_sock(logd_path)?;
    println!("[aggregator] sockets ready");

    let (log_tx, _) = broadcast::channel::<LogEvent>(BROADCAST_CAPACITY);
    let log_history = Arc::new(Mutex::new(VecDeque::with_capacity(LOG_HISTORY_CAPACITY)));
    let web_state = WebState {
        log_tx: log_tx.clone(),
        log_history: log_history.clone(),
    };

    let mut recv_task = tokio::spawn(run_receiver(logd, log_tx, log_history));
    let mut http_task = tokio::spawn(run_http_server(web_state, default_http_addr()));

    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("[aggregator] shutting down");
        }
        res = &mut recv_task => {
            if let Err(err) = res {
                eprintln!("[aggregator] receiver task failed: {err}");
            }
        }
        res = &mut http_task => {
            if let Err(err) = res {
                eprintln!("[aggregator] http server task failed: {err}");
            }
        }
    }

    if !recv_task.is_finished() {
        recv_task.abort();
    }
    if !http_task.is_finished() {
        http_task.abort();
    }

    let _ = recv_task.await;
    let _ = http_task.await;
    cleanup_socket(logd_path);

    Ok(())
}
