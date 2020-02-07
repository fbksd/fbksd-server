//! Reads project info from GitLab CI configuration files (.gitlab-ci.yml) and environment variables.

use crate::system_config::SystemConfig;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::env;
use std::error;
use std::fmt;
use std::fs;

#[derive(Debug)]
pub enum CIError {
    MissingEnvVar(String),
    InvalidID,
    CIConfigNotFound,
    CIConfigMissingInclude,
    CIConfigImageNotFound,
    BadCIConfig,
    Unspecified,
}
impl fmt::Display for CIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use CIError::*;
        match &*self {
            MissingEnvVar(var) => write!(f, "missing CI environment variable: {}", var),
            InvalidID => "invalid project ID".fmt(f),
            CIConfigNotFound => "\".gitlab-ci.yml\" file not found".fmt(f),
            CIConfigMissingInclude => {
                "\".gitlab-ci.yml\" file missing correct \"include\" statement".fmt(f)
            }
            CIConfigImageNotFound => "config in \".gitlab-ci.yml\" was not found".fmt(f),
            BadCIConfig => "bad \".gitlab-ci.yml\" format".fmt(f),
            Unspecified => "unspecified error".fmt(f),
        }
    }
}
impl error::Error for CIError {}
type CIConfigResult = Result<CIConfig, CIError>;
pub type ProjectInfoResult = Result<ProjectInfo, CIError>;

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
struct CIProjInclude {
    project: String,
    #[serde(rename = "ref")]
    git_ref: String,
    file: String,
    #[serde(skip)]
    docker_img: String,
}

/// `.gitlab-ci.yml` file's content.
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
struct CIConfig {
    #[serde(rename = "include")]
    includes: Vec<CIProjInclude>,
}

impl CIConfig {
    fn load() -> CIConfigResult {
        let data = fs::read_to_string(".gitlab-ci.yml");
        if data.is_err() {
            return Err(CIError::CIConfigNotFound);
        }
        match serde_yaml::from_str::<CIConfig>(&data.unwrap()) {
            Ok(mut config) => {
                // only one include allowed
                if config.includes.len() != 1 {
                    return Err(CIError::BadCIConfig);
                }

                // the include should have the write project and branch
                let mut inc = &mut config.includes[0];
                if inc.project != "fbksd/fbksd_ci_config" || inc.git_ref != "master" {
                    return Err(CIError::CIConfigMissingInclude);
                }

                let sys_config = SystemConfig::load();
                match sys_config.configs.get(&inc.file[1..inc.file.len() - 4]) {
                    Some(img) => {
                        inc.docker_img = img.clone();
                        return Ok(config);
                    }
                    None => Err(CIError::CIConfigImageNotFound),
                }
            }
            Err(_) => Err(CIError::Unspecified),
        }
    }

    pub fn docker_img(&self) -> &str {
        &self.includes[0].docker_img
    }
}

/// Project information from GitLab env variables and CI config file.
#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectInfo {
    /// From CI_PROJECT_ID.
    pub id: i32,
    /// From CI_COMMIT_SHORT_SHA.
    pub commit_sha: String,
    /// From GITLAB_USER_EMAIL
    pub user_email: String,
    /// From CIConfig::docker_img().
    pub docker_img: String,
    /// From FBKSD_TOKEN.
    pub token: String,
}

impl ProjectInfo {
    pub fn load() -> ProjectInfoResult {
        const CI_PROJECT_ID: &str = "CI_PROJECT_ID";
        let id = env::var(CI_PROJECT_ID);
        if id.is_err() {
            return Err(CIError::MissingEnvVar(String::from(CI_PROJECT_ID)));
        }
        // TODO: remove the unwraps.
        let id = id.unwrap().parse::<i32>();
        if id.is_err() {
            return Err(CIError::InvalidID);
        }
        let id = id.unwrap();

        const CI_COMMIT_SHORT_SHA: &str = "CI_COMMIT_SHORT_SHA";
        let commit_sha = env::var(CI_COMMIT_SHORT_SHA);
        if commit_sha.is_err() {
            return Err(CIError::MissingEnvVar(String::from(CI_COMMIT_SHORT_SHA)));
        }
        let commit_sha = commit_sha.unwrap();

        const GITLAB_USER_EMAIL: &str = "GITLAB_USER_EMAIL";
        let user_email = env::var(GITLAB_USER_EMAIL);
        if user_email.is_err() {
            return Err(CIError::MissingEnvVar(String::from(GITLAB_USER_EMAIL)));
        }
        let user_email = user_email.unwrap();

        const FBKSD_TOKEN: &str = "FBKSD_TOKEN";
        let token = env::var(FBKSD_TOKEN);
        if token.is_err() {
            return Err(CIError::MissingEnvVar(String::from(FBKSD_TOKEN)));
        }
        let token = token.unwrap();

        // loads and validates the `.gitlab-ci.yml` file.
        let ci_config = CIConfig::load()?;
        Ok(ProjectInfo {
            id,
            commit_sha,
            user_email,
            docker_img: String::from(ci_config.docker_img()),
            token,
        })
    }
}
