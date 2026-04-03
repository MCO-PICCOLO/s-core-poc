/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! `SchedInfoService` gRPC server — receives workloads from Pullpiri.
//!
//! Implements the `AddSchedInfo` RPC:
//!   1. Convert proto `TaskInfo` list → internal `Vec<Task>`.
//!   2. Calculate hyperperiod (LCM of all task periods).
//!   3. Run `GlobalScheduler` to assign tasks to nodes and CPUs.
//!   4. Acquire `WorkloadStore` lock briefly, cancel previous workload's
//!      sync barrier, store the new `WorkloadState`, release lock.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

use crate::config::NodeConfigManager;
use crate::fault::FaultNotifier;
use crate::hyperperiod::HyperperiodManager;
use crate::proto::schedinfo_v1::{
    sched_info_service_server::SchedInfoService, Response as ProtoResponse, SchedInfo, TaskInfo,
};
use crate::scheduler::GlobalScheduler;
use crate::task::{CpuAffinity, SchedPolicy, Task};

use super::{BarrierStatus, WorkloadState, WorkloadStore};

// ── Service struct ────────────────────────────────────────────────────────────

/// tonic implementation of `SchedInfoService`.
///
/// `Clone` is required by tonic (it clones the service for each connection).
/// All fields are `Arc`-wrapped so cloning is cheap.
#[derive(Clone)]
pub struct SchedInfoServiceImpl {
    scheduler: Arc<GlobalScheduler>,
    workload_store: WorkloadStore,
    /// Injected fault notifier — used for future scheduler-error forwarding.
    /// Not yet called in the port; present so the injection pipeline exists.
    #[allow(dead_code)]
    fault_notifier: Arc<dyn FaultNotifier>,
}

impl SchedInfoServiceImpl {
    pub fn new(
        node_config_manager: Arc<NodeConfigManager>,
        workload_store: WorkloadStore,
        fault_notifier: Arc<dyn FaultNotifier>,
    ) -> Self {
        Self {
            scheduler: Arc::new(GlobalScheduler::new(node_config_manager)),
            workload_store,
            fault_notifier,
        }
    }
}

// ── Proto → Task conversion ───────────────────────────────────────────────────

/// Convert a proto `TaskInfo` into an internal `Task`.
///
/// `workload_id` comes from the enclosing `SchedInfo` message; every task in
/// one RPC call shares the same value.
fn task_from_proto(t: &TaskInfo, workload_id: &str) -> Task {
    Task {
        name: t.name.clone(),
        workload_id: workload_id.to_owned(),
        // node_id in the proto is the preferred/required target node.
        target_node: t.node_id.clone(),
        policy: SchedPolicy::from_proto_int(t.policy),
        priority: t.priority,
        affinity: CpuAffinity::from_proto(t.cpu_affinity),
        period_us: t.period.max(0) as u64,
        runtime_us: t.runtime.max(0) as u64,
        deadline_us: t.deadline.max(0) as u64,
        release_time_us: t.release_time.max(0) as u32,
        max_dmiss: t.max_dmiss,
        memory_mb: 0, // not in proto yet — dormant (D-003)
        ..Task::default()
    }
}

// ── SchedInfoService implementation ──────────────────────────────────────────

