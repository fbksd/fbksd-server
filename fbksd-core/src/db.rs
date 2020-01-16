//! Manages the database operations.
//!
//! The database location is expected to be in the env var `DATABASE_URL`.

use crate::ci::ProjectInfo;
use crate::error::Error;
use crate::info;
use crate::schema::{techniques, workspaces};
use crate::system_config::SystemConfig;

use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::{insert_into, Associations, Identifiable, Insertable, Queryable};
use log;
use serde_json;
use std::env;
use std::io;
use uuid::Uuid;

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
to_unspecified!(diesel::result::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Queryable, Identifiable)]
struct Technique {
    id: i32,
    technique_type: i32,
    short_name: String,
    full_name: String,
    citation: String,
    comment: String,
    num_workspaces: i32,
}
impl Technique {
    fn get_type(&self) -> info::TechniqueType {
        if info::TechniqueType::DENOISER as i32 == self.technique_type {
            return info::TechniqueType::DENOISER;
        } else {
            return info::TechniqueType::SAMPLER;
        }
    }
}

#[derive(Insertable)]
#[table_name = "techniques"]
struct NewTechnique<'a> {
    id: i32, // the id comes from GitLab
    technique_type: i32,
    short_name: &'a str,
    full_name: &'a str,
    citation: &'a str,
    comment: &'a str,
}

#[derive(Debug, PartialEq, Clone)]
enum WorkspaceStatus {
    /// Workspace is new (no results in it yet)
    New,
    /// Benchmark was executed and results are saved in the workspace
    Finished,
    /// Results were published in the public page (creation, publication).
    Published,
}

#[derive(Debug, Clone, Identifiable, Associations, Queryable)]
#[belongs_to(Technique)]
struct Workspace {
    id: i32,
    technique_id: i32,
    uuid: String,
    commit_sha: String,
    docker_image: String,
    status: i32,
    creation_time: NaiveDateTime,
    finish_time: Option<NaiveDateTime>,
    publication_time: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "workspaces"]
pub struct NewWorkspace<'a> {
    technique_id: i32,
    uuid: &'a str,
    commit_sha: &'a str,
    docker_image: &'a str,
    status: i32,
}

/// Register a technique with the given id and name.
///
/// This method can also be used to change the current name of a technique.
/// Multiple technique versions are not allowed (the info.json file can have only the default version).
pub fn register(info: &ProjectInfo, in_tech: &info::TechniqueInfo) -> Result<()> {
    if in_tech.versions.len() > 1 {
        log::trace!("Multiple technique versions are not allowed in the info.json file");
        return Err(Error::InvalidInfoFile);
    } else if in_tech.versions.len() == 1 && in_tech.versions[0].name != "default" {
        log::trace!("Only the default version is allowed in the info.json file");
        return Err(Error::InvalidInfoFile);
    }

    let conn = establish_connection();
    // check if technique is already registered
    match techniques::table.find(info.id).first::<Technique>(&conn) {
        Ok(tech) => {
            // update technique
            diesel::update(&tech)
                .set((
                    techniques::dsl::short_name.eq(&in_tech.short_name),
                    techniques::dsl::full_name.eq(&in_tech.full_name),
                    techniques::dsl::citation.eq(&in_tech.citation),
                    techniques::dsl::comment.eq(&in_tech.comment),
                ))
                .execute(&conn)?;
            return Ok(());
        }
        Err(_) => {
            let tech = NewTechnique {
                id: info.id,
                technique_type: in_tech.technique_type as i32,
                short_name: &in_tech.short_name,
                full_name: &in_tech.full_name,
                citation: &in_tech.citation,
                comment: &in_tech.comment,
            };
            match insert_into(techniques::table).values(&tech).execute(&conn) {
                Ok(_) => {
                    log::trace!("Technique {} registered.", tech.short_name);
                    return Ok(());
                }
                Err(err) => {
                    if let diesel::result::Error::DatabaseError(kind, _) = err {
                        match kind {
                            diesel::result::DatabaseErrorKind::UniqueViolation => {
                                log::trace!(
                                    "Other technique with the name {} already exists.",
                                    tech.short_name
                                );
                                return Err(Error::NameAlreadyExists);
                            }
                            _ => return Err(Error::Unspecified),
                        }
                    }
                    return Err(Error::Unspecified);
                }
            }
        }
    }
}

/// Returns the TechniqueType of the given technique.
pub fn technique_type(id: i32) -> Result<info::TechniqueType> {
    let conn = establish_connection();
    let tech_type: info::TechniqueType = match techniques::table.find(id).first::<Technique>(&conn)
    {
        Ok(tech) => tech.get_type(),
        Err(_) => {
            return Err(Error::NotRegistered);
        }
    };
    return Ok(tech_type);
}

