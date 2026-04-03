/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! gRPC service wiring — shared state and service module roots.
//!
//! Both gRPC services ([`schedinfo_service`] and [`node_service`]) share a
//! single [`WorkloadStore`] that holds the currently active schedule.
//!
//! # Concurrency model
//!
//! ```text
//!   Pullpiri ──AddSchedInfo──► SchedInfoServiceImpl
//!                                     │  writes
//!                                     ▼
//!                             WorkloadStore  (Arc<Mutex<Option<WorkloadState>>>)
//!                                     │  reads
//!                                     ▼
//!   Timpani-N ──GetSchedInfo──► NodeServiceImpl
//!   Timpani-N ──SyncTimer    ──► NodeServiceImpl  (holds watch::Receiver)
//!   Timpani-N ──ReportDMiss  ──► NodeServiceImpl
//! ```
//!
//! The `Mutex` is held briefly: only while reading/writing `WorkloadState`.
//! `SyncTimer` acquires the lock to register the node and obtain a
//! `watch::Receiver`, then releases it before awaiting the barrier.

pub mod node_service;
pub mod schedinfo_service;

use std::collections::BTreeSet;
use std::sync::Arc;

use tokio::sync::{watch, Mutex};

use crate::hyperperiod::HyperperiodInfo;
use crate::task::NodeSchedMap;

// ── BarrierStatus ─────────────────────────────────────────────────────────────

/// State of the SyncTimer synchronisation barrier for the active workload.
///
/// Sent over a `tokio::sync::watch` channel so that all waiting `SyncTimer`
/// handlers wake up simultaneously when the barrier fires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BarrierStatus {
    /// Waiting for all active nodes to call `SyncTimer`.
    Waiting,

    /// All nodes have checked in.  Every RT loop must arm its first timer
    /// for this absolute `CLOCK_REALTIME` time.
    Released {
        start_time_sec: i64,
        start_time_nsec: i32,
    },

    /// The workload was replaced before the barrier fired.
    /// Waiting handlers should abort with `Status::aborted`.
    Cancelled,

    /// No response from all nodes before the configured deadline.
    /// Waiting handlers should return `Status::deadline_exceeded`.
    TimedOut,
}

// ── WorkloadState ─────────────────────────────────────────────────────────────

/// All per-workload state shared between the two gRPC services.
///
/// Created by `SchedInfoService` when a new workload arrives and stored inside
/// the [`WorkloadStore`].  Dropped (with [`BarrierStatus::Cancelled`] broadcast)
/// when the next workload replaces it.
pub struct WorkloadState {
    /// Workload identifier from the `AddSchedInfo` proto request.
    pub workload_id: String,

    /// Per-node scheduled task lists produced by `GlobalScheduler`.
    /// `NodeSchedMap = HashMap<node_id, Vec<SchedTask>>`
    pub schedule: NodeSchedMap,

    /// Hyperperiod computed before scheduling.
    pub hyperperiod: HyperperiodInfo,

    /// Nodes that received at least one task — the expected `SyncTimer` callers.
    /// Derived from `schedule.keys()` at construction time.
    pub active_nodes: BTreeSet<String>,

    /// Nodes that have called `SyncTimer` and are waiting for the barrier.
    pub synced_nodes: BTreeSet<String>,

    /// Barrier broadcast channel.
    ///
    /// Transitions:
    ///   `Waiting` → `Released { ... }` when all active nodes have synced.
    ///   `Waiting` → `Cancelled`        when a new workload replaces this one.
    ///
    /// `NodeService::sync_timer` subscribes to this sender while holding the
    /// `WorkloadStore` lock, then awaits the receiver after releasing the lock.
    pub barrier_tx: watch::Sender<BarrierStatus>,
}

impl WorkloadState {
    /// Construct a fresh `WorkloadState` for a newly scheduled workload.
    ///
    /// `active_nodes` is derived from the keys of `schedule` — only nodes
    /// that actually received tasks must participate in the sync barrier.
    pub fn new(workload_id: String, schedule: NodeSchedMap, hyperperiod: HyperperiodInfo) -> Self {
        let active_nodes: BTreeSet<String> = schedule.keys().cloned().collect();
        let (barrier_tx, _) = watch::channel(BarrierStatus::Waiting);
        Self {
            workload_id,
            schedule,
            hyperperiod,
            active_nodes,
            synced_nodes: BTreeSet::new(),
            barrier_tx,
        }
    }
}

// ── WorkloadStore ─────────────────────────────────────────────────────────────

/// The single shared mutable state.
///
/// ```text
/// Arc<Mutex<Option<WorkloadState>>>
///  │    │      └─ None = no workload submitted yet
///  │    └─ tokio async Mutex: held across .await only when strictly needed
///  └─ shared by SchedInfoService, NodeService, and main
/// ```
pub type WorkloadStore = Arc<Mutex<Option<WorkloadState>>>;

/// Construct an empty `WorkloadStore`.
pub fn new_workload_store() -> WorkloadStore {
    Arc::new(Mutex::new(None))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::NodeSchedMap;

    fn dummy_hyperperiod() -> HyperperiodInfo {
        HyperperiodInfo {
            workload_id: "wl".into(),
            hyperperiod_us: 10_000,
            unique_periods: vec![10_000],
            task_count: 1,
        }
    }

    #[tokio::test]
    async fn new_workload_store_is_initially_none() {
        let store = new_workload_store();
        assert!(store.lock().await.is_none());
    }

    #[test]
    fn workload_state_new_derives_active_nodes_from_schedule_keys() {
        let mut schedule = NodeSchedMap::new();
        schedule.insert("node01".into(), vec![]);
        schedule.insert("node02".into(), vec![]);

        let state = WorkloadState::new("wl1".into(), schedule, dummy_hyperperiod());

        assert_eq!(state.workload_id, "wl1");
        assert_eq!(state.active_nodes.len(), 2);
        assert!(state.active_nodes.contains("node01"));
        assert!(state.active_nodes.contains("node02"));
        assert!(state.synced_nodes.is_empty());
    }

    #[test]
    fn workload_state_new_empty_schedule_has_no_active_nodes() {
        let state = WorkloadState::new("wl_empty".into(), NodeSchedMap::new(), dummy_hyperperiod());
        assert!(state.active_nodes.is_empty());
    }

    #[test]
    fn barrier_status_equality_and_inequality() {
        assert_eq!(BarrierStatus::Waiting, BarrierStatus::Waiting);
        assert_eq!(BarrierStatus::Cancelled, BarrierStatus::Cancelled);
        assert_eq!(
            BarrierStatus::Released {
                start_time_sec: 42,
                start_time_nsec: 0
            },
            BarrierStatus::Released {
                start_time_sec: 42,
                start_time_nsec: 0
            },
        );
        assert_eq!(BarrierStatus::TimedOut, BarrierStatus::TimedOut);
        assert_ne!(BarrierStatus::Waiting, BarrierStatus::Cancelled);
        assert_ne!(BarrierStatus::Waiting, BarrierStatus::TimedOut);
        assert_ne!(BarrierStatus::Cancelled, BarrierStatus::TimedOut);
        assert_ne!(
            BarrierStatus::Released {
                start_time_sec: 1,
                start_time_nsec: 0
            },
            BarrierStatus::Released {
                start_time_sec: 2,
                start_time_nsec: 0
            },
        );
    }
}
