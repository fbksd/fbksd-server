//! fbksd-producer binary.
//!
//! The fbksd-producer binary is executed (inside a container) by the CI runner.
//! It is responsible for getting tasks from the CI system and pushing them to the database.
//! These tasks will be executed by fbksd-ci process latter in another container.
//!
//! There are two queues: a normal queue, and a priority queue.
//! The consumer always processes all the tasks in the priority queue before start processing
//! tasks in the normal queue.

use fbksd_core;
use fbksd_core::ci::ProjectInfo;
use fbksd_core::db;
use fbksd_core::info::TechniqueInfo;
use fbksd_core::paths;

use clap::{load_yaml, App};
use std::env;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::process::Command;

fn register(info: &ProjectInfo, tech: &TechniqueInfo) {
    fbksd_core::register(info, tech).expect("Failed to register technique.");
}

fn print_msg(msg: &str) {
    println!("{}", "*".repeat(msg.len()));
    println!("{}", msg);
    println!("{}", "*".repeat(msg.len()));
}

// This tasks is executed immediately here, not queued in the database.
// It's executed before the container is created.
fn validate_ci() {
    ProjectInfo::load().unwrap();
}

fn install() {
    let info = ProjectInfo::load().unwrap();
    let tech = TechniqueInfo::read(PathBuf::from("info.json")).unwrap();
    register(&info, &tech);
    db::push_build_task(&info).expect("Failed to crate enqueue build task.");

    // copy source
    let src = paths::tech_project_src_path(tech.technique_type, info.id).join("");
    create_dir_all(&src).expect("Failed to create source code directory");
    let status = Command::new("rsync")
        .args(&["-a", ".", src.to_str().unwrap()])
        .status();
    if status.is_err() || !status.unwrap().success() {
        panic!("Failed to copy project source code.");
    }

    print_msg("Build task was queued. You will receive a notification by e-mail when the tasks is executed.");
}

fn run() {
    let info = ProjectInfo::load().unwrap();
    db::push_run_task(&info).expect("Failed to crate enqueue run task.");

    print_msg("Benchmark execution task was queued. You will receive a notification by e-mail when the tasks is executed.");
}

fn publish() {
    const FBKSD_PUBLISH: &str = "FBKSD_PUBLISH";
    let info = ProjectInfo::load().unwrap();
    let uuid = env::var(FBKSD_PUBLISH).expect(&format!("Evn var {} not defined", FBKSD_PUBLISH));
    db::push_publish_task(&info, &uuid).expect("Failed to crate publish run task.");
}

fn delete_workspace() {}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    match matches.subcommand_name() {
        Some("validate-ci") => validate_ci(),
        Some("install") => install(),
        Some("run") => run(),
        Some("publish") => publish(),
        Some("delete-workspace") => delete_workspace(),
        None => println!("No subcommand was used"),
        _ => unreachable!(),
    }
}
