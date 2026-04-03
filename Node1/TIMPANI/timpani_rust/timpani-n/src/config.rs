/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: MIT
 */

use crate::error::{TimpaniError, TimpaniResult};
use clap::Parser;
use tracing::info;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Log level constants
pub mod log_level {
    pub const SILENT: u8 = 0;
    pub const ERROR: u8 = 1;
    pub const WARNING: u8 = 2;
    pub const INFO: u8 = 3;
    pub const DEBUG: u8 = 4;
    pub const VERBOSE: u8 = 5;
}

/// Default configuration constants
pub mod defaults {
    pub const CPU_NO_AFFINITY: i32 = -1;
    pub const PRIORITY_DEFAULT: i32 = -1;
    pub const PORT: u16 = 7777;
    pub const ADDRESS: &str = "127.0.0.1";
    pub const NODE_ID: &str = "1";
    pub const LOG_LEVEL: u8 = super::log_level::INFO;
}

/// Validation range constants
pub mod validation {
    pub const PRIORITY_MIN: i32 = -1;
    pub const PRIORITY_MAX: i32 = 99;
    pub const PRIORITY_RT_MIN: i32 = 1;
    pub const CPU_MIN: i32 = -1;
    pub const CPU_MAX: i32 = 1024;
    pub const PORT_MIN: u16 = 1;
    pub const PORT_MAX: u16 = 65535;
    pub const PORT_INVALID: u16 = 0;
}

/// Exit code constants
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const FAILURE: i32 = 1;
}

/// Test constants for unit tests
#[cfg(test)]
pub mod test_values {
    pub const TEST_CPU_AFFINITY: i32 = 2;
    pub const TEST_CPU_ZERO: i32 = 0;
    pub const TEST_CPU_ONE: i32 = 1;
    pub const TEST_PRIORITY: i32 = 50;
    pub const TEST_PRIORITY_LOW: i32 = 1;
    pub const TEST_PRIORITY_MID: i32 = 10;
    pub const TEST_NODE_ID: &str = "test-node";
    pub const TEST_NODE_ID_SHORT: &str = "test";
    pub const TEST_RESULT_VALUE: i32 = 42;
    pub const LOG_LEVEL_RANGE_MAX: u8 = super::log_level::VERBOSE;
}

/// Log level enum matching tt_log_level_t from C
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(u8)]
pub enum LogLevel {
    Silent = log_level::SILENT,
    Error = log_level::ERROR,
    Warning = log_level::WARNING,
    #[default]
    Info = log_level::INFO,
    Debug = log_level::DEBUG,
    Verbose = log_level::VERBOSE,
}

impl LogLevel {
    /// Parse log level from integer
    pub fn from_u8(level: u8) -> Option<Self> {
        match level {
            log_level::SILENT => Some(LogLevel::Silent),
            log_level::ERROR => Some(LogLevel::Error),
            log_level::WARNING => Some(LogLevel::Warning),
            log_level::INFO => Some(LogLevel::Info),
            log_level::DEBUG => Some(LogLevel::Debug),
            log_level::VERBOSE => Some(LogLevel::Verbose),
            _ => None,
        }
    }

    /// Convert to tracing filter string
    pub fn to_filter_string(self) -> &'static str {
        match self {
            LogLevel::Silent => "off",
            LogLevel::Error => "error",
            LogLevel::Warning => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Verbose => "trace",
        }
    }
}

/// Clock type enum matching clockid_t usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClockType {
    #[default]
    Realtime,
    Monotonic,
}

/// Configuration structure matching the C context.config
#[derive(Debug, Clone)]
pub struct Config {
    /// CPU affinity for time trigger (defaults::CPU_NO_AFFINITY for no affinity)
    pub cpu: i32,

    /// RT priority (validation::PRIORITY_RT_MIN-validation::PRIORITY_MAX, defaults::PRIORITY_DEFAULT for default)
    pub prio: i32,

    /// Port to connect to
    pub port: u16,

    /// Server address
    pub addr: String,

    /// Node ID
    pub node_id: String,

    /// Enable timer synchronization across multiple nodes
    pub enable_sync: bool,

    /// Enable saving plot data file by using BPF
    pub enable_plot: bool,

    /// Enable Apex.OS test mode
    pub enable_apex: bool,

    /// Clock type (CLOCK_REALTIME or CLOCK_MONOTONIC)
    pub clockid: ClockType,

