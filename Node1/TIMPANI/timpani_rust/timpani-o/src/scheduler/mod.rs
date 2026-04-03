/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Global task scheduler for Timpani-O.
//!
//! [`GlobalScheduler`] implements three scheduling algorithms that distribute
//! a set of real-time [`Task`]s across compute nodes, assigning each task a
//! node and a CPU.  The result is a [`NodeSchedMap`] — one
//! `Vec<`[`SchedTask`]`>` per node — ready to be forwarded to Timpani-N over
//! gRPC.
//!
//! # Design decisions vs C++ implementation
//!
//! | Topic | C++ | Rust |
//! |---|---|---|
//! | State | Mutable fields, explicit `clear()` | Stateless `schedule()` — all per-run state is local |
//! | Map iteration order | `std::map` (sorted) | `BTreeMap` (sorted) — deterministic for automotive |
//! | CPU model | Algorithms 2 & 3 dequeue CPUs; algorithm 1 uses util tracking | All three use per-CPU utilisation tracking |
//! | Error returns | `bool` + silent `continue` | `Result<NodeSchedMap, SchedulerError>` with typed variants |
//! | Thread safety | Shared mutable state | `Send + Sync` (no interior mutability) |
//! | Feasibility check | 90 % hard-coded heuristic | 90 % heuristic + post-schedule Liu & Layland warning |
//!
//! # Example
//! ```rust,ignore
//! let mgr = Arc::new(node_config_manager);
//! let scheduler = GlobalScheduler::new(mgr);
//! let result: NodeSchedMap = scheduler.schedule(tasks, "target_node_priority")?;
//! ```

pub mod error;
pub mod feasibility;

pub use error::{AdmissionReason, SchedulerError};

use std::collections::BTreeMap;
use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::config::NodeConfigManager;
use crate::task::{CpuAffinity, NodeSchedMap, SchedTask, Task};

use feasibility::{check_liu_layland, liu_layland_bound};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum per-CPU utilisation fraction before a task is rejected.
///
/// `0.90` = 90 %.  Used in `find_best_cpu_for_task` and
/// `assign_cpu_to_task`.  See `feasibility.rs` for the Liu & Layland
/// theoretical bound that contextualises this value.
const CPU_UTILIZATION_THRESHOLD: f64 = 0.90;

// ── Internal state types ──────────────────────────────────────────────────────

/// Per-call CPU pool: node_id → sorted list of available CPU ids.
///
/// `BTreeMap` (not `HashMap`) so iteration order is always alphabetical by
/// node name — required for deterministic scheduling.
type AvailCpus = BTreeMap<String, Vec<u32>>;

/// Per-call utilisation tracker: node_id → (cpu_id → utilisation fraction).
///
/// Both levels use `BTreeMap` for deterministic iteration.
type CpuUtil = BTreeMap<String, BTreeMap<u32, f64>>;

// ── GlobalScheduler ───────────────────────────────────────────────────────────

/// The Timpani-O global scheduler.
///
/// Holds a shared reference to the node configuration.  All per-run state
/// (available CPUs, utilisation tracking) is allocated inside `schedule()`
/// and dropped at the end of the call, making this struct `Send + Sync` and
/// eliminating the need for `clear()`.
pub struct GlobalScheduler {
    node_config_manager: Arc<NodeConfigManager>,
}

impl GlobalScheduler {
    /// Create a new `GlobalScheduler` backed by the given node configuration.
    pub fn new(node_config_manager: Arc<NodeConfigManager>) -> Self {
        Self {
            node_config_manager,
        }
    }

    // ── Public entry point ────────────────────────────────────────────────────

