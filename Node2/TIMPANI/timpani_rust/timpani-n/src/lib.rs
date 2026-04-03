/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

pub mod config;
pub mod context;
pub mod error;

use config::Config;
use context::Context;
use error::TimpaniResult;
use tracing::info;
use tracing_subscriber::fmt::SubscriberBuilder;

/// Initialize logging with the specified log level
pub fn init_logging(log_level: config::LogLevel) {
    let _ = SubscriberBuilder::default()
        .with_max_level(log_level_to_tracing_level(log_level))
        .with_target(false)
        .try_init();
}

/// Convert LogLevel to tracing::Level
fn log_level_to_tracing_level(log_level: config::LogLevel) -> tracing::Level {
    match log_level {
        config::LogLevel::Silent => tracing::Level::ERROR,
        config::LogLevel::Error => tracing::Level::ERROR,
        config::LogLevel::Warning => tracing::Level::WARN,
        config::LogLevel::Info => tracing::Level::INFO,
        config::LogLevel::Debug => tracing::Level::DEBUG,
        config::LogLevel::Verbose => tracing::Level::TRACE,
    }
}

/// Initialize the context
pub fn initialize(ctx: &mut Context) -> TimpaniResult<()> {
    ctx.initialize()
}

/// Run the main loop
pub fn run(_ctx: &mut Context) -> TimpaniResult<()> {
    info!("Runtime loop not yet implemented");
    Ok(())
}

/// Main application logic, extracted for testability
pub fn run_app(config: Config) -> TimpaniResult<()> {
    config.log_config();
    let mut ctx = Context::new(config);
    initialize(&mut ctx)?;
    run(&mut ctx)?;
    ctx.cleanup();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_initialization() {
        let config = Config::default();
        let mut ctx = Context::new(config);
        assert!(initialize(&mut ctx).is_ok());
    }

    #[test]
    fn test_run_with_default_context() {
        let config = Config::default();
        let mut ctx = Context::new(config);
        assert!(run(&mut ctx).is_ok());
    }

    #[test]
    fn test_run_app_success() {
        let config = Config::default();
        assert!(run_app(config).is_ok());
    }

    #[test]
    fn test_run_app_with_custom_config() {
        let config = Config {
            cpu: config::test_values::TEST_CPU_AFFINITY,
            prio: config::test_values::TEST_PRIORITY,
            node_id: config::test_values::TEST_NODE_ID.to_string(),
            log_level: config::LogLevel::Debug,
            ..Default::default()
        };

        assert!(run_app(config).is_ok());
    }

    #[test]
    fn test_initialize_multiple_times() {
        let config = Config::default();
        let mut ctx = Context::new(config);

        // Initialize should be idempotent
        assert!(initialize(&mut ctx).is_ok());
        assert!(initialize(&mut ctx).is_ok());
    }

    #[test]
    fn test_context_lifecycle() {
        let config = Config::default();
        let mut ctx = Context::new(config);

        // Full lifecycle
        assert!(initialize(&mut ctx).is_ok());
        assert!(run(&mut ctx).is_ok());
        ctx.cleanup();
    }

    #[test]
    fn test_run_app_lifecycle() {
        // Test the full run_app function which includes all lifecycle steps
        let config = Config::default();
        assert!(run_app(config).is_ok());

        // Test with various configurations
        let config = Config {
            cpu: config::test_values::TEST_CPU_ZERO,
            prio: config::test_values::TEST_PRIORITY_LOW,
            ..Default::default()
        };
        assert!(run_app(config).is_ok());

        let config = Config {
            enable_sync: true,
            enable_plot: true,
            ..Default::default()
        };
        assert!(run_app(config).is_ok());
    }

    #[test]
    fn test_log_level_mapping() {
        // Test that all log level mappings are correct
        use config::LogLevel;

        assert_eq!(
            log_level_to_tracing_level(LogLevel::Silent),
            tracing::Level::ERROR
        );
        assert_eq!(
            log_level_to_tracing_level(LogLevel::Error),
            tracing::Level::ERROR
        );
        assert_eq!(
            log_level_to_tracing_level(LogLevel::Warning),
            tracing::Level::WARN
        );
        assert_eq!(
            log_level_to_tracing_level(LogLevel::Info),
            tracing::Level::INFO
        );
        assert_eq!(
            log_level_to_tracing_level(LogLevel::Debug),
            tracing::Level::DEBUG
        );
        assert_eq!(
            log_level_to_tracing_level(LogLevel::Verbose),
            tracing::Level::TRACE
        );
    }

    #[test]
    fn test_all_log_levels_conversion() {
        // Comprehensive test for all log levels
        use config::LogLevel;

        let levels = vec![
            LogLevel::Silent,
            LogLevel::Error,
            LogLevel::Warning,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Verbose,
        ];

        for level in levels {
            // Just ensure conversion works for all levels
            let _tracing_level = log_level_to_tracing_level(level);
        }
    }

    #[test]
    fn test_run_and_initialize_combinations() {
        // Test various initialization and run combinations
        let configs = vec![
            Config::default(),
            Config {
                cpu: config::test_values::TEST_CPU_ONE,
                ..Default::default()
            },
            Config {
                prio: config::test_values::TEST_PRIORITY_MID,
                ..Default::default()
            },
            Config {
                enable_sync: true,
                enable_plot: true,
                enable_apex: true,
                ..Default::default()
            },
        ];

        for config in configs {
            let mut ctx = Context::new(config);
            assert!(initialize(&mut ctx).is_ok());
            assert!(run(&mut ctx).is_ok());
            ctx.cleanup();
        }
    }

    #[test]
    fn test_error_handling_in_run_app() {
        // Test run_app with valid configurations
        let config = Config {
            log_level: config::LogLevel::Debug,
            ..Default::default()
        };
        assert!(run_app(config).is_ok());

        let config = Config {
            log_level: config::LogLevel::Silent,
            ..Default::default()
        };
        assert!(run_app(config).is_ok());

        let config = Config {
            log_level: config::LogLevel::Verbose,
            ..Default::default()
        };
        assert!(run_app(config).is_ok());
    }

    #[test]
    fn test_init_logging() {
        // Test init_logging with various log levels
        // Uses try_init so it won't fail if already initialized
        init_logging(config::LogLevel::Info);
        init_logging(config::LogLevel::Debug);
        init_logging(config::LogLevel::Error);
    }

    #[test]
    fn test_init_logging_all_levels() {
        // Test all log levels
        for level_num in config::log_level::SILENT..=config::test_values::LOG_LEVEL_RANGE_MAX {
            let level = config::LogLevel::from_u8(level_num).unwrap();
            init_logging(level);
        }
    }
}
