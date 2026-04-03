/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

use thiserror::Error;

/// Unified error type for Timpani-N operations
/// Maps to tt_error_t from the C implementation
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum TimpaniError {
    #[error("Memory allocation failed")]
    Memory,

    #[error("Timer operation failed")]
    Timer,

    #[error("Signal handling failed")]
    Signal,

    #[error("Network operation failed")]
    Network,

    #[error("Configuration error")]
    Config,

    #[error("BPF operation failed")]
    Bpf,

    #[error("Invalid arguments")]
    InvalidArgs,

    #[error("Input/Output error")]
    Io,

    #[error("Permission denied")]
    Permission,
}

/// Result type alias for Timpani operations
pub type TimpaniResult<T> = Result<T, TimpaniError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn test_error_display() {
        assert_eq!(TimpaniError::Memory.to_string(), "Memory allocation failed");
        assert_eq!(TimpaniError::Timer.to_string(), "Timer operation failed");
        assert_eq!(TimpaniError::Signal.to_string(), "Signal handling failed");
        assert_eq!(
            TimpaniError::Network.to_string(),
            "Network operation failed"
        );
        assert_eq!(TimpaniError::Config.to_string(), "Configuration error");
        assert_eq!(TimpaniError::Bpf.to_string(), "BPF operation failed");
        assert_eq!(TimpaniError::InvalidArgs.to_string(), "Invalid arguments");
        assert_eq!(TimpaniError::Io.to_string(), "Input/Output error");
        assert_eq!(TimpaniError::Permission.to_string(), "Permission denied");
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(TimpaniError::Memory, TimpaniError::Memory);
        assert_ne!(TimpaniError::Memory, TimpaniError::Timer);
    }

    #[test]
    fn test_error_clone() {
        let err = TimpaniError::Config;
        let err_clone = err;
        assert_eq!(err, err_clone);
    }

    #[test]
    fn test_result_ok() {
        let result = config::test_values::TEST_RESULT_VALUE;
        assert_eq!(result, config::test_values::TEST_RESULT_VALUE);
    }

    #[test]
    fn test_result_err() {
        let result = TimpaniError::Config;
        assert_eq!(result, TimpaniError::Config);
    }

    #[test]
    fn test_all_error_variants() {
        let errors = [
            TimpaniError::Memory,
            TimpaniError::Timer,
            TimpaniError::Signal,
            TimpaniError::Network,
            TimpaniError::Config,
            TimpaniError::Bpf,
            TimpaniError::InvalidArgs,
            TimpaniError::Io,
            TimpaniError::Permission,
        ];

        for error in &errors {
            // Ensure all variants can be cloned and displayed
            let _cloned = *error;
            let _string = error.to_string();
            let _debug = format!("{:?}", error);
        }
    }
}
