/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

use crate::config::Config;

/// Runtime state structure
/// Maps to context.runtime from C implementation
#[derive(Debug, Default)]
pub struct RuntimeState {
    /// Shutdown request flag
    pub shutdown_requested: bool,
    // TODO: Add fields as we port more modules:
    // - tt_list (time trigger task list)
    // - sched_info (scheduling information)
    // - starttimer_ts (start timer timestamp)
    // - apex_list (Apex.OS task list)
}

/// Communication state structure
/// Maps to context.comm from C implementation
#[derive(Debug, Default)]
pub struct CommState {
    // TODO: Add fields as we port more modules:
    // - event (systemd event loop)
    // - dbus (D-Bus connection)
    // - apex_fd (Apex.OS Monitor Socket FD)
}

/// Hyperperiod manager structure
/// Maps to context.hp_manager from C implementation
#[derive(Debug, Default)]
pub struct HyperperiodManager {
    // TODO: Add fields as we port hyperperiod module:
    // - hyperperiod_us
    // - current_cycle
    // - workload_id
    // - etc.
}

/// Main context structure for Timpani-N
/// Maps to the C struct context
/// Centralizes all state and configuration
#[derive(Debug)]
pub struct Context {
    /// System configuration
    pub config: Config,

    /// Runtime state (dynamic state during execution)
    pub runtime: RuntimeState,

    /// Communication state (D-Bus, event loop)
    pub comm: CommState,

    /// Hyperperiod manager
    pub hp_manager: HyperperiodManager,
}

impl Context {
    /// Create a new context with the given configuration
    pub fn new(config: Config) -> Self {
        Context {
            config,
            runtime: RuntimeState::default(),
            comm: CommState::default(),
            hp_manager: HyperperiodManager::default(),
        }
    }

    /// Initialize the context (placeholder for future initialization logic)
    pub fn initialize(&mut self) -> crate::error::TimpaniResult<()> {
        // TODO: Add initialization logic as we port more modules:
        // - setup_signal_handlers
        // - set_affinity
        // - set_schedattr
        // - calibrate_bpf_time_offset
        // - init_trpc
        // - init_task_list or init_apex_list
        // - apex_monitor_init

        Ok(())
    }

    /// Cleanup resources (placeholder for future cleanup logic)
    pub fn cleanup(&mut self) {
        // TODO: Add cleanup logic as we port more modules:
        // - cleanup time triggers
        // - cleanup BPF resources
        // - cleanup network connections
        // - cleanup hyperperiod manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let config = Config::default();
        let ctx = Context::new(config);
        assert!(!ctx.runtime.shutdown_requested);
    }

    #[test]
    fn test_runtime_default() {
        let runtime = RuntimeState::default();
        assert!(!runtime.shutdown_requested);
    }

    #[test]
    fn test_context_initialization() {
        let config = Config::default();
        let mut ctx = Context::new(config);
        assert!(ctx.initialize().is_ok());
    }

    #[test]
    fn test_context_cleanup() {
        let config = Config::default();
        let mut ctx = Context::new(config);
        ctx.cleanup(); // Should not panic
    }

    #[test]
    fn test_comm_state_default() {
        let comm = CommState::default();
        // Just ensure it constructs without issues
        let _ = format!("{:?}", comm);
    }

    #[test]
    fn test_hyperperiod_manager_default() {
        let hp_mgr = HyperperiodManager::default();
        // Just ensure it constructs without issues
        let _ = format!("{:?}", hp_mgr);
    }

    #[test]
    fn test_context_with_custom_config() {
        let mut config = Config::default();
        config.cpu = crate::config::test_values::TEST_CPU_AFFINITY;
        config.prio = crate::config::test_values::TEST_PRIORITY;
        config.node_id = crate::config::test_values::TEST_NODE_ID_SHORT.to_string();

        let mut ctx = Context::new(config);
        assert!(ctx.initialize().is_ok());
        ctx.cleanup();
    }
}
