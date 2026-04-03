/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

//! Integration tests for timpani-n

use timpani_n::config::Config;
use timpani_n::context::Context;
use timpani_n::run_app;

#[test]
fn test_run_app_integration() {
    let config = Config::default();
    assert!(run_app(config).is_ok());
}

#[test]
fn test_full_lifecycle_with_various_configs() {
    // Test with default config
    let config = Config::default();
    assert!(run_app(config).is_ok());

    // Test with CPU affinity
    let config = Config {
        cpu: 2,
        ..Default::default()
    };
    assert!(run_app(config).is_ok());

    // Test with priority
    let config = Config {
        prio: 50,
        ..Default::default()
    };
    assert!(run_app(config).is_ok());

    // Test with all flags enabled
    let config = Config {
        enable_sync: true,
        enable_plot: true,
        enable_apex: true,
        ..Default::default()
    };
    assert!(run_app(config).is_ok());

    // Test with different log levels
    for level in 0..=5 {
        let mut config = Config::default();
        config.log_level = timpani_n::config::LogLevel::from_u8(level).unwrap();
        assert!(run_app(config).is_ok());
    }
}

#[test]
fn test_context_lifecycle() {
    let config = Config::default();
    let mut ctx = Context::new(config);

    // Initialize
    assert!(ctx.initialize().is_ok());

    // Cleanup
    ctx.cleanup();
}

#[test]
fn test_multiple_context_instances() {
    // Test creating multiple context instances
    for i in 0..5 {
        let config = Config {
            node_id: format!("node-{}", i),
            ..Default::default()
        };
        let mut ctx = Context::new(config);
        assert!(ctx.initialize().is_ok());
        ctx.cleanup();
    }
}
