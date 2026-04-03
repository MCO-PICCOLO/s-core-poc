/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Hyperperiod calculation and management.
//!
//! The hyperperiod of a set of periodic tasks is the Least Common Multiple
//! (LCM) of all their periods.  It represents the smallest time window after
//! which the entire task set repeats.
//!
//! # Improvements over the C++ implementation
//!
//! | C++ issue | Rust fix |
//! |-----------|----------|
//! | `CalculateHyperperiod` returns `0` for both "no tasks" and "overflow" — caller cannot distinguish them | `Result<HyperperiodInfo, HyperperiodError>` — each failure case is a distinct variant |
//! | `(a / gcd) * b` overflows silently | `checked_mul` — overflow is `Err(Overflow)` |
//! | Warning-only sanity check — scheduler proceeds with a multi-hour hyperperiod | `Err(TooLarge)` — caller decides whether to reject or warn |
//! | `CalculateHyperperiod(workload_id, tasks)` copies the whole vector into a filtered sub-vector | `&[Task]` borrow + `filter` iterator — zero copies |

pub mod math;

use std::collections::HashMap;

use tracing::{debug, info, warn};

use crate::task::Task;
use math::lcm_of_slice;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default upper limit on hyperperiod (1 hour in microseconds).
///
/// Matches the C++ warning threshold.  Callers that want a different limit can
/// pass their own value to [`HyperperiodManager::with_limit`].
pub const DEFAULT_HYPERPERIOD_LIMIT_US: u64 = 3_600_000_000; // 1 h

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors that can occur during hyperperiod calculation.
#[derive(Debug, PartialEq, Eq)]
pub enum HyperperiodError {
    /// The task slice was empty (or all tasks had `period_us == 0`).
    NoValidPeriods,

    /// LCM calculation overflowed `u64`.
    ///
    /// Contains the two operands that caused the overflow so the caller can
    /// log a useful message.
    Overflow { a: u64, b: u64 },

    /// The calculated hyperperiod exceeded the configured limit.
    ///
    /// This is not necessarily a hard error — the caller can choose to warn
    /// and continue, or reject the workload.
    TooLarge { value_us: u64, limit_us: u64 },
}

impl std::fmt::Display for HyperperiodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HyperperiodError::NoValidPeriods => {
                write!(f, "no tasks with a valid (non-zero) period")
            }
            HyperperiodError::Overflow { a, b } => {
                write!(f, "LCM overflow computing lcm({a}, {b})")
            }
            HyperperiodError::TooLarge { value_us, limit_us } => write!(
                f,
                "hyperperiod {value_us}µs ({:.1}s) exceeds limit {limit_us}µs ({:.1}s)",
                *value_us as f64 / 1_000_000.0,
                *limit_us as f64 / 1_000_000.0
            ),
        }
    }
}

impl std::error::Error for HyperperiodError {}

// ── HyperperiodInfo ───────────────────────────────────────────────────────────

/// Calculated hyperperiod result for one workload.
///
/// Mirrors the C++ `HyperperiodInfo` struct.
#[derive(Debug, Clone)]
pub struct HyperperiodInfo {
    /// Workload identifier this hyperperiod belongs to.
    pub workload_id: String,

    /// Hyperperiod in microseconds (LCM of all unique task periods).
    pub hyperperiod_us: u64,

    /// Unique periods present in the workload (sorted, deduplicated).
    pub unique_periods: Vec<u64>,

    /// Number of tasks in the workload that contributed to this hyperperiod.
    pub task_count: usize,
}

// ── HyperperiodManager ────────────────────────────────────────────────────────

/// Calculates and stores hyperperiod information per workload.
///
/// Mirrors the C++ `HyperperiodManager` class.
///
/// # Example
/// ```rust
/// use timpani_o::hyperperiod::HyperperiodManager;
/// use timpani_o::task::{Task, SchedPolicy, CpuAffinity};
///
/// let mut mgr = HyperperiodManager::new();
///
/// let tasks = vec![
///     Task { workload_id: "w1".into(), period_us: 1_000, ..Default::default() },
///     Task { workload_id: "w1".into(), period_us: 2_000, ..Default::default() },
/// ];
///
/// let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
/// assert_eq!(info.hyperperiod_us, 2_000);
/// ```
#[derive(Debug)]
pub struct HyperperiodManager {
    /// Per-workload hyperperiod results.
    map: HashMap<String, HyperperiodInfo>,

