//! fbksd-ci binary.
//!
//! The fbksd-ci binary is executed by the CI runner and is responsible for getting tasks from the CI system
//! and pushing them to the queue as fast as possible.
//! These tasks will be executed by fbksd-consumer process latter.
//!
//! This process runs in the host (no container) with the `fbksd-ci` user, and does not directly executes
//! untrusted code.
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

fn print_msg(msg: &str) {
    let msg_string = String::from(msg);
    let segs = msg_string.split('\n');
    let mut len = 0;
    for seg in segs {
        len = std::cmp::max(len, seg.len());
    }
    println!("{}", "*".repeat(len));
    println!("{}", msg);
    println!("{}", "*".repeat(len));
}

fn register(info: &ProjectInfo, tech: &TechniqueInfo) {
    if let Err(err) = fbksd_core::register(info, tech) {
        print_msg(&format!("Failed to register technique.\n  - {}", err));
        std::process::exit(1);
    }
}

// This function is executed immediately, and is always executed before the others
// (using the GitLab runner's `pre_build_scrip` method).
fn get_project_info() -> ProjectInfo {
    match ProjectInfo::load() {
        Ok(info) => info,
        Err(err) => {
            print_msg(&format!("CI configuration is invalid.\n  - {}", err));
            std::process::exit(1);
        }
    }
}

fn install() {
    let info = get_project_info();
    let tech = TechniqueInfo::read(PathBuf::from("info.json")).unwrap();
    register(&info, &tech);
    if let Err(err) = db::push_build_task(&info) {
        print_msg(&format!("Failed to queue the build task.\n  - {}", err));
        std::process::exit(1);
    }

    // copy source
    let src = paths::tech_project_src_path(tech.technique_type, info.id);
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
    let info = get_project_info();
    if let Err(err) = db::push_run_task(&info) {
        print_msg(&format!("Failed to queue run task.\n  - {}", err));
        std::process::exit(1);
    }

    print_msg("Benchmark execution task was queued. You will receive a notification by e-mail when the tasks is executed.");
}

fn publish() {
    let info = get_project_info();
    const FBKSD_PUBLISH: &str = "FBKSD_PUBLISH";
    let uuid = env::var(FBKSD_PUBLISH).expect(&format!("Evn var {} not defined", FBKSD_PUBLISH));
    db::push_publish_task(&info, &uuid).expect("Failed to crate publish run task.");
}

fn delete_workspace() {}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    match matches.subcommand_name() {
        Some("install") => install(),
        Some("run") => run(),
        Some("publish") => publish(),
        Some("delete-workspace") => delete_workspace(),
        None => println!("No subcommand was used"),
        _ => unreachable!(),
    }
}
