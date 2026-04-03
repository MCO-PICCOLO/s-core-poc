/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::statemanager::{ErrorCode, ResourceType};
use std::collections::HashMap;
use tokio::time::Instant;
// ========================================
// CORE DATA STRUCTURES
// ========================================

/// Action execution command for async processing
#[derive(Debug, Clone)]
pub struct ActionCommand {
    pub action: String,
    pub resource_key: String,
    pub resource_type: ResourceType,
    pub transition_id: String,
    pub context: HashMap<String, String>,
}

/// Represents a state transition in the state machine
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransition {
    pub from_state: i32,
    pub event: String,
    pub to_state: i32,
    pub condition: Option<String>,
    pub action: String,
}

/// Health status tracking for resources
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub status_message: String,
    pub last_check: Instant,
    pub consecutive_failures: u32,
}

/// Represents the current state of a resource with metadata
#[derive(Debug, Clone)]
pub struct ResourceState {
    pub resource_type: ResourceType,
    pub resource_name: String,
    pub current_state: i32,
    pub desired_state: Option<i32>,
    pub last_transition_time: Instant,
    pub transition_count: u64,
    pub metadata: HashMap<String, String>,
    pub health_status: HealthStatus,
}

/// Result of a state transition attempt - aligned with proto StateChangeResponse
#[derive(Debug, Clone)]
pub struct TransitionResult {
    pub new_state: i32,
    pub error_code: ErrorCode,
    pub message: String,
    pub actions_to_execute: Vec<String>,
    pub transition_id: String,
    pub error_details: String,
}

/// Container state representation for internal processing
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Created,
    Initialized,
    Running,
    Paused,
    Exited,
    Unknown,
    Dead,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::time::Instant;

    #[test]
    fn test_state_transition_equality() {
        let t1 = StateTransition {
            from_state: 1,
            event: "ev".to_string(),
            to_state: 2,
            condition: Some("cond".to_string()),
            action: "act".to_string(),
        };
        let t2 = StateTransition {
            from_state: 1,
            event: "ev".to_string(),
            to_state: 2,
            condition: Some("cond".to_string()),
            action: "act".to_string(),
        };
        let t3 = StateTransition {
            from_state: 1,
            event: "other".to_string(),
            to_state: 3,
            condition: None,
            action: "act2".to_string(),
        };

        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
    }

    #[test]
    fn test_action_command_clone_and_independence() {
        let mut ctx = HashMap::new();
        ctx.insert("k".to_string(), "v".to_string());

        let cmd = ActionCommand {
            action: "doit".to_string(),
            resource_key: "rk".to_string(),
            resource_type: ResourceType::Scenario,
            transition_id: "tid".to_string(),
            context: ctx,
        };

        let mut cloned = cmd.clone();
        // Mutate clone's context and ensure original is unchanged
        cloned.context.insert("new".to_string(), "val".to_string());
        assert!(cmd.context.get("new").is_none());
        assert_eq!(cloned.context.get("new").unwrap(), "val");
        assert_eq!(cloned.resource_key, "rk");
    }

    #[test]
    fn test_container_state_variants_and_debug() {
        let a = ContainerState::Running;
        let b = ContainerState::Running;
        let c = ContainerState::Exited;
        assert_eq!(format!("{:?}", a), "Running");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_resource_state_construction() {
        use common::statemanager::ScenarioState;

        let now = Instant::now();
        let hs = HealthStatus {
            healthy: true,
            status_message: "ok".to_string(),
            last_check: now,
            consecutive_failures: 0,
        };

        let rs = ResourceState {
            resource_type: ResourceType::Scenario,
            resource_name: "rname".to_string(),
            current_state: ScenarioState::Idle as i32,
            desired_state: Some(ScenarioState::Waiting as i32),
            last_transition_time: now,
            transition_count: 0,
            metadata: HashMap::new(),
            health_status: hs.clone(),
        };

        assert_eq!(rs.resource_name, "rname");
        assert!(rs.health_status.healthy);
        assert_eq!(rs.desired_state.unwrap(), ScenarioState::Waiting as i32);
    }
}