    /// Upper bound on the hyperperiod.  A calculated value above this limit
    /// causes [`HyperperiodError::TooLarge`] to be returned.
    limit_us: u64,
}

impl HyperperiodManager {
    /// Create a manager with the default 1-hour limit.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            limit_us: DEFAULT_HYPERPERIOD_LIMIT_US,
        }
    }

    /// Create a manager with a custom hyperperiod limit (in microseconds).
    pub fn with_limit(limit_us: u64) -> Self {
        Self {
            map: HashMap::new(),
            limit_us,
        }
    }

    /// Calculate and store the hyperperiod for `workload_id`.
    ///
    /// # Arguments
    /// * `workload_id` – identifier for the workload being processed.
    /// * `tasks` – slice of **all** tasks available to the caller; only those
    ///   whose `workload_id` field matches the `workload_id` argument are used.
    ///   This intentional design (matching the C++ API) allows callers to pass
    ///   a larger pool of tasks without pre-filtering.
    ///
    /// # Errors
    /// * [`HyperperiodError::NoValidPeriods`] – no tasks matched or all
    ///   periods were zero.
    /// * [`HyperperiodError::Overflow`] – LCM computation exceeded `u64`.
    /// * [`HyperperiodError::TooLarge`] – result exceeds the configured limit.
    pub fn calculate_hyperperiod(
        &mut self,
        workload_id: &str,
        tasks: &[Task],
    ) -> Result<&HyperperiodInfo, HyperperiodError> {
        // Filter to tasks belonging to this workload with a non-zero period
        let matching: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.workload_id == workload_id && t.period_us > 0)
            .collect();

        if matching.is_empty() {
            warn!("No tasks with valid periods found for workload '{workload_id}'");
            return Err(HyperperiodError::NoValidPeriods);
        }

        // Collect unique periods (sorted for deterministic output)
        let unique_periods: Vec<u64> = {
            let mut v: Vec<u64> = matching.iter().map(|t| t.period_us).collect();
            v.sort_unstable();
            v.dedup();
            v
        };

        let hyperperiod_us = lcm_of_slice(&unique_periods)?;

        // Sanity-check: too-large hyperperiod — return Err so caller decides
        if hyperperiod_us > self.limit_us {
            warn!(
                hyperperiod_us,
                limit_us = self.limit_us,
                workload_id,
                "Hyperperiod exceeds configured limit"
            );
            return Err(HyperperiodError::TooLarge {
                value_us: hyperperiod_us,
                limit_us: self.limit_us,
            });
        }

        info!(
            workload_id,
            task_count = matching.len(),
            unique_count = unique_periods.len(),
            hyperperiod_ms = hyperperiod_us / 1_000,
            "Calculated hyperperiod"
        );
        for p in &unique_periods {
            debug!(period_us = p, period_ms = p / 1_000, "  unique period");
        }

        let info = HyperperiodInfo {
            workload_id: workload_id.to_string(),
            hyperperiod_us,
            unique_periods,
            task_count: matching.len(),
        };

        self.map.insert(workload_id.to_string(), info);

        // Return a reference to the stored value
        Ok(self.map.get(workload_id).unwrap())
    }

    /// Look up the stored hyperperiod for `workload_id`.
    ///
    /// Returns `None` if `calculate_hyperperiod` has not been called for this
    /// workload, or if it was cleared.  Mirrors `GetHyperperiodInfo()`.
    pub fn get(&self, workload_id: &str) -> Option<&HyperperiodInfo> {
        self.map.get(workload_id)
    }

    /// Returns `true` if a hyperperiod has been stored for `workload_id`.
    pub fn has(&self, workload_id: &str) -> bool {
        self.map.contains_key(workload_id)
    }

    /// Remove the hyperperiod entry for `workload_id`.
    ///
    /// Mirrors `ClearWorkload()`.
    pub fn clear_workload(&mut self, workload_id: &str) {
        if self.map.remove(workload_id).is_some() {
            info!("Cleared hyperperiod for workload '{workload_id}'");
        }
    }

    /// Remove all stored hyperperiod entries.
    ///
    /// Mirrors `Clear()`.
    pub fn clear_all(&mut self) {
        if !self.map.is_empty() {
            info!(
                "Cleared hyperperiod data for {} workload(s)",
                self.map.len()
            );
            self.map.clear();
        }
    }

    /// Read-only access to all stored hyperperiod entries.
    pub fn all(&self) -> &HashMap<String, HyperperiodInfo> {
        &self.map
    }
}

