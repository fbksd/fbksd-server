//! fbksd-ctl binary.
//!
//! This is a command line utility that performs administrative tasks in the server.

use fbksd_core::docker;
use fbksd_core::paths;
use fbksd_core::registry as reg;
use fbksd_core::try_flock;
use fbksd_core::utils::CD;
use fbksd_core::utils::*;
use fbksd_core::workspace as wp;
use wp::Workspace;

use clap::{load_yaml, App};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

fn status() {
    if !Path::new(paths::LOCK_FILE).exists() {
        println!("Lock file does not exist.");
        File::create(paths::LOCK_FILE).expect("Failed to create lock file");
    }
    match FLock::try_new() {
        None => println!("Could not acquire lock."),
        Some(_) => println!("Lock acquired."),
    }
}

fn run_all() {
    //TODO: techniques can require different docker images.
    let _lock = FLock::new();
    println!("building temporary workspace...");
    wp::create_tmp_workspace(true);
    println!(" - OK");

    let tmp_workspace = paths::tmp_workspace_path();
    let _cd = CD::new(&tmp_workspace);
    docker::run("fbksd", &["run"]).unwrap();
    docker::run("fbksd", &["results", "compute"]).unwrap();

    println!("saving results...");
    let reg = reg::Registry::load();
    for group in vec![reg::TechniqueType::DENOISER, reg::TechniqueType::SAMPLER] {
        let published = reg.get_published(&group);
        for p in published {
            let base = paths::tech_workspace_path(&group, &p.0, &p.1);
            let tech =
                reg::Technique::read(base.join(paths::TECH_INSTALL_DIR).join("info.json")).unwrap();
            let src = PathBuf::from("results/.current")
                .join(group.as_str())
                .join(&tech.short_name)
                .join("");
            let dest = base.join("results/");
            let status = Command::new("rsync")
                .args(&[
                    "-a",
                    "--ignore-existing",
                    src.to_str().unwrap(),
                    dest.to_str().unwrap(),
                ])
                .status();
            if status.is_err() || !status.unwrap().success() {
                std::process::exit(1);
            }
        }
    }
    println!(" - OK");

    println!("updating page...");
    update_page();
    println!(" - OK");
}

fn update_page() {
    let public_page = paths::public_page_path();
    let wp = Workspace::load();
    wp.export_page(&public_page);
    wp.export_reference_images();
    wp::export_images();
    wp::set_public_page_permissions().unwrap();
}

fn unpublish(id: i32) {
    wp::unpublish_technique(id).expect("Failed to unpublish.");
}

fn update_scenes() {
    try_flock!(
        wp::update_scenes(),
        println!("not updated: being used by other process")
    );
}

fn trim() {
    try_flock!(wp::trim_unpublished(), println!("failed to acquire lock"));
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    match matches.subcommand() {
        ("status", Some(_)) => status(),
        ("run-all", Some(_)) => run_all(),
        ("unpublish", Some(sub)) => {
            let id: i32 = match sub.value_of("id") {
                Some(id) => id.parse().unwrap(),
                None => {
                    eprintln!("missing id argument");
                    std::process::exit(1);
                }
            };
            unpublish(id);
        }
        ("update-page", Some(_)) => update_page(),
        ("update-scenes", Some(_)) => update_scenes(),
        ("trim", Some(_)) => trim(),
        _ => println!("No subcommand was used"),
    }
}
