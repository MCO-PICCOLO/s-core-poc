/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Pure arithmetic helpers: GCD and checked LCM.
//!
//! These are free functions rather than methods so they can be used and tested
//! independently of the `HyperperiodManager`.

use super::HyperperiodError;

/// Iterative Euclidean GCD.  Always returns `0` when either input is `0`.
///
/// The iterative form is preferred over the recursive one because it does not
/// risk a stack overflow for very large inputs.
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Checked LCM: returns `Err(HyperperiodError::Overflow { a, b })` if the
/// result would overflow `u64`.
///
/// Uses the overflow-safe formulation `(a / gcd(a, b)) * b` — the division
/// happens first, making overflow far less likely — but the final
/// multiplication is still checked with `checked_mul`.
///
/// Returns `Ok(0)` when either input is `0`, matching standard LCM convention.
pub fn lcm(a: u64, b: u64) -> Result<u64, HyperperiodError> {
    if a == 0 || b == 0 {
        return Ok(0);
    }

    let g = gcd(a, b);
    // a / g is exact (g divides a by definition)
    let reduced = a / g;

    reduced
        .checked_mul(b)
        .ok_or(HyperperiodError::Overflow { a, b })
}

/// Reduce a slice of periods to their overall LCM.
///
/// Returns:
/// * `Ok(0)` for an empty slice.
/// * `Err` on the first overflow encountered.
pub fn lcm_of_slice(periods: &[u64]) -> Result<u64, HyperperiodError> {
    periods
        .iter()
        .try_fold(periods.first().copied().unwrap_or(0), |acc, &p| lcm(acc, p))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── gcd ───────────────────────────────────────────────────────────────────

    #[test]
    fn gcd_basic_cases() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(7, 3), 1);
        assert_eq!(gcd(100, 25), 25);
    }

    #[test]
    fn gcd_with_zero() {
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(5, 0), 5);
        assert_eq!(gcd(0, 0), 0);
    }

    #[test]
    fn gcd_same_values() {
        assert_eq!(gcd(42, 42), 42);
    }

    #[test]
    fn gcd_coprime() {
        assert_eq!(gcd(17, 13), 1);
    }

    // ── lcm ───────────────────────────────────────────────────────────────────

    #[test]
    fn lcm_basic_cases() {
        assert_eq!(lcm(4, 6).unwrap(), 12);
        assert_eq!(lcm(3, 5).unwrap(), 15);
        assert_eq!(lcm(12, 18).unwrap(), 36);
    }

    #[test]
    fn lcm_with_zero_returns_zero() {
        assert_eq!(lcm(0, 5).unwrap(), 0);
        assert_eq!(lcm(5, 0).unwrap(), 0);
    }

    #[test]
    fn lcm_same_value() {
        assert_eq!(lcm(7, 7).unwrap(), 7);
    }

    #[test]
    fn lcm_overflow_returns_error() {
        // Two large coprime numbers whose LCM exceeds u64::MAX
        let a = u64::MAX / 2 + 1; // 9_223_372_036_854_775_808
        let b = u64::MAX / 2 + 3; // 9_223_372_036_854_775_810  (coprime to a)
        let result = lcm(a, b);
        assert!(matches!(result, Err(HyperperiodError::Overflow { .. })));
    }

    // ── realistic real-time periods (microseconds) ────────────────────────────

    #[test]
    fn lcm_typical_rt_periods_us() {
        // 1 ms, 2 ms, 5 ms, 10 ms — hyperperiod should be 10 ms
        assert_eq!(lcm(1_000, 2_000).unwrap(), 2_000);
        assert_eq!(lcm(2_000, 5_000).unwrap(), 10_000);
        assert_eq!(lcm(5_000, 10_000).unwrap(), 10_000);
    }

    // ── lcm_of_slice ──────────────────────────────────────────────────────────

    #[test]
    fn lcm_of_slice_empty_returns_zero() {
        assert_eq!(lcm_of_slice(&[]).unwrap(), 0);
    }

    #[test]
    fn lcm_of_slice_single_element() {
        assert_eq!(lcm_of_slice(&[42]).unwrap(), 42);
    }

    #[test]
    fn lcm_of_slice_multiple_periods() {
        // 1 ms, 2 ms, 4 ms → LCM = 4 ms
        assert_eq!(lcm_of_slice(&[1_000, 2_000, 4_000]).unwrap(), 4_000);
    }

    #[test]
    fn lcm_of_slice_all_same() {
        assert_eq!(lcm_of_slice(&[5_000, 5_000, 5_000]).unwrap(), 5_000);
    }

    #[test]
    fn lcm_of_slice_propagates_overflow_error() {
        let huge = u64::MAX / 2 + 1;
        let result = lcm_of_slice(&[huge, huge - 1]);
        assert!(result.is_err());
    }
}
