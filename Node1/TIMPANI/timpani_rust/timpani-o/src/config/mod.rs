/*
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
SPDX-License-Identifier: MIT
*/

//! Node configuration loading and management.
//!
//! This module mirrors the C++ `NodeConfig` struct and `NodeConfigManager` class
//! from `node_config.h / node_config.cpp`.
//!
//! The expected YAML structure is:
//! ```yaml
//! nodes:
//!   node01:
//!     available_cpus: [2, 3]
//!     max_memory_mb: 4096
//!     architecture: "aarch64"
//!     location: "front_sensor_unit"
//!     description: "Perception and sensor fusion node"
//! ```

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info, warn};

// ── Private YAML deserialization types ────────────────────────────────────────

/// Top-level wrapper that maps directly onto the YAML file layout.
///
/// This is kept private – callers work with [`NodeConfig`] / [`NodeConfigManager`]
/// instead.
#[derive(Debug, Deserialize)]
struct NodeConfigFile {
    nodes: HashMap<String, NodeConfigEntry>,
}

/// Per-node fields as they appear in the YAML file.
///
/// Every field except `available_cpus` is optional so that partial configs
/// are accepted gracefully (missing values fall back to their defaults).
#[derive(Debug, Deserialize)]
struct NodeConfigEntry {
    #[serde(default)]
    available_cpus: Vec<u32>,
    /// Maximum memory this node can allocate to tasks, in MB.
    /// Defaults to `u64::MAX` (unconstrained) when absent from YAML.
    #[serde(default = "default_max_memory_mb")]
    max_memory_mb: u64,
    architecture: Option<String>,
    location: Option<String>,
    description: Option<String>,
}

/// Serde default for `max_memory_mb`: `u64::MAX` means "no constraint".
fn default_max_memory_mb() -> u64 {
    u64::MAX
}

// ── Public data structures ────────────────────────────────────────────────────

/// Hardware specification and available resources for a single compute node.
///
/// Mirrors the C++ `NodeConfig` struct in `node_config.h`.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub name: String,
    pub available_cpus: Vec<u32>,
    /// Maximum memory this node can allocate to tasks, in MB.
    /// `u64::MAX` means unconstrained (no YAML value supplied).
    pub max_memory_mb: u64,
    pub architecture: String,
    pub location: String,
    pub description: String,
}

impl NodeConfig {
    /// Returns the fallback `NodeConfig` used when no configuration file is
    /// supplied.
    ///
    /// Mirrors `NodeConfigManager::GetDefaultNodeConfig()` in C++.
    pub fn default_config(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            available_cpus: vec![0, 1, 2, 3],
            max_memory_mb: 4096_u64,
            architecture: String::from("aarch64"),
            location: String::from("default_location"),
            description: String::from("Default node configuration"),
        }
    }

    /// Returns the number of CPUs available on this node.
    pub fn cpu_count(&self) -> usize {
        self.available_cpus.len()
    }
}

// ── NodeConfigManager ─────────────────────────────────────────────────────────

/// Loads and manages node configurations from a YAML file.
#[derive(Debug, Default)]
pub struct NodeConfigManager {
    /// Map of node name → [`NodeConfig`].
    nodes: HashMap<String, NodeConfig>,

    /// Set to `true` after a successful [`load_from_file`](Self::load_from_file).
    loaded: bool,
}

