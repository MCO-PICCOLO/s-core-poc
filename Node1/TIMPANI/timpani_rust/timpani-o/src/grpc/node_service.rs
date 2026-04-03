/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! `NodeService` gRPC server — serves Timpani-N nodes.
//!
//! Three RPCs (mirroring the D-Bus / libtrpc interface from the C++ port):
//!
//! | RPC           | C++ equivalent              | Purpose                              |
//! |---------------|-----------------------------|--------------------------------------|
//! | `GetSchedInfo`  | `trpc_client_schedinfo`   | Timpani-N pulls its task list        |
//! | `SyncTimer`     | `trpc_client_sync`        | Barrier — all nodes start together   |
//! | `ReportDMiss`   | `trpc_client_dmiss`       | Deadline miss forwarded to Pullpiri  |
//!
//! # SyncTimer barrier design
//!
//! `SyncTimer` is a blocking unary RPC.  When a node calls it:
//!
//! 1. The handler acquires `WorkloadStore`, registers the node in
//!    `synced_nodes`, and subscribes to `barrier_tx` — **all under the same
//!    lock hold** so a fast-firing barrier cannot be missed.
//! 2. If this node completes the set (`synced_nodes == active_nodes`), it
//!    broadcasts `Released { start_time_* }` on the watch channel.
//! 3. Every pending `SyncTimer` handler wakes from `changed().await` and
//!    returns the same `start_time` to its caller.
//!
//! The lock is **not** held during the `changed().await` wait, so it does not
//! block concurrent `GetSchedInfo` or `ReportDMiss` calls.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

use crate::fault::{FaultNotification, FaultNotifier};
use crate::proto::schedinfo_v1::{
    node_service_server::NodeService, DeadlineMissInfo, FaultType, NodeResponse, NodeSchedRequest,
    NodeSchedResponse, ScheduledTask, SyncRequest, SyncResponse,
};

use super::{BarrierStatus, WorkloadStore};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Nanoseconds in one second — used to split a nanosecond timestamp into
/// `(seconds, nanoseconds)` matching `struct timespec`.
const NANOS_PER_SEC: i64 = 1_000_000_000;

/// Offset added to `now` when computing the barrier start time.
///
/// Gives every Timpani-N node time to receive the `SyncResponse` and arm its
/// first hyperperiod timer before the start instant arrives.
/// Matches the C++ `ts->tv_sec += 1` in `SyncCallback`.
const SYNC_START_OFFSET_NS: i64 = 1_000_000_000; // 1 second

/// Default timeout for the SyncTimer barrier.
///
/// If not all active nodes call `SyncTimer` within this window, the barrier is
/// cancelled and every waiting handler returns `Status::DEADLINE_EXCEEDED`.
/// Configurable via `--sync-timeout-secs` on the CLI.
pub const DEFAULT_SYNC_TIMEOUT_SECS: u64 = 30;

// ── Service struct ────────────────────────────────────────────────────────────

/// tonic implementation of `NodeService`.
///
/// `Clone` is required by tonic.  All fields are `Arc`-wrapped.
#[derive(Clone)]
pub struct NodeServiceImpl {
    workload_store: WorkloadStore,
    fault_notifier: Arc<dyn FaultNotifier>,
    sync_timeout: Duration,
}

