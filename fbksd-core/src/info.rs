use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{error, fmt, fs, io};

#[derive(Debug)]
pub enum Error {
    InvalidInfoFile,
    Unspecified,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match *self {
            InvalidInfoFile => "invalid info.json file".fmt(f),
            Unspecified => "unspecified error".fmt(f),
        }
    }
}
impl error::Error for Error {}

macro_rules! to_unspecified {
    ( $x:ty ) => {
        impl From<$x> for Error {
            fn from(_: $x) -> Self {
                Error::Unspecified
            }
        }
    };
}
to_unspecified!(io::Error);
to_unspecified!(serde_json::error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
    pub fn read(path: PathBuf) -> Result<TechniqueInfo> {
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
