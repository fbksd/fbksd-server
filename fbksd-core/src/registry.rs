use crate::ci::ProjectInfo;
use crate::config;
use crate::paths;
use chrono::{DateTime, Utc};
use log;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug)]
pub enum Error {
    /// No technique with the give id was found.
    NotRegistered,
    InvalidInfoFile,
    AlreadyPublished,
    MaxWorkspacesExceeded,
    Unspecified,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match *self {
            NotRegistered => "no technique with the given id was found".fmt(f),
            InvalidInfoFile => "invalid info.json file".fmt(f),
            AlreadyPublished => "technique is already published".fmt(f),
            MaxWorkspacesExceeded => "maximum number of workspaces exceeded".fmt(f),
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

#[derive(Debug, Serialize, Deserialize)]
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
pub struct Technique {
    pub technique_type: TechniqueType,
    pub short_name: String,
    pub full_name: String,
    pub comment: String,
    pub citation: String,
    #[serde(default)]
    pub versions: Vec<Version>,
}

impl Technique {
    /// Read a info.json file.
    pub fn read(path: PathBuf) -> Result<Technique> {
        let data = fs::read_to_string(path)?;
        let tech: Technique = serde_json::from_str(&data)?;
        Ok(tech)
    }

    /// Write a info.json file.
    pub fn write(&self, path: PathBuf) {
        let data = serde_json::to_string_pretty(self).expect("Error serializing technique info.");
        fs::write(path, &data).expect("Error saving technique info.");
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
enum WorkspaceStatus {
    /// Workspace is new (no results in it yet)
    New,
    /// Benchmark was executed and results are saved in the workspace
    Finished(DateTime<Utc>),
    /// Results were published in the public page (creation, publication).
    Published(DateTime<Utc>, DateTime<Utc>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Workspace {
    uuid: String,
    commit_sha: String,
    docker_image: String,
    status: WorkspaceStatus,
    creation_time: DateTime<Utc>,
}

impl Workspace {
    fn new(info: &ProjectInfo) -> Workspace {
        Self {
            uuid: Uuid::new_v4().to_string(),
            commit_sha: info.commit_sha.clone(),
            docker_image: info.docker_img.clone(),
            status: WorkspaceStatus::New,
            creation_time: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Entry {
    name: String, // Last name obtained from a build.
    workspaces: Vec<Workspace>,
}

impl Entry {
    fn get_published_mut(&mut self) -> Option<&mut Workspace> {
        self.workspaces.iter_mut().find(|w| match w.status {
            WorkspaceStatus::Published(_, _) => true,
            _ => false,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Registry {
    denoisers: HashMap<String, Entry>,
    samplers: HashMap<String, Entry>,
}

impl Registry {
    fn path() -> PathBuf {
        paths::data_root().join("registry.json")
    }

    /// Load the registry.
    ///
    /// If the file doesn't exist, it creates one.
    pub fn load() -> Registry {
        let path = Self::path();
        if !path.exists() {
            let registry = Registry {
                denoisers: HashMap::new(),
                samplers: HashMap::new(),
            };
            let data =
                serde_json::to_string_pretty(&registry).expect("Error serializing registry.");
            fs::write(path, data).expect("Failed creating registry file");
            return registry;
        }

        let data = fs::read_to_string(path).expect("Failed reading the registry file");
        let registry: Registry =
            serde_json::from_str(&data).expect("Failed deserializing registry file");
        registry
    }

    pub fn save(&self) {
        let data = serde_json::to_string_pretty(self).expect("Error serializing registry.");
        fs::write(Self::path(), &data).expect("Error saving registry.");
    }

    pub fn technique_type(&self, id: &str) -> Option<TechniqueType> {
        match self.get_entry(id) {
            Some((t, _)) => Some(t),
            None => None,
        }
    }

    /// Register a technique with the given id and name.
    ///
    /// Trying to register a new id with same name than other technique causes error.
    /// This method can also be used to change the current name of a technique.
    /// Multiple technique versions are not allowed (the info.json file can have only the default version).
    pub fn register(&mut self, info: &ProjectInfo, tech: &Technique) -> Result<()> {
        if tech.versions.len() > 1 {
            log::trace!("Multiple technique versions are not allowed in the info.json file");
            return Err(Error::InvalidInfoFile);
        } else if tech.versions.len() == 1 && tech.versions[0].name != "default" {
            log::trace!("Only the default version is allowed in the info.json file");
            return Err(Error::InvalidInfoFile);
        }

        //FIXME: prevent tech changing its type.
        let map = match tech.technique_type {
            TechniqueType::DENOISER => &mut self.denoisers,
            TechniqueType::SAMPLER => &mut self.samplers,
        };
        if let Some((id, entry)) = map.iter_mut().find(|(_, e)| e.name == tech.short_name) {
            if *id == info.id {
                entry.name = String::from(tech.short_name.as_str());
                return Ok(());
            }
            log::trace!(
                "Other technique with the name {} already exists.",
                entry.name
            );
            return Err(Error::InvalidInfoFile);
        }

        if let Err(_) = fs::create_dir_all(paths::tech_data_path(&tech.technique_type, &info.id)) {
            log::trace!("failed to create data directory");
            return Err(Error::Unspecified);
        }
        map.insert(
            info.id.clone(),
            Entry {
                name: String::from(tech.short_name.as_str()),
                workspaces: Vec::new(),
            },
        );
        log::trace!("Technique {} registered.", tech.short_name);
        return Ok(());
    }

    fn get_workspace_mut(&mut self, id: &str, uuid: &str) -> Option<&mut Workspace> {
        match self.get_entry_mut(id) {
            Some((_, entry)) => entry.workspaces.iter_mut().find(|w| w.uuid == uuid),
            None => return None,
        }
    }

    /// Return the entry (TechniqueType, Entry) for the given technique id.
    fn get_entry_mut(&mut self, id: &str) -> Option<(TechniqueType, &mut Entry)> {
        match self.denoisers.get_mut(id) {
            Some(entry) => Some((TechniqueType::DENOISER, entry)),
            None => match self.samplers.get_mut(id) {
                Some(entry) => Some((TechniqueType::SAMPLER, entry)),
                None => None,
            },
        }
    }

    fn get_entry(&self, id: &str) -> Option<(TechniqueType, &Entry)> {
        match self.denoisers.get(id) {
            Some(entry) => Some((TechniqueType::DENOISER, entry)),
            None => match self.samplers.get(id) {
                Some(entry) => Some((TechniqueType::SAMPLER, entry)),
                None => None,
            },
        }
    }

    /// Add a workspace entry for the technique.
    ///
    /// Returns the uuid string of the new workspace.
    /// An error can occur if the technique is not registered is has its number of workspaces exceeded.
    pub fn add_workspace(&mut self, info: &ProjectInfo) -> Result<String> {
        let max_workspaces = config::SystemConfig::load().max_num_workspaces as usize;
        let entry: &mut Entry = match self.get_entry_mut(&info.id) {
            Some((_, entry)) => entry,
            None => return Err(Error::NotRegistered),
        };
        if entry.workspaces.len() >= max_workspaces {
            return Err(Error::MaxWorkspacesExceeded);
        }
        let wp = Workspace::new(&info);
        let uuid = wp.uuid.clone();
        entry.workspaces.push(wp);
        Ok(uuid)
    }

    /// Sets the status of the workspace as "finished"
    pub fn publish_workspace_private(&mut self, info: &ProjectInfo, uuid: &str) -> Result<()> {
        let w = match self.get_workspace_mut(&info.id, uuid) {
            Some(w) => w,
            None => return Err(Error::Unspecified),
        };
        w.status = WorkspaceStatus::Finished(Utc::now());
        return Ok(());
    }

    /// Sets a workspace as published.
    ///
    /// If the project already has is published, error is returned.
    pub fn publish_workspace_public(&mut self, info: &ProjectInfo, uuid: &str) -> Result<()> {
        let w = match self.get_workspace_mut(&info.id, uuid) {
            Some(w) => w,
            None => return Err(Error::Unspecified),
        };
        match w.status {
            WorkspaceStatus::Finished(on) => {
                w.status = WorkspaceStatus::Published(on, Utc::now());
                Ok(())
            }
            WorkspaceStatus::New => return Err(Error::Unspecified),
            WorkspaceStatus::Published(_, _) => return Err(Error::AlreadyPublished),
        }
    }

    /// Unpublishes a technique (setting its published workspace as as "Finished").
    /// Returns the (group, uuid) of the unpublished workspace.
    /// If the technique is not published, returns error.
    pub fn unpublish_workspace(&mut self, id: &str) -> Result<(TechniqueType, &str)> {
        let (tech_type, entry) = match self.get_entry_mut(id) {
            Some(e) => e,
            None => return Err(Error::Unspecified),
        };
        if let Some(w) = entry.get_published_mut() {
            if let WorkspaceStatus::Published(finished_on, _) = w.status {
                w.status = WorkspaceStatus::Finished(finished_on);
                return Ok((tech_type, &w.uuid));
            }
        }
        Err(Error::Unspecified)
    }

    /// Returns the published techniques as (id, uuid) pairs.
    pub fn get_published(&self, group: &TechniqueType) -> impl Iterator<Item = (&String, &String)> {
        let map = match group {
            TechniqueType::DENOISER => &self.denoisers,
            TechniqueType::SAMPLER => &self.samplers,
        };
        map.iter().filter_map(|x| {
            if let Some(w) = x.1.workspaces.iter().find(|w| match w.status {
                WorkspaceStatus::Published(_, _) => true,
                _ => false,
            }) {
                return Some((x.0, &w.uuid));
            }
            return None;
        })
    }

    pub fn remove_workspace(&mut self, id: &str, uuid: &str) -> Result<()> {
        let entry = match self.get_entry_mut(id) {
            Some((_, entry)) => entry,
            None => return Err(Error::Unspecified),
        };
        if let Some(item) = entry
            .workspaces
            .iter()
            .enumerate()
            .find(|w| w.1.uuid == uuid)
        {
            entry.workspaces.remove(item.0);
            return Ok(());
        }
        Err(Error::Unspecified)
    }

    /// Return the unpublished workspaces uuids for the technique.
    /// Panics if id is not registered.
    pub fn get_unpublished_wps(&self, id: &str) -> impl Iterator<Item = &String> {
        let (_, entry) = self.get_entry(id).unwrap();
        entry.workspaces.iter().filter_map(move |w| match w.status {
            WorkspaceStatus::Finished(_) => Some(&w.uuid),
            _ => None,
        })
    }

    /// Return unpublished workspaces (id, uuid) older than the given number of days.
    pub fn get_unpub_older_than(
        &self,
        group: &TechniqueType,
        days_limit: u64,
    ) -> impl Iterator<Item = (&String, &String)> {
        let now = Utc::now();
        let limit = Duration::new(days_limit * 86400, 0);
        let map = match group {
            TechniqueType::DENOISER => &self.denoisers,
            TechniqueType::SAMPLER => &self.samplers,
        };
        map.iter().filter_map(move |x| {
            if let Some(w) = x.1.workspaces.iter().find(|w| match w.status {
                WorkspaceStatus::Finished(creation_time) => {
                    match now.signed_duration_since(creation_time).to_std() {
                        Ok(elapsed) if elapsed > limit => true,
                        _ => false,
                    }
                }
                _ => false,
            }) {
                return Some((x.0, &w.uuid));
            }
            return None;
        })
    }
}
