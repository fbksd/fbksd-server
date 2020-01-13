//! Reads the server system configuration.
//!
//! The system configuration is kept in the `CONFIG_FILE` file, and controls general policies, limits and behavior of
//! the overall system.

use crate::paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Server system configurations.
#[derive(Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    /// Maximum number of workspaces allowed per project.
    pub max_num_workspaces: i32,
    /// Number of days a unpublished workspace will remain saved.
    pub unpublished_days_limit: u64,
    /// List of spps used to execute benchmarks.
    pub spps: Vec<i32>,
    /// Map of docker images available. The key is the alias for an image.
    pub configs: HashMap<String, String>,
}

impl SystemConfig {
    /// Loads the system configurations from the file.
    pub fn load() -> Self {
        let data = fs::read_to_string(paths::config_path()).unwrap();
        serde_json::from_str(&data).unwrap()
    }
}
