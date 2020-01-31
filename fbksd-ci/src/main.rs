//! fbksd-ci binary.
//!
//! The fbksd-ci binary will be called from within the ci build docker container.
//! The working directory is the root of the cloned technique folder.

mod client;
mod cmake;
use client::Client;

use fbksd_core;
use fbksd_core::cd;
use fbksd_core::ci::ProjectInfo;
use fbksd_core::config;
use fbksd_core::flock;
use fbksd_core::info::TechniqueInfo;
use fbksd_core::paths;
use fbksd_core::utils::CD;

use clap::{load_yaml, App};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs as unixfs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn register_current_technique() {
    let proj = ProjectInfo::load().unwrap();
    let tech = TechniqueInfo::read(PathBuf::from("info.json")).unwrap();
    Client::new().register(proj, tech);
}

/// Makes sure all files were installed inside the cmake install prefix and that the `info.json` file was installed.
fn verify_install() {
    let prefix = env::current_dir()
        .expect("Error getting current directory")
        .join("install");
    let prefix_str = prefix.to_str().expect("Invalid path");
    let file = fs::File::open("install_manifest.txt").expect("Can not open file");
    let has_bad_lines = BufReader::new(file)
        .lines()
        .map(|x| x.unwrap())
        .skip_while(|x| x.starts_with(prefix_str))
        .count()
        > 0;
    if has_bad_lines {
        eprintln!("Something was installed in the wrong place");
        std::process::exit(1);
    }
    TechniqueInfo::read(prefix.join("info.json")).expect("Failed to open info.json file");
}

fn validate_ci() {
    ProjectInfo::load().unwrap();
}

fn install() {
    register_current_technique();

    fs::create_dir("build").expect("Failed to create build directory");
    let _cd = CD::new("build");
    // configure
    let status = cmake::config(cmake::BuildType::Release, "install", "../")
        .expect("Failed to execute cmake");
    if !status.success() {
        std::process::exit(1);
    }

    // build and install
    let status = cmake::install().expect("Failed to execute cmake");
    if !status.success() {
        std::process::exit(1);
    }

    verify_install();
}

fn run() {
    let proj = ProjectInfo::load().unwrap();
    let tech = TechniqueInfo::read(PathBuf::from("info.json"))
        .expect("Failed to read info.json from project root dir");

    let client = Client::new();
    client.can_run(proj.clone());

    // create temporary workspace
    let tmp_workspace = paths::tmp_workspace_path();
    if tmp_workspace.is_dir() {
        fs::remove_dir_all(&tmp_workspace).expect("Failed to clean temporary workspace dir");
    }
    fs::create_dir_all(&tmp_workspace).expect("Failed to create temporary workspace");
    unixfs::symlink(tmp_workspace, "workspace").expect("Failed to crate local workspace");

    // init and config new workspace
    {
        let _cd = CD::new("workspace");
        unixfs::symlink(paths::renderers_path(), "renderers")
            .expect("Failed to crate results link");
        let status = Command::new("fbksd")
            .args(&[
                "init",
                "--scenes-dir",
                paths::scenes_path().to_str().unwrap(),
            ])
            .stdout(Stdio::null())
            .status()
            .expect("Failed to execute command");
        if !status.success() {
            std::process::exit(1);
        }
    }
    let status = Command::new("mv")
        .args(&[
            "build/install",
            format!("workspace/{}/{}", tech.technique_type.as_str(), &proj.id).as_str(),
        ])
        .stdout(Stdio::null())
        .status()
        .expect("Failed to execute command");
    if !status.success() {
        std::process::exit(1);
    }
    {
        let _cd = CD::new("workspace");
        if config::fbksd_config().is_err() {
            std::process::exit(1);
        }

        // fbksd run
        let status = Command::new("fbksd")
            .arg("run")
            .status()
            .expect("Failed to execute command");
        if !status.success() {
            std::process::exit(1);
        }

        // fbksd results compute
        let status = Command::new("fbksd")
            .args(&["results", "compute"])
            .status()
            .expect("Failed to execute command");
        if !status.success() {
            std::process::exit(1);
        }
    }

    let uuid = client.save_results(proj.clone(), tech);
    client.publish_results_private(proj, &uuid);

    let link = format!("https://fbksd.inf.ufrgs.br/results/{}", &uuid);
    println!("Results Link:");
    println!("{}", "*".repeat(link.len()));
    println!("{}", link);
    println!("{}", "*".repeat(link.len()));
}

fn publish() {
    let proj = ProjectInfo::load().unwrap();
    const FBKSD_PUBLISH: &str = "FBKSD_PUBLISH";
    let uuid = env::var(FBKSD_PUBLISH).expect(&format!("Evn var {} not defined", FBKSD_PUBLISH));
    let client = Client::new();
    // run benchmark for missing scenes (if any)
    if client
        .init_missing_scenes_workspace(proj.clone(), &uuid)
        .is_some()
    {
        cd!(paths::tmp_workspace_path(), {
            // fbksd run
            let status = Command::new("fbksd")
                .arg("run")
                .status()
                .expect("Failed to execute command");
            if !status.success() {
                std::process::exit(1);
            }
            // fbksd results compute
            let status = Command::new("fbksd")
                .args(&["results", "compute"])
                .status()
                .expect("Failed to execute command");
            if !status.success() {
                std::process::exit(1);
            }
            client.update_results(proj.clone(), &uuid);
        });
    }
    client.publish_results_public(proj, &uuid);
    let link = "https://fbksd.inf.ufrgs.br/results/";
    println!("Results published:");
    println!("{}", "*".repeat(link.len()));
    println!("{}", link);
    println!("{}", "*".repeat(link.len()));
}

fn delete_workspace() {
    let proj = ProjectInfo::load().unwrap();
    const FBKSD_DELETE_WORKSPACE: &str = "FBKSD_DELETE_WORKSPACE";
    let uuid = env::var(FBKSD_DELETE_WORKSPACE)
        .expect(&format!("Evn var {} not defined", FBKSD_DELETE_WORKSPACE));
    let client = Client::new();
    client.delete_workspace(proj, &uuid);
    println!("Workspace deleted");
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    match matches.subcommand_name() {
        Some("validate-ci") => validate_ci(),
        Some("install") => install(),
        Some("run") => flock! { run() },
        Some("publish") => flock! { publish() },
        Some("delete-workspace") => flock! { delete_workspace() },
        None => println!("No subcommand was used"),
        _ => unreachable!(),
    }
}