    /// Log level
    pub log_level: LogLevel,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            cpu: defaults::CPU_NO_AFFINITY,
            prio: defaults::PRIORITY_DEFAULT,
            port: defaults::PORT,
            addr: defaults::ADDRESS.to_string(),
            node_id: defaults::NODE_ID.to_string(),
            enable_sync: false,
            enable_plot: false,
            enable_apex: false,
            clockid: ClockType::Realtime,
            log_level: LogLevel::Info,
        }
    }
}

/// Command-line arguments structure using clap
#[derive(Parser, Debug)]
#[command(name = "timpani-n")]
#[command(about = "Timpani-N node executor", long_about = None)]
pub struct CliArgs {
    /// CPU affinity for timetrigger
    #[arg(short = 'c', long, value_name = "CPU_NUM")]
    pub cpu: Option<i32>,

    /// RT priority (1~99) for timetrigger
    #[arg(short = 'P', long, value_name = "PRIO")]
    pub prio: Option<i32>,

    /// Port to connect to
    #[arg(short = 'p', long, value_name = "PORT", default_value_t = defaults::PORT)]
    pub port: u16,

    /// Node ID
    #[arg(short = 'n', long, value_name = "NODE_ID", default_value = defaults::NODE_ID)]
    pub node_id: String,

    /// Log level (0=silent, 1=error, 2=warning, 3=info, 4=debug, 5=verbose)
    #[arg(short = 'l', long, value_name = "LEVEL", default_value_t = defaults::LOG_LEVEL)]
    pub log_level: u8,

    /// Enable timer synchronization across multiple nodes
    #[arg(short = 's', long)]
    pub enable_sync: bool,

    /// Enable saving plot data file by using BPF (<node id>.gpdata)
    #[arg(short = 'g', long)]
    pub enable_plot: bool,

    /// Enable Apex.OS test mode which works without TT schedule info
    #[arg(short = 'a', long)]
    pub enable_apex: bool,

    /// Server host address
    #[arg(value_name = "HOST")]
    pub host: Option<String>,
}

impl Config {
    /// Parse configuration from command-line arguments
    pub fn from_args() -> TimpaniResult<Self> {
        let args = CliArgs::parse();
        Self::from_cli_args(args)
    }

