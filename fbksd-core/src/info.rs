//! Handle technique's `info.json` file.

use crate::error::Error;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TechniqueType {
    DENOISER,
    SAMPLER,
}
impl TechniqueType {
    pub fn as_str(&self) -> &str {
        match self {
            TechniqueType::DENOISER => "denoisers",
            TechniqueType::SAMPLER => "samplers",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub comment: String,
    pub executable: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TechniqueInfo {
    pub technique_type: TechniqueType,
    pub short_name: String,
    pub full_name: String,
    pub comment: String,
    pub citation: String,
    #[serde(default)]
    pub versions: Vec<Version>,
}

impl TechniqueInfo {
    /// Read a info.json file.
    pub fn read(path: PathBuf) -> Result<TechniqueInfo, Error> {
        let data = fs::read_to_string(path)?;
        let tech: TechniqueInfo = serde_json::from_str(&data)?;
        Ok(tech)
    }

    /// Write a info.json file.
    pub fn write(&self, path: PathBuf) {
        let data = serde_json::to_string_pretty(self).expect("Error serializing technique info.");
        fs::write(path, &data).expect("Error saving technique info.");
    }
}
