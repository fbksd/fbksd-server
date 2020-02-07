//! Reads the server system configuration.
//!
//! The system configuration is kept in the `CONFIG_FILE` file, and controls general policies, limits and behavior of
//! the overall system.

use crate::paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/*
{
    "max_num_workspaces": 3,
    "unpublished_days_limit": 1,
    "spps": [2, 4, 8, 16, 32],
    "configs": {
        "config": "fbksd-ci:latest",
        "default": "fbksd-ci:latest"
    },
    "mailer_smtp_domain": "smtp.gmail.com",
    "mailer_email_user": "fbksd.gitlab@gmail.com",
    "mailer_email_password": "************",
    "mailer_polling_rate": 20,
    "mailer_timeout": 120
}
*/

/// Server system configurations.
#[derive(Debug, Deserialize, Serialize)]
pub struct SystemConfig {
    /// Maximum number of workspaces allowed per project.
    pub max_num_workspaces: i32,
    /// Number of days a unpublished workspace will remain saved.
    pub unpublished_days_limit: i32,
    /// List of spps used to execute benchmarks.
    pub spps: Vec<i32>,
    /// Map of docker images available. The key is the alias for an image.
    pub configs: HashMap<String, String>,
    pub mailer_smtp_domain: String,
    pub mailer_email_user: String,
    pub mailer_email_password: String,
    /// Polling rate for checking email tasks (in seconds).
    pub mailer_polling_rate: i32,
    /// Timeout for connection with SMTP server (in seconds).
    pub mailer_timeout: i32,
}

impl SystemConfig {
    /// Loads the system configurations from the file.
    pub fn load() -> Self {
        let data = fs::read_to_string(paths::config_path()).unwrap();
        serde_json::from_str(&data).unwrap()
    }
}
