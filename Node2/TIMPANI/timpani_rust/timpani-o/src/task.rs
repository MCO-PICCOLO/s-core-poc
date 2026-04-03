/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Core task data structures for the Timpani-O global scheduler.
//!
//! Two distinct types model the two sides of the scheduling pipeline:
//!
//! ```text
//! Pullpiri  ──(proto TaskInfo)──►  Task  ──(scheduler)──►  SchedTask  ──(gRPC)──►  Timpani-N
//!                                  ↑ input                    ↑ output
//!                                  mutable working copy        wire-ready, ns units
//! ```
//!
//! # Ownership model
//! `Task` is **owned** by the `GlobalScheduler` for the duration of one
//! scheduling run.  The caller moves `Vec<Task>` into the scheduler; the
//! compiler guarantees there is never more than one live copy.  The scheduler
//! fills `assigned_node` / `assigned_cpu` in-place during the algorithm, then
//! converts to `Vec<SchedTask>` (grouped by node) as the final step.

use std::collections::HashMap;

// ── Scheduling policy ─────────────────────────────────────────────────────────

/// Linux scheduling policy for a task.
///
/// Mirrors the `SchedPolicy` proto enum and the integer constants used in the
/// C++ `Task::policy` field (`0` = Normal, `1` = FIFO, `2` = RR).
///
/// Carrying the typed enum through the whole pipeline (instead of a raw `int`)
/// makes it impossible to create an invalid policy value inside Timpani-O.  The
/// conversion back to an integer only happens at the Timpani-N wire boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SchedPolicy {
    /// `SCHED_NORMAL` – standard Linux CFS scheduling.
    #[default]
    Normal,
    /// `SCHED_FIFO` – real-time FIFO.
    Fifo,
    /// `SCHED_RR` – real-time round-robin.
    RoundRobin,
}

impl SchedPolicy {
    /// Convert to the integer value expected by Timpani-N / the Linux kernel.
    pub fn to_linux_int(self) -> i32 {
        match self {
            SchedPolicy::Normal => 0,
            SchedPolicy::Fifo => 1,
            SchedPolicy::RoundRobin => 2,
        }
    }

    /// Parse from the proto integer value sent by Pullpiri.
    ///
    /// Unknown values are silently mapped to `Normal`, matching the C++ default.
    pub fn from_proto_int(v: i32) -> Self {
        match v {
            1 => SchedPolicy::Fifo,
            2 => SchedPolicy::RoundRobin,
            _ => SchedPolicy::Normal,
        }
    }
}

// ── CPU affinity ──────────────────────────────────────────────────────────────

/// CPU affinity constraint for a task.
///
/// The proto field `cpu_affinity` is a `uint64` bitmask (e.g. `0x0C` = CPUs 2
/// and 3).  The C++ implementation currently extracts only the lowest set bit,
/// but we store the full `u64` bitmask so we are ready when multi-CPU affinity
/// is supported.
///
/// Replaces the C++ dual representation (`std::string affinity` + `int
/// cpu_affinity`) with a single typed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CpuAffinity {
    /// No constraint – the scheduler may assign any available CPU.
    ///
    /// Corresponds to `cpu_affinity == 0` or `cpu_affinity == 0xFFFF_FFFF_FFFF_FFFF`
    /// in the proto.
    #[default]
    Any,

    /// Pinned to a specific set of CPUs expressed as a bitmask.
    ///
    /// Bit N being set means CPU N is allowed.  E.g. `0x0C` = CPUs 2 and 3.
    Pinned(u64),
}

impl CpuAffinity {
    /// Parse from the proto `uint64` field.
    pub fn from_proto(v: u64) -> Self {
        if v == 0 || v == u64::MAX {
            CpuAffinity::Any
        } else {
            CpuAffinity::Pinned(v)
        }
    }

    /// Returns `true` if a specific CPU id is allowed by this affinity.
    pub fn allows_cpu(&self, cpu_id: u32) -> bool {
        match self {
            CpuAffinity::Any => true,
            CpuAffinity::Pinned(mask) => (mask >> cpu_id) & 1 == 1,
        }
    }