impl Default for HyperperiodManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;

    fn make_task(workload_id: &str, period_us: u64) -> Task {
        Task {
            workload_id: workload_id.into(),
            period_us,
            ..Default::default()
        }
    }

    // ── calculate_hyperperiod ─────────────────────────────────────────────────

    #[test]
    fn basic_hyperperiod_two_periods() {
        let tasks = vec![make_task("w1", 1_000), make_task("w1", 2_000)];
        let mut mgr = HyperperiodManager::new();
        let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert_eq!(info.hyperperiod_us, 2_000);
        assert_eq!(info.task_count, 2);
    }

    #[test]
    fn hyperperiod_three_periods_lcm() {
        // LCM(1000, 2000, 5000) = 10000
        let tasks = vec![
            make_task("w1", 1_000),
            make_task("w1", 2_000),
            make_task("w1", 5_000),
        ];
        let mut mgr = HyperperiodManager::new();
        let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert_eq!(info.hyperperiod_us, 10_000);
    }

    #[test]
    fn hyperperiod_all_same_period() {
        let tasks = vec![
            make_task("w1", 5_000),
            make_task("w1", 5_000),
            make_task("w1", 5_000),
        ];
        let mut mgr = HyperperiodManager::new();
        let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert_eq!(info.hyperperiod_us, 5_000);
        // Three tasks but only one unique period
        assert_eq!(info.unique_periods.len(), 1);
        assert_eq!(info.task_count, 3);
    }

    #[test]
    fn hyperperiod_single_task() {
        let tasks = vec![make_task("w1", 4_000)];
        let mut mgr = HyperperiodManager::new();
        let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert_eq!(info.hyperperiod_us, 4_000);
    }

    // ── workload_id filter ────────────────────────────────────────────────────

    #[test]
    fn filters_to_matching_workload_only() {
        // Mix of two workloads — only w1 should be considered
        let tasks = vec![
            make_task("w1", 1_000),
            make_task("w2", 3_000), // different workload — ignored
            make_task("w1", 2_000),
        ];
        let mut mgr = HyperperiodManager::new();
        let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        // LCM(1000, 2000) = 2000, NOT LCM(1000, 2000, 3000) = 6000
        assert_eq!(info.hyperperiod_us, 2_000);
        assert_eq!(info.task_count, 2);
    }

    #[test]
    fn empty_tasks_returns_no_valid_periods_error() {
        let mut mgr = HyperperiodManager::new();
        let result = mgr.calculate_hyperperiod("w1", &[]);
        assert_eq!(result.unwrap_err(), HyperperiodError::NoValidPeriods);
    }

    #[test]
    fn all_zero_periods_returns_no_valid_periods_error() {
        let tasks = vec![make_task("w1", 0), make_task("w1", 0)];
        let mut mgr = HyperperiodManager::new();
        let result = mgr.calculate_hyperperiod("w1", &tasks);
        assert_eq!(result.unwrap_err(), HyperperiodError::NoValidPeriods);
    }

    #[test]
    fn no_matching_workload_returns_no_valid_periods_error() {
        let tasks = vec![make_task("w2", 1_000)];
        let mut mgr = HyperperiodManager::new();
        let result = mgr.calculate_hyperperiod("w1", &tasks);
        assert_eq!(result.unwrap_err(), HyperperiodError::NoValidPeriods);
    }

    // ── too-large limit ───────────────────────────────────────────────────────

    #[test]
    fn hyperperiod_exceeding_limit_returns_too_large_error() {
        let tasks = vec![
            make_task("w1", 1_000_000), // 1 s
            make_task("w1", 7_000_000), // 7 s  → LCM = 7 s
        ];
        // Set limit to 5 seconds
        let mut mgr = HyperperiodManager::with_limit(5_000_000);
        let result = mgr.calculate_hyperperiod("w1", &tasks);
        assert!(matches!(
            result,
            Err(HyperperiodError::TooLarge {
                value_us: 7_000_000,
                ..
            })
        ));
    }

    #[test]
    fn hyperperiod_at_exactly_the_limit_is_accepted() {
        let tasks = vec![make_task("w1", 5_000_000)];
        let mut mgr = HyperperiodManager::with_limit(5_000_000);
        let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert_eq!(info.hyperperiod_us, 5_000_000);
    }

    // ── get / has ─────────────────────────────────────────────────────────────

    #[test]
    fn get_returns_stored_info() {
        let tasks = vec![make_task("w1", 1_000)];
        let mut mgr = HyperperiodManager::new();
        mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert!(mgr.has("w1"));
        assert_eq!(mgr.get("w1").unwrap().hyperperiod_us, 1_000);
    }

    #[test]
    fn get_returns_none_for_unknown_workload() {
        let mgr = HyperperiodManager::new();
        assert!(!mgr.has("unknown"));
        assert!(mgr.get("unknown").is_none());
    }

    // ── clear_workload / clear_all ────────────────────────────────────────────

    #[test]
    fn clear_workload_removes_entry() {
        let tasks = vec![make_task("w1", 1_000)];
        let mut mgr = HyperperiodManager::new();
        mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert!(mgr.has("w1"));
        mgr.clear_workload("w1");
        assert!(!mgr.has("w1"));
    }

    #[test]
    fn clear_workload_noop_for_unknown() {
        let mut mgr = HyperperiodManager::new();
        // Should not panic
        mgr.clear_workload("nonexistent");
    }

    #[test]
    fn clear_all_removes_everything() {
        let t1 = vec![make_task("w1", 1_000)];
        let t2 = vec![make_task("w2", 2_000)];
        let mut mgr = HyperperiodManager::new();
        mgr.calculate_hyperperiod("w1", &t1).unwrap();
        mgr.calculate_hyperperiod("w2", &t2).unwrap();
        assert_eq!(mgr.all().len(), 2);
        mgr.clear_all();
        assert_eq!(mgr.all().len(), 0);
    }

    // ── recalculate replaces previous entry ───────────────────────────────────

    #[test]
    fn recalculate_overwrites_previous_result() {
        let tasks_v1 = vec![make_task("w1", 1_000)];
        let tasks_v2 = vec![make_task("w1", 3_000)];

        let mut mgr = HyperperiodManager::new();
        mgr.calculate_hyperperiod("w1", &tasks_v1).unwrap();
        assert_eq!(mgr.get("w1").unwrap().hyperperiod_us, 1_000);

        mgr.calculate_hyperperiod("w1", &tasks_v2).unwrap();
        assert_eq!(mgr.get("w1").unwrap().hyperperiod_us, 3_000);
    }

    // ── unique_periods are sorted and deduplicated ────────────────────────────

    #[test]
    fn unique_periods_are_sorted_and_deduped() {
        let tasks = vec![
            make_task("w1", 5_000),
            make_task("w1", 1_000),
            make_task("w1", 5_000),
            make_task("w1", 2_000),
        ];
        let mut mgr = HyperperiodManager::new();
        let info = mgr.calculate_hyperperiod("w1", &tasks).unwrap();
        assert_eq!(info.unique_periods, vec![1_000, 2_000, 5_000]);
    }
}
