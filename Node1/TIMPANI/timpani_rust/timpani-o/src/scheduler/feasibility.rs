/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Real-time scheduling feasibility analysis.
//!
//! # Status: implemented, pending management approval for enforcement
//!
//! The Liu & Layland bound is **computed and logged** after every scheduling
//! run.  It is currently a **warning only** — the schedule is returned even if
//! the bound is exceeded.  The practical hard gate is the
//! `CPU_UTILIZATION_THRESHOLD` of 90 % applied per-CPU during the scheduling
//! algorithms themselves.
//!
//! Once management confirms, the intent is to use the L&L bound to set
//! `CPU_UTILIZATION_THRESHOLD` dynamically (per node, based on the number of
//! tasks), rather than a fixed 90 % heuristic.
//!
//! # Theory
//! **Liu & Layland (1973)**: Under Rate Monotonic scheduling (shorter period →
//! higher priority), a task set of `n` independent periodic tasks is
//! **guaranteed** schedulable on one CPU if and only if:
//!
//! $$U = \sum_{i=1}^{n} \frac{C_i}{T_i} \leq n \left(2^{1/n} - 1\right)$$
//!
//! The bound tightens as `n` grows, converging to `ln(2) ≈ 0.693`.
//!
//! | n | Bound |
//! |---|---|
//! | 1 | 1.000 |
//! | 2 | 0.828 |
//! | 3 | 0.780 |
//! | 5 | 0.743 |
//! | ∞ | ln(2) ≈ 0.693 |
//!
//! If `U` is between the L&L bound and 1.0, the task set **may or may not** be
//! schedulable — deeper Response Time Analysis (RTA) is required.

use crate::task::Task;

// ── Public API ────────────────────────────────────────────────────────────────

/// Compute the Liu & Layland utilisation upper bound for `n` tasks.
///
/// `U_bound(n) = n × (2^(1/n) − 1)`
///
/// Returns `1.0` for `n = 1` (a single task always fits if `U ≤ 1`),
/// and `0.0` for `n = 0`.
pub fn liu_layland_bound(n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let nf = n as f64;
    nf * (2.0_f64.powf(1.0 / nf) - 1.0)
}

/// Check whether the tasks assigned to a single CPU/node satisfy the Liu &
/// Layland schedulability bound.
///
/// Returns `None` if the task set is **provably schedulable** (total
/// utilisation ≤ L&L bound).
///
/// Returns `Some(total_utilisation)` if the bound is **exceeded** — the
/// caller should emit a warning; the schedule is not automatically invalidated.
///
/// Tasks with `period_us == 0` are excluded from the utilisation sum (they
/// contribute zero utilisation by definition).
pub fn check_liu_layland(tasks_on_node: &[&Task]) -> Option<f64> {
    let feasible: Vec<&Task> = tasks_on_node
        .iter()
        .copied()
        .filter(|t| t.period_us > 0)
        .collect();

    if feasible.is_empty() {
        return None;
    }

    let total_u: f64 = feasible
        .iter()
        .map(|t| t.runtime_us as f64 / t.period_us as f64)
        .sum();

    let bound = liu_layland_bound(feasible.len());

    if total_u > bound {
        Some(total_u)
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;

    fn task_with_timing(period_us: u64, runtime_us: u64) -> Task {
        Task {
            period_us,
            runtime_us,
            ..Default::default()
        }
    }

    #[test]
    fn bound_zero_tasks_is_zero() {
        assert_eq!(liu_layland_bound(0), 0.0);
    }

    #[test]
    fn bound_one_task_is_one() {
        let b = liu_layland_bound(1);
        assert!((b - 1.0).abs() < 1e-10, "bound(1) should be 1.0, got {b}");
    }

    #[test]
    fn bound_two_tasks_is_approximately_0_828() {
        let b = liu_layland_bound(2);
        assert!((b - 0.8284).abs() < 1e-3, "bound(2) ≈ 0.828, got {b}");
    }

    #[test]
    fn bound_converges_toward_ln2() {
        // For large n the bound approaches ln(2) ≈ 0.6931
        let b = liu_layland_bound(1000);
        assert!(
            (b - 2.0_f64.ln()).abs() < 1e-3,
            "bound(1000) should be close to ln(2) ≈ 0.6931, got {b}"
        );
    }

    #[test]
    fn classic_three_task_set_is_feasible() {
        // From Liu & Layland's original paper:
        //   Task A: T=10ms, C=3ms  → U=0.30
        //   Task B: T=20ms, C=5ms  → U=0.25
        //   Task C: T=50ms, C=8ms  → U=0.16
        //   Total U = 0.71, bound(3) ≈ 0.780 → FEASIBLE
        let a = task_with_timing(10_000, 3_000);
        let b = task_with_timing(20_000, 5_000);
        let c = task_with_timing(50_000, 8_000);
        let result = check_liu_layland(&[&a, &b, &c]);
        assert!(
            result.is_none(),
            "classic 3-task set should be feasible, got utilization = {:?}",
            result
        );
    }

    #[test]
    fn overloaded_set_exceeds_bound() {
        // Three tasks each consuming 35% of a CPU → total 1.05, clearly not schedulable
        let a = task_with_timing(10_000, 3_500);
        let b = task_with_timing(10_000, 3_500);
        let c = task_with_timing(10_000, 3_500);
        let result = check_liu_layland(&[&a, &b, &c]);
        assert!(result.is_some(), "overloaded set should exceed bound");
        let u = result.unwrap();
        assert!(
            (u - 1.05).abs() < 1e-6,
            "utilization should be 1.05, got {u}"
        );
    }

    #[test]
    fn tasks_with_zero_period_are_excluded() {
        // A zero-period task is excluded; remaining single task (period=10, runtime=5 → U=0.5)
        // bound(1) = 1.0, so U=0.5 is feasible
        let zero = task_with_timing(0, 100);
        let valid = task_with_timing(10_000, 5_000);
        let result = check_liu_layland(&[&zero, &valid]);
        assert!(
            result.is_none(),
            "zero-period task should be excluded, set should be feasible"
        );
    }

    #[test]
    fn empty_task_set_is_feasible() {
        let result = check_liu_layland(&[]);
        assert!(result.is_none(), "empty set is trivially feasible");
    }

    #[test]
    fn boundary_exactly_at_bound_is_feasible() {
        // Construct one task with utilization exactly equal to bound(1) = 1.0
        // period=1000, runtime=1000 → U=1.0 exactly
        let t = task_with_timing(1_000, 1_000);
        let result = check_liu_layland(&[&t]);
        assert!(
            result.is_none(),
            "utilization == bound should be feasible (≤, not <)"
        );
    }
}
