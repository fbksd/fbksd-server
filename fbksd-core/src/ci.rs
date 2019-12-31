use crate::config::SystemConfig;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::env;
use std::fs;

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

#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(deny_unknown_fields)]
struct CIConfig {
    #[serde(rename = "include")]
    includes: Vec<CIProjInclude>,
}

impl CIConfig {
    // fn new() -> CIConfig {
    //     CIConfig {
    //         includes: vec![CIProjInclude {
    //             project: String::from("fbksd/fbksd_ci_config"),
    //             git_ref: String::from("master"),
    //             file: String::from("/config.yml"),
    //         }],
    //     }
    // }

    fn load() -> Result<CIConfig, ()> {
        let data = fs::read_to_string(".gitlab-ci.yml");
        if data.is_err() {
            return Err(());
        }
        match serde_yaml::from_str::<CIConfig>(&data.unwrap()) {
            Ok(mut config) => {
                if config.includes.len() != 1 {
                    return Err(());
                }

                let mut inc = &mut config.includes[0];
                if inc.project != "fbksd/fbksd_ci_config" || inc.git_ref != "master" {
                    return Err(());
                }

                let sys_config = SystemConfig::load();
                match sys_config.configs.get(&inc.file[1..inc.file.len() - 4]) {
                    Some(img) => {
                        inc.docker_img = img.clone();
                        return Ok(config);
                    }
                    None => Err(()),
                }
            }
            Err(_) => Err(()),
        }
    }

    pub fn docker_img(&self) -> &str {
        &self.includes[0].docker_img
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectInfo {
    pub id: String,
    pub commit_sha: String,
    pub docker_img: String,
}

impl ProjectInfo {
    pub fn load() -> ProjectInfo {
        const CI_PROJECT_ID: &str = "CI_PROJECT_ID";
        const CI_COMMIT_SHORT_SHA: &str = "CI_COMMIT_SHORT_SHA";
        let id = env::var(CI_PROJECT_ID).expect(&format!("Evn var {} not defined", CI_PROJECT_ID));
        let commit_sha = env::var(CI_COMMIT_SHORT_SHA)
            .expect(&format!("Evn var {} not defined", CI_COMMIT_SHORT_SHA));
        ProjectInfo {
            id,
            commit_sha,
            docker_img: String::from(CIConfig::load().unwrap().docker_img()),
        }
    }
}