/// Add a workspace entry for the technique.
///
/// Returns the uuid string of the new workspace.
/// An error can occur if the technique is not registered is has its number of workspaces exceeded.
pub fn add_workspace(info: &ProjectInfo) -> Result<String> {
    let max_workspaces = SystemConfig::load().max_num_workspaces;
    let id = info.id;
    let conn = establish_connection();
    conn.transaction::<_, Error, _>(|| {
        match techniques::table.find(id).first::<Technique>(&conn) {
            Ok(tech) => {
                // check maximum number of workspaces
                if tech.num_workspaces >= max_workspaces {
                    return Err(Error::MaxWorkspacesExceeded);
                }

                let workspace = NewWorkspace {
                    technique_id: id,
                    uuid: &Uuid::new_v4().to_string(),
                    commit_sha: &info.commit_sha,
                    status: WorkspaceStatus::New as i32,
                    docker_image: &info.docker_img,
                };
                insert_into(workspaces::table)
                    .values(&workspace)
                    .execute(&conn)
                    .expect("");
                diesel::update(&tech)
                    .set(techniques::dsl::num_workspaces.eq(techniques::dsl::num_workspaces + 1))
                    .execute(&conn)
                    .expect("");
                return Ok(String::from(workspace.uuid));
            }
            Err(_) => {
                return Err(Error::NotRegistered);
            }
        }
    })
}

/// Removes the workspace entry for the given technique.
pub fn remove_workspace(id: i32, uuid: &str) -> Result<()> {
    use self::workspaces::dsl;
    let conn = establish_connection();
    conn.transaction::<_, Error, _>(|| {
        match techniques::table.find(id).first::<Technique>(&conn) {
            Ok(tech) => {
                let tech_workspace = Workspace::belonging_to(&tech)
                    .filter(dsl::uuid.eq(&uuid))
                    .first::<Workspace>(&conn)?;
                // delete the workspace row
                diesel::delete(&tech_workspace).execute(&conn)?;
                // decrement number of workspaces for the technique
                diesel::update(&tech)
                    .set(techniques::dsl::num_workspaces.eq(techniques::dsl::num_workspaces - 1))
                    .execute(&conn)?;
                return Ok(());
            }
            Err(_) => {
                return Err(Error::NotRegistered);
            }
        }
    })
}

/// Sets the status of the workspace as "finished"
pub fn publish_workspace_private(info: &ProjectInfo, uuid: &str) -> Result<()> {
    let conn = establish_connection();
    let id = info.id;
    match techniques::table.find(id).first::<Technique>(&conn) {
        Ok(tech) => {
            let tech_workspace = Workspace::belonging_to(&tech)
                .filter(workspaces::dsl::uuid.eq(uuid))
                .first::<Workspace>(&conn)?;
            diesel::update(&tech_workspace)
                .set((
                    workspaces::dsl::status.eq(WorkspaceStatus::Finished as i32),
                    workspaces::dsl::finish_time.eq(Utc::now().naive_utc()),
                ))
                .execute(&conn)?;
            return Ok(());
        }
        Err(_) => {
            return Err(Error::NotRegistered);
        }
    }
}

/// Sets a workspace as published.
///
/// The workspace status must be "Finished", otherwise returns error.
pub fn publish_workspace_public(info: &ProjectInfo, uuid: &str) -> Result<()> {
    let id = info.id;
    let conn = establish_connection();
    match techniques::table.find(id).first::<Technique>(&conn) {
        Ok(tech) => {
            let tech_workspace = Workspace::belonging_to(&tech)
                .filter(workspaces::dsl::uuid.eq(uuid))
                .first::<Workspace>(&conn)?;
            if tech_workspace.status == WorkspaceStatus::Published as i32 {
                return Err(Error::AlreadyPublished);
            } else if tech_workspace.status == WorkspaceStatus::New as i32 {
                return Err(Error::Unspecified);
            }

            diesel::update(&tech_workspace)
                .set((
                    workspaces::dsl::status.eq(WorkspaceStatus::Published as i32),
                    workspaces::dsl::publication_time.eq(Utc::now().naive_utc()),
                ))
                .execute(&conn)?;
            return Ok(());
        }
        Err(_) => {
            return Err(Error::NotRegistered);
        }
    }
}