impl NodeConfigManager {
    /// Creates a new, empty `NodeConfigManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses `path` and populates the internal node map.
    ///
    /// * If the file contains no nodes a single `"default_node"` is inserted,
    ///   matching the C++ fallback behaviour.
    /// * Calling this method a second time replaces all previously loaded nodes.
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened or if the YAML is
    /// structurally invalid.
    pub fn load_from_file(&mut self, path: &Path) -> Result<()> {
        info!("Loading node configuration from: {}", path.display());

        // Reset state before (re-)loading
        self.nodes.clear();
        self.loaded = false;

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot open configuration file: {}", path.display()))?;

        let file: NodeConfigFile = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {}", path.display()))?;

        for (name, entry) in file.nodes {
            let node = NodeConfig {
                name: name.clone(),
                available_cpus: entry.available_cpus,
                max_memory_mb: entry.max_memory_mb,
                architecture: entry.architecture.unwrap_or_default(),
                location: entry.location.unwrap_or_default(),
                description: entry.description.unwrap_or_default(),
            };

            debug!(
                "  Node: {} | CPUs: {} | Memory: {}MB | Arch: {}",
                node.name,
                node.available_cpus.len(),
                node.max_memory_mb,
                node.architecture,
            );
            debug!("    Available CPUs: {:?}", node.available_cpus);

            self.nodes.insert(name, node);
        }

        // Fallback: no nodes parsed → insert a default entry (mirrors C++)
        if self.nodes.is_empty() {
            warn!("No nodes found in configuration file, using default configuration");
            let default = NodeConfig::default_config("default_node");
            self.nodes.insert("default_node".to_string(), default);
        }

        self.loaded = true;

        info!(
            "Successfully loaded {} node configuration(s):",
            self.nodes.len()
        );
        for node in self.nodes.values() {
            info!(
                "  Node: {} | CPUs: {} | Memory: {}MB | Arch: {}",
                node.name,
                node.available_cpus.len(),
                node.max_memory_mb,
                node.architecture,
            );
        }

        Ok(())
    }

    /// Returns a reference to the [`NodeConfig`] for `name`, or `None` if no
    /// node with that name has been loaded.
    ///
    /// Mirrors `NodeConfigManager::GetNodeConfig()`.
    pub fn get_node_config(&self, name: &str) -> Option<&NodeConfig> {
        self.nodes.get(name)
    }

    /// Returns a reference to the full map of loaded node configurations.
    ///
    /// Mirrors `NodeConfigManager::GetAllNodes()`.
    pub fn get_all_nodes(&self) -> &HashMap<String, NodeConfig> {
        &self.nodes
    }

    /// Returns the available CPU IDs for `name`.
    ///
    /// Falls back to `[0, 1, 2, 3]` (the C++ fallback) if the node is not
    /// found, matching `NodeConfigManager::GetAvailableCpus()`.
    pub fn get_available_cpus(&self, name: &str) -> Vec<u32> {
        self.nodes
            .get(name)
            .map(|n| n.available_cpus.clone())
            .unwrap_or_else(|| vec![0, 1, 2, 3])
    }

