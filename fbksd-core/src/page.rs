use glob::glob;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs as unixfs;
use std::path::Path;
use std::process::Command;

use crate::paths;
use crate::utils;

#[derive(Serialize, Deserialize)]
pub struct Scene {
    pub id: i32,
    pub name: String,
    pub renderer: String,
    pub reference: String,
    pub thumbnail: String,
}

#[derive(Serialize, Deserialize)]
pub struct Version {
    pub id: i32,
    pub tag: String,
    pub message: String,
    pub status: String,
    pub results_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct Technique {
    pub id: i32,
    pub name: String,
    pub full_name: String,
    pub comment: String,
    pub citation: String,
    pub versions: Vec<Version>,
}

#[derive(Serialize, Deserialize)]
pub struct Metric {
    pub acronym: String,
    pub name: String,
    pub reference: String,
    pub lower_is_better: bool,
    pub has_error_map: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Result {
    pub scene_id: i32,
    pub spp: i32,
    pub filter_version_id: i32,
    pub exec_time: i64,
    pub aborted: bool,
    pub metrics: HashMap<String, f32>,
}

/// Creates a new page directory with the given path.
///
/// The new page is created to be a cheap copy of the public page,
/// using symbolic links instead of copying the data. Only the html/js stuff is copied.
///
/// The `individual_links_group` is expanded and all subfolders are linked individually.
/// The `ignored_tech` is a technique name that will not have its results linked.
///
/// The created page also contains:
///  - "scenes" link to the "public/scenes" folder
///  - empty "data" folder
pub fn copy_public_page(dest: &Path, individual_links_group: &str, ignored_tech: &str) -> bool {
    if dest.is_dir() {
        return false;
    }
    let page_dir = paths::page_path().join("");
    // copy empty page files form template
    let status = Command::new("rsync")
        .args(&[
            "-a",
            "--no-links",
            page_dir.to_str().unwrap(),
            dest.to_str().unwrap(),
        ])
        .status();
    if status.is_err() || !status.unwrap().success() {
        return false;
    }
    // link scenes dir
    let scenes_dir = utils::relative_from(&paths::public_page_path().join("scenes"), dest).unwrap();
    if unixfs::symlink(&scenes_dir, &dest.join("scenes")).is_err() {
        return false;
    }
    // create empty data folder
    fs::create_dir_all(dest.join("data").join(&individual_links_group))
        .expect("Failed to create results dir");

    let full_link_group = match individual_links_group {
        "denoisers" => "samplers",
        "samplers" => "denoisers",
        _ => panic!("invalid group"),
    };
    let pub_res = paths::public_page_path()
        .join("data")
        .join(&full_link_group);
    let pub_res = utils::relative_from(&pub_res, &dest.join("data")).unwrap();
    unixfs::symlink(&pub_res, &dest.join("data").join(&full_link_group))
        .expect("Failed to crate local workspace");

    let cd = dest.join("data").join(&individual_links_group);
    cd!(&cd, {
        let pattern = paths::public_page_path()
            .join("data")
            .join(&individual_links_group)
            .join("*");
        for entry in glob(pattern.to_str().expect("Failed path to string"))
            .expect("Failed to read glob pattern")
        {
            if let Ok(path) = entry {
                let tech = path.file_name().unwrap();
                if tech == ignored_tech {
                    continue;
                }
                let path = utils::relative_from(&path, &cd).unwrap();
                unixfs::symlink(&path, &tech).expect("Failed to crate local workspace");
            }
        }
    });
    return true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_public_page() {}
}