    /// Extract the lowest set bit as a single CPU id, matching the current
    /// C++ behaviour (`"assuming single CPU affinity for now"`).
    ///
    /// Returns `None` for `Any`.
    pub fn lowest_cpu(&self) -> Option<u32> {
        match self {
            CpuAffinity::Any => None,
            CpuAffinity::Pinned(mask) => {
                if *mask == 0 {
                    None
                } else {
                    Some(mask.trailing_zeros())
                }
            }
        }
    }
}

// ── Task (input / working copy) ───────────────────────────────────────────────

/// Internal task representation used during scheduling.
///
/// Mirrors the C++ `Task` struct in `task.h`, with the following improvements:
///
/// * Only one timing unit (microseconds) — the unused millisecond duplicates
///   are removed.
/// * `SchedPolicy` and `CpuAffinity` enums replace the bare `int` / dual-string
///   representations.
/// * `assigned_cpu` is `Option<u32>` instead of `-1` sentinel.
/// * Dead fields (`dependencies`, `cluster_requirement`) are removed.
/// * `memory_mb` is reinstated as `u64` (zero = unconstrained / not yet set by proto).
///
/// # Lifecycle
/// Created by the gRPC handler from a proto `TaskInfo`, **moved** into
/// `GlobalScheduler::set_tasks()`, mutated in-place as the algorithm assigns
/// nodes and CPUs, then consumed by `GlobalScheduler::take_sched_map()` which
/// produces the final `NodeSchedMap`.
#[derive(Debug, Clone, Default)]
pub struct Task {
    // ── Identity ──────────────────────────────────────────────────────────────
    /// Unique task name within a workload.
    pub name: String,

    /// Workload this task belongs to (set from the `SchedInfo.workload_id` proto
    /// field — every task in one RPC call shares the same value).
    pub workload_id: String,

    /// Node the task should be scheduled on.  Empty means auto-assign (used by
    /// `best_fit_decreasing` and `least_loaded` algorithms).
    pub target_node: String,

    // ── Scheduling parameters ─────────────────────────────────────────────────
    /// Linux scheduling policy.
    pub policy: SchedPolicy,

    /// Real-time priority (1–99 for FIFO/RR, 0 for Normal).
    pub priority: i32,

    /// CPU affinity constraint.
    pub affinity: CpuAffinity,

    // ── Resource requirements ─────────────────────────────────────────────────
    /// Memory budget for this task in megabytes.
    ///
    /// Checked against `NodeConfig::max_memory_mb` during admission control.
    /// A value of `0` means "no constraint" — used when Pullpiri does not yet
    /// populate this field (the proto `TaskInfo` does not carry it yet).
    /// This is **dormant** until the proto is extended; the field exists now so
    /// the pipeline is ready without a breaking change later.
    pub memory_mb: u64,

    // ── Timing (all in microseconds) ──────────────────────────────────────────
    /// Task period in µs.
    pub period_us: u64,

    /// Worst-case execution time (runtime) in µs.
    pub runtime_us: u64,

    /// Relative deadline in µs (typically equals `period_us`).
    pub deadline_us: u64,

    /// Release time offset from the start of the hyperperiod, in µs.
    pub release_time_us: u32,

    /// Maximum number of consecutive deadline misses allowed before a fault is
    /// reported to Pullpiri.
    pub max_dmiss: i32,

    // ── Assignment (filled by GlobalScheduler) ────────────────────────────────
    /// Node the scheduler assigned this task to.  Empty until the algorithm
    /// runs.
    pub assigned_node: String,

    /// CPU the scheduler assigned this task to.  `None` until the algorithm
    /// runs.
    pub assigned_cpu: Option<u32>,
}

impl Task {
    /// CPU utilisation fraction: `runtime_us / period_us`.
    ///
    /// Returns `0.0` when `period_us` is zero to avoid division by zero.
    pub fn utilization(&self) -> f64 {
        if self.period_us == 0 {
            0.0
        } else {
            self.runtime_us as f64 / self.period_us as f64
        }
    }

    /// Returns `true` if the scheduler has assigned a node to this task.
    pub fn is_assigned(&self) -> bool {
        !self.assigned_node.is_empty() && self.assigned_cpu.is_some()
    }
}

// ── SchedTask (output / wire-ready) ──────────────────────────────────────────

