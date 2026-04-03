/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! DesiredState structures for NodeAgent Self-Healing
//!
//! This module defines the `DesiredState` struct and related types used by the
//! NodeAgent to track the desired state of workloads for self-healing purposes.
//! The desired state is stored in an in-memory cache (`Arc<Mutex<HashMap<String, DesiredState>>>`)
//! and is populated when workloads are started and removed when workloads are stopped.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Represents the desired state of a workload (pod/container).
///
/// This is the reference state for self-healing: the NodeAgent compares the
/// actual running state against this desired state and takes corrective action.
///
/// # Memory Usage
/// Each `DesiredState` instance is approximately 331 bytes, making the in-memory
/// cache suitable for hundreds of containers without significant overhead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesiredState {
    /// Name of the pod, used as the HashMap key and for container identification.
    pub pod_name: String,
    /// Podman container ID, set after the container is successfully started.
    /// Used for Podman API calls (inspect, restart, stop, etc.).
    /// Updated when a missing container is recreated with a new ID.
    pub container_id: String,
    /// Restart policy that determines self-healing behavior when the container exits.
    pub restart_policy: RestartPolicy,
    /// Optional probe configuration for liveness checking.
    pub probe_config: Option<ProbeConfig>,
    /// Timestamp when the desired state was first created (for debugging/logging).
    pub created_at: SystemTime,
    /// The original Pod YAML used to start this container.
    /// Stored so the reconciliation loop can recreate the container if it is
    /// completely removed from Podman (missing, not just exited).
    pub pod_yaml: String,
}

impl DesiredState {
    /// Creates a new `DesiredState` with default values.
    ///
    /// The `container_id` is initially empty and should be updated after the
    /// container is successfully started via Podman.
    pub fn new(pod_name: String) -> Self {
        Self {
            pod_name,
            container_id: String::new(),
            restart_policy: RestartPolicy::OnFailure,
            probe_config: None,
            created_at: SystemTime::now(),
            pod_yaml: String::new(),
        }
    }
}

/// Defines how the NodeAgent should handle a container that has exited.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestartPolicy {
    /// Always restart the container regardless of exit code.
    Always,
    /// Restart the container only if it exited with a non-zero exit code.
    OnFailure,
    /// Never restart the container.
    Never,
}

/// Configuration for health probes associated with a workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    /// Optional liveness probe configuration.
    pub liveness: Option<LivenessProbe>,
}

/// Configuration for a liveness probe that checks if the container is healthy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessProbe {
    /// The type and parameters of the probe.
    pub probe_type: ProbeType,
    /// Number of seconds to wait after the container starts before running the first probe.
    pub initial_delay_seconds: u32,
    /// How often (in seconds) to perform the probe.
    pub period_seconds: u32,
    /// Number of seconds after which the probe times out.
    pub timeout_seconds: u32,
    /// Minimum consecutive failures required to mark the container as unhealthy.
    pub failure_threshold: u8,
}

/// Specifies the mechanism used to perform a liveness probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProbeType {
    /// HTTP GET probe against a specific path and port.
    Http { path: String, port: u16 },
    /// TCP socket probe against a specific port.
    Tcp { port: u16 },
    /// Execute a command inside the container.
    Exec { command: Vec<String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desired_state_new_defaults() {
        let state = DesiredState::new("test-pod".to_string());

        assert_eq!(state.pod_name, "test-pod");
        assert_eq!(state.container_id, "");
        assert_eq!(state.restart_policy, RestartPolicy::Always);
        assert!(state.probe_config.is_none());
        assert_eq!(state.pod_yaml, "");
    }

    #[test]
    fn test_desired_state_clone() {
        let original = DesiredState::new("my-pod".to_string());
        let cloned = original.clone();

        assert_eq!(cloned.pod_name, original.pod_name);
        assert_eq!(cloned.container_id, original.container_id);
        assert_eq!(cloned.restart_policy, original.restart_policy);
    }

    #[test]
    fn test_desired_state_with_container_id() {
        let mut state = DesiredState::new("my-pod".to_string());
        state.container_id = "abc123def456".to_string();

        assert_eq!(state.container_id, "abc123def456");
    }

    #[test]
    fn test_desired_state_with_probe_config() {
        let mut state = DesiredState::new("probe-pod".to_string());
        state.probe_config = Some(ProbeConfig {
            liveness: Some(LivenessProbe {
                probe_type: ProbeType::Http {
                    path: "/healthz".to_string(),
                    port: 8080,
                },
                initial_delay_seconds: 5,
                period_seconds: 10,
                timeout_seconds: 3,
                failure_threshold: 3,
            }),
        });

        assert!(state.probe_config.is_some());
        let probe_config = state.probe_config.unwrap();
        assert!(probe_config.liveness.is_some());
        let liveness = probe_config.liveness.unwrap();
        assert_eq!(liveness.initial_delay_seconds, 5);
        assert_eq!(liveness.period_seconds, 10);
        assert_eq!(liveness.failure_threshold, 3);
    }

    #[test]
    fn test_restart_policy_variants() {
        assert_eq!(RestartPolicy::Always, RestartPolicy::Always);
        assert_ne!(RestartPolicy::Always, RestartPolicy::Never);
        assert_ne!(RestartPolicy::Always, RestartPolicy::OnFailure);
        assert_ne!(RestartPolicy::OnFailure, RestartPolicy::Never);
    }

    #[test]
    fn test_probe_type_http() {
        let probe = ProbeType::Http {
            path: "/health".to_string(),
            port: 9090,
        };
        if let ProbeType::Http { path, port } = probe {
            assert_eq!(path, "/health");
            assert_eq!(port, 9090);
        } else {
            panic!("Expected Http probe type");
        }
    }

    #[test]
    fn test_probe_type_tcp() {
        let probe = ProbeType::Tcp { port: 8080 };
        if let ProbeType::Tcp { port } = probe {
            assert_eq!(port, 8080);
        } else {
            panic!("Expected Tcp probe type");
        }
    }

    #[test]
    fn test_probe_type_exec() {
        let probe = ProbeType::Exec {
            command: vec!["cat".to_string(), "/tmp/healthy".to_string()],
        };
        if let ProbeType::Exec { command } = probe {
            assert_eq!(command, vec!["cat", "/tmp/healthy"]);
        } else {
            panic!("Expected Exec probe type");
        }
    }

    #[test]
    fn test_desired_state_serialization() {
        let state = DesiredState::new("serial-pod".to_string());
        let json = serde_json::to_string(&state);
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("serial-pod"));
    }

    #[test]
    fn test_desired_state_with_pod_yaml() {
        let mut state = DesiredState::new("yaml-pod".to_string());
        let yaml = "apiVersion: v1\nkind: Pod\nmetadata:\n  name: yaml-pod\nspec:\n  containers:\n  - name: c\n    image: nginx\n";
        state.pod_yaml = yaml.to_string();

        assert_eq!(state.pod_yaml, yaml);
        // Ensure pod_yaml survives a clone
        let cloned = state.clone();
        assert_eq!(cloned.pod_yaml, yaml);
    }
}