#[tonic::async_trait]
impl SchedInfoService for SchedInfoServiceImpl {
    async fn add_sched_info(
        &self,
        request: Request<SchedInfo>,
    ) -> Result<Response<ProtoResponse>, Status> {
        let req = request.into_inner();
        let workload_id = req.workload_id.clone();

        info!(
            workload_id = %workload_id,
            task_count  = req.tasks.len(),
            "AddSchedInfo received"
        );

        // Log per-task details at debug level (mirrors C++ TLOG_DEBUG block).
        for (i, t) in req.tasks.iter().enumerate() {
            tracing::debug!(
                idx          = i,
                name         = %t.name,
                node_id      = %t.node_id,
                priority     = t.priority,
                cpu_affinity = %format!("0x{:016x}", t.cpu_affinity),
                period_us    = t.period,
                runtime_us   = t.runtime,
                deadline_us  = t.deadline,
                "task"
            );
        }

        // ── 1. Convert proto tasks to internal representation ─────────────────
        let tasks: Vec<Task> = req
            .tasks
            .iter()
            .map(|t| task_from_proto(t, &workload_id))
            .collect();

        // ── 2. Calculate hyperperiod ──────────────────────────────────────────
        // Create a fresh HyperperiodManager per call — we only need the result
        // once and storing it in WorkloadState.  The clone gives us ownership.
        let hyperperiod_info = {
            let mut hp_mgr = HyperperiodManager::new();
            match hp_mgr.calculate_hyperperiod(&workload_id, &tasks) {
                Ok(info) => info.clone(),
                Err(e) => {
                    error!(
                        workload_id = %workload_id,
                        error = %e,
                        "Hyperperiod calculation failed"
                    );
                    return Ok(Response::new(ProtoResponse { status: -1 }));
                }
            }
        };

        info!(
            workload_id    = %workload_id,
            hyperperiod_ms = hyperperiod_info.hyperperiod_us / 1_000,
            task_count     = hyperperiod_info.task_count,
            "Hyperperiod calculated"
        );

        // ── 3. Run GlobalScheduler ────────────────────────────────────────────
        let schedule = match self.scheduler.schedule(tasks, "target_node_priority") {
            Ok(s) => s,
            Err(e) => {
                error!(
                    workload_id = %workload_id,
                    error = %e,
                    "GlobalScheduler::schedule() failed"
                );
                return Ok(Response::new(ProtoResponse { status: -1 }));
            }
        };

        info!(
            workload_id = %workload_id,
            node_count  = schedule.len(),
            "Schedule produced"
        );
        for (node, tasks) in &schedule {
            info!("  node '{node}': {} task(s)", tasks.len());
        }

        // ── 4. Store workload (brief lock) ────────────────────────────────────
        {
            let mut guard = self.workload_store.lock().await;

            if let Some(prev) = guard.as_ref() {
                warn!(
                    prev_workload = %prev.workload_id,
                    new_workload  = %workload_id,
                    "Replacing existing workload \
                     (single-workload limitation — see DEVELOPER_NOTES D-016)"
                );
                // Wake all SyncTimer handlers waiting on the previous barrier.
                let _ = prev.barrier_tx.send(BarrierStatus::Cancelled);
            }

            *guard = Some(WorkloadState::new(
                workload_id.clone(),
                schedule,
                hyperperiod_info,
            ));
        } // lock released here

        info!(workload_id = %workload_id, "Workload stored, awaiting node sync");
        Ok(Response::new(ProtoResponse { status: 0 }))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    use crate::config::{NodeConfig, NodeConfigManager};
    use crate::fault::{test_support::MockFaultNotifier, FaultNotifier};
    use crate::grpc::{new_workload_store, BarrierStatus};
    use crate::proto::schedinfo_v1::{
        sched_info_service_server::SchedInfoService, SchedInfo, TaskInfo,
    };

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn two_node_config() -> Arc<NodeConfigManager> {
        Arc::new(NodeConfigManager::from_nodes(vec![
            NodeConfig {
                name: "n1".into(),
                available_cpus: vec![0, 1],
                max_memory_mb: 4096,
                architecture: "x86_64".into(),
                location: "test".into(),
                description: "test node 1".into(),
            },
            NodeConfig {
                name: "n2".into(),
                available_cpus: vec![0, 1],
                max_memory_mb: 4096,
                architecture: "x86_64".into(),
                location: "test".into(),
                description: "test node 2".into(),
            },
        ]))
    }

    fn task_for(name: &str, node: &str) -> TaskInfo {
        TaskInfo {
            name: name.into(),
            node_id: node.into(),
            priority: 50,
            policy: 1,
            cpu_affinity: 0,
            period: 10_000,
            runtime: 1_000,
            deadline: 10_000,
            release_time: 0,
            max_dmiss: 3,
        }
    }

    fn make_svc_with_store(store: WorkloadStore) -> SchedInfoServiceImpl {
        let mock = MockFaultNotifier::arc();
        SchedInfoServiceImpl::new(two_node_config(), store, mock as Arc<dyn FaultNotifier>)
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn add_sched_info_two_nodes_returns_ok_status() {
        let svc = make_svc_with_store(new_workload_store());
        let si = SchedInfo {
            workload_id: "wl_ok".into(),
            tasks: vec![task_for("t1", "n1"), task_for("t2", "n2")],
        };
        let resp = svc.add_sched_info(Request::new(si)).await.unwrap();
        assert_eq!(resp.into_inner().status, 0);
    }

    #[tokio::test]
    async fn add_sched_info_empty_tasks_returns_error_status() {
        let svc = make_svc_with_store(new_workload_store());
        let resp = svc
            .add_sched_info(Request::new(SchedInfo {
                workload_id: "wl_empty".into(),
                tasks: vec![],
            }))
            .await
            .unwrap();
        assert_ne!(resp.into_inner().status, 0);
    }

    #[tokio::test]
    async fn add_sched_info_unknown_node_returns_error_status() {
        let svc = make_svc_with_store(new_workload_store());
        let resp = svc
            .add_sched_info(Request::new(SchedInfo {
                workload_id: "wl_bad".into(),
                tasks: vec![task_for("t1", "node_not_in_config")],
            }))
            .await
            .unwrap();
        assert_ne!(resp.into_inner().status, 0);
    }

    #[tokio::test]
    async fn add_sched_info_stores_workload_in_workload_store() {
        let store = new_workload_store();
        let svc = make_svc_with_store(Arc::clone(&store));

        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl_stored".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        let guard = store.lock().await;
        let ws = guard.as_ref().expect("workload should be in the store");
        assert_eq!(ws.workload_id, "wl_stored");
        assert!(ws.active_nodes.contains("n1"));
    }

    #[tokio::test]
    async fn add_sched_info_replaces_previous_workload_and_cancels_barrier() {
        let store = new_workload_store();
        let svc = make_svc_with_store(Arc::clone(&store));

        // First workload — subscribe to its barrier before replacing
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl_first".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        let barrier_rx = {
            let guard = store.lock().await;
            guard.as_ref().unwrap().barrier_tx.subscribe()
        };

        // Replace with second workload
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl_second".into(),
            tasks: vec![task_for("t2", "n2")],
        }))
        .await
        .unwrap();

        // First barrier should now be Cancelled
        assert_eq!(*barrier_rx.borrow(), BarrierStatus::Cancelled);

        // Active workload should be the second one
        let guard = store.lock().await;
        assert_eq!(guard.as_ref().unwrap().workload_id, "wl_second");
    }
}
