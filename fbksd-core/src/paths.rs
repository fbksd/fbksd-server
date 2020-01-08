//! All file system paths (files and directories) used throughout the system.

use crate::registry::TechniqueType;
use lazy_static::lazy_static;
use std::env;
use std::path::{Path, PathBuf};

static REGISTRY_FILE: &str = "registry.json";
static CONFIG_FILE: &str = "config.json";
static SCENES_DIR: &str = "scenes";
static IQA_DIR: &str = "iqa";
static RENDERERS_DIR: &str = "renderers";
static WORKSPACES_DIR: &str = "workspaces";
static DENOISERS_WORKSPACES_DIR: &str = "denoisers";
static SAMPLERS_WORKSPACES_DIR: &str = "samplers";
static PAGE_DIR: &str = "page";
static TMP_WORKSPACE_DIR: &str = "tmp/workspace";
static PUBLIC_PAGE_DIR: &str = "public";
static TECH_PUBLISHED_DIR: &str = "published";

pub static LOCK_FILE: &str = "/var/lock/fbksd.lock";
pub static TECH_INSTALL_DIR: &str = "install";
pub static TECH_RESULTS_DIR: &str = "results";

pub fn data_root() -> &'static Path {
    const VAR: &str = "FBKSD_DATA_ROOT";
    lazy_static! {
        static ref VALUE: String = env::var(VAR).expect(&format!("Evn var {} not defined", VAR));
        static ref PATH: PathBuf = PathBuf::from(VALUE.to_owned());
    }
    &PATH
}

pub fn config_path() -> PathBuf {
    data_root().join(&CONFIG_FILE)
}

pub fn registry_path() -> PathBuf {
    data_root().join(&REGISTRY_FILE)
}

pub fn workspaces_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = data_root().join(&WORKSPACES_DIR);
    }
    &PATH
}

pub fn scenes_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = data_root().join(&SCENES_DIR);
    }
    &PATH
}

pub fn iqa_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = data_root().join(&IQA_DIR);
    }
    &PATH
}

pub fn renderers_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = data_root().join(&RENDERERS_DIR);
    }
    &PATH
}

pub fn page_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = data_root().join(&PAGE_DIR);
    }
    &PATH
}

pub fn tmp_workspace_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = data_root().join(&TMP_WORKSPACE_DIR);
    }
    &PATH
}

pub fn public_page_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = data_root().join(&PUBLIC_PAGE_DIR);
    }
    &PATH
}

pub fn denoisers_workspaces_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = workspaces_path().join(DENOISERS_WORKSPACES_DIR);
    }
    &PATH
}

pub fn samplers_workspaces_path() -> &'static Path {
    lazy_static! {
        static ref PATH: PathBuf = workspaces_path().join(SAMPLERS_WORKSPACES_DIR);
    }
    &PATH
}

pub fn tech_data_path(group: &TechniqueType, id: &str) -> PathBuf {
    match group {
        TechniqueType::DENOISER => denoisers_workspaces_path().join(&id),
        TechniqueType::SAMPLER => samplers_workspaces_path().join(&id),
    }
}

pub fn tech_workspace_path(group: &TechniqueType, id: &str, uuid: &str) -> PathBuf {
    tech_data_path(group, &id).join(&uuid)
}

pub fn tech_install_path(group: &TechniqueType, id: &str, uuid: &str) -> PathBuf {
    tech_workspace_path(group, &id, &uuid).join(&TECH_INSTALL_DIR)
}

pub fn tech_results_path(group: &TechniqueType, id: &str, uuid: &str) -> PathBuf {
    tech_workspace_path(group, &id, &uuid).join(&TECH_RESULTS_DIR)
}

pub fn tech_published_wp_path(group: &TechniqueType, id: &str) -> PathBuf {
    tech_workspace_path(group, &id, &TECH_PUBLISHED_DIR)
}

pub fn tech_published_install_path(group: &TechniqueType, id: &str) -> PathBuf {
    tech_published_wp_path(group, &id).join(&TECH_INSTALL_DIR)
}

pub fn tech_published_results_path(group: &TechniqueType, id: &str) -> PathBuf {
    tech_published_wp_path(group, &id).join(&TECH_RESULTS_DIR)
}
