//! fbksd-server binary.
//!
//! This server program continually runs on a docker container and is responsible for executing tasks requested
//! by the fbksd-ci program that handle sensitive data.
//! This separation prevents the fbksd-ci program (which handles untrusted code) from having direct access to the data.

use fbksd_core;
use fbksd_core::ci::ProjectInfo;
use fbksd_core::system_config::SystemConfig;
use fbksd_core::msgs::{Error, Msg, MsgResult};
use fbksd_core::page;
use fbksd_core::paths;
use fbksd_core::registry as reg;
use fbksd_core::workspace as wp;
use reg::{Registry, Technique};
use wp::Workspace;

use glob::glob;
use log;
use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
// use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
// use log4rs::encode::pattern::PatternEncoder;
use serde::Deserialize;
use serde_json;
use std::fs;
use std::fs::File;
use std::net::TcpListener;
use std::os::unix::fs as unixfs;
use std::process::{Command, Stdio};

fn register(info: ProjectInfo, tech: Technique) -> MsgResult {
    log::info!("register: id = {}, name = {}", &info.id, &tech.short_name);
    let mut registry = Registry::load();
    if let Err(_) = registry.register(&info, &tech) {
        log::warn!("failed to register technique");
        return Err(Error::Unspecified);
    }
    registry.save();
    Ok(String::from("Registered."))
}

fn save_results(proj: ProjectInfo, tech: Technique) -> MsgResult {
    log::info!(
        "save results: id = {}, name = {}",
        &proj.id,
        &tech.short_name
    );
    let mut registry = Registry::load();
    let uuid = match registry.add_workspace(&proj) {
        Ok(uuid) => uuid,
        Err(reg::Error::MaxWorkspacesExceeded) => return Err(Error::MaxWorkspacesExceeded),
        Err(_) => return Err(Error::Unspecified),
    };

    let group = registry.technique_type(&proj.id).unwrap();
    let base = paths::tech_workspace_path(&group, &proj.id, &uuid);
    if fs::create_dir_all(&base.join(paths::TECH_RESULTS_DIR)).is_err() {
        log::error!("failed to create workspace results folder");
        return Err(Error::Unspecified);
    }

    // move install files
    let src = paths::tmp_workspace_path()
        .join(group.as_str())
        .join(&proj.id);
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
    registry.save();
    log::info!("results saved in private folder");
    Ok(uuid)
}

fn publish_private(proj: ProjectInfo, uuid: String) -> MsgResult {
    log::info!("publish private: id = {}, uuid = {}", &proj.id, &uuid);
    let group = reg::Registry::load().technique_type(&proj.id).unwrap();
    let base_path = paths::tech_workspace_path(&group, &proj.id, &uuid);
    let install_path = base_path.join(paths::TECH_INSTALL_DIR);
    let tech = reg::Technique::read(install_path.join("info.json"));
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
    if !wp::export_technique_images(&src, &dest, false) {
        return Err(Error::Unspecified);
    }
    let mut reg = reg::Registry::load();
    if reg.publish_workspace_private(&proj, &uuid).is_err() {
        return Err(Error::Unspecified);
    }
    if wp::set_public_page_permissions().is_err() {
        return Err(Error::Unspecified);
    }
    reg.save();
    Ok(String::new())
}

fn init_missing_scenes_workspace(proj: ProjectInfo, uuid: String) -> MsgResult {
    log::info!(
        "init missing scenes workspace: id = {}, uuid = {}",
        &proj.id,
        &uuid
    );
    let group = reg::Registry::load().technique_type(&proj.id).unwrap();
    match wp::create_tmp_technique_workspace(&group, proj, &uuid) {
        Ok(has_scenes) => {
            if has_scenes {
                Ok(String::new())
            } else {
                Ok(String::from("NO_SCENE"))
            }
        }
        Err(_) => Err(Error::Unspecified),
    }
}

fn update_results(proj: ProjectInfo, uuid: String) -> MsgResult {
    log::info!("update results: id = {}, uuid = {}", &proj.id, &uuid);
    wp::save_technique_tmp_workspace(&proj.id, &uuid, false, false);

    // update unpublished results page
    let group = reg::Registry::load().technique_type(&proj.id).unwrap();
    let install_path = paths::tech_install_path(&group, &proj.id, &uuid);
    let tech = reg::Technique::read(install_path.join("info.json"));
    if tech.is_err() {
        return Err(Error::Unspecified);
    }
    let tech = tech.unwrap();
    let src = paths::tech_results_path(&group, &proj.id, &uuid);
    let dest = paths::public_page_path()
        .join(&uuid)
        .join("data")
        .join(group.as_str())
        .join(&tech.short_name);
    if !wp::export_technique_images(&src, &dest, true) {
        return Err(Error::Unspecified);
    }
    if wp::set_public_page_permissions().is_err() {
        return Err(Error::Unspecified);
    }
    Ok(String::new())
}

