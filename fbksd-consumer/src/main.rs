//! fbksd-consumer binary.
//!
//! The consumer executes the tasks pushed to the database by `fbksd-producer`.
//! This process will run continually in the server.
//! Since this process will launch containers to execute the tasks,
//! it needs to run outside a container to avoid docker-in-docker issues.

use fbksd_core;
use fbksd_core::db;
use fbksd_core::docker;
use fbksd_core::paths;

use log;
use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};

use std::{thread, time};

fn main() {
    // config logger
    let stdout = ConsoleAppender::builder().build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config).unwrap();

    log::info!("Process started.");

    loop {
        let task = db::pop_next_task();
        match task {
            Ok(Some(task)) => {
                log::info!("Got new task: {:?}", &task.get_type());
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
            // docker::run("fbksd-ci", &["build"]).expect("failed to execute build task");
            build(task);
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
            (
                paths::database_path().to_str().unwrap(),
                "/mnt/fbksd-data/server.db",
            ),
            (
                paths::tmp_workspace_path().to_str().unwrap(),
                "/mnt/fbksd-data/tmp",
            ),
        ])
        .mounts_ro(&[
            (
                paths::scenes_path().to_str().unwrap(),
                "/mnt/fbksd-data/scenes",
            ),
            (
                paths::renderers_path().to_str().unwrap(),
                "/mnt/fbksd-data/renderers",
            ),
            (paths::LOCK_FILE, paths::LOCK_FILE),
        ])
        .network("fbksd-net")
        .run("fbksd-ci", &["build"]);
    match result {
        Ok(_) => {
            db::push_msg_task(
                "maneh@musicalnr.com",
                "FBKSD status update",
                "Task succeeded.",
            )
            .expect("Failed to queue message");
        }
        Err(_) => {
            db::push_msg_task("maneh@musicalnr.com", "FBKSD status update", "Task failed.")
                .expect("Failed to queue message");
        }
    }
}
