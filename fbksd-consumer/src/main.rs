//! fbksd-consumer binary.
//!
//! The consumer executes the tasks pushed to the database by `fbksd-producer`.
//! This process will run continually in the server.
//! Since this process will launch containers to execute the tasks,
//! it needs to run outside a container to avoid docker-in-docker issues.

use fbksd_core;
use fbksd_core::ci::ProjectInfo;
use fbksd_core::db;
use fbksd_core::docker;
use fbksd_core::info::TechniqueInfo;
use fbksd_core::paths;

use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::process::Command;
use std::{thread, time};

fn main() {
    loop {
        let task = db::pop_next_task();
        match task {
            Ok(Some(task)) => {
                execute_task(task);
                return;
            }
            _ => {}
        }
        thread::sleep(time::Duration::from_secs(10));
    }
}

fn execute_task(task: Box<dyn db::Task>) {
    match task.get_type() {
        db::TaskType::Build => {
            //TODO: select the correct docker image
            //TODO: set environment for docker and mount volumes
            // docker::run("fbksd-ci", &["build"]).expect("failed to execute build task");
            db::push_msg_task("maneh@musicalnr.com", "FBKSD status update", "test")
                .expect("Failed to queue message");
        }
        db::TaskType::RunBenchmark => {}
        db::TaskType::PublishResults => {}
    }
}

fn build(task: Box<dyn db::Task>) {
    let result = docker::Docker::new("fbksd-ci")
        .env_vars(&[
            &format!("FBKSD_DATA_ROOT={:?}", paths::data_root()),
            "FBKSD_SERVER_ADDR=fbksd-server:8096",
        ])
        .mounts(&[
            &format!(
                "{:?}:{}",
                paths::database_path(),
                "/mnt/fbksd-data/server.db"
            ),
            &format!(
                "{:?}:{}",
                paths::tmp_workspace_path(),
                "/mnt/fbksd-data/tmp"
            ),
            &format!("{:?}:{}:ro", paths::scenes_path(), "/mnt/fbksd-data/scenes"),
            &format!(
                "{:?}:{}:ro",
                paths::renderers_path(),
                "/mnt/fbksd-data/renderers"
            ),
            &format!("{:?}:{}:ro", paths::LOCK_FILE, paths::LOCK_FILE),
        ])
        .network("fbksd-net")
        .run("fbksd-ci");
}