// Assumes that publish_private was called for this uuid.
fn publish_public(info: ProjectInfo, uuid: String) -> MsgResult {
    log::info!("publish public: id = {}, uuid = {}", &info.id, &uuid);
    let mut registry = Registry::load();
    match registry.publish_workspace_public(&info, &uuid) {
        Ok(()) => (),
        Err(reg::Error::AlreadyPublished) => return Err(Error::AlreadyPublished),
        _ => return Err(Error::Unspecified),
    }

    let public_page = paths::public_page_path();
    let private_page = public_page.join(&uuid);
    let mut wp = Workspace::load();
    let group = reg::Registry::load().technique_type(&info.id).unwrap();
    wp.load_technique(&group, &info, uuid.to_string());
    wp.export_page(&public_page);
    registry.save();

    // create link to published data
    let base = paths::tech_workspace_path(&group, &info.id, &uuid);
    let link_path = paths::tech_published_wp_path(&group, &info.id);
    if fs::read_link(&link_path).is_ok() {
        fs::remove_file(&link_path)?;
    }
    unixfs::symlink(&uuid, link_path)?;

    let install_path = base.join(paths::TECH_INSTALL_DIR);
    let tech = reg::Technique::read(install_path.join("info.json"));
    if tech.is_err() {
        return Err(Error::Unspecified);
    }
    let tech = tech.unwrap();
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

    Ok(String::from("Published."))
}

fn can_run(info: ProjectInfo) -> MsgResult {
    log::info!("can run: id = {}", &info.id);
    let num = reg::Registry::load().get_unpublished_wps(&info.id).count();
    let max = SystemConfig::load().max_num_workspaces as usize;
    if num >= max {
        log::info!("can not run: num({}) >= max({})", num, max);
        return Err(Error::MaxWorkspacesExceeded);
    }
    log::info!("can run: num({}) < max({})", num, max);
    Ok(num.to_string())
}

fn delete_workspace(info: ProjectInfo, uuid: String) -> MsgResult {
    log::info!("delete workspace: id = {}, uuid = {}", &info.id, &uuid);
    if wp::delete_unpublished_workspace(&info.id, &uuid).is_ok() {
        return Ok(String::from("Workspace removed"));
    }
    Err(Error::Unspecified)
}

fn main() {
    // config logger
    let stdout = ConsoleAppender::builder().build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config).unwrap();

    // create lock file if it doesn't exist.
    File::create(paths::LOCK_FILE).expect("Failed to create lock file");

    // run tcp server
    let listener = TcpListener::bind("0.0.0.0:8096").unwrap_or_else(|err| {
        log::error!("failed listening to 0.0.0.0:8096: {}", err);
        std::process::exit(1);
    });
    log::info!("server started: port {}", 8096);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                log::info!("new connection");
                loop {
                    let mut de = serde_json::Deserializer::from_reader(&stream);
                    let msg = Msg::deserialize(&mut de).unwrap_or(Msg::Invalid);
                    let res = match msg {
                        Msg::Register(info, tech) => register(info, tech),
                        Msg::SaveResults(info, tech) => save_results(info, tech),
                        Msg::PublishPrivate(info, uuid) => publish_private(info, uuid),
                        Msg::InitMissingScenesWP(info, uuid) => {
                            init_missing_scenes_workspace(info, uuid)
                        }
                        Msg::UpdateResults(info, uuid) => update_results(info, uuid),
                        Msg::PublishPublic(info, uuid) => publish_public(info, uuid),
                        Msg::CanRun(info) => can_run(info),
                        Msg::DeleteWorkspace(info, uuid) => delete_workspace(info, uuid),
                        Msg::End => {
                            log::info!("connection ended by client");
                            break;
                        }
                        Msg::Invalid => {
                            log::warn!("invalid message received");
                            Err(Error::InvalidMessage)
                        }
                    };
                    if serde_json::to_writer(&stream, &res).is_err() {
                        log::warn!("failed to send response: broken pipe");
                        break;
                    }
                    if let Err(err) = res {
                        log::warn!("request caused an error: {}", &err);
                        break;
                    }
                }
            }
            Err(_) => {
                log::error!("connection failed");
                std::process::exit(1);
            }
        }
    }
}
