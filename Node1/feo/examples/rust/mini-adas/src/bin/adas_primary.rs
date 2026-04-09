/********************************************************************************
 * Copyright (c) 2025 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

#[cfg(not(feature = "com_mw"))]
use feo::agent::com_init::initialize_com_primary;
use feo::ids::AgentId;
use feo_time::Duration;
#[cfg(feature = "com_mw")]
use mini_adas::config::init_mw_com_runtime;
#[cfg(not(feature = "com_mw"))]
use mini_adas::config::{agent_assignments_ids, topic_dependencies, COM_BACKEND, MAX_ADDITIONAL_SUBSCRIBERS};
use score_log::{error, info, warn, LevelFilter};
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use stdout_logger::StdoutLoggerBuilder;

#[cfg(feature = "lifecycle")]
use feo::scheduler::CycleObserver;
#[cfg(feature = "lifecycle")]
use health_monitoring_lib::*;
#[cfg(feature = "lifecycle")]
use serde::Deserialize;

const AGENT_ID: AgentId = AgentId::new(100);
const DEFAULT_FEO_CYCLE_TIME: Duration = Duration::from_secs(5);

#[cfg(feature = "lifecycle")]
const HM_CONFIG_PATH: &str = "./etc/hm_config.json";

#[cfg(feature = "lifecycle")]
/// Health Monitor configuration loaded from JSON
#[derive(Debug, Deserialize)]
struct HmConfig {
    monitor_tag: String,
    deadline_tag: String,
    deadline_window: DeadlineWindow,
    health_monitor: HealthMonitorCycles,
}

#[cfg(feature = "lifecycle")]
#[derive(Debug, Deserialize)]
struct DeadlineWindow {
    min_ms: u64,
    max_ms: u64,
}

#[cfg(feature = "lifecycle")]
#[derive(Debug, Deserialize)]
struct HealthMonitorCycles {
    internal_processing_cycle_ms: u64,
    supervisor_api_cycle_ms: u64,
}

#[cfg(feature = "lifecycle")]
struct HmCycleObserver {
    deadline: deadline::Deadline,
    running: bool,
}

#[cfg(feature = "lifecycle")]
impl HmCycleObserver {
    fn new(deadline: deadline::Deadline) -> Self {
        Self {
            deadline,
            running: false,
        }
    }
}

#[cfg(feature = "lifecycle")]
impl CycleObserver for HmCycleObserver {
    fn on_cycle_start(&mut self) -> Result<(), &'static str> {
        if self.running {
            return Err("deadline cycle already running");
        }

        self.deadline
            .start_cycle()
            .map_err(|_| "failed to start HM deadline cycle")?;
        self.running = true;
        Ok(())
    }

    fn on_cycle_end(&mut self) -> Result<(), &'static str> {
        if self.running {
            self.deadline.stop_cycle();
            self.running = false;
        }
        Ok(())
    }
}

fn main() {
    let log_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LevelFilter::Info);

    StdoutLoggerBuilder::new()
        .context("adas-primary")
        .show_module(false)
        .show_file(false)
        .show_line(false)
        .log_level(log_level)
        .set_as_default_logger();

    #[cfg(feature = "lifecycle")]
    {
        // Set process name from environment (for Launch Manager)
        set_process_name();
    }

    let params = Params::from_args();

    info!("Starting primary agent {} with cycle time {:?}", AGENT_ID, params.feo_cycle_time);

    #[cfg(feature = "lifecycle")]
    let (mut hm, deadline_monitor) = {
        // Load Health Monitor configuration
        let hm_config = load_hm_config();
        info!("Loaded HM config: deadline window [{}, {}] ms",
              hm_config.deadline_window.min_ms,
              hm_config.deadline_window.max_ms);

        // Initialize Health Monitor
        let (hm, dm) = initialize_health_monitor(&hm_config)
            .unwrap_or_else(|err| {
                error!("Failed to initialize Health Monitor: {:?}", err);
                std::process::exit(1);
            });

        info!("Health Monitor initialized");
        (hm, dm)
    };

    #[cfg(feature = "lifecycle")]
    {
        // Start Health Monitor background thread
        hm.start();
        info!("Health Monitor started");
    }

    #[cfg(feature = "lifecycle")]
    let hm_cycle_observer = {
        let hm_config = load_hm_config();
        let deadline = deadline_monitor
            .get_deadline(DeadlineTag::from(hm_config.deadline_tag.as_str()))
            .expect("Failed to get deadline");
        info!("Configuring per-cycle deadline monitoring for FEO execution");
        Some(Box::new(HmCycleObserver::new(deadline)) as Box<dyn CycleObserver>)
    };

    #[cfg(not(feature = "lifecycle"))]
    let hm_cycle_observer = None;

    let config = cfg::make_config(params, hm_cycle_observer);

    // Initialize topics. Do not drop.
    #[cfg(not(feature = "com_mw"))]
    let _topic_guards = initialize_com_primary(
        COM_BACKEND,
        AGENT_ID,
        topic_dependencies(),
        &agent_assignments_ids(),
        MAX_ADDITIONAL_SUBSCRIBERS,
    );

    // Initialize MW COM
    #[cfg(feature = "com_mw")]
    init_mw_com_runtime(AGENT_ID);

    #[cfg(feature = "lifecycle")]
    {
        // Report Running state after COM setup so dependent processes start when IPC is already prepared.
        if !lifecycle_client_rs::report_execution_state_running() {
            warn!("Failed to report execution state to Launch Manager (LM daemon not running?)");
            warn!("Continuing with HM-only mode - deadline monitoring active");
        } else {
            info!("Reported RUNNING state to Launch Manager");
        }
    }

    // Setup primary
    let mut primary = cfg::Primary::new(config)
        .unwrap_or_else(|err| {
            error!("Failed to initialize primary agent: {:?}", err);
            std::process::exit(1);
        });

    #[cfg(feature = "lifecycle")]
    {
        // Setup signal handler for graceful shutdown
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);
        if let Err(e) = signal_hook::flag::register(signal_hook::consts::SIGTERM, shutdown_clone) {
            error!("Failed to register SIGTERM handler: {:?}", e);
        }
        let shutdown_clone = Arc::clone(&shutdown);
        if let Err(e) = signal_hook::flag::register(signal_hook::consts::SIGINT, shutdown_clone) {
            error!("Failed to register SIGINT handler: {:?}", e);
        }

        // Run primary - the scheduler starts/stops the HM deadline every cycle.
        primary.run()
            .unwrap_or_else(|err| {
                error!("Primary agent execution failed: {:?}", err);
                std::process::exit(1);
            });
    }

    #[cfg(not(feature = "lifecycle"))]
    {
        // Run primary without Health Monitor (standalone mode)
        info!("Running in standalone mode (without lifecycle integration)");
        primary.run()
            .unwrap_or_else(|err| {
                error!("Primary agent execution failed: {:?}", err);
                std::process::exit(1);
            });
    }

    info!("Primary agent shutting down");
}

#[cfg(feature = "lifecycle")]
/// Set process name from PROCESSIDENTIFIER environment variable
fn set_process_name() {
    if let Ok(val) = std::env::var("PROCESSIDENTIFIER") {
        let name = std::ffi::CString::new(val).expect("CString::new failed");
        #[cfg(target_os = "linux")]
        unsafe {
            libc::prctl(libc::PR_SET_NAME, name.as_ptr());
        }
        #[cfg(target_os = "nto")]
        unsafe {
            libc::pthread_setname_np(libc::pthread_self(), name.as_ptr());
        }
    }
}

#[cfg(feature = "lifecycle")]
/// Load Health Monitor configuration from JSON file
fn load_hm_config() -> HmConfig {
    let config_path = std::path::Path::new(HM_CONFIG_PATH);
    let config_data = std::fs::read_to_string(config_path)
        .unwrap_or_else(|err| {
            error!("Failed to read HM config from {}: {:?}", HM_CONFIG_PATH, err);
            error!("Using default HM configuration");
            // Return default config as JSON string
            r#"{
                "monitor_tag": "adas_primary_monitor",
                "deadline_tag": "feo_cycle_deadline",
                "deadline_window": {"min_ms": 200, "max_ms": 600},
                "health_monitor": {"internal_processing_cycle_ms": 50, "supervisor_api_cycle_ms": 50}
            }"#.to_string()
        });

    serde_json::from_str(&config_data)
        .unwrap_or_else(|err| {
            error!("Failed to parse HM config: {:?}, using defaults", err);
            panic!("Invalid HM configuration");
        })
}

#[cfg(feature = "lifecycle")]
/// Initialize Health Monitor with deadline monitoring
fn initialize_health_monitor(config: &HmConfig) -> Result<(HealthMonitor, deadline::DeadlineMonitor), HealthMonitorError> {
    // Build deadline monitor
    let deadline_builder = deadline::DeadlineMonitorBuilder::new()
        .add_deadline(
            DeadlineTag::from(config.deadline_tag.as_str()),
            TimeRange::new(
                std::time::Duration::from_millis(config.deadline_window.min_ms),
                std::time::Duration::from_millis(config.deadline_window.max_ms),
            ),
        );

    // Build health monitor
    let mut hm = HealthMonitorBuilder::new()
        .add_deadline_monitor(MonitorTag::from(config.monitor_tag.as_str()), deadline_builder)
        .with_internal_processing_cycle(std::time::Duration::from_millis(
            config.health_monitor.internal_processing_cycle_ms,
        ))
        .with_supervisor_api_cycle(std::time::Duration::from_millis(
            config.health_monitor.supervisor_api_cycle_ms,
        ))
        .build()?;

    // Get deadline monitor handle
    let deadline_monitor = hm
        .get_deadline_monitor(MonitorTag::from(config.monitor_tag.as_str()))
        .expect("Failed to get deadline monitor");

    Ok((hm, deadline_monitor))
}

/// Parameters of the primary
struct Params {
    /// Cycle time in milli seconds
    feo_cycle_time: Duration,
    /// Recorder IDs
    #[allow(dead_code)]
    recorder_ids: Vec<AgentId>,
}

impl Params {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();

        // First argument is the cycle time in milli seconds, e.g. 30 or 2500
        let feo_cycle_time = args
            .get(1)
            .and_then(|x| x.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(DEFAULT_FEO_CYCLE_TIME);

        // Second argument are the recorder IDs to wait for as dot-separated list, e.g. 900 or 900.901
        let recorder_ids = args
            .get(2)
            .and_then(|s| {
                s.split('.')
                    .map(|id| id.parse::<u64>().map(AgentId::from))
                    .collect::<Result<_, _>>()
                    .ok()
            })
            .unwrap_or_default();

        Self {
            feo_cycle_time,
            recorder_ids,
        }
    }
}

#[cfg(feature = "signalling_direct_mpsc")]
mod cfg {
    use super::{Duration, Params, AGENT_ID};
    use mini_adas::config::{activity_dependencies, agent_assignments};

    pub(super) use feo::agent::direct::primary_mpsc::{Primary, PrimaryConfig};

    pub(super) fn make_config(
        params: Params,
        cycle_observer: Option<Box<dyn feo::scheduler::CycleObserver>>,
    ) -> PrimaryConfig {
        PrimaryConfig {
            id: AGENT_ID,
            cycle_time: params.feo_cycle_time,
            activity_dependencies: activity_dependencies(),
            // With only one agent, we cannot attach a recorder
            recorder_ids: vec![],
            worker_assignments: agent_assignments().remove(&AGENT_ID).unwrap(),
            timeout: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(10),
            cycle_observer,
        }
    }
}

#[cfg(feature = "signalling_direct_tcp")]
mod cfg {
    use super::{check_ids, Duration, Params, AGENT_ID};
    use feo::{
        agent::NodeAddress,
        ids::{ActivityId, AgentId, WorkerId},
    };
    use mini_adas::config::{activity_dependencies, agent_assignments, worker_agent_map, BIND_ADDR};
    use std::collections::{HashMap, HashSet};

    pub(super) use feo::agent::direct::primary::{Primary, PrimaryConfig};

    pub(super) fn make_config(
        params: Params,
        cycle_observer: Option<Box<dyn feo::scheduler::CycleObserver>>,
    ) -> PrimaryConfig {
        let agent_ids: HashSet<AgentId> = agent_assignments().keys().copied().collect();
        check_ids(&params.recorder_ids, &agent_ids);

        let activity_worker_map: HashMap<ActivityId, WorkerId> = agent_assignments()
            .values()
            .flat_map(|vec| {
                vec.iter()
                    .flat_map(move |(wid, aid_b)| aid_b.iter().map(|v| (v.0, *wid)))
            })
            .collect();

        PrimaryConfig {
            id: AGENT_ID,
            cycle_time: params.feo_cycle_time,
            activity_dependencies: activity_dependencies(),
            recorder_ids: params.recorder_ids,
            worker_assignments: agent_assignments().remove(&AGENT_ID).unwrap(),
            timeout: Duration::from_secs(10),
            connection_timeout: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(10),
            endpoint: NodeAddress::Tcp(BIND_ADDR),
            cycle_observer,
            activity_agent_map: activity_worker_map
                .iter()
                .map(|(activity_id, worker_id)| {
                    let agent_id = worker_agent_map().get(worker_id).copied().unwrap();
                    (*activity_id, agent_id)
                })
                .collect(),
        }
    }
}

#[cfg(feature = "signalling_direct_unix")]
mod cfg {
    use super::{check_ids, Duration, Params, AGENT_ID};
    use feo::{
        agent::NodeAddress,
        ids::{ActivityId, AgentId, WorkerId},
    };
    use mini_adas::config::{activity_dependencies, agent_assignments, socket_paths, worker_agent_map};
    use std::collections::{HashMap, HashSet};

    pub(super) use feo::agent::direct::primary::{Primary, PrimaryConfig};

    pub(super) fn make_config(
        params: Params,
        cycle_observer: Option<Box<dyn feo::scheduler::CycleObserver>>,
    ) -> PrimaryConfig {
        let agent_ids: HashSet<AgentId> = agent_assignments().keys().copied().collect();
        check_ids(&params.recorder_ids, &agent_ids);

        let activity_worker_map: HashMap<ActivityId, WorkerId> = agent_assignments()
            .values()
            .flat_map(|vec| {
                vec.iter()
                    .flat_map(move |(wid, aid_b)| aid_b.iter().map(|v| (v.0, *wid)))
            })
            .collect();

        PrimaryConfig {
            id: AGENT_ID,
            cycle_time: params.feo_cycle_time,
            activity_dependencies: activity_dependencies(),
            recorder_ids: params.recorder_ids,
            worker_assignments: agent_assignments().remove(&AGENT_ID).unwrap(),
            timeout: Duration::from_secs(10),
            connection_timeout: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(10),
            endpoint: NodeAddress::UnixSocket(socket_paths().0),
            cycle_observer,
            activity_agent_map: activity_worker_map
                .iter()
                .map(|(activity_id, worker_id)| {
                    let agent_id = worker_agent_map().get(worker_id).copied().unwrap();
                    (*activity_id, agent_id)
                })
                .collect(),
        }
    }
}

#[cfg(feature = "signalling_relayed_tcp")]
mod cfg {
    use super::{check_ids, Duration, Params, AGENT_ID};
    use feo::agent::NodeAddress;
    use feo::ids::{ActivityId, AgentId, WorkerId};
    use mini_adas::config::{activity_dependencies, agent_assignments, worker_agent_map, BIND_ADDR, BIND_ADDR2};
    use std::collections::{HashMap, HashSet};

    pub(super) use feo::agent::relayed::primary::{Primary, PrimaryConfig};

    pub(super) fn make_config(
        params: Params,
        cycle_observer: Option<Box<dyn feo::scheduler::CycleObserver>>,
    ) -> PrimaryConfig {
        let activity_worker_map: HashMap<ActivityId, WorkerId> = agent_assignments()
            .values()
            .flat_map(|vec| {
                vec.iter()
                    .flat_map(move |(wid, aid_b)| aid_b.iter().map(|v| (v.0, *wid)))
            })
            .collect();

        let agent_ids: HashSet<AgentId> = agent_assignments().keys().copied().collect();
        check_ids(&params.recorder_ids, &agent_ids);

        PrimaryConfig {
            cycle_time: params.feo_cycle_time,
            activity_dependencies: activity_dependencies(),
            recorder_ids: params.recorder_ids,
            worker_assignments: agent_assignments().remove(&AGENT_ID).unwrap(),
            timeout: Duration::from_secs(10),
            connection_timeout: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(10),
            bind_address_senders: NodeAddress::Tcp(BIND_ADDR),
            bind_address_receivers: NodeAddress::Tcp(BIND_ADDR2),
            id: AGENT_ID,
            worker_agent_map: worker_agent_map(),
            activity_worker_map,
            cycle_observer,
        }
    }
}

#[cfg(feature = "signalling_relayed_unix")]
mod cfg {
    use super::{check_ids, Duration, Params, AGENT_ID};
    use feo::agent::NodeAddress;
    use feo::ids::{ActivityId, AgentId, WorkerId};
    use mini_adas::config::{activity_dependencies, agent_assignments, socket_paths, worker_agent_map};
    use std::collections::{HashMap, HashSet};

    pub(super) use feo::agent::relayed::primary::{Primary, PrimaryConfig};

    pub(super) fn make_config(
        params: Params,
        cycle_observer: Option<Box<dyn feo::scheduler::CycleObserver>>,
    ) -> PrimaryConfig {
        let activity_worker_map: HashMap<ActivityId, WorkerId> = agent_assignments()
            .values()
            .flat_map(|vec| {
                vec.iter()
                    .flat_map(move |(wid, aid_b)| aid_b.iter().map(|v| (v.0, *wid)))
            })
            .collect();

        let agent_ids: HashSet<AgentId> = agent_assignments().keys().copied().collect();
        check_ids(&params.recorder_ids, &agent_ids);

        PrimaryConfig {
            cycle_time: params.feo_cycle_time,
            activity_dependencies: activity_dependencies(),
            recorder_ids: params.recorder_ids,
            worker_assignments: agent_assignments().remove(&AGENT_ID).unwrap(),
            timeout: Duration::from_secs(10),
            connection_timeout: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(10),
            bind_address_senders: NodeAddress::UnixSocket(socket_paths().0),
            bind_address_receivers: NodeAddress::UnixSocket(socket_paths().1),
            id: AGENT_ID,
            worker_agent_map: worker_agent_map(),
            activity_worker_map,
            cycle_observer,
        }
    }
}

#[allow(dead_code)]
fn check_ids<'t, T>(recorder_ids: &'t T, agent_ids: &HashSet<AgentId>)
where
    &'t T: IntoIterator<Item = &'t AgentId>,
{
    let mut ids = agent_ids.clone();
    for recorder_id in recorder_ids {
        let is_new = ids.insert(*recorder_id);
        assert!(
            is_new,
            "Agent id {recorder_id} of recorder is not unique within all agents"
        );
    }
}