/// Unpublishes a technique (setting its published workspace as as "Finished").
///
/// Returns the (group, uuid) of the unpublished workspace.
/// If the technique is not published, returns error.
pub fn unpublish_workspace(id: i32) -> Result<(info::TechniqueType, String)> {
    let conn = establish_connection();
    match techniques::table.find(id).first::<Technique>(&conn) {
        Ok(tech) => {
            let tech_workspace = Workspace::belonging_to(&tech)
                .filter(workspaces::dsl::status.eq(WorkspaceStatus::Published as i32))
                .first::<Workspace>(&conn)?;
            let null_time: Option<NaiveDateTime> = None;
            diesel::update(&tech_workspace)
                .set((
                    workspaces::dsl::status.eq(WorkspaceStatus::Finished as i32),
                    workspaces::dsl::publication_time.eq(null_time),
                ))
                .execute(&conn)?;
            let tech_type = match tech.technique_type {
                0 => info::TechniqueType::DENOISER,
                1 => info::TechniqueType::SAMPLER,
                _ => {
                    return Err(Error::Unspecified);
                }
            };
            return Ok((tech_type, tech_workspace.uuid.clone()));
        }
        Err(_) => {
            return Err(Error::NotRegistered);
        }
    }
}

/// Returns the published techniques as vector of (technique_id, workspace_uuid) pairs.
pub fn get_published(group: info::TechniqueType) -> Result<std::vec::Vec<(i32, String)>> {
    let conn = establish_connection();
    let techs = techniques::table
        .filter(techniques::dsl::technique_type.eq(group as i32))
        .load::<Technique>(&conn)?;
    let published = Workspace::belonging_to(&techs)
        .filter(workspaces::dsl::status.eq(WorkspaceStatus::Published as i32))
        .select((workspaces::dsl::technique_id, workspaces::dsl::uuid))
        .load::<(i32, String)>(&conn)?;
    return Ok(published);
}

/// Return the finished unpublished workspaces uuids for the technique.
pub fn get_unpublished(id: i32) -> Result<std::vec::Vec<String>> {
    let conn = establish_connection();
    let tech = techniques::table.find(id).first::<Technique>(&conn)?;
    let unpublished = Workspace::belonging_to(&tech)
        .filter(workspaces::dsl::status.eq(WorkspaceStatus::Finished as i32))
        .select(workspaces::dsl::uuid)
        .load::<String>(&conn)?;
    return Ok(unpublished);
}

/// Return finished unpublished workspaces with finish_time older than the given number of days.
pub fn get_unpub_older_than(
    group: info::TechniqueType,
    days_limit: i32,
) -> Result<std::vec::Vec<(i32, String)>> {
    let oldest_allowed = (Utc::now() - Duration::days(days_limit as i64)).naive_utc();
    let conn = establish_connection();
    let techs = techniques::table
        .filter(techniques::dsl::technique_type.eq(group as i32))
        .load::<Technique>(&conn)?;
    let unpublished = Workspace::belonging_to(&techs)
        .filter(
            workspaces::dsl::status
                .eq(WorkspaceStatus::Finished as i32)
                .and(workspaces::dsl::finish_time.le(oldest_allowed)),
        )
        .select((workspaces::dsl::technique_id, workspaces::dsl::uuid))
        .load::<(i32, String)>(&conn)?;
    return Ok(unpublished);
}

fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[should_panic]
    fn test_register() {
        let info = ProjectInfo {
            id: 12,
            commit_sha: String::from("jhosidf"),
            docker_img: String::from("default"),
        };
        let tech = &info::TechniqueInfo::read(PathBuf::from("/mnt/hdd/home/jonas/Documents/Doutorado/Documents/Benchmark/Code/fbksd/fbksd-package/denoisers/RDFC/info.json")).unwrap();
        register(&info, &tech).expect("");
        let uuid1 = add_workspace(&info).expect("");
        add_workspace(&info).expect("");
        let uuid2 = add_workspace(&info).expect("");
        publish_workspace_private(&info, &uuid1).expect("");
        publish_workspace_private(&info, &uuid2).expect("");
        // publish_workspace_public(&info, &uuid).expect("");
        // unpublish_workspace(info.id.parse::<i32>().unwrap()).expect("");
        // let published = get_published(info::TechniqueType::DENOISER).unwrap();
        // assert_eq!(published.len(), 0);
        // remove_workspace(info.id.parse::<i32>().unwrap(), &uuid).expect("");
    }

    #[test]
    fn test_limit() {
        let list = get_unpub_older_than(info::TechniqueType::DENOISER, 2).unwrap();
        assert_eq!(list.len(), 0)
    }
}