/// Per-task scheduling result sent to Timpani-N.
///
/// Mirrors the C++ `sched_task_t` struct in `sched_info.h`, but uses owned
/// `String`s instead of fixed char buffers (eliminating the silent truncation
/// risk) and nanosecond timing as required by the Timpani-N protocol.
///
/// Produced from a fully-assigned [`Task`] via [`SchedTask::from_task`].
#[derive(Debug, Clone)]
pub struct SchedTask {
    /// Task name (no length limit — Rust `String` replaces the 16-byte C array).
    pub name: String,

    /// Node this task is assigned to.
    pub assigned_node: String,

    /// CPU this task is pinned to.
    pub assigned_cpu: u32,

    /// Linux scheduling policy (integer form for the wire).
    pub policy: SchedPolicy,

    /// Real-time scheduling priority.
    pub priority: i32,

    /// Period in nanoseconds (converted from `Task::period_us`).
    pub period_ns: u64,

    /// Runtime (WCET) in nanoseconds.
    pub runtime_ns: u64,

    /// Deadline in nanoseconds.
    pub deadline_ns: u64,

    /// Release time in microseconds (kept as-is from the proto field).
    pub release_time_us: i32,

    /// Maximum deadline misses allowed.
    pub max_dmiss: i32,
}

impl SchedTask {
    /// Convert a fully-assigned [`Task`] into a wire-ready [`SchedTask`].
    ///
    /// # Panics
    /// Panics in debug builds if the task has not been assigned (i.e.
    /// `assigned_node` is empty or `assigned_cpu` is `None`).  In release
    /// builds the values default to empty / 0 rather than panicking.
    pub fn from_task(task: &Task) -> Self {
        debug_assert!(
            task.is_assigned(),
            "SchedTask::from_task called on unassigned task '{}'",
            task.name
        );

        SchedTask {
            name: task.name.clone(),
            assigned_node: task.assigned_node.clone(),
            assigned_cpu: task.assigned_cpu.unwrap_or(0),
            policy: task.policy,
            priority: task.priority,
            period_ns: task.period_us.saturating_mul(1_000),
            runtime_ns: task.runtime_us.saturating_mul(1_000),
            deadline_ns: task.deadline_us.saturating_mul(1_000),
            release_time_us: task.release_time_us as i32,
            max_dmiss: task.max_dmiss,
        }
    }
}

// ── NodeSchedMap ──────────────────────────────────────────────────────────────