    /// Returns `true` after a successful call to [`load_from_file`](Self::load_from_file).
    ///
    /// Mirrors `NodeConfigManager::IsLoaded()`.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

// ── Test helpers ──────────────────────────────────────────────────────────────

#[cfg(test)]
impl NodeConfigManager {
    /// Construct a `NodeConfigManager` directly from a list of `NodeConfig` values.
    ///
    /// Only available in test builds.  Use [`load_from_file`](Self::load_from_file)
    /// in production.  This avoids the need for a temp file in unit tests that
    /// require a populated node configuration.
    pub fn from_nodes(nodes: Vec<NodeConfig>) -> Self {
        let nodes_map = nodes.into_iter().map(|n| (n.name.clone(), n)).collect();
        Self {
            nodes: nodes_map,
            loaded: true,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper: write a YAML string to a temp file and return it.
    fn yaml_tempfile(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    // ── NodeConfig ────────────────────────────────────────────────────────────

    #[test]
    fn default_config_has_expected_values() {
        let cfg = NodeConfig::default_config("default_node");
        assert_eq!(cfg.name, "default_node");
        assert_eq!(cfg.available_cpus, vec![0, 1, 2, 3]);
        assert_eq!(cfg.max_memory_mb, 4096);
        assert_eq!(cfg.architecture, "aarch64");
        assert_eq!(cfg.location, "default_location");
        assert!(!cfg.description.is_empty());
    }

    #[test]
    fn cpu_count_matches_available_cpus_length() {
        let cfg = NodeConfig::default_config("n");
        assert_eq!(cfg.cpu_count(), cfg.available_cpus.len());
    }

    // ── NodeConfigManager: load_from_file ─────────────────────────────────────

    #[test]
    fn load_example_yaml() {
        // Matches the layout of examples/node_configurations.yaml
        let yaml = r#"
nodes:
  node01:
    available_cpus: [2, 3]
    max_memory_mb: 4096
    architecture: "aarch64"
    location: "front_sensor_unit"
    description: "Perception and sensor fusion node"
  node02:
    available_cpus: [2, 3, 4, 5]
    max_memory_mb: 8192
    architecture: "aarch64"
    location: "vehicle_control_unit"
    description: "Motion control node"
  node03:
    available_cpus: [2, 3, 6, 7]
    max_memory_mb: 4096
    architecture: "x86_64"
    location: "communication_unit"
    description: "Communication and navigation node"
"#;
        let f = yaml_tempfile(yaml);
        let mut mgr = NodeConfigManager::new();
        mgr.load_from_file(f.path()).unwrap();

        assert!(mgr.is_loaded());
        assert_eq!(mgr.get_all_nodes().len(), 3);

        let n1 = mgr.get_node_config("node01").unwrap();
        assert_eq!(n1.available_cpus, vec![2, 3]);
        assert_eq!(n1.max_memory_mb, 4096);
        assert_eq!(n1.architecture, "aarch64");
        assert_eq!(n1.location, "front_sensor_unit");

        let n2 = mgr.get_node_config("node02").unwrap();
        assert_eq!(n2.available_cpus, vec![2, 3, 4, 5]);
        assert_eq!(n2.max_memory_mb, 8192);

        let n3 = mgr.get_node_config("node03").unwrap();
        assert_eq!(n3.architecture, "x86_64");
        assert_eq!(n3.available_cpus, vec![2, 3, 6, 7]);
    }

    #[test]
    fn optional_fields_use_defaults_when_absent() {
        let yaml = r#"
nodes:
  minimal_node:
    available_cpus: [0]
"#;
        let f = yaml_tempfile(yaml);
        let mut mgr = NodeConfigManager::new();
        mgr.load_from_file(f.path()).unwrap();

        let node = mgr.get_node_config("minimal_node").unwrap();
        assert_eq!(node.max_memory_mb, u64::MAX); // default = unconstrained
        assert_eq!(node.architecture, ""); // default (empty)
        assert_eq!(node.location, ""); // default (empty)
    }

    #[test]
    fn empty_nodes_section_inserts_default_node() {
        let yaml = "nodes: {}\n";
        let f = yaml_tempfile(yaml);
        let mut mgr = NodeConfigManager::new();
        mgr.load_from_file(f.path()).unwrap();

        assert!(mgr.is_loaded());
        assert!(mgr.get_node_config("default_node").is_some());
    }

    #[test]
    fn missing_file_returns_error() {
        let mut mgr = NodeConfigManager::new();
        let result = mgr.load_from_file(Path::new("/nonexistent/path/config.yaml"));
        assert!(result.is_err());
        assert!(!mgr.is_loaded());
    }

    #[test]
    fn malformed_yaml_returns_error() {
        let f = yaml_tempfile("this is: not: valid: yaml: content:::");
        let mut mgr = NodeConfigManager::new();
        let result = mgr.load_from_file(f.path());
        assert!(result.is_err());
        assert!(!mgr.is_loaded());
    }

    // ── NodeConfigManager: get_available_cpus ─────────────────────────────────

    #[test]
    fn get_available_cpus_returns_correct_list() {
        let yaml = r#"
nodes:
  node01:
    available_cpus: [2, 3]
"#;
        let f = yaml_tempfile(yaml);
        let mut mgr = NodeConfigManager::new();
        mgr.load_from_file(f.path()).unwrap();

        assert_eq!(mgr.get_available_cpus("node01"), vec![2, 3]);
    }

    #[test]
    fn get_available_cpus_falls_back_for_unknown_node() {
        let mgr = NodeConfigManager::new();
        // mirrors C++ fallback: {0, 1, 2, 3}
        assert_eq!(mgr.get_available_cpus("nonexistent"), vec![0, 1, 2, 3]);
    }

    // ── NodeConfigManager: reload ─────────────────────────────────────────────

    #[test]
    fn reload_replaces_previous_nodes() {
        let yaml1 = "nodes:\n  n1:\n    available_cpus: [0]\n";
        let yaml2 = "nodes:\n  n2:\n    available_cpus: [1]\n";

        let f1 = yaml_tempfile(yaml1);
        let f2 = yaml_tempfile(yaml2);

        let mut mgr = NodeConfigManager::new();
        mgr.load_from_file(f1.path()).unwrap();
        assert!(mgr.get_node_config("n1").is_some());

        mgr.load_from_file(f2.path()).unwrap();
        assert!(mgr.get_node_config("n1").is_none(), "old node must be gone");
        assert!(mgr.get_node_config("n2").is_some());
    }
}
