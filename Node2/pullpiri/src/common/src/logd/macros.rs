//! Convenience macros that bridge application code to the async logger.

/// Enqueue a formatted message into the async logger without awaiting.
///
/// # Arguments
/// * `$level` - Integer log level.
/// * `$($arg:tt)*` - `format!`-style tokens that build the message body.
#[macro_export]
macro_rules! logd {
    ($level:expr, $($arg:tt)*) => {{
        $crate::logd::logger::log_nowait(
            $level, format!($($arg)*)
        );
    }};
}