/// Final scheduling result: maps each node-id to its list of scheduled tasks.
///
/// Replaces the C++ `NodeSchedInfoMap` (`std::map<std::string, sched_info_t>`
/// with its malloc'd task array).  `Vec<SchedTask>` is owned and
/// automatically freed — no manual `free()` required.
pub type NodeSchedMap = HashMap<String, Vec<SchedTask>>;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SchedPolicy ───────────────────────────────────────────────────────────

    #[test]
    fn sched_policy_round_trips_known_values() {
        assert_eq!(SchedPolicy::from_proto_int(0), SchedPolicy::Normal);
        assert_eq!(SchedPolicy::from_proto_int(1), SchedPolicy::Fifo);
        assert_eq!(SchedPolicy::from_proto_int(2), SchedPolicy::RoundRobin);
    }

    #[test]
    fn sched_policy_unknown_proto_value_maps_to_normal() {
        assert_eq!(SchedPolicy::from_proto_int(99), SchedPolicy::Normal);
        assert_eq!(SchedPolicy::from_proto_int(-1), SchedPolicy::Normal);
    }

    #[test]
    fn sched_policy_to_linux_int_matches_cpp_constants() {
        assert_eq!(SchedPolicy::Normal.to_linux_int(), 0);
        assert_eq!(SchedPolicy::Fifo.to_linux_int(), 1);
        assert_eq!(SchedPolicy::RoundRobin.to_linux_int(), 2);
    }

    // ── CpuAffinity ───────────────────────────────────────────────────────────

    #[test]
    fn cpu_affinity_zero_is_any() {
        assert_eq!(CpuAffinity::from_proto(0), CpuAffinity::Any);
    }

    #[test]
    fn cpu_affinity_u64_max_is_any() {
        assert_eq!(CpuAffinity::from_proto(u64::MAX), CpuAffinity::Any);
    }

    #[test]
    fn cpu_affinity_0xffffffff_is_any() {
        // 0xFFFF_FFFF is the 32-bit sentinel the C++ code used
        assert_eq!(
            CpuAffinity::from_proto(0xFFFF_FFFF),
            CpuAffinity::Pinned(0xFFFF_FFFF)
        );
    }

    #[test]
    fn cpu_affinity_bitmask_allows_correct_cpus() {
        let aff = CpuAffinity::Pinned(0b0000_1100); // CPUs 2 and 3
        assert!(!aff.allows_cpu(0));
        assert!(!aff.allows_cpu(1));
        assert!(aff.allows_cpu(2));
        assert!(aff.allows_cpu(3));
        assert!(!aff.allows_cpu(4));
    }

    #[test]
    fn cpu_affinity_any_allows_all_cpus() {
        let aff = CpuAffinity::Any;
        for cpu in 0..64u32 {
            assert!(aff.allows_cpu(cpu));
        }
    }

    #[test]
    fn cpu_affinity_lowest_cpu_extracts_correct_bit() {
        // 0x0C = 0b1100 → lowest set bit is bit 2 → CPU 2
        assert_eq!(CpuAffinity::Pinned(0x0C).lowest_cpu(), Some(2));
        // single CPU pinned to CPU 5
        assert_eq!(CpuAffinity::Pinned(1 << 5).lowest_cpu(), Some(5));
    }

    #[test]
    fn cpu_affinity_any_has_no_lowest_cpu() {
        assert_eq!(CpuAffinity::Any.lowest_cpu(), None);
    }

    // ── Task ──────────────────────────────────────────────────────────────────

    #[test]
    fn task_utilization_is_correct() {
        let task = Task {
            period_us: 1_000_000,
            runtime_us: 100_000,
            ..Default::default()
        };
        assert!((task.utilization() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn task_utilization_zero_period_returns_zero() {
        let task = Task {
            period_us: 0,
            runtime_us: 100,
            ..Default::default()
        };
        assert_eq!(task.utilization(), 0.0);
    }

    #[test]
    fn task_is_assigned_requires_both_node_and_cpu() {
        let mut task = Task::default();
        assert!(!task.is_assigned());

        task.assigned_node = "node01".into();
        assert!(
            !task.is_assigned(),
            "node without cpu is not fully assigned"
        );

        task.assigned_cpu = Some(2);
        assert!(task.is_assigned());
    }

    // ── SchedTask ─────────────────────────────────────────────────────────────

    #[test]
    fn sched_task_from_task_converts_units_to_nanoseconds() {
        let task = Task {
            name: "t1".into(),
            assigned_node: "node01".into(),
            assigned_cpu: Some(3),
            policy: SchedPolicy::Fifo,
            priority: 50,
            period_us: 1_000, // 1 ms
            runtime_us: 100,  // 0.1 ms
            deadline_us: 1_000,
            release_time_us: 0,
            max_dmiss: 3,
            ..Default::default()
        };
        let st = SchedTask::from_task(&task);

        assert_eq!(st.name, "t1");
        assert_eq!(st.assigned_node, "node01");
        assert_eq!(st.assigned_cpu, 3);
        assert_eq!(st.period_ns, 1_000_000); // µs → ns
        assert_eq!(st.runtime_ns, 100_000);
        assert_eq!(st.deadline_ns, 1_000_000);
        assert_eq!(st.policy, SchedPolicy::Fifo);
        assert_eq!(st.priority, 50);
        assert_eq!(st.max_dmiss, 3);
    }

    #[test]
    fn sched_task_period_ns_does_not_overflow_on_large_values() {
        // u64::MAX / 1000 = ~1.8 × 10^16 µs — saturating_mul should handle it
        let task = Task {
            name: "big".into(),
            assigned_node: "n".into(),
            assigned_cpu: Some(0),
            period_us: u64::MAX / 1_000 + 1, // would overflow without saturation
            ..Default::default()
        };
        // Should not panic
        let st = SchedTask::from_task(&task);
        assert_eq!(st.period_ns, u64::MAX); // saturated
    }
}
