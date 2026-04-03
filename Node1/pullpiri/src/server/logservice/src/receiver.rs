// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Unix socket receiver that decodes `LogEnvelope`s and multicasts the
//! formatted lines to stdout plus an in-process broadcast channel.

use crate::{log_entry::LogEvent, LOG_HISTORY_CAPACITY};
use chrono::{DateTime, Local};
use common::logd::LogEnvelope;
use prost::Message;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration as StdDuration, UNIX_EPOCH};
use tokio::net::UnixDatagram;
use tokio::sync::{broadcast, Mutex};

/// Create (or recreate) a Unix datagram socket at `path` and bind it.
///
/// # Errors
/// Returns underlying I/O errors when preparing the socket file or binding.
pub fn bind_sock(path: &str) -> std::io::Result<UnixDatagram> {
    let p = Path::new(path);
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir)?;
    }
    /*if p.exists() {
        let _ = fs::remove_file(p);
    }*/
    UnixDatagram::bind(path)
}

/// Remove the socket file if it exists so the next run can bind cleanly.
pub fn cleanup_socket(path: &str) {
    if Path::new(path).exists() {
        let _ = fs::remove_file(path);
    }
}

/// Receive datagrams, decode them, and fan them out to stdout, history buffer, and broadcast.
pub async fn run(
    sock: UnixDatagram,
    log_tx: broadcast::Sender<LogEvent>,
    log_history: Arc<Mutex<VecDeque<LogEvent>>>,
) {
    let mut buf = vec![0u8; 8192];
    loop {
        let n = match sock.recv(&mut buf).await {
            Ok(n) => n,
            Err(_) => continue,
        };
        let data = &buf[..n];

        let env = match LogEnvelope::decode(data) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let sys_time = UNIX_EPOCH + StdDuration::from_nanos(env.ts_real_ns);
        let chrono_time: DateTime<Local> = DateTime::from(sys_time);
        let time_str = chrono_time.format("%Y-%m-%d %H:%M:%S%.3f");

        let level = match env.level {
            1 => "V",
            2 => "D",
            3 => "I",
            4 => "W",
            5 => "E",
            6 => "F",
            _ => "?",
        };

        let entry = LogEvent {
            timestamp: time_str.to_string(),
            level: level.to_string(),
            tag: env.tag.clone(),
            message: env.message.clone(),
        };

        {
            let mut history = log_history.lock().await;
            history.push_back(entry.clone());
            if history.len() > LOG_HISTORY_CAPACITY {
                history.pop_front();
            }
        }

        println!(
            "{:<24} │ {:<2} │ {:<30} │ {}",
            entry.timestamp, entry.level, entry.tag, entry.message
        );

        let _ = log_tx.send(entry);
    }
}
