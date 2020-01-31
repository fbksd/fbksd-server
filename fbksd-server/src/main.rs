//! fbksd-server binary.
//!
//! This server program continually runs on a docker container and is responsible for executing tasks requested
//! by the fbksd-ci program that handle sensitive data.
//! This separation prevents the fbksd-ci program (which handles untrusted code) from having direct access to the data.

use fbksd_core;
use fbksd_core::ci::ProjectInfo;
use fbksd_core::info::TechniqueInfo;
use fbksd_core::msgs::{Error, Msg, MsgResult};
use fbksd_core::paths;

use log;
use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};
use serde::Deserialize;
use serde_json;
use std::fs::File;
use std::net::TcpListener;

fn register(info: ProjectInfo, tech: TechniqueInfo) -> MsgResult {
    match fbksd_core::register(&info, &tech) {
        Ok(_) => Ok(String::from("Registered.")),
        Err(err) => Err(Error::from(err)),
    }
}

fn save_results(info: ProjectInfo, tech: TechniqueInfo) -> MsgResult {
    match fbksd_core::save_results(info, tech) {
        Ok(uuid) => Ok(uuid),
        Err(err) => Err(Error::from(err)),
    }
}

fn publish_private(info: ProjectInfo, uuid: String) -> MsgResult {
    match fbksd_core::publish_private(info, uuid) {
        Ok(_) => Ok(String::from("Published in the user's private location.")),
        Err(err) => Err(Error::from(err)),
    }
}

fn init_missing_scenes_workspace(proj: ProjectInfo, uuid: String) -> MsgResult {
    match fbksd_core::init_missing_scenes_workspace(&proj, &uuid) {
        Ok(has_scenes) => {
            if has_scenes {
                Ok(String::new())
            } else {
                Ok(String::from("NO_SCENE"))
            }
        }
        Err(err) => Err(Error::from(err)),
    }
}

fn update_results(proj: ProjectInfo, uuid: String) -> MsgResult {
    match fbksd_core::update_results(proj, uuid) {
        Ok(_) => Ok(String::from("Results page updated")),
        Err(err) => Err(Error::from(err)),
    }
}

// Assumes that publish_private was called for this uuid.
fn publish_public(info: ProjectInfo, uuid: String) -> MsgResult {
    match fbksd_core::publish_public(info, uuid) {
        Ok(_) => Ok(String::from("Published")),
        Err(err) => Err(Error::from(err)),
    }
}

fn can_run(info: ProjectInfo) -> MsgResult {
    match fbksd_core::can_run(info) {
        Ok(can) => {
            if can {
                return Ok(String::from("true"));
            } else {
                return Ok(String::from("false"));
            }
        }
        Err(err) => Err(Error::from(err)),
    }
}

fn delete_workspace(info: ProjectInfo, uuid: String) -> MsgResult {
    match fbksd_core::delete_unpublished_workspace(info.id, &uuid) {
        Ok(_) => Ok(String::from("Workspace removed")),
        Err(err) => Err(Error::from(err)),
    }
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
                    let res = match Msg::deserialize(&mut de) {
                        Ok(msg) => match msg {
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
                        },
                        Err(_) => {
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