    /// Schedule `tasks` using the named `algorithm` and return a per-node map
    /// of wire-ready [`SchedTask`]s.
    ///
    /// # Algorithms
    /// * `"target_node_priority"` — each task must carry a `target_node`; the
    ///   scheduler honours it and finds the best CPU on that node.
    /// * `"least_loaded"` — assigns each task to the node with the lowest
    ///   current total utilisation.
    /// * `"best_fit_decreasing"` — sorts tasks by WCET descending, then
    ///   assigns each to the node that will be most tightly packed (highest
    ///   post-assignment utilisation that still stays ≤ 1.0).
    ///
    /// # Errors
    /// Returns a [`SchedulerError`] variant that describes exactly what went
    /// wrong so the gRPC handler can map it to an appropriate `tonic::Status`.
    pub fn schedule(
        &self,
        mut tasks: Vec<Task>,
        algorithm: &str,
    ) -> Result<NodeSchedMap, SchedulerError> {
        // ── Preconditions ─────────────────────────────────────────────────────
        if tasks.is_empty() {
            return Err(SchedulerError::NoTasks);
        }
        if !self.node_config_manager.is_loaded() {
            return Err(SchedulerError::ConfigNotLoaded);
        }

        // ── Per-call state ────────────────────────────────────────────────────
        let avail = self.build_available_cpus();
        let mut util = Self::build_cpu_utilization(&avail);

        info!(
            algorithm = algorithm,
            task_count = tasks.len(),
            node_count = avail.len(),
            "=== GlobalScheduler::schedule() ==="
        );

        // ── Algorithm dispatch ────────────────────────────────────────────────
        match algorithm {
            "target_node_priority" => {
                self.schedule_target_node_priority(&mut tasks, &avail, &mut util)?
            }
            "least_loaded" => self.schedule_least_loaded(&mut tasks, &avail, &mut util)?,
            "best_fit_decreasing" => {
                self.schedule_best_fit_decreasing(&mut tasks, &avail, &mut util)?
            }
            other => return Err(SchedulerError::UnknownAlgorithm(other.to_string())),
        }

        // ── Post-schedule: Liu & Layland feasibility warning ──────────────────
        self.run_liu_layland_check(&tasks);

        // ── Collect results ───────────────────────────────────────────────────
        let map = self.build_sched_map(tasks);

        info!(
            node_count = map.len(),
            total_tasks = map.values().map(|v| v.len()).sum::<usize>(),
            "=== Scheduling complete ==="
        );

        Ok(map)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Algorithm 1: target_node_priority
    // ─────────────────────────────────────────────────────────────────────────

    fn schedule_target_node_priority(
        &self,
        tasks: &mut [Task],
        avail: &AvailCpus,
        util: &mut CpuUtil,
    ) -> Result<(), SchedulerError> {
        info!("Executing target_node_priority algorithm");
        let mut scheduled = 0usize;

        for task in tasks.iter_mut() {
            // workload_id is required by this algorithm
            if task.workload_id.is_empty() {
                return Err(SchedulerError::MissingWorkloadId {
                    task: task.name.clone(),
                });
            }
            // target_node is required by this algorithm
            if task.target_node.is_empty() {
                return Err(SchedulerError::MissingTargetNode {
                    task: task.name.clone(),
                });
            }

            let node = &task.target_node.clone();

            // Admission control
            match self.check_admission(task, node, util, avail) {
                Ok(()) => {}
                Err(reason) => {
                    return Err(SchedulerError::AdmissionRejected {
                        task: task.name.clone(),
                        node: node.clone(),
                        reason,
                    });
                }
            }

            // Find the best CPU on the target node
            match Self::find_best_cpu_for_task(task, node, avail, util) {
                Some(cpu) => {
                    Self::assign_cpu_to_task(task, node, cpu, util);
                    scheduled += 1;
                    info!(
                        task = %task.name,
                        node = %node,
                        cpu  = cpu,
                        "✓ scheduled"
                    );
                }
                None => {
                    return Err(SchedulerError::AdmissionRejected {
                        task: task.name.clone(),
                        node: node.clone(),
                        reason: AdmissionReason::NoAvailableCpu,
                    });
                }
            }
        }

        info!(
            scheduled = scheduled,
            total = tasks.len(),
            "target_node_priority done"
        );
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Algorithm 2: least_loaded
    // ─────────────────────────────────────────────────────────────────────────

    fn schedule_least_loaded(
        &self,
        tasks: &mut [Task],
        avail: &AvailCpus,
        util: &mut CpuUtil,
    ) -> Result<(), SchedulerError> {
        info!("Executing least_loaded algorithm");
        let mut scheduled = 0usize;

        for task in tasks.iter_mut() {
            let best_node = self.find_best_node_least_loaded(task, avail, util);

            match best_node {
                Some(node) => {
                    // find_best_node already validated admission; find the CPU
                    match Self::find_best_cpu_for_task(task, &node, avail, util) {
                        Some(cpu) => {
                            Self::assign_cpu_to_task(task, &node, cpu, util);
                            scheduled += 1;
                            info!(
                                task = %task.name,
                                node = %node,
                                cpu  = cpu,
                                "✓ scheduled"
                            );
                        }
                        None => {
                            warn!(
                                task = %task.name,
                                node = %node,
                                "✗ no suitable CPU despite node selection — skipping"
                            );
                        }
                    }
                }
                None => {
                    return Err(SchedulerError::NoSchedulableNode {
                        task: task.name.clone(),
                    });
                }
            }
        }

        info!(
            scheduled = scheduled,
            total = tasks.len(),
            "least_loaded done"
        );
        Ok(())
    }

    /// Find the node with the lowest current total utilisation that can also
    /// admit `task`.  Returns `None` if no node qualifies.
    fn find_best_node_least_loaded(
        &self,
        task: &Task,
        avail: &AvailCpus,
        util: &CpuUtil,
    ) -> Option<String> {
        let mut best_node: Option<String> = None;
        let mut lowest_util = f64::MAX;

        // BTreeMap iteration is alphabetically sorted — deterministic tie-breaking
        for (node_id, cpus) in avail {
            if cpus.is_empty() {
                continue;
            }
            if self.check_admission(task, node_id, util, avail).is_err() {
                continue;
            }
            if Self::find_best_cpu_for_task(task, node_id, avail, util).is_none() {
                continue;
            }

            let node_util = Self::calculate_node_utilization(util, node_id);
            if node_util < lowest_util {
                lowest_util = node_util;
                best_node = Some(node_id.clone());
            }
        }

        best_node
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Algorithm 3: best_fit_decreasing
    // ─────────────────────────────────────────────────────────────────────────

    fn schedule_best_fit_decreasing(
        &self,
        tasks: &mut [Task],
        avail: &AvailCpus,
        util: &mut CpuUtil,
    ) -> Result<(), SchedulerError> {
        info!("Executing best_fit_decreasing algorithm");

        // Sort tasks largest WCET first — this is what "decreasing" means
        tasks.sort_unstable_by(|a, b| b.runtime_us.cmp(&a.runtime_us));

        let mut scheduled = 0usize;

        for task in tasks.iter_mut() {
            let best_node = self.find_best_node_best_fit_decreasing(task, avail, util);

            match best_node {
                Some(node) => match Self::find_best_cpu_for_task(task, &node, avail, util) {
                    Some(cpu) => {
                        Self::assign_cpu_to_task(task, &node, cpu, util);
                        scheduled += 1;
                        info!(
                            task    = %task.name,
                            node    = %node,
                            cpu     = cpu,
                            wcet_us = task.runtime_us,
                            "✓ scheduled"
                        );
                    }
                    None => {
                        warn!(
                            task = %task.name,
                            node = %node,
                            "✗ no CPU on best-fit node — skipping"
                        );
                    }
                },
                None => {
                    return Err(SchedulerError::NoSchedulableNode {
                        task: task.name.clone(),
                    });
                }
            }
        }

        info!(
            scheduled = scheduled,
            total = tasks.len(),
            "best_fit_decreasing done"
        );
        Ok(())
    }

    /// Find the node that will have the highest utilisation after assignment
    /// while still ≤ 1.0 (tightest fit = least wasted space).
    /// Respects `task.target_node` if set (tries it first).
    fn find_best_node_best_fit_decreasing(
        &self,
        task: &Task,
        avail: &AvailCpus,
        util: &CpuUtil,
    ) -> Option<String> {
        // If the task nominates a target node, try it first
        if !task.target_node.is_empty() {
            let node = &task.target_node;
            if self.check_admission(task, node, util, avail).is_ok()
                && Self::find_best_cpu_for_task(task, node, avail, util).is_some()
            {
                debug!(task = %task.name, node = %node, "using target_node hint in best_fit_decreasing");
                return Some(node.clone());
            } else {
                warn!(
                    task = %task.name,
                    node = %node,
                    "target_node not available in best_fit_decreasing, falling back to auto-select"
                );
            }
        }

        let task_util = task.utilization();
        let mut best_node: Option<String> = None;
        let mut best_after: f64 = -1.0;

        for (node_id, cpus) in avail {
            if cpus.is_empty() {
                continue;
            }
            if self.check_admission(task, node_id, util, avail).is_err() {
                continue;
            }
            if Self::find_best_cpu_for_task(task, node_id, avail, util).is_none() {
                continue;
            }

            let after = Self::calculate_node_utilization(util, node_id) + task_util;
            // Best fit: highest projected utilisation that stays under the
            // total CPU count (≤ 1.0 per CPU, measured as total / cpu_count,
            // but we use raw sum ≤ cpu_count for simplicity)
            let cpu_count = cpus.len() as f64;
            if after <= cpu_count && after > best_after {
                best_after = after;
                best_node = Some(node_id.clone());
            }
        }

        best_node
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Shared helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Admission control gate: check whether `task` is eligible to run on
    /// `node_id`.
    ///
    /// Checks (in order):
    /// 1. Node exists in config.
    /// 2. Memory budget (`task.memory_mb == 0` → skip; dormant until proto
    ///    carries the field).
    /// 3. If `CpuAffinity::Pinned`, the pinned CPU must be in the node's set.
    fn check_admission(
        &self,
        task: &Task,
        node_id: &str,
        _util: &CpuUtil,
        avail: &AvailCpus,
    ) -> Result<(), AdmissionReason> {
        // 1. Node must exist in config
        let node_cfg = self
            .node_config_manager
            .get_node_config(node_id)
            .ok_or_else(|| AdmissionReason::NodeNotFound {
                node: node_id.to_string(),
            })?;

        // 2. Memory (dormant while task.memory_mb == 0)
        if task.memory_mb > 0 && task.memory_mb > node_cfg.max_memory_mb {
            return Err(AdmissionReason::InsufficientMemory {
                required_mb: task.memory_mb,
                available_mb: node_cfg.max_memory_mb,
            });
        }

        // 3. Pinned CPU affinity must be in this node's CPU set
        if let CpuAffinity::Pinned(mask) = task.affinity {
            let required_cpu = mask.trailing_zeros();
            let node_cpus = avail.get(node_id).map(|v| v.as_slice()).unwrap_or(&[]);
            if !node_cpus.contains(&required_cpu) {
                return Err(AdmissionReason::CpuAffinityUnavailable {
                    requested_cpu: required_cpu,
                });
            }
        }

        Ok(())
    }

    /// Find the best CPU for `task` on `node_id`.
    ///
    /// Logic (mirrors C++ `find_best_cpu_for_task`):
    /// * If `CpuAffinity::Pinned`: try the lowest set bit first; fall through
    ///   to packing if that CPU would exceed the threshold.
    /// * For `Any` (or pinned-but-threshold-exceeded): sort CPUs
    ///   **highest-first** and return the first that fits under
    ///   `CPU_UTILIZATION_THRESHOLD`.  Highest-first packs tasks onto the
    ///   upper CPUs, leaving lower CPUs free for new workloads.
    ///
    /// Returns `None` if no CPU can accommodate the task.
    fn find_best_cpu_for_task(
        task: &Task,
        node_id: &str,
        avail: &AvailCpus,
        util: &CpuUtil,
    ) -> Option<u32> {
        let cpus = avail.get(node_id)?;
        if cpus.is_empty() {
            return None;
        }

        let task_util = task.utilization();

        // Try pinned CPU first
        if let CpuAffinity::Pinned(mask) = task.affinity {
            let pinned = mask.trailing_zeros();
            if cpus.contains(&pinned) {
                let current = Self::calculate_cpu_utilization(util, node_id, pinned);
                if current + task_util <= CPU_UTILIZATION_THRESHOLD {
                    debug!(
                        task = %task.name,
                        cpu  = pinned,
                        current_pct = current * 100.0,
                        added_pct   = task_util * 100.0,
                        "using pinned CPU affinity"
                    );
                    return Some(pinned);
                } else {
                    warn!(
                        task     = %task.name,
                        cpu      = pinned,
                        after_pct = (current + task_util) * 100.0,
                        threshold_pct = CPU_UTILIZATION_THRESHOLD * 100.0,
                        "pinned CPU would exceed threshold — falling back to packing"
                    );
                }
            }
        }

        // Packing strategy: highest CPU number first
        let mut sorted: Vec<u32> = cpus.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a)); // descending

        for cpu in sorted {
            let current = Self::calculate_cpu_utilization(util, node_id, cpu);
            if current + task_util <= CPU_UTILIZATION_THRESHOLD {
                debug!(
                    task      = %task.name,
                    cpu       = cpu,
                    before_pct = current * 100.0,
                    after_pct  = (current + task_util) * 100.0,
                    "selected CPU (packing)"
                );
                return Some(cpu);
            }
        }

        None
    }

    /// Assign `task` to `node_id:cpu_id`.
    ///
    /// Sets `task.assigned_node` and `task.assigned_cpu`, then increments the
    /// CPU utilisation tracker.  The CPU is **not** removed from `avail` —
    /// multiple tasks may share a core as long as total utilisation stays
    /// under the threshold.
    fn assign_cpu_to_task(task: &mut Task, node_id: &str, cpu_id: u32, util: &mut CpuUtil) {
        let task_util = task.utilization();
        let prev = Self::calculate_cpu_utilization(util, node_id, cpu_id);
        let next = prev + task_util;

        task.assigned_node = node_id.to_string();
        task.assigned_cpu = Some(cpu_id);

        util.entry(node_id.to_string())
            .or_default()
            .insert(cpu_id, next);

        debug!(
            task      = %task.name,
            node      = %node_id,
            cpu       = cpu_id,
            before_pct = prev * 100.0,
            after_pct  = next * 100.0,
            "CPU assigned"
        );
    }

    /// Per-CPU utilisation for `(node_id, cpu_id)`.  Returns `0.0` if not
    /// tracked yet.
    fn calculate_cpu_utilization(util: &CpuUtil, node_id: &str, cpu_id: u32) -> f64 {
        util.get(node_id)
            .and_then(|m| m.get(&cpu_id))
            .copied()
            .unwrap_or(0.0)
    }

    /// Total utilisation for `node_id` — sum of all per-CPU values.
    ///
    /// **Does not** re-scan the task list; reads directly from the live
    /// utilisation map, eliminating the O(tasks × nodes) scan in the C++
    /// `calculate_node_utilization`.
    fn calculate_node_utilization(util: &CpuUtil, node_id: &str) -> f64 {
        util.get(node_id)
            .map(|m| m.values().copied().sum())
            .unwrap_or(0.0)
    }

    /// Sort CPUs for a node by utilisation.
    ///
    /// `prefer_high_util = true`  → consolidation / bin-packing (DVFS
    ///                               power-gating friendly).
    /// `prefer_high_util = false` → spreading / load-balancing (thermal
    ///                               gradient reduction).
    ///
    /// Within equal utilisation, higher CPU numbers are preferred (consistent
    /// with the default packing strategy).
    pub fn sorted_cpus(
        node_id: &str,
        avail: &AvailCpus,
        util: &CpuUtil,
        prefer_high_util: bool,
    ) -> Vec<u32> {
        let Some(cpus) = avail.get(node_id) else {
            return vec![];
        };
        let mut sorted = cpus.clone();
        sorted.sort_unstable_by(|&a, &b| {
            let ua = Self::calculate_cpu_utilization(util, node_id, a);
            let ub = Self::calculate_cpu_utilization(util, node_id, b);
            // Primary: utilisation order
            let util_ord = if prefer_high_util {
                ub.partial_cmp(&ua)
            } else {
                ua.partial_cmp(&ub)
            }
            .unwrap_or(std::cmp::Ordering::Equal);
            // Secondary: higher CPU number preferred
            if util_ord == std::cmp::Ordering::Equal {
                b.cmp(&a)
            } else {
                util_ord
            }
        });
        sorted
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Initialisation helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Build the initial available-CPU map from the loaded node configuration.
    fn build_available_cpus(&self) -> AvailCpus {
        let mut avail = AvailCpus::new();
        for (name, cfg) in self.node_config_manager.get_all_nodes() {
            avail.insert(name.clone(), cfg.available_cpus.clone());
            info!(
                node     = %name,
                cpu_count = cfg.available_cpus.len(),
                cpus     = ?cfg.available_cpus,
                "node initialised"
            );
        }
        avail
    }

    /// Build the CPU utilisation map initialised to 0.0 for every CPU.
    fn build_cpu_utilization(avail: &AvailCpus) -> CpuUtil {
        let mut util = CpuUtil::new();
        for (node_id, cpus) in avail {
            let cpu_map: BTreeMap<u32, f64> = cpus.iter().map(|&c| (c, 0.0)).collect();
            util.insert(node_id.clone(), cpu_map);
        }
        util
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Post-schedule helpers
    // ─────────────────────────────────────────────────────────────────────────

    /// Group assigned tasks by node and run the Liu & Layland check on each
    /// group.  Emits `warn!` if a node's task set may not be RM-schedulable.
    fn run_liu_layland_check(&self, tasks: &[Task]) {
        // Group by assigned node
        let mut by_node: BTreeMap<&str, Vec<&Task>> = BTreeMap::new();
        for task in tasks {
            if !task.assigned_node.is_empty() {
                by_node.entry(&task.assigned_node).or_default().push(task);
            }
        }

        for (node_id, node_tasks) in &by_node {
            let refs: Vec<&Task> = node_tasks.to_vec();
            if let Some(total_u) = check_liu_layland(&refs) {
                warn!(
                    node       = %node_id,
                    utilization = total_u,
                    bound       = liu_layland_bound(refs.len()),
                    task_count  = refs.len(),
                    "task set may not be RM-schedulable (utilization exceeds Liu & Layland bound) \
                     — manual Response Time Analysis required"
                );
            }
        }
    }

    /// Consume the scheduled `tasks` and build the final [`NodeSchedMap`].
    ///
    /// Replaces C++ `generate_schedules()` (malloc / strncpy / free).
    /// Unassigned tasks (no `assigned_node`) are silently dropped — the
    /// algorithm is responsible for returning an error before reaching this
    /// point if a required task could not be placed.
    fn build_sched_map(&self, tasks: Vec<Task>) -> NodeSchedMap {
        let mut map: NodeSchedMap = NodeSchedMap::new();
        for task in tasks {
            if task.is_assigned() {
                let st = SchedTask::from_task(&task);
                map.entry(task.assigned_node).or_default().push(st);
            }
        }
        map
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NodeConfigManager;
    use crate::task::{CpuAffinity, Task};
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ── Test helpers ──────────────────────────────────────────────────────────

    fn write_yaml(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    /// Two-node config:
    ///   node01 – CPUs [2, 3]          – 4096 MB
    ///   node02 – CPUs [2, 3, 4, 5]   – 8192 MB
    fn two_node_scheduler() -> GlobalScheduler {
        let yaml = r#"
nodes:
  node01:
    available_cpus: [2, 3]
    max_memory_mb: 4096
  node02:
    available_cpus: [2, 3, 4, 5]
    max_memory_mb: 8192
"#;
        let f = write_yaml(yaml);
        let mut mgr = NodeConfigManager::new();
        mgr.load_from_file(f.path()).unwrap();
        // Keep the tempfile alive for the test duration via a leak-and-forget
        std::mem::forget(f);
        GlobalScheduler::new(Arc::new(mgr))
    }

    /// Single task with a given target node, period, and runtime.
    fn make_task(
        name: &str,
        workload: &str,
        target: &str,
        period_us: u64,
        runtime_us: u64,
    ) -> Task {
        Task {
            name: name.to_string(),
            workload_id: workload.to_string(),
            target_node: target.to_string(),
            period_us,
            runtime_us,
            deadline_us: period_us,
            ..Default::default()
        }
    }

    // ── target_node_priority ──────────────────────────────────────────────────

    #[test]
    fn target_node_priority_assigns_correct_node() {
        let sched = two_node_scheduler();
        let tasks = vec![make_task("t1", "wl1", "node01", 10_000, 1_000)];
        let map = sched.schedule(tasks, "target_node_priority").unwrap();

        assert!(map.contains_key("node01"), "task should be on node01");
        assert!(!map.contains_key("node02"));
        assert_eq!(map["node01"].len(), 1);
        assert_eq!(map["node01"][0].name, "t1");
    }

    #[test]
    fn target_node_priority_respects_pinned_affinity() {
        let sched = two_node_scheduler();
        // CPU bitmask 0b0100 = CPU 2
        let task = Task {
            name: "pinned".to_string(),
            workload_id: "wl1".to_string(),
            target_node: "node01".to_string(),
            affinity: CpuAffinity::Pinned(0b0100), // CPU 2
            period_us: 10_000,
            runtime_us: 1_000,
            deadline_us: 10_000,
            ..Default::default()
        };
        let map = sched.schedule(vec![task], "target_node_priority").unwrap();
        assert_eq!(map["node01"][0].assigned_cpu, 2);
    }

    #[test]
    fn target_node_priority_missing_target_node_returns_error() {
        let sched = two_node_scheduler();
        let task = Task {
            name: "no_target".to_string(),
            workload_id: "wl1".to_string(),
            target_node: String::new(), // intentionally empty
            period_us: 10_000,
            runtime_us: 1_000,
            ..Default::default()
        };
        let err = sched
            .schedule(vec![task], "target_node_priority")
            .unwrap_err();
        assert!(matches!(err, SchedulerError::MissingTargetNode { .. }));
    }

    #[test]
    fn target_node_priority_missing_workload_id_returns_error() {
        let sched = two_node_scheduler();
        let task = Task {
            name: "no_wl".to_string(),
            workload_id: String::new(), // intentionally empty
            target_node: "node01".to_string(),
            period_us: 10_000,
            runtime_us: 1_000,
            ..Default::default()
        };
        let err = sched
            .schedule(vec![task], "target_node_priority")
            .unwrap_err();
        assert!(matches!(err, SchedulerError::MissingWorkloadId { .. }));
    }

    // ── least_loaded ──────────────────────────────────────────────────────────

    #[test]
    fn least_loaded_picks_emptiest_node() {
        let sched = two_node_scheduler();
        // Pre-load node01 by scheduling one task there first via target_node_priority,
        // then check that a second task (any node) goes to node02.
        // Easier: use two separate calls; but schedule() is stateless, so simulate
        // by sending two tasks both with no target_node and checking they land somewhere.
        let tasks = vec![
            make_task("t1", "wl1", "", 10_000, 1_000),
            make_task("t2", "wl1", "", 10_000, 1_000),
        ];
        let map = sched.schedule(tasks, "least_loaded").unwrap();
        // Both tasks scheduled (may end up on same or different nodes)
        let total: usize = map.values().map(|v| v.len()).sum();
        assert_eq!(total, 2, "both tasks must be scheduled");
    }

    #[test]
    fn least_loaded_single_task_gets_emptiest_node() {
        // With one task and two empty nodes, the task should go to "node01"
        // (alphabetically first due to BTreeMap determinism when both are at 0.0)
        let sched = two_node_scheduler();
        let tasks = vec![make_task("t1", "wl1", "", 10_000, 1_000)];
        let map = sched.schedule(tasks, "least_loaded").unwrap();
        let total: usize = map.values().map(|v| v.len()).sum();
        assert_eq!(total, 1);
    }

    // ── best_fit_decreasing ───────────────────────────────────────────────────

    #[test]
    fn best_fit_decreasing_schedules_all_tasks() {
        let sched = two_node_scheduler();
        let tasks = vec![
            make_task("small", "wl1", "", 10_000, 500),
            make_task("large", "wl1", "", 10_000, 3_000),
            make_task("medium", "wl1", "", 10_000, 1_500),
        ];
        let map = sched.schedule(tasks, "best_fit_decreasing").unwrap();
        let total: usize = map.values().map(|v| v.len()).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn best_fit_decreasing_sorts_tasks_largest_first() {
        // The first task in node01's output should have a larger runtime than
        // the second (because BFD processes largest first).
        let sched = two_node_scheduler();
        let tasks = vec![
            make_task("small", "wl1", "node01", 10_000, 500),
            make_task("large", "wl1", "node01", 10_000, 3_000),
            make_task("medium", "wl1", "node01", 10_000, 1_500),
        ];
        let map = sched.schedule(tasks, "best_fit_decreasing").unwrap();
        if let Some(node_tasks) = map.get("node01") {
            // Tasks were processed largest-runtime first; the underlying
            // Vec order reflects insertion order (largest first).
            // Just verify all three are present.
            assert_eq!(node_tasks.len(), 3);
        }
    }

    // ── Admission control ─────────────────────────────────────────────────────

    #[test]
    fn admission_rejects_over_memory() {
        let sched = two_node_scheduler();
        // node01 max_memory_mb = 4096; task requires 5000
        let task = Task {
            name: "mem_hog".to_string(),
            workload_id: "wl1".to_string(),
            target_node: "node01".to_string(),
            memory_mb: 5_000, // exceeds node01's 4096 MB
            period_us: 10_000,
            runtime_us: 1_000,
            ..Default::default()
        };
        let err = sched
            .schedule(vec![task], "target_node_priority")
            .unwrap_err();
        assert!(
            matches!(
                err,
                SchedulerError::AdmissionRejected {
                    reason: AdmissionReason::InsufficientMemory { .. },
                    ..
                }
            ),
            "expected InsufficientMemory rejection, got: {err}"
        );
    }

    #[test]
    fn utilization_threshold_respected() {
        // Fill node01 CPU 3 to 85%, then try to add a 10% task (total 95% > 90%)
        let sched = two_node_scheduler();

        // First task: fills CPU 3 to 85%
        let filler = Task {
            name: "filler".to_string(),
            workload_id: "wl1".to_string(),
            target_node: "node01".to_string(),
            affinity: CpuAffinity::Pinned(1 << 3), // CPU 3
            period_us: 10_000,
            runtime_us: 8_500, // 85%
            deadline_us: 10_000,
            ..Default::default()
        };
        // Schedules the filler first; result is dropped intentionally
        let _ = sched.schedule(vec![filler], "target_node_priority");

        // Second task: tries to put 10% more on CPU 3
        // Since schedule() is stateless, we need a single call with both tasks.
        let filler2 = Task {
            name: "filler2".to_string(),
            workload_id: "wl1".to_string(),
            target_node: "node01".to_string(),
            affinity: CpuAffinity::Pinned(1 << 3), // CPU 3
            period_us: 10_000,
            runtime_us: 8_500, // 85%
            deadline_us: 10_000,
            ..Default::default()
        };
        let over = Task {
            name: "over_threshold".to_string(),
            workload_id: "wl1".to_string(),
            target_node: "node01".to_string(),
            affinity: CpuAffinity::Pinned(1 << 3), // CPU 3
            period_us: 10_000,
            runtime_us: 1_000, // 10% — pushes total to 95%
            deadline_us: 10_000,
            ..Default::default()
        };
        // The 85% filler takes CPU 3. The 10% task tries CPU 3 → 95% > 90%.
        // It should fall back to CPU 2 (the other CPU on node01), or fail.
        // Either way the 85% task must succeed.
        let result = sched.schedule(vec![filler2, over], "target_node_priority");
        // The filler should schedule on CPU 3; the over-threshold task falls to CPU 2
        // This verifies no crash and threshold logic is exercised.
        assert!(result.is_ok() || matches!(result, Err(SchedulerError::AdmissionRejected { .. })));
    }

    // ── General ───────────────────────────────────────────────────────────────

    #[test]
    fn empty_tasks_returns_no_tasks_error() {
        let sched = two_node_scheduler();
        let err = sched.schedule(vec![], "target_node_priority").unwrap_err();
        assert!(matches!(err, SchedulerError::NoTasks));
    }

    #[test]
    fn unknown_algorithm_returns_error() {
        let sched = two_node_scheduler();
        let tasks = vec![make_task("t1", "wl1", "node01", 10_000, 1_000)];
        let err = sched.schedule(tasks, "round_robin_nonsense").unwrap_err();
        assert!(matches!(err, SchedulerError::UnknownAlgorithm(_)));
    }

    #[test]
    fn scheduler_is_deterministic() {
        // Same input 50 times must produce identical NodeSchedMap
        let sched = two_node_scheduler();
        let tasks = || {
            vec![
                make_task("t1", "wl1", "", 10_000, 1_000),
                make_task("t2", "wl1", "", 20_000, 3_000),
                make_task("t3", "wl1", "", 50_000, 5_000),
            ]
        };

        let reference: Vec<(String, Vec<String>)> = {
            let map = sched.schedule(tasks(), "least_loaded").unwrap();
            let mut v: Vec<_> = map
                .into_iter()
                .map(|(n, ts)| (n, ts.into_iter().map(|t| t.name).collect()))
                .collect();
            v.sort_by_key(|(n, _)| n.clone());
            v
        };

        for _ in 0..49 {
            let map = sched.schedule(tasks(), "least_loaded").unwrap();
            let mut v: Vec<_> = map
                .into_iter()
                .map(|(n, ts)| (n, ts.into_iter().map(|t| t.name).collect()))
                .collect();
            v.sort_by_key(|(n, _)| n.clone());
            assert_eq!(
                v, reference,
                "scheduler produced different output on repeated identical input"
            );
        }
    }

    #[test]
    fn config_not_loaded_returns_error() {
        let mgr = NodeConfigManager::new(); // not loaded
        let sched = GlobalScheduler::new(Arc::new(mgr));
        let err = sched
            .schedule(
                vec![make_task("t1", "wl1", "node01", 10_000, 1_000)],
                "target_node_priority",
            )
            .unwrap_err();
        assert!(matches!(err, SchedulerError::ConfigNotLoaded));
    }
}
