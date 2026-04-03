/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Liveness probe executor for NodeAgent.
//!
//! This module provides `check_liveness_probe`, which dispatches to the appropriate
//! probe checker (HTTP, TCP, or Exec) based on the probe type configured in `LivenessProbe`.

use crate::desired_state::{LivenessProbe, ProbeType};

/// Execute a liveness probe for the given container and return whether it succeeded.
///
/// Dispatches to the appropriate checker based on `probe.probe_type`:
/// - `ProbeType::Http` → HTTP GET to container's target IP (host network: localhost, bridge: container IP)
/// - `ProbeType::Tcp`  → TCP connection attempt to container's target IP
/// - `ProbeType::Exec` → `podman exec <container_id> <command>`
pub async fn check_liveness_probe(container_id: &str, probe: &LivenessProbe) -> bool {
    match &probe.probe_type {
        ProbeType::Http { path, port } => {
            println!(
                "[Probe] Checking liveness probe for container {}: HTTP GET {} on port {}",
                container_id, path, port
            );
            super::checker::check_http(container_id, path, *port, probe.timeout_seconds).await
        }
        ProbeType::Tcp { port } => {
            println!(
                "[Probe] Checking liveness probe for container {}: TCP on port {}",
                container_id, port
            );
            super::checker::check_tcp(container_id, *port, probe.timeout_seconds).await
        }
        ProbeType::Exec { command } => {
            println!(
                "[Probe] Checking liveness probe for container {}: Exec {:?}",
                container_id, command
            );
            super::checker::check_exec(container_id, command, probe.timeout_seconds).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desired_state::{LivenessProbe, ProbeType};

    #[tokio::test]
    async fn test_check_liveness_probe_http_closed_port_returns_false() {
        let probe = LivenessProbe {
            probe_type: ProbeType::Http {
                path: "/healthz".to_string(),
                port: 19998,
            },
            initial_delay_seconds: 0,
            period_seconds: 10,
            timeout_seconds: 1,
            failure_threshold: 3,
        };
        let result = check_liveness_probe("test-container", &probe).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_liveness_probe_tcp_closed_port_returns_false() {
        let probe = LivenessProbe {
            probe_type: ProbeType::Tcp { port: 19997 },
            initial_delay_seconds: 0,
            period_seconds: 10,
            timeout_seconds: 1,
            failure_threshold: 3,
        };
        let result = check_liveness_probe("test-container", &probe).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_liveness_probe_exec_nonexistent_container_returns_false() {
        let probe = LivenessProbe {
            probe_type: ProbeType::Exec {
                command: vec!["true".to_string()],
            },
            initial_delay_seconds: 0,
            period_seconds: 10,
            timeout_seconds: 5,
            failure_threshold: 3,
        };
        let result = check_liveness_probe("nonexistent-container-xyz", &probe).await;
        assert!(!result);
    }
}
