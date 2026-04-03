/// Stub implementation of the Eclipse S-CORE `score_log` crate.
///
/// This shim exists so the FEO source tree (which depends on the Bazel-only
/// `@score_baselibs_rust//src/log/score_log`) can be built with plain `cargo`.
/// It forwards logging macros to the standard `log` crate and provides
/// no-op or minimal implementations of the custom formatting traits.

// ── Re-export the #[derive(ScoreDebug)] proc-macro ───────────────────────────
// Occupies the *macro* namespace; does not conflict with the ScoreDebug trait
// that occupies the *type* namespace below.
pub use score_log_derive::ScoreDebug;

// ── Re-export standard log macros ─────────────────────────────────────────────
pub use log::{debug, error, info, trace, warn, LevelFilter};

/// Forward to `log::set_max_level`.
pub fn set_max_level(level: LevelFilter) {
    log::set_max_level(level);
}

// ── fmt module ────────────────────────────────────────────────────────────────
pub mod fmt {
    /// Stub write target for ScoreDebug formatting (analogous to `std::fmt::Write`).
    pub trait ScoreWrite {
        fn write_str(&mut self, s: &str, spec: &FormatSpec) -> Result;
        fn write_u64(&mut self, v: &u64, spec: &FormatSpec) -> Result;
    }

    /// Stub format specification (no fields needed for the stub).
    #[derive(Default)]
    pub struct FormatSpec;

    /// Stub unordered debug set builder.
    pub struct DebugSet;

    impl DebugSet {
        pub fn new(_w: &mut dyn ScoreWrite, _spec: &FormatSpec) -> Self {
            DebugSet
        }
        pub fn entry<T: ScoreDebug + ?Sized>(&mut self, _value: &T) -> &mut Self {
            self
        }
        pub fn entries<'a, T: ScoreDebug + 'a, I: Iterator<Item = &'a T>>(&mut self, _iter: I) -> &mut Self {
            self
        }
        pub fn finish(&mut self) -> Result {
            Ok(())
        }
    }

    /// Error type mirrors `std::fmt::Error`.
    pub type Error = std::fmt::Error;

    /// Result type for ScoreDebug formatting.
    pub type Result = std::result::Result<(), Error>;

    /// The core custom-debug trait.
    pub trait ScoreDebug {
        fn fmt(&self, w: &mut dyn ScoreWrite, spec: &FormatSpec) -> Result;
    }

    impl ScoreDebug for f64 {
        fn fmt(&self, _w: &mut dyn ScoreWrite, _spec: &FormatSpec) -> Result {
            Ok(())
        }
    }

    impl ScoreDebug for f32 {
        fn fmt(&self, _w: &mut dyn ScoreWrite, _spec: &FormatSpec) -> Result {
            Ok(())
        }
    }
}
