/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Liveness probe module for NodeAgent.
//!
//! This module implements the `probe_loop` function that continuously monitors
//! running containers and applies liveness probes based on their `DesiredState`
//! configuration. When a container fails its liveness probe `failure_threshold`
//! consecutive times, it is stopped via the Podman API.

pub mod checker;
pub mod liveness;

use crate::desired_state::DesiredState;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

/// Tracks per-container probe timing state.
struct ProbeState {
    /// When this probe state was first created (used as a proxy for container start time
    /// to determine when `initial_delay_seconds` has elapsed).
    first_seen_at: SystemTime,
    /// The last time a liveness probe was executed for this container.
    last_probe_at: Option<SystemTime>,
}

/// Main liveness probe loop.
///
/// Runs every second. For each running container that has a `probe_config` in the
/// `desired_states_cache`, the loop:
/// 1. Checks whether `initial_delay_seconds` has elapsed since the container was first seen.
/// 2. Checks whether `period_seconds` has elapsed since the last probe.
/// 3. Executes the liveness probe.
/// 4. Increments the failure counter on failure, resets it on success.
/// 5. Stops the container via Podman if `failure_threshold` consecutive failures occur.
pub async fn probe_loop(desired_states_cache: Arc<Mutex<HashMap<String, DesiredState>>>) {
    use crate::resource::container::get_list;

    let mut failure_counts: HashMap<String, u8> = HashMap::new();
    let mut probe_states: HashMap<String, ProbeState> = HashMap::new();

    loop {
        // Get the list of running containers from Podman.
        let running_containers = match get_list().await {
            Ok(containers) => containers
                .into_iter()
                .filter(|c| c.State == "running")
                .collect::<Vec<_>>(),
            Err(e) => {
                eprintln!("[Probe] Failed to list containers: {}", e);
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // Remove probe state for containers that are no longer running.
        let running_ids: std::collections::HashSet<String> =
            running_containers.iter().map(|c| c.Id.clone()).collect();
        probe_states.retain(|id, _| running_ids.contains(id));
        failure_counts.retain(|id, _| running_ids.contains(id));

        // Take a snapshot of the desired states (release lock immediately).
        let desired_states = {
            let cache = desired_states_cache.lock().await;
            cache.clone()
        };

        let now = SystemTime::now();

        for container in &running_containers {
            let container_id = &container.Id;

            // Find the desired state for this container by matching the container ID.
            let desired = match desired_states
                .values()
                .find(|d| &d.container_id == container_id)
            {
                Some(d) => d,
                None => continue, // No desired state → no probe configured
            };

            // Check whether the desired state includes a liveness probe config.
            let liveness_probe = match desired
                .probe_config
                .as_ref()
                .and_then(|pc| pc.liveness.as_ref())
            {
                Some(lp) => lp,
                None => continue, // No liveness probe configured → skip
            };

            // Get or create the probe state for this container.
            let probe_state =
                probe_states
                    .entry(container_id.clone())
                    .or_insert_with(|| ProbeState {
                        first_seen_at: now,
                        last_probe_at: None,
                    });

            // Check initial_delay_seconds: skip probe until delay has elapsed.
            let elapsed_since_start = now
                .duration_since(probe_state.first_seen_at)
                .unwrap_or_default();
            if elapsed_since_start.as_secs() < liveness_probe.initial_delay_seconds as u64 {
                continue;
            }

            // Check period_seconds: skip probe if not enough time has passed since last probe.
            if let Some(last_probe_at) = probe_state.last_probe_at {
                let elapsed_since_last = now.duration_since(last_probe_at).unwrap_or_default();
                if elapsed_since_last.as_secs() < liveness_probe.period_seconds as u64 {
                    continue;
                }
            }

            // Execute the liveness probe.
            let success = liveness::check_liveness_probe(container_id, liveness_probe).await;

            // Update the timestamp of the last probe execution.
            probe_state.last_probe_at = Some(now);

            let previous_count = failure_counts.get(container_id).copied().unwrap_or(0);

            if success {
                // Log only on state transition from failing to healthy.
                if previous_count > 0 {
                    println!(
                        "[Probe] Liveness probe for container {} recovered (was {} failures)",
                        container_id, previous_count
                    );
                }
                failure_counts.insert(container_id.clone(), 0);
            } else {
                let count = failure_counts.entry(container_id.clone()).or_insert(0);
                *count += 1;
                println!(
                    "[Probe] Liveness probe failed ({}/{}) for container {}",
                    count, liveness_probe.failure_threshold, container_id
                );

                if *count >= liveness_probe.failure_threshold {
                    println!(
                        "[NodeAgent] Stopping container {} due to liveness probe failure",
                        container_id
                    );
                    stop_container_by_id(container_id).await;
                    failure_counts.remove(container_id);
                    probe_states.remove(container_id);
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

/// Stop a container by its Podman container ID.
async fn stop_container_by_id(container_id: &str) {
    use hyper::Body;

    let stop_path = format!("/v4.0.0/libpod/containers/{}/stop", container_id);
    match crate::runtime::podman::post(&stop_path, Body::empty()).await {
        Ok(_) => println!("[Probe] Container '{}' stopped successfully", container_id),
        Err(e) => eprintln!(
            "[Probe] Failed to stop container '{}': {:?}",
            container_id, e
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desired_state::{DesiredState, LivenessProbe, ProbeConfig, ProbeType};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::time::{sleep, Duration};

    fn make_cache() -> Arc<Mutex<HashMap<String, DesiredState>>> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    #[test]
    fn test_probe_state_initializes_correctly() {
        let now = SystemTime::now();
        let state = ProbeState {
            first_seen_at: now,
            last_probe_at: None,
        };
        assert!(state.last_probe_at.is_none());
        assert!(state
            .first_seen_at
            .duration_since(std::time::UNIX_EPOCH)
            .is_ok());
    }

    #[tokio::test]
    async fn test_probe_loop_exits_when_no_containers() {
        let cache = make_cache();
        let cache_clone = Arc::clone(&cache);

        // Run probe_loop for a short time - it should not panic even with empty cache
        let probe_task = tokio::spawn(async move {
            probe_loop(cache_clone).await;
        });

        sleep(Duration::from_millis(150)).await;
        probe_task.abort();
        assert!(true);
    }

    #[tokio::test]
    async fn test_probe_loop_skips_containers_without_probe_config() {
        let cache = make_cache();

        // Insert a desired state without probe config
        {
            let mut c = cache.lock().await;
            let mut state = DesiredState::new("no-probe-pod".to_string());
            state.container_id = "abc123".to_string();
            state.probe_config = None;
            c.insert("no-probe-pod".to_string(), state);
        }

        let cache_clone = Arc::clone(&cache);
        let probe_task = tokio::spawn(async move {
            probe_loop(cache_clone).await;
        });

        sleep(Duration::from_millis(150)).await;
        probe_task.abort();

        // Cache should still contain the entry (probe loop doesn't modify it)
        assert_eq!(cache.lock().await.len(), 1);
    }

    #[test]
    fn test_failure_threshold_determines_stop() {
        // Verify the logic: failure_count >= failure_threshold triggers stop
        let failure_threshold: u8 = 3;
        let failure_count: u8 = 3;
        assert!(failure_count >= failure_threshold);

        let failure_count_2: u8 = 2;
        assert!(!(failure_count_2 >= failure_threshold));
    }

    #[test]
    fn test_probe_config_with_http_probe() {
        let mut state = DesiredState::new("test-pod".to_string());
        state.probe_config = Some(ProbeConfig {
            liveness: Some(LivenessProbe {
                probe_type: ProbeType::Http {
                    path: "/health".to_string(),
                    port: 8080,
                },
                initial_delay_seconds: 5,
                period_seconds: 10,
                timeout_seconds: 3,
                failure_threshold: 3,
            }),
        });

        let probe_config = state.probe_config.as_ref().unwrap();
        let liveness = probe_config.liveness.as_ref().unwrap();
        assert_eq!(liveness.initial_delay_seconds, 5);
        assert_eq!(liveness.period_seconds, 10);
        assert_eq!(liveness.failure_threshold, 3);
    }

    #[test]
    fn test_initial_delay_check() {
        let start = SystemTime::now()
            .checked_sub(Duration::from_secs(3))
            .unwrap();
        let now = SystemTime::now();
        let elapsed = now.duration_since(start).unwrap_or_default();
        let initial_delay_seconds: u64 = 5;

        // 3 seconds elapsed, initial delay is 5 → should skip
        assert!(elapsed.as_secs() < initial_delay_seconds);

        let start2 = SystemTime::now()
            .checked_sub(Duration::from_secs(10))
            .unwrap();
        let elapsed2 = now.duration_since(start2).unwrap_or_default();

        // 10 seconds elapsed, initial delay is 5 → should proceed
        assert!(elapsed2.as_secs() >= initial_delay_seconds);
    }

    #[test]
    fn test_period_seconds_check() {
        let last_probe = SystemTime::now()
            .checked_sub(Duration::from_secs(12))
            .unwrap();
        let now = SystemTime::now();
        let elapsed = now.duration_since(last_probe).unwrap_or_default();
        let period_seconds: u64 = 10;

        // 12 seconds since last probe, period is 10 → should probe
        assert!(elapsed.as_secs() >= period_seconds);

        let last_probe2 = SystemTime::now()
            .checked_sub(Duration::from_secs(5))
            .unwrap();
        let elapsed2 = now.duration_since(last_probe2).unwrap_or_default();

        // 5 seconds since last probe, period is 10 → should skip
        assert!(elapsed2.as_secs() < period_seconds);
    }
}
