//! Module for handling fbksd configurations.
//!
//! fbksd configurations describe how a benchmark should be executed: what scenes, techniques, and spps.

use crate::paths;
use crate::utils;
use crate::workspace as wp;
use crate::system_config::SystemConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs as unixfs;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize, Serialize)]
struct Technique {
    name: String,
    versions: Vec<String>,
}

impl Technique {
    fn new(name: &str) -> Self {
        Technique {
            name: name.to_string(),
            versions: vec![String::from("default")],
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Scene {
    name: String,
    spps: Vec<i32>,
}

impl Scene {
    fn new(name: &str) -> Self {
        Scene {
            name: name.to_string(),
            spps: vec![],
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Renderer {
    name: String,
    scenes: Vec<Scene>,
}

impl Renderer {
    fn new(name: &str, scenes: Vec<Scene>) -> Self {
        Renderer {
            name: name.to_string(),
            scenes,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    renderers: Vec<Renderer>,
    filters: Vec<Technique>,
    samplers: Vec<Technique>,
}

impl Config {
    fn new() -> Config {
        Config {
            renderers: Vec::new(),
            filters: Vec::new(),
            samplers: Vec::new(),
        }
    }

    fn save(&self, file: &Path) {
        let data = serde_json::to_string_pretty(self).expect("Error serializing config.");
        fs::write(file, &data).expect("Error saving config.");
    }

    fn add_technique(&mut self, group: &str, name: &str) {
        let tech = Technique::new(name);
        match group {
            "denoisers" => self.filters.push(tech),
            "samplers" => self.samplers.push(tech),
            _ => panic!("invalid group"),
        }
    }

    fn set_spps(&mut self, spps: &[i32]) {
        for r in &mut self.renderers {
            for s in &mut r.scenes {
                s.spps = Vec::from(spps);
            }
        }
    }
}

/// Generates a config for the given technique and scenes.
///
/// This does not uses the `fbksd` script.
/// Binaries, and results are not copied.
pub fn gen_config<'a, I, J, K>(path: &Path, denoisers: I, samplers: I, scenes: J)
where
    I: IntoIterator<Item = &'a K>,
    J: IntoIterator<Item = &'a K>,
    K: AsRef<str> + 'a,
{
    unixfs::symlink(
        utils::relative_from(paths::scenes_path(), path).unwrap(),
        path.join("scenes"),
    )
    .expect("failed to link scenes folder");
    unixfs::symlink(
        utils::relative_from(paths::renderers_path(), path).unwrap(),
        path.join("renderers"),
    )
    .expect("failed to link renderers folder");
    unixfs::symlink(
        utils::relative_from(paths::iqa_path(), path).unwrap(),
        path.join("iqa"),
    )
    .expect("failed to link iqa folder");
    fs::create_dir(path.join("denoisers")).expect("failed to create denoisers dir");
    fs::create_dir(path.join("samplers")).expect("failed to create samplers dir");
    fs::create_dir(path.join("configs")).expect("failed to create configs dir");
    fs::create_dir_all(path.join("results/Results 1")).expect("failed to create results dir");
    unixfs::symlink("Results 1", path.join("results/.current")).expect("failed to link iqa folder");

    let data = fs::read_to_string(paths::scenes_path().join(".fbksd-scenes-cache.json"))
        .expect("Failed reading the scenes file");
    let all_scenes: Vec<wp::Renderer> =
        serde_json::from_str(&data).expect("Failed deserializing the info json");
    let mut scene_render_map: HashMap<String, String> = HashMap::new();
    for (r, s) in all_scenes.iter().flat_map(|r| {
        r.scenes
            .iter()
            .map(move |s| (r.renderer.clone(), s.name.clone()))
    }) {
        scene_render_map.insert(s.clone(), r.clone());
    }

    let mut config = Config::new();
    for tech in denoisers {
        config.add_technique("denoisers", tech.as_ref());
    }
    for tech in samplers {
        config.add_technique("samplers", tech.as_ref());
    }
    let mut renderers: HashMap<String, Vec<Scene>> = HashMap::new();
    for s in scenes {
        let renderer_name = match scene_render_map.get(s.as_ref()) {
            Some(name) => name,
            None => continue,
        };
        let config_scene = Scene::new(s.as_ref());
        match renderers.get_mut(renderer_name) {
            Some(scenes) => scenes.push(config_scene),
            None => {
                renderers.insert(renderer_name.clone(), vec![config_scene]);
            }
        }
    }
    for r in renderers {
        let renderer = Renderer::new(&r.0, r.1);
        config.renderers.push(renderer);
    }
    config.set_spps(&SystemConfig::load().spps);

    let config_file = path.join("configs/all.json");
    config.save(&config_file);
    unixfs::symlink("all.json", path.join("configs/.current.json")).unwrap();
}

/// Runs `fbksd config new` on the current directory.
///
/// Expects `fbksd` in the current PATH.
pub fn fbksd_config() -> Result<(), ()> {
    let config = SystemConfig::load();
    let spps: Vec<String> = config.spps.iter().map(|i| i.to_string()).collect();
    let spps = spps.iter().map(|i| i.as_str());

    let mut args = vec![
        "config",
        "new",
        "all",
        "--scenes-all",
        "--filters-all",
        "--samplers-all",
        "--select",
        "--spps",
    ];
    args.extend(spps);

    let status = Command::new("fbksd")
        .args(args)
        .stdout(Stdio::null())
        .status();
    if status.is_err() || !status.unwrap().success() {
        return Err(());
    }
    Ok(())
}