impl NodeServiceImpl {
    pub fn new(
        workload_store: WorkloadStore,
        fault_notifier: Arc<dyn FaultNotifier>,
        sync_timeout: Duration,
    ) -> Self {
        Self {
            workload_store,
            fault_notifier,
            sync_timeout,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Compute an absolute `CLOCK_REALTIME` start time `SYNC_START_OFFSET_NS` in
/// the future.  Returns `(seconds, nanoseconds)` matching `struct timespec`.
fn compute_start_time() -> (i64, i32) {
    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64;
    let start_ns = now_ns + SYNC_START_OFFSET_NS;
    (start_ns / NANOS_PER_SEC, (start_ns % NANOS_PER_SEC) as i32)
}

/// Convert an internal `SchedTask` to the proto wire type `ScheduledTask`.
///
/// `period_ns / 1_000` converts back to microseconds because `ScheduledTask`
/// carries µs (matching `task_info.period` in Timpani-N's C headers).
///
/// `cpu_affinity` is encoded as a single-bit mask (`1 << assigned_cpu`)
/// because the scheduler picked a specific CPU; Timpani-N calls
/// `set_affinity_cpumask` with this value.
fn to_proto_task(t: &crate::task::SchedTask) -> ScheduledTask {
    ScheduledTask {
        name: t.name.clone(),
        sched_priority: t.priority,
        sched_policy: t.policy.to_linux_int(),
        period_us: (t.period_ns / 1_000) as i32,
        release_time_us: t.release_time_us,
        runtime_us: (t.runtime_ns / 1_000) as i32,
        deadline_us: (t.deadline_ns / 1_000) as i32,
        cpu_affinity: 1u64 << t.assigned_cpu,
        max_dmiss: t.max_dmiss,
        assigned_node: t.assigned_node.clone(),
    }
}

// ── NodeService implementation ────────────────────────────────────────────────

#[tonic::async_trait]
impl NodeService for NodeServiceImpl {
    // ── GetSchedInfo ──────────────────────────────────────────────────────────

    async fn get_sched_info(
        &self,
        request: Request<NodeSchedRequest>,
    ) -> Result<Response<NodeSchedResponse>, Status> {
        let node_id = request.into_inner().node_id;
        info!(node_id = %node_id, "GetSchedInfo request");

        let guard = self.workload_store.lock().await;
        let ws = guard.as_ref().ok_or_else(|| {
            warn!(node_id = %node_id, "GetSchedInfo: no workload scheduled yet");
            Status::not_found("no workload has been scheduled yet")
        })?;

        // Return this node's task list.  If the node received no tasks, return
        // an empty list (not an error — the node can legitimately idle).
        let tasks: Vec<ScheduledTask> = ws
            .schedule
            .get(&node_id)
            .map(|v| v.iter().map(to_proto_task).collect())
            .unwrap_or_default();

        info!(
            node_id     = %node_id,
            workload_id = %ws.workload_id,
            task_count  = tasks.len(),
            "GetSchedInfo: serving schedule"
        );

        Ok(Response::new(NodeSchedResponse {
            workload_id: ws.workload_id.clone(),
            hyperperiod_us: ws.hyperperiod.hyperperiod_us,
            tasks,
        }))
    }

    // ── SyncTimer ─────────────────────────────────────────────────────────────

    async fn sync_timer(
        &self,
        request: Request<SyncRequest>,
    ) -> Result<Response<SyncResponse>, Status> {
        let node_id = request.into_inner().node_id;
        info!(node_id = %node_id, "SyncTimer: node checking in");

        // ── Phase 1: register the node and obtain a barrier receiver ──────────
        //
        // The receiver is obtained INSIDE the lock so we cannot miss a
        // Released notification that fires concurrently.  The lock is released
        // before the async wait in Phase 2.
        let mut barrier_rx = {
            let mut guard = self.workload_store.lock().await;
            let ws = guard
                .as_mut()
                .ok_or_else(|| Status::not_found("no workload has been scheduled yet"))?;

            if ws.active_nodes.is_empty() {
                return Err(Status::failed_precondition("workload has no active nodes"));
            }
            if !ws.active_nodes.contains(&node_id) {
                warn!(
                    node_id = %node_id,
                    "SyncTimer: node is not part of the active workload"
                );
                return Err(Status::not_found(format!(
                    "node '{}' did not receive any tasks in the active workload",
                    node_id
                )));
            }

            // Subscribe before potentially firing so we cannot miss Released.
            let rx = ws.barrier_tx.subscribe();

            ws.synced_nodes.insert(node_id.clone());

            let all_synced = ws.active_nodes.iter().all(|n| ws.synced_nodes.contains(n));

            if all_synced {
                let (sec, nsec) = compute_start_time();
                let _ = ws.barrier_tx.send(BarrierStatus::Released {
                    start_time_sec: sec,
                    start_time_nsec: nsec,
                });
                info!(
                    workload_id = %ws.workload_id,
                    node_count  = ws.active_nodes.len(),
                    start_sec   = sec,
                    "SyncTimer: barrier fired — all nodes ready"
                );
            } else {
                let waiting: Vec<&String> = ws
                    .active_nodes
                    .iter()
                    .filter(|n| !ws.synced_nodes.contains(*n))
                    .collect();
                info!(
                    node_id = %node_id,
                    ?waiting,
                    "SyncTimer: registered, waiting for remaining nodes"
                );
            }

            rx
        }; // WorkloadStore lock released here

        // ── Phase 2: wait for the barrier ─────────────────────────────────────
        //
        // `borrow_and_update()` marks the current value as "seen" so that
        // `changed()` fires only on the NEXT state transition — no spurious
        // wake-ups.
        //
        // A `tokio::time::sleep` races against every `changed()` call.  The
        // future is pinned once so the deadline advances correctly across loop
        // iterations.  The first handler to hit the deadline broadcasts
        // `TimedOut` on the shared channel so all other waiters also wake up.
        let timeout_sleep = tokio::time::sleep(self.sync_timeout);
        tokio::pin!(timeout_sleep);

        loop {
            let status = (*barrier_rx.borrow_and_update()).clone();

            match status {
                BarrierStatus::Released {
                    start_time_sec,
                    start_time_nsec,
                } => {
                    info!(
                        node_id   = %node_id,
                        start_sec = start_time_sec,
                        "SyncTimer: ack sent"
                    );
                    return Ok(Response::new(SyncResponse {
                        ack: true,
                        start_time_sec,
                        start_time_nsec,
                    }));
                }
                BarrierStatus::Cancelled => {
                    warn!(
                        node_id = %node_id,
                        "SyncTimer: workload replaced while waiting — aborting"
                    );
                    return Err(Status::aborted(
                        "workload was replaced while waiting for the sync barrier",
                    ));
                }
                BarrierStatus::TimedOut => {
                    warn!(
                        node_id = %node_id,
                        "SyncTimer: barrier timed out — returning deadline exceeded"
                    );
                    return Err(Status::deadline_exceeded(
                        "sync barrier timed out waiting for all nodes to check in",
                    ));
                }
                BarrierStatus::Waiting => {
                    // Not all nodes have checked in yet — wait.
                }
            }

            tokio::select! {
                result = barrier_rx.changed() => {
                    if result.is_err() {
                        // The Sender was dropped (workload replaced after Cancelled was
                        // sent and before this handler woke up).
                        return Err(Status::aborted("sync barrier channel closed unexpectedly"));
                    }
                }
                _ = &mut timeout_sleep => {
                    warn!(
                        node_id      = %node_id,
                        timeout_secs = self.sync_timeout.as_secs(),
                        "SyncTimer: timeout — broadcasting TimedOut to all waiting nodes"
                    );
                    // Wake all other handlers that are waiting on this barrier.
                    {
                        let guard = self.workload_store.lock().await;
                        if let Some(ws) = guard.as_ref() {
                            let _ = ws.barrier_tx.send(BarrierStatus::TimedOut);
                        }
                    }
                    return Err(Status::deadline_exceeded(format!(
                        "sync barrier timed out after {}s — not all nodes checked in",
                        self.sync_timeout.as_secs(),
                    )));
                }
            }
        }
    }

    // ── ReportDMiss ───────────────────────────────────────────────────────────

    async fn report_d_miss(
        &self,
        request: Request<DeadlineMissInfo>,
    ) -> Result<Response<NodeResponse>, Status> {
        let info = request.into_inner();
        let node_id = info.node_id.clone();
        let task_name = info.task_name.clone();

        warn!(
            node_id   = %node_id,
            task_name = %task_name,
            "DeadlineMiss reported"
        );

        // Resolve workload_id from the active schedule.
        // If the task is not found (race with workload replacement), fall back
        // to the current workload_id — mirrors the C++ DMissCallback fallback.
        let workload_id = {
            let guard = self.workload_store.lock().await;
            match guard.as_ref() {
                None => {
                    warn!("ReportDMiss: no active workload");
                    return Ok(Response::new(NodeResponse {
                        status: -1,
                        error_message: "no active workload".into(),
                    }));
                }
                Some(ws) => {
                    let found = ws
                        .schedule
                        .get(&node_id)
                        .and_then(|tasks| tasks.iter().find(|t| t.name == task_name))
                        .is_some();

                    if !found {
                        warn!(
                            node_id   = %node_id,
                            task_name = %task_name,
                            "ReportDMiss: task not found in schedule; \
                             using current workload_id as fallback"
                        );
                    }
                    ws.workload_id.clone()
                }
            }
        };

        // Forward the fault to Pullpiri.
        let notification = FaultNotification {
            workload_id,
            node_id,
            task_name,
            fault_type: FaultType::Dmiss,
        };

        if let Err(e) = self.fault_notifier.notify_fault(notification).await {
            error!(error = %e, "Failed to notify Pullpiri of deadline miss");
            return Ok(Response::new(NodeResponse {
                status: -1,
                error_message: format!("fault notification failed: {e}"),
            }));
        }

        Ok(Response::new(NodeResponse {
            status: 0,
            error_message: String::new(),
        }))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tonic::Request;

    use crate::config::{NodeConfig, NodeConfigManager};
    use crate::fault::{
        test_support::MockFaultNotifier, FaultError, FaultNotification, FaultNotifier,
    };
    use crate::grpc::{new_workload_store, schedinfo_service::SchedInfoServiceImpl};
    use crate::proto::schedinfo_v1::{
        node_service_server::NodeService, sched_info_service_server::SchedInfoService,
        DeadlineMissInfo, NodeSchedRequest, SchedInfo, SyncRequest, TaskInfo,
    };

    use super::{NodeServiceImpl, DEFAULT_SYNC_TIMEOUT_SECS};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn two_node_config() -> Arc<NodeConfigManager> {
        Arc::new(NodeConfigManager::from_nodes(vec![
            NodeConfig {
                name: "n1".into(),
                available_cpus: vec![0, 1],
                max_memory_mb: 4096,
                architecture: "x86_64".into(),
                location: "test".into(),
                description: "".into(),
            },
            NodeConfig {
                name: "n2".into(),
                available_cpus: vec![0, 1],
                max_memory_mb: 4096,
                architecture: "x86_64".into(),
                location: "test".into(),
                description: "".into(),
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

    /// Returns both services sharing one WorkloadStore plus the raw mock for inspection.
    fn test_services() -> (
        SchedInfoServiceImpl,
        NodeServiceImpl,
        Arc<MockFaultNotifier>,
    ) {
        let store = new_workload_store();
        let mock = MockFaultNotifier::arc();
        let svc = SchedInfoServiceImpl::new(
            two_node_config(),
            Arc::clone(&store),
            Arc::clone(&mock) as Arc<dyn FaultNotifier>,
        );
        let node_svc = NodeServiceImpl::new(
            Arc::clone(&store),
            Arc::clone(&mock) as Arc<dyn FaultNotifier>,
            Duration::from_secs(DEFAULT_SYNC_TIMEOUT_SECS),
        );
        (svc, node_svc, mock)
    }

    // ── GetSchedInfo ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_sched_info_no_workload_returns_not_found() {
        let (_, node_svc, _) = test_services();
        let err = node_svc
            .get_sched_info(Request::new(NodeSchedRequest {
                node_id: "n1".into(),
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn get_sched_info_returns_tasks_for_requesting_node() {
        let (svc, node_svc, _) = test_services();
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1"), task_for("t2", "n2")],
        }))
        .await
        .unwrap();

        let resp = node_svc
            .get_sched_info(Request::new(NodeSchedRequest {
                node_id: "n1".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(resp.workload_id, "wl");
        assert_eq!(resp.tasks.len(), 1);
        assert_eq!(resp.tasks[0].name, "t1");
        assert!(resp.hyperperiod_us > 0);
    }

    #[tokio::test]
    async fn get_sched_info_unknown_node_returns_empty_task_list() {
        let (svc, node_svc, _) = test_services();
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        let resp = node_svc
            .get_sched_info(Request::new(NodeSchedRequest {
                node_id: "no_such_node".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        // Unknown node is not an error — it just receives an empty task list.
        assert!(resp.tasks.is_empty());
    }

    // ── SyncTimer ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn sync_timer_no_workload_returns_not_found() {
        let (_, node_svc, _) = test_services();
        let err = node_svc
            .sync_timer(Request::new(SyncRequest {
                node_id: "n1".into(),
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn sync_timer_unknown_node_returns_not_found() {
        let (svc, node_svc, _) = test_services();
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        let err = node_svc
            .sync_timer(Request::new(SyncRequest {
                node_id: "unknown_node".into(),
            }))
            .await
            .unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn sync_timer_single_node_workload_fires_barrier_immediately() {
        let (svc, node_svc, _) = test_services();
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        let resp = node_svc
            .sync_timer(Request::new(SyncRequest {
                node_id: "n1".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert!(resp.ack);
        assert!(
            resp.start_time_sec > 0,
            "start_time should be a real timestamp"
        );
    }

    #[tokio::test]
    async fn sync_timer_all_nodes_receive_identical_start_time() {
        let (svc, node_svc, _) = test_services();
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1"), task_for("t2", "n2")],
        }))
        .await
        .unwrap();

        let nsvc1 = node_svc.clone();
        let nsvc2 = node_svc.clone();

        let (r1, r2) = tokio::join!(
            nsvc1.sync_timer(Request::new(SyncRequest {
                node_id: "n1".into()
            })),
            nsvc2.sync_timer(Request::new(SyncRequest {
                node_id: "n2".into()
            })),
        );

        let s1 = r1.unwrap().into_inner();
        let s2 = r2.unwrap().into_inner();

        assert!(s1.ack && s2.ack);
        assert_eq!(
            s1.start_time_sec, s2.start_time_sec,
            "all nodes must share the same start second"
        );
        assert_eq!(
            s1.start_time_nsec, s2.start_time_nsec,
            "all nodes must share the same start nanosecond"
        );
    }

    // ── SyncTimer timeout ─────────────────────────────────────────────────────

    /// When a node joins the barrier but a second node never arrives, the
    /// barrier must fire `TimedOut` and return `DEADLINE_EXCEEDED` to the
    /// waiting node.
    #[tokio::test]
    async fn sync_timer_returns_deadline_exceeded_when_timeout_fires() {
        let store = new_workload_store();
        let mock = MockFaultNotifier::arc();
        let svc = SchedInfoServiceImpl::new(
            two_node_config(),
            Arc::clone(&store),
            Arc::clone(&mock) as Arc<dyn FaultNotifier>,
        );
        // Very short timeout so the test runs quickly.
        let node_svc = NodeServiceImpl::new(
            Arc::clone(&store),
            Arc::clone(&mock) as Arc<dyn FaultNotifier>,
            Duration::from_millis(50),
        );

        // Two-node workload: n2 never calls SyncTimer, so n1 must time out.
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1"), task_for("t2", "n2")],
        }))
        .await
        .unwrap();

        let err = tokio::time::timeout(
            Duration::from_secs(2),
            node_svc.sync_timer(Request::new(SyncRequest {
                node_id: "n1".into(),
            })),
        )
        .await
        .expect("sync_timer should complete within 2s")
        .unwrap_err();

        assert_eq!(err.code(), tonic::Code::DeadlineExceeded);
    }

    /// When the timeout fires while multiple nodes are waiting, the broadcast
    /// ensures **all** waiters receive `DEADLINE_EXCEEDED`, not just the node
    /// whose handler happened to own the sleep future.
    #[tokio::test]
    async fn sync_timer_timeout_wakes_all_waiting_nodes() {
        // Three-node workload; n1 and n2 join but n3 never does.
        let ncm = NodeConfigManager::from_nodes(vec![
            NodeConfig {
                name: "n1".into(),
                available_cpus: vec![0],
                max_memory_mb: 1024,
                architecture: "x86_64".into(),
                location: "test".into(),
                description: "".into(),
            },
            NodeConfig {
                name: "n2".into(),
                available_cpus: vec![0],
                max_memory_mb: 1024,
                architecture: "x86_64".into(),
                location: "test".into(),
                description: "".into(),
            },
            NodeConfig {
                name: "n3".into(),
                available_cpus: vec![0],
                max_memory_mb: 1024,
                architecture: "x86_64".into(),
                location: "test".into(),
                description: "".into(),
            },
        ]);
        let _ = ncm; // suppress unused warning

        let store = new_workload_store();
        let mock = MockFaultNotifier::arc();
        let svc = SchedInfoServiceImpl::new(
            Arc::new(NodeConfigManager::from_nodes(vec![
                NodeConfig {
                    name: "n1".into(),
                    available_cpus: vec![0, 1],
                    max_memory_mb: 4096,
                    architecture: "x86_64".into(),
                    location: "test".into(),
                    description: "".into(),
                },
                NodeConfig {
                    name: "n2".into(),
                    available_cpus: vec![0, 1],
                    max_memory_mb: 4096,
                    architecture: "x86_64".into(),
                    location: "test".into(),
                    description: "".into(),
                },
                NodeConfig {
                    name: "n3".into(),
                    available_cpus: vec![0, 1],
                    max_memory_mb: 4096,
                    architecture: "x86_64".into(),
                    location: "test".into(),
                    description: "".into(),
                },
            ])),
            Arc::clone(&store),
            Arc::clone(&mock) as Arc<dyn FaultNotifier>,
        );
        let node_svc = NodeServiceImpl::new(
            Arc::clone(&store),
            Arc::clone(&mock) as Arc<dyn FaultNotifier>,
            Duration::from_millis(80),
        );

        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl3".into(),
            tasks: vec![
                task_for("t1", "n1"),
                task_for("t2", "n2"),
                task_for("t3", "n3"),
            ],
        }))
        .await
        .unwrap();

        // n1 and n2 join; n3 never does.
        let nsvc1 = node_svc.clone();
        let nsvc2 = node_svc.clone();

        let h1 = tokio::spawn(async move {
            nsvc1
                .sync_timer(Request::new(SyncRequest {
                    node_id: "n1".into(),
                }))
                .await
        });
        let h2 = tokio::spawn(async move {
            nsvc2
                .sync_timer(Request::new(SyncRequest {
                    node_id: "n2".into(),
                }))
                .await
        });

        let (r1, r2) = tokio::time::timeout(Duration::from_secs(2), async { tokio::join!(h1, h2) })
            .await
            .expect("both handlers should complete within 2s");

        let e1 = r1.unwrap().unwrap_err();
        let e2 = r2.unwrap().unwrap_err();

        assert_eq!(
            e1.code(),
            tonic::Code::DeadlineExceeded,
            "n1 should get DeadlineExceeded"
        );
        assert_eq!(
            e2.code(),
            tonic::Code::DeadlineExceeded,
            "n2 should get DeadlineExceeded"
        );
    }

    #[tokio::test]
    async fn sync_timer_returns_aborted_when_workload_replaced_while_waiting() {
        let (svc, node_svc, _) = test_services();

        // Two-node workload: n1 will block because n2 never joins.
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl1".into(),
            tasks: vec![task_for("t1", "n1"), task_for("t2", "n2")],
        }))
        .await
        .unwrap();

        let nsvc = node_svc.clone();
        let handle = tokio::spawn(async move {
            nsvc.sync_timer(Request::new(SyncRequest {
                node_id: "n1".into(),
            }))
            .await
        });

        // Give n1 time to register and begin awaiting the barrier.
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Replace the workload — this broadcasts Cancelled to the old barrier.
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl2".into(),
            tasks: vec![task_for("t3", "n1")],
        }))
        .await
        .unwrap();

        let result = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("SyncTimer should complete after workload replacement")
            .expect("spawned task should not panic");

        assert_eq!(result.unwrap_err().code(), tonic::Code::Aborted);
    }

    // ── ReportDMiss ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn report_d_miss_no_workload_returns_error_status() {
        let (_, node_svc, _) = test_services();
        let resp = node_svc
            .report_d_miss(Request::new(DeadlineMissInfo {
                node_id: "n1".into(),
                task_name: "t1".into(),
            }))
            .await
            .unwrap()
            .into_inner();
        assert_ne!(resp.status, 0);
    }

    #[tokio::test]
    async fn report_d_miss_known_task_calls_fault_notifier() {
        let (svc, node_svc, mock) = test_services();
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        let resp = node_svc
            .report_d_miss(Request::new(DeadlineMissInfo {
                node_id: "n1".into(),
                task_name: "t1".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_eq!(resp.status, 0);
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].workload_id, "wl");
        assert_eq!(calls[0].node_id, "n1");
        assert_eq!(calls[0].task_name, "t1");
    }

    #[tokio::test]
    async fn report_d_miss_unknown_task_uses_fallback_workload_id_and_still_notifies() {
        let (svc, node_svc, mock) = test_services();
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl_fallback".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        // Report a miss for a task name that doesn't exist in the schedule.
        let resp = node_svc
            .report_d_miss(Request::new(DeadlineMissInfo {
                node_id: "n1".into(),
                task_name: "task_that_does_not_exist".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        // Should still succeed via the fallback path (uses current workload_id).
        assert_eq!(resp.status, 0);
        let calls = mock.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].workload_id, "wl_fallback");
    }

    #[tokio::test]
    async fn report_d_miss_notifier_failure_returns_error_status() {
        // Custom notifier that always fails.
        struct FailingNotifier;
        #[tonic::async_trait]
        impl FaultNotifier for FailingNotifier {
            async fn notify_fault(&self, _: FaultNotification) -> Result<(), FaultError> {
                Err(FaultError::RemoteError(-1))
            }
        }

        let store = new_workload_store();
        // Populate the workload via a service that uses a working mock.
        let mock = MockFaultNotifier::arc();
        let svc = SchedInfoServiceImpl::new(
            two_node_config(),
            Arc::clone(&store),
            Arc::clone(&mock) as Arc<dyn FaultNotifier>,
        );
        svc.add_sched_info(Request::new(SchedInfo {
            workload_id: "wl".into(),
            tasks: vec![task_for("t1", "n1")],
        }))
        .await
        .unwrap();

        // Node service uses the failing notifier.
        let failing_svc = NodeServiceImpl::new(
            Arc::clone(&store),
            Arc::new(FailingNotifier) as Arc<dyn FaultNotifier>,
            Duration::from_secs(DEFAULT_SYNC_TIMEOUT_SECS),
        );

        let resp = failing_svc
            .report_d_miss(Request::new(DeadlineMissInfo {
                node_id: "n1".into(),
                task_name: "t1".into(),
            }))
            .await
            .unwrap()
            .into_inner();

        assert_ne!(resp.status, 0);
        assert!(!resp.error_message.is_empty());
    }
}
