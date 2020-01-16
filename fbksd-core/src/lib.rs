#[macro_use]
extern crate diesel;
pub mod ci;
pub mod config;
pub mod db;
pub mod docker;
pub mod error;
pub mod info;
pub mod msgs;
pub mod page;
pub mod paths;
pub mod schema;
pub mod system_config;
pub mod utils;
pub mod workspace;

use error::Error;
use system_config::SystemConfig;
use workspace::Workspace;

use glob::glob;
use std::fs;
use std::os::unix::fs as unixfs;
use std::process::{Command, Stdio};

/// Register/update a technique.
/// 
/// Only short name, full name, citation, and comment are updated.
pub fn register(info: ci::ProjectInfo, tech: info::TechniqueInfo) -> Result<(), Error> {
    log::info!("register: id = {}, name = {}", &info.id, &tech.short_name);
    if let Err(_) = db::register(&info, &tech) {
        log::warn!("failed to register technique");
        return Err(Error::Unspecified);
    }
    Ok(())
}

/// Saves the results from a technique into it's workspace storage.
///
/// Returns the uuid of the saved workspace.
pub fn save_results(proj: ci::ProjectInfo, tech: info::TechniqueInfo) -> Result<String, Error> {
    log::info!(
        "save results: id = {}, name = {}",
        &proj.id,
        &tech.short_name
    );
    let uuid = db::add_workspace(&proj)?;
    let group = db::technique_type(proj.id)?;
    let base = paths::tech_workspace_path(&group, proj.id, &uuid);
    if fs::create_dir_all(&base.join(paths::TECH_RESULTS_DIR)).is_err() {
        log::error!("failed to create workspace results folder");
        return Err(Error::Unspecified);
    }

    // move install files
    let src = paths::tmp_workspace_path()
        .join(group.as_str())
        .join(proj.id.to_string());
    let dest = base.join(paths::TECH_INSTALL_DIR);
    let status = Command::new("mv").args(&[&src, &dest]).status();
    if status.is_err() || !status.unwrap().success() {
        log::error!("Failed to copy pave");
        return Err(Error::Unspecified);
    }

    let src = paths::tmp_workspace_path()
        .join("results/.current")
        .join(group.as_str())
        .join(&tech.short_name)
        .join("*");
    let dest = base.join(paths::TECH_RESULTS_DIR).join("");
    for entry in glob(src.to_str().unwrap()).expect("Failed to read glob pattern") {
        match entry {
            Ok(src) => {
                let status = Command::new("mv").args(&[&src, &dest]).status();
                if status.is_err() || !status.unwrap().success() {
                    log::error!("Failed to copy pave");
                    return Err(Error::Unspecified);
                }
            }
            Err(e) => log::error!("{:?}", e),
        }
    }
    log::info!("results saved in private folder");
    Ok(uuid)
}

// Publish a workspace to the user private location.
pub fn publish_private(proj: ci::ProjectInfo, uuid: String) -> Result<(), Error> {
    log::info!("publish private: id = {}, uuid = {}", &proj.id, &uuid);
    let group = db::technique_type(proj.id).unwrap();
    let base_path = paths::tech_workspace_path(&group, proj.id, &uuid);
    let install_path = base_path.join(paths::TECH_INSTALL_DIR);
    let tech = info::TechniqueInfo::read(install_path.join("info.json"));
    if tech.is_err() {
        return Err(Error::Unspecified);
    }
    let tech = tech.unwrap();

    let public_dir = paths::public_page_path();
    let private_dir = public_dir.join(uuid.to_string());
    if !page::copy_public_page(&private_dir, group.as_str(), &tech.short_name) {
        return Err(Error::Unspecified);
    }

    let mut wp = Workspace::load();
    wp.load_technique(&group, &proj, uuid.to_string());
    wp.export_page(&private_dir);
    // copy result images to unpublished dir
    let src = base_path.join(paths::TECH_RESULTS_DIR);
    let dest = private_dir
        .join("data")
        .join(group.as_str())
        .join(&tech.short_name);
    workspace::export_technique_images(&src, &dest, false)?;
    db::publish_workspace_private(&proj, &uuid)?;
    workspace::set_public_page_permissions()?;
    Ok(())
}