    /// Parse configuration from CliArgs (for testing)
    pub fn from_cli_args(args: CliArgs) -> TimpaniResult<Self> {
        let mut config = Config::default();

        // Parse CPU affinity
        if let Some(cpu) = args.cpu {
            config.cpu = cpu;
        }

        // Parse priority
        if let Some(prio) = args.prio {
            config.prio = prio;
        }

        // Parse port
        config.port = args.port;

        // Parse node ID
        config.node_id = args.node_id;

        // Parse log level
        config.log_level = LogLevel::from_u8(args.log_level).ok_or(TimpaniError::Config)?;

        // Parse boolean flags
        config.enable_sync = args.enable_sync;
        config.enable_plot = args.enable_plot;
        config.enable_apex = args.enable_apex;

        // Parse host address
        if let Some(host) = args.host {
            config.addr = host;
        }

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> TimpaniResult<()> {
        // Validate priority
        if self.prio < validation::PRIORITY_MIN || self.prio > validation::PRIORITY_MAX {
            eprintln!(
                "[ERROR] Invalid priority: {} (must be {} or {}-{})",
                self.prio,
                validation::PRIORITY_MIN,
                validation::PRIORITY_RT_MIN,
                validation::PRIORITY_MAX
            );
            return Err(TimpaniError::Config);
        }

        // Port validation is already handled by u16 type (validation::PORT_MIN-validation::PORT_MAX)
        if self.port == validation::PORT_INVALID {
            eprintln!(
                "[ERROR] Invalid port: {} (must be {}-{})",
                validation::PORT_INVALID,
                validation::PORT_MIN,
                validation::PORT_MAX
            );
            return Err(TimpaniError::Config);
        }

        // Validate CPU
        if self.cpu < validation::CPU_MIN || self.cpu > validation::CPU_MAX {
            eprintln!("[ERROR] Invalid CPU number: {}", self.cpu);
            return Err(TimpaniError::Config);
        }

        // Validate node ID
        if self.node_id.is_empty() {
            eprintln!("[ERROR] Node ID cannot be empty");
            return Err(TimpaniError::Config);
        }

        Ok(())
    }

    /// Log the configuration (matching C implementation's log output)
    pub fn log_config(&self) {
        info!("Configuration:");
        info!("  CPU affinity: {}", self.cpu);
        info!("  Priority: {}", self.prio);
        info!("  Server: {}:{}", self.addr, self.port);
        info!("  Node ID: {}", self.node_id);
        info!("  Log level: {:?}", self.log_level);
        info!(
            "  Sync enabled: {}",
            if self.enable_sync { "yes" } else { "no" }
        );
        info!(
            "  Plot enabled: {}",
            if self.enable_plot { "yes" } else { "no" }
        );
        info!(
            "  Apex.OS test mode: {}",
            if self.enable_apex { "yes" } else { "no" }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.cpu, defaults::CPU_NO_AFFINITY);
        assert_eq!(config.prio, defaults::PRIORITY_DEFAULT);
        assert_eq!(config.port, defaults::PORT);
        assert_eq!(config.addr, defaults::ADDRESS);
        assert_eq!(config.node_id, defaults::NODE_ID);
        assert!(!config.enable_sync);
        assert!(!config.enable_plot);
        assert!(!config.enable_apex);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_priority() {
        let config = Config {
            prio: validation::PRIORITY_MAX + 1, // 100 is invalid
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = Config {
            prio: validation::PRIORITY_MIN - 1, // -2 is invalid
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_cpu() {
        let config = Config {
            cpu: 2000,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = Config {
            cpu: -5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_node_id() {
        let config = Config {
            node_id: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_port() {
        let config = Config {
            port: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::from_u8(0), Some(LogLevel::Silent));
        assert_eq!(LogLevel::from_u8(1), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_u8(2), Some(LogLevel::Warning));
        assert_eq!(LogLevel::from_u8(3), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_u8(4), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_u8(5), Some(LogLevel::Verbose));
        assert_eq!(LogLevel::from_u8(6), None);
        assert_eq!(LogLevel::from_u8(255), None);
    }

    #[test]
    fn test_log_level_default() {
        let level = LogLevel::default();
        assert_eq!(level, LogLevel::Info);
    }

    #[test]
    fn test_log_level_to_filter_string() {
        assert_eq!(LogLevel::Silent.to_filter_string(), "off");
        assert_eq!(LogLevel::Error.to_filter_string(), "error");
        assert_eq!(LogLevel::Warning.to_filter_string(), "warn");
        assert_eq!(LogLevel::Info.to_filter_string(), "info");
        assert_eq!(LogLevel::Debug.to_filter_string(), "debug");
        assert_eq!(LogLevel::Verbose.to_filter_string(), "trace");
    }

    #[test]
    fn test_clock_type_default() {
        let clock = ClockType::default();
        assert_eq!(clock, ClockType::Realtime);
    }

    #[test]
    fn test_config_with_custom_values() {
        let config = Config {
            cpu: 4,
            prio: 50,
            port: 8888,
            addr: "192.168.1.1".to_string(),
            node_id: "test-node".to_string(),
            enable_sync: true,
            enable_plot: true,
            enable_apex: true,
            log_level: LogLevel::Debug,
            ..Default::default()
        };

        assert!(config.validate().is_ok());
        assert_eq!(config.cpu, 4);
        assert_eq!(config.prio, 50);
        assert_eq!(config.port, 8888);
        assert_eq!(config.addr, "192.168.1.1");
        assert_eq!(config.node_id, "test-node");
        assert!(config.enable_sync);
        assert!(config.enable_plot);
        assert!(config.enable_apex);
        assert_eq!(config.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_validate_valid_priority_range() {
        // Test valid priorities
        let config = Config {
            prio: -1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = Config {
            prio: 1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = Config {
            prio: 50,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = Config {
            prio: 99,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_cpu_range() {
        // Test valid CPU values
        let config = Config {
            cpu: -1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = Config {
            cpu: 0,
            ..Default::default()
        };
        assert!(config.validate().is_ok());

        let config = Config {
            cpu: 1024,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_log_config() {
        // This test ensures log_config doesn't panic
        let config = Config::default();
        config.log_config();

        let config = Config {
            cpu: 8,
            prio: 75,
            port: 9999,
            addr: "10.0.0.1".to_string(),
            node_id: "node-test".to_string(),
            enable_sync: true,
            enable_plot: true,
            enable_apex: true,
            log_level: LogLevel::Verbose,
            ..Default::default()
        };
        config.log_config();
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Silent < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Warning);
        assert!(LogLevel::Warning < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Verbose);
    }

    #[test]
    fn test_cli_args_parsing() {
        use clap::Parser;

        // Test with default arguments
        let args = CliArgs::try_parse_from(["timpani-n"]).unwrap();
        assert_eq!(args.port, 7777);
        assert_eq!(args.node_id, "1");
        assert_eq!(args.log_level, 3);
        assert!(!args.enable_sync);
        assert!(!args.enable_plot);
        assert!(!args.enable_apex);

        // Test with custom arguments
        let args = CliArgs::try_parse_from([
            "timpani-n",
            "-c",
            "2",
            "-P",
            "50",
            "-p",
            "8888",
            "-n",
            "test-node",
            "-l",
            "4",
            "-s",
            "-g",
            "-a",
            "192.168.1.1",
        ])
        .unwrap();
        assert_eq!(args.cpu, Some(2));
        assert_eq!(args.prio, Some(50));
        assert_eq!(args.port, 8888);
        assert_eq!(args.node_id, "test-node");
        assert_eq!(args.log_level, 4);
        assert!(args.enable_sync);
        assert!(args.enable_plot);
        assert!(args.enable_apex);
        assert_eq!(args.host, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_from_cli_args_default() {
        use clap::Parser;

        let args = CliArgs::try_parse_from(["timpani-n"]).unwrap();
        let config = Config::from_cli_args(args).unwrap();

        assert_eq!(config.cpu, -1);
        assert_eq!(config.prio, -1);
        assert_eq!(config.port, 7777);
        assert_eq!(config.node_id, "1");
        assert_eq!(config.log_level, LogLevel::Info);
        assert!(!config.enable_sync);
    }

    #[test]
    fn test_from_cli_args_custom() {
        use clap::Parser;

        let args = CliArgs::try_parse_from([
            "timpani-n",
            "-c",
            "4",
            "-P",
            "80",
            "-p",
            "9999",
            "-n",
            "node-5",
            "-l",
            "5",
            "-s",
            "-g",
            "-a",
            "10.0.0.1",
        ])
        .unwrap();
        let config = Config::from_cli_args(args).unwrap();

        assert_eq!(config.cpu, 4);
        assert_eq!(config.prio, 80);
        assert_eq!(config.port, 9999);
        assert_eq!(config.node_id, "node-5");
        assert_eq!(config.log_level, LogLevel::Verbose);
        assert!(config.enable_sync);
        assert!(config.enable_plot);
        assert!(config.enable_apex);
        assert_eq!(config.addr, "10.0.0.1");
    }

    #[test]
    fn test_from_cli_args_invalid_log_level() {
        use clap::Parser;

        let args = CliArgs::try_parse_from(["timpani-n", "-l", "10"]).unwrap();
        let result = Config::from_cli_args(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_cli_args_invalid_priority() {
        use clap::Parser;

        let args = CliArgs::try_parse_from(["timpani-n", "-P", "100"]).unwrap();
        let result = Config::from_cli_args(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_cli_args_invalid_cpu() {
        use clap::Parser;

        let args = CliArgs::try_parse_from(["timpani-n", "-c", "2000"]).unwrap();
        let result = Config::from_cli_args(args);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_long_options() {
        use clap::Parser;

        let args = CliArgs::try_parse_from([
            "timpani-n",
            "--cpu",
            "1",
            "--prio",
            "30",
            "--port",
            "5555",
            "--node-id",
            "long-node",
            "--log-level",
            "2",
            "--enable-sync",
            "--enable-plot",
            "--enable-apex",
        ])
        .unwrap();

        assert_eq!(args.cpu, Some(1));
        assert_eq!(args.prio, Some(30));
        assert_eq!(args.port, 5555);
        assert_eq!(args.node_id, "long-node");
        assert_eq!(args.log_level, 2);
        assert!(args.enable_sync);
        assert!(args.enable_plot);
        assert!(args.enable_apex);
    }

    #[test]
    fn test_validate_all_log_levels() {
        for level_num in 0..=5 {
            let config = Config {
                log_level: LogLevel::from_u8(level_num).unwrap(),
                ..Default::default()
            };
            assert!(config.validate().is_ok());
        }
    }

    #[test]
    fn test_config_validation_all_errors() {
        // Test priority too high
        let config = Config {
            prio: 100,
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(TimpaniError::Config)));

        // Test priority too low
        let config = Config {
            prio: -2,
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(TimpaniError::Config)));

        // Test CPU too high
        let config = Config {
            cpu: 1025,
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(TimpaniError::Config)));

        // Test CPU too low
        let config = Config {
            cpu: -2,
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(TimpaniError::Config)));

        // Test port zero
        let config = Config {
            port: 0,
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(TimpaniError::Config)));

        // Test empty node_id
        let config = Config {
            node_id: String::new(),
            ..Default::default()
        };
        assert!(matches!(config.validate(), Err(TimpaniError::Config)));
    }
}