/// Publish a private workspace.
pub fn publish_public(info: ci::ProjectInfo, uuid: String) -> Result<(), Error> {
    log::info!("publish public: id = {}, uuid = {}", &info.id, &uuid);
    db::publish_workspace_public(&info, &uuid)?;

    let public_page = paths::public_page_path();
    let private_page = public_page.join(&uuid);
    let mut wp = Workspace::load();
    let group = db::technique_type(info.id)?;
    wp.load_technique(&group, &info, uuid.to_string());
    wp.export_page(&public_page);

    // TODO: remove unwraps/expects

    // create link to published data
    let base = paths::tech_workspace_path(&group, info.id, &uuid);
    let link_path = paths::tech_published_wp_path(&group, info.id);
    if fs::read_link(&link_path).is_ok() {
        fs::remove_file(&link_path).unwrap();
    }
    unixfs::symlink(&uuid, link_path).unwrap();

    let install_path = base.join(paths::TECH_INSTALL_DIR);
    let tech = info::TechniqueInfo::read(install_path.join("info.json"))?;
    let src = private_page
        .join("data")
        .join(group.as_str())
        .join(&tech.short_name);
    let dest = public_page
        .join("data")
        .join(&group.as_str())
        .join(&tech.short_name);
    if dest.is_dir() {
        if fs::remove_dir_all(&dest).is_err() {
            log::error!(
                "failed removing previous published results folder: id {}, uuid {}",
                info.id,
                uuid
            );
            return Err(Error::Unspecified);
        }
    }
    let status = Command::new("mv")
        .args(&[src.to_str().unwrap(), dest.to_str().unwrap()])
        .stdout(Stdio::null())
        .status();
    if status.is_err() || !status.unwrap().success() {
        return Err(Error::Unspecified);
    }

    // remove private results page
    if let Err(_) = fs::remove_dir_all(private_page) {
        return Err(Error::Unspecified);
    }

    Ok(())
}

/// Deletes an unpublished workspace of the technique.
pub fn delete_unpublished_workspace(id: i32, uuid: &str) -> Result<(), Error> {
    db::remove_workspace(id, &uuid).unwrap();
    let group = db::technique_type(id).unwrap();
    fs::remove_dir_all(paths::tech_workspace_path(&group, id, &uuid)).unwrap();
    fs::remove_dir_all(paths::public_page_path().join(&uuid)).unwrap();
    Ok(())
}

pub fn can_run(info: ci::ProjectInfo) -> Result<bool, Error> {
    log::info!("can run: id = {}", &info.id);
    let num = db::get_unpublished(info.id).unwrap().len();
    let max = SystemConfig::load().max_num_workspaces as usize;
    if num >= max {
        log::info!("can not run: num({}) >= max({})", num, max);
        return Ok(false);
    }
    log::info!("can run: num({}) < max({})", num, max);
    Ok(true)
}

/// Deletes all unpublished workspaces that are older than the configured limit number of days.
pub fn trim_unpublished() {
    let config = SystemConfig::load();
    for group in vec![info::TechniqueType::DENOISER, info::TechniqueType::SAMPLER] {
        let to_delete = db::get_unpub_older_than(group, config.unpublished_days_limit).unwrap();
        for item in to_delete {
            fs::remove_dir_all(paths::tech_workspace_path(&group, item.0, &item.1))
                .expect("failed to remove workspace");
            fs::remove_dir_all(paths::public_page_path().join(&item.1))
                .expect("failed to remove private page");
            db::remove_workspace(item.0, &item.1).unwrap();
            log::info!(
                "old workspace deleted: id = {}, uuid = {}",
                &item.0,
                &item.1
            );
        }
    }
}

// Unpublished a technique, setting it's status to `Finished`.
pub fn unpublish_technique(id: i32) -> Result<(), Error> {
    let (group, uuid) = db::unpublish_workspace(id).unwrap();
    workspace::unpublish_technique(group, id, &uuid).unwrap();
    Ok(())
}

/// Create a temporary workspace for a technique including missing scenes.
/// Returns Ok(true) if any missing scene was included.
pub fn init_missing_scenes_workspace(proj: &ci::ProjectInfo, uuid: &String) -> Result<bool, Error> {
    log::info!(
        "init missing scenes workspace: id = {}, uuid = {}",
        &proj.id,
        &uuid
    );
    let group = db::technique_type(proj.id).unwrap();
    match workspace::create_tmp_technique_workspace(&group, proj, &uuid) {
        Ok(has_scenes) => Ok(has_scenes),
        Err(_) => Err(Error::Unspecified),
    }
}

/// Updates unpublished results page.
pub fn update_results(proj: ci::ProjectInfo, uuid: String) -> Result<(), Error> {
    log::info!("update results: id = {}, uuid = {}", &proj.id, &uuid);
    let group = db::technique_type(proj.id).unwrap();
    workspace::save_technique_tmp_workspace(group, proj.id, &uuid, false, false);

    // update unpublished results page
    let install_path = paths::tech_install_path(&group, proj.id, &uuid);
    let tech = info::TechniqueInfo::read(install_path.join("info.json"))?;
    let src = paths::tech_results_path(&group, proj.id, &uuid);
    let dest = paths::public_page_path()
        .join(&uuid)
        .join("data")
        .join(group.as_str())
        .join(&tech.short_name);
    workspace::export_technique_images(&src, &dest, true)?;
    workspace::set_public_page_permissions()?;
    Ok(())
}
