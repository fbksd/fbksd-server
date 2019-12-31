use crate::ci;
use crate::config;
use crate::page;
use crate::paths;
use crate::registry as reg;
use crate::utils;
use reg::TechniqueType;

use glob::glob;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::env;
use std::error;
use std::fmt;
use std::fs;
use std::os::unix::fs as unixfs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug)]
pub enum Error {
    UuidNotFound,
    Unspecified,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match *self {
            UuidNotFound => "uuid not found for the given technique".fmt(f),
            Unspecified => "unspecified error".fmt(f),
        }
    }
}
impl error::Error for Error {}
type WPResult<T> = std::result::Result<T, Error>;

#[derive(Debug, Deserialize, Serialize)]
pub struct Scene {
    pub name: String,
    path: String,

    #[serde(rename = "ref-img")]
    ref_img: String,
    #[serde(rename = "ref", default)]
    citation: String,
    #[serde(skip)]
    png: String,
    #[serde(skip)]
    thumb: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Renderer {
    pub renderer: String,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Deserialize)]
struct Metric {
    acronym: String,
    name: String,
    reference: String,
    lower_is_better: bool,
    has_error_map: bool,
    command: String,
}

impl Metric {
    pub fn read(info: PathBuf) -> Metric {
        let data = fs::read_to_string(info).expect("Failed reading the \"info.json\" file");
        let metric: Metric =
            serde_json::from_str(&data).expect("Failed deserializing the info json");
        metric
    }
}

#[derive(Debug, Deserialize)]
struct ExecTime {
    time_ms: i64,
}

#[derive(Debug, Deserialize)]
struct Result {
    aborted: bool,
    date: String,
    exec_time: ExecTime,
    spp_budget: i32,

    #[serde(skip)]
    id: i32,
    #[serde(skip)]
    metrics: HashMap<String, f32>,
    #[serde(skip)]
    scene_name: String,
}

impl Result {
    /// Given the path to the <SPP>_0_log.json, it reads that and the metrics.
    fn read(log: PathBuf, scene_name: &str) -> Result {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(\d+)_0_([^/]+)_value.json$").unwrap();
        }
        let data = fs::read_to_string(&log).expect("Failed reading the \"info.json\" file");
        let mut res: Result = serde_json::from_str(&data).expect("Failed to deserialize log file");
        res.scene_name = String::from(scene_name);
        let spp = res.spp_budget;
        let _cd = utils::CD::new(&log.parent().unwrap());
        for entry in
            glob(format!("{}_0_*_value.json", spp).as_str()).expect("Failed to read glob pattern")
        {
            if let Ok(path) = entry {
                let caps = RE.captures(path.to_str().unwrap()).unwrap();
                let metric = caps.get(2).unwrap().as_str();
                let data = fs::read_to_string(&path).expect("Failed reading metric value file");
                let val: HashMap<String, f32> =
                    serde_json::from_str(&data).expect("Failed to deserialize log file");
                let _ = res
                    .metrics
                    .insert(String::from(metric), *val.get(metric).unwrap());
            }
        }
        res
    }
}

#[derive(Deserialize, Debug)]
struct Technique {
    pub short_name: String,
    pub full_name: String,
    pub comment: String,
    pub citation: String,

    #[serde(skip)]
    pub results: Vec<Result>,
    #[serde(skip)]
    id: i32,
}

impl Technique {
    fn read(id: i32, path: PathBuf) -> WPResult<Technique> {
        let data = match fs::read_to_string(path.join("install/info.json")) {
            Ok(data) => data,
            Err(_) => return Err(Error::Unspecified),
        };

        let mut tech: Self = match serde_json::from_str(&data) {
            Ok(tech) => tech,
            Err(_) => return Err(Error::Unspecified),
        };
        tech.id = id;

        // load all results
        for entry in glob(path.join("results/*/*/*_log.json").to_str().unwrap())
            .expect("Failed to read glob pattern")
        {
            if let Ok(path) = entry {
                let scene_name = path.parent().unwrap();
                let scene_name: Vec<_> = scene_name
                    .components()
                    .map(|comp| comp.as_os_str())
                    .collect();
                let scene_name = scene_name.last().unwrap().to_str().unwrap();
                tech.results.push(Result::read(path.clone(), &scene_name));
            }
        }
        Ok(tech)
    }

    /// Returns a set with the names of all scenes this technique has results for.
    fn scenes(&self) -> HashSet<String> {
        self.results.iter().map(|x| x.scene_name.clone()).collect()
    }
}

#[derive(Debug)]
pub struct Workspace {
    renderers: Vec<Renderer>,
    denoisers: Vec<Technique>,
    samplers: Vec<Technique>,
    metrics: Vec<Metric>,
}

impl Workspace {
    fn load_scenes(&mut self, file: PathBuf) {
        let data = fs::read_to_string(&file).expect("Failed reading the scenes file");
        self.renderers = serde_json::from_str(&data).expect("Failed deserializing the info json");
        // complete ref paths
        for r in &mut self.renderers {
            for s in &mut r.scenes {
                let exr = PathBuf::from(&r.renderer).join(&s.ref_img);
                let thumb = exr.file_stem().unwrap();
                let mut thumb = String::from(thumb.to_str().unwrap());
                thumb.push_str("_thumb256.jpg");
                let thumb = exr.with_file_name(thumb);
                let png = exr.with_extension("png");
                // all images are relative to the scenes root folder
                s.ref_img = String::from(exr.to_str().unwrap());
                s.png = String::from(png.to_str().unwrap());
                s.thumb = String::from(thumb.to_str().unwrap());
            }
        }
    }

    fn load_metrics(&mut self, path: &Path) {
        if !path.is_dir() {
            return;
        }
        let _cd = utils::CD::new(&path);
        let pattern = path.join("*/info.json");
        for entry in glob(pattern.to_str().expect("Failed path to string"))
            .expect("Failed to read glob pattern")
        {
            if let Ok(path) = entry {
                self.metrics.push(Metric::read(path));
            }
        }
    }

    fn load_denoisers(&mut self, path: &Path) {
        if !path.is_dir() {
            return;
        }
        let re = Regex::new(r"workspaces/denoisers/(\d+)/published").unwrap();
        let _cd = utils::CD::new(&path);
        let pattern = path.join("*/published");
        for entry in glob(pattern.to_str().expect("Failed path to string"))
            .expect("Failed to read glob pattern")
        {
            if let Ok(path) = entry {
                let caps = re.captures(path.to_str().unwrap()).unwrap();
                let id: i32 = caps.get(1).unwrap().as_str().parse().unwrap();
                let tech = Technique::read(id, path).unwrap();
                self.denoisers.push(tech);
            }
        }
    }

    fn load_samplers(&mut self, path: &Path) {
        if !path.is_dir() {
            return;
        }
        let _cd = utils::CD::new(&path);
        let pattern = path.join("*/published/info.json");
        let re = Regex::new(r"results/samplers/(\d+)/published/info.json$").unwrap();
        for entry in glob(pattern.to_str().expect("Failed path to string"))
            .expect("Failed to read glob pattern")
        {
            if let Ok(path) = entry {
                let caps = re.captures(path.to_str().unwrap()).unwrap();
                let id: i32 = caps.get(1).unwrap().as_str().parse().unwrap();
                let tech = Technique::read(id, path).unwrap();
                self.samplers.push(tech);
            }
        }
    }

    fn update_indices(&mut self) {
        let mut next_id = 0;
        for f in &mut self.denoisers {
            for r in &mut f.results {
                r.id = next_id;
                next_id += 1;
            }
        }

        let mut next_id = 0;
        for f in &mut self.samplers {
            for r in &mut f.results {
                r.id = next_id;
                next_id += 1;
            }
        }
    }

    /// Reads the registry and loads all published techniques
    pub fn load() -> Workspace {
        let mut wp = Workspace {
            renderers: Vec::new(),
            denoisers: Vec::new(),
            samplers: Vec::new(),
            metrics: Vec::new(),
        };
        wp.load_scenes(paths::scenes_path().join(".fbksd-scenes-cache.json"));
        wp.load_metrics(paths::iqa_path());
        wp.load_denoisers(paths::denoisers_workspaces_path());
        wp.load_samplers(paths::samplers_workspaces_path());
        wp.update_indices();
        wp
    }

    pub fn load_technique(&mut self, group: &TechniqueType, proj: &ci::ProjectInfo, uuid: String) {
        let id: i32 = proj.id.parse().unwrap();
        let tech = Technique::read(id, paths::tech_workspace_path(group, &proj.id, &uuid)).unwrap();
        let techs = match group {
            TechniqueType::DENOISER => &mut self.denoisers,
            TechniqueType::SAMPLER => &mut self.samplers,
        };
        // if the technique is already loaded, replace it, otherwise, add it.
        match techs
            .iter()
            .enumerate()
            .find_map(|(i, t)| if t.id == id { Some(i) } else { None })
        {
            Some(i) => techs[i] = tech,
            None => techs.push(tech),
        }
        self.update_indices();
    }

    /// Copy all scenes reference images (png and thumbnail) to the public scenes image folder.
    pub fn export_reference_images(&self) {
        let path = paths::public_page_path().join("scenes");
        if !path.is_dir() {
            fs::create_dir_all(&path)
                .expect("Failed to create destination folder for scene images");
        }
        let src_scenes = paths::scenes_path();
        for r in &self.renderers {
            for s in &r.scenes {
                let exr = path.join(&s.ref_img);
                let dest_img_path = exr.parent().unwrap();
                if !dest_img_path.is_dir() {
                    fs::create_dir_all(dest_img_path)
                        .expect("Failed to create destination folder for scene images");
                }
                fs::copy(src_scenes.join(&s.png), path.join(&s.png))
                    .expect("Failed to export png scene image");
                fs::copy(src_scenes.join(&s.thumb), path.join(&s.thumb))
                    .expect("Failed to export jpg scene image");
            }
        }
    }

    /// saves the page data to the given page folder (not including images).
    pub fn export_page(&self, path: &Path) {
        let path = path.join("data");
        // scenes
        let mut scenes: HashMap<String, page::Scene> = HashMap::new();
        let mut scenes_ids_map: HashMap<String, i32> = HashMap::new();
        let mut next_id = 0;
        for r in &self.renderers {
            for s in &r.scenes {
                scenes.insert(
                    next_id.to_string(),
                    page::Scene {
                        id: next_id,
                        name: s.name.clone(),
                        renderer: r.renderer.clone(),
                        reference: s.png.clone(),
                        thumbnail: s.thumb.clone(),
                    },
                );
                scenes_ids_map.insert(s.name.clone(), next_id);
                next_id += 1;
            }
        }
        let scenes_data =
            serde_json::to_string_pretty(&scenes).expect("Error serializing page scenes.");

        // metrics
        let mut metrics: HashMap<String, page::Metric> = HashMap::new();
        for m in &self.metrics {
            metrics.insert(
                m.acronym.clone(),
                page::Metric {
                    acronym: m.acronym.clone(),
                    name: m.name.clone(),
                    reference: m.reference.clone(),
                    lower_is_better: m.lower_is_better,
                    has_error_map: m.has_error_map,
                },
            );
        }
        let metrics_data =
            serde_json::to_string_pretty(&metrics).expect("Error serializing page metrics.");

        // results
        let mut results: HashMap<String, page::Result> = HashMap::new();
        next_id = 0;
        for f in &self.denoisers {
            for r in &f.results {
                results.insert(
                    next_id.to_string(),
                    page::Result {
                        scene_id: *scenes_ids_map.get(&r.scene_name).unwrap(),
                        spp: r.spp_budget,
                        filter_version_id: f.id,
                        exec_time: r.exec_time.time_ms,
                        aborted: r.aborted,
                        metrics: r.metrics.clone(),
                    },
                );
                next_id += 1;
            }
        }
        let results_data =
            serde_json::to_string_pretty(&results).expect("Error serializing page results.");

        // samplers results
        let mut samplers_results: HashMap<String, page::Result> = HashMap::new();
        next_id = 0;
        for f in &self.samplers {
            for r in &f.results {
                samplers_results.insert(
                    next_id.to_string(),
                    page::Result {
                        scene_id: *scenes_ids_map.get(&r.scene_name).unwrap(),
                        spp: r.spp_budget,
                        filter_version_id: f.id,
                        exec_time: r.exec_time.time_ms,
                        aborted: r.aborted,
                        metrics: r.metrics.clone(),
                    },
                );
                next_id += 1;
            }
        }
        let samplers_results_data = serde_json::to_string_pretty(&samplers_results)
            .expect("Error serializing page results.");

        // filters
        let mut filters: Vec<page::Technique> = Vec::new();
        for f in &self.denoisers {
            let results = f.results.iter().map(|r| r.id).collect();
            filters.push(page::Technique {
                id: f.id,
                name: f.short_name.clone(),
                full_name: f.full_name.clone(),
                comment: f.comment.clone(),
                citation: f.citation.clone(),
                versions: vec![page::Version {
                    id: f.id,
                    tag: String::from("default"),
                    message: f.comment.clone(),
                    status: String::from("ready"),
                    results_ids: results,
                }],
            });
        }
        let filters_data =
            serde_json::to_string_pretty(&filters).expect("Error serializing page filters.");

        // samplers
        let mut samplers: Vec<page::Technique> = Vec::new();
        for f in &self.samplers {
            let results = f.results.iter().map(|r| r.id).collect();
            samplers.push(page::Technique {
                id: f.id,
                name: f.short_name.clone(),
                full_name: f.full_name.clone(),
                comment: f.comment.clone(),
                citation: f.citation.clone(),
                versions: vec![page::Version {
                    id: f.id,
                    tag: String::from("default"),
                    message: f.comment.clone(),
                    status: String::from("ready"),
                    results_ids: results,
                }],
            });
        }
        let samplers_data =
            serde_json::to_string_pretty(&samplers).expect("Error serializing page filters.");

        fs::write(path.join("scenes.json"), &scenes_data).expect("Error saving page scenes.");
        fs::write(path.join("iqa_metrics.json"), &metrics_data)
            .expect("Error saving page metrics.");
        fs::write(path.join("results.json"), &results_data).expect("Error saving page results.");
        fs::write(path.join("samplers_results.json"), &samplers_results_data)
            .expect("Error saving page results.");
        fs::write(path.join("filters.json"), &filters_data).expect("Error saving page filters.");
        fs::write(path.join("samplers.json"), &samplers_data).expect("Error saving page samplers.");
    }
}

/// Create a temporary workspace for a technique including missing scenes.
/// Returns Ok(true) if any missing scene was included.
pub fn create_tmp_technique_workspace(
    group: &TechniqueType,
    proj: ci::ProjectInfo,
    uuid: &str,
) -> WPResult<bool> {
    let tmp_workspace = paths::tmp_workspace_path();
    if tmp_workspace.is_dir() {
        fs::remove_dir_all(&tmp_workspace).expect("Failed to clean temporary workspace dir");
    }
    fs::create_dir_all(&tmp_workspace).expect("Failed to create temporary workspace");

    let tech = match Technique::read(
        proj.id.parse().unwrap(),
        paths::tech_workspace_path(group, &proj.id, &uuid),
    ) {
        Ok(tech) => tech,
        Err(err) => {
            return Err(err);
        }
    };

    // find missing scenes and generate config
    let tech_scenes = tech.scenes();
    let data = fs::read_to_string(paths::scenes_path().join(".fbksd-scenes-cache.json"))
        .expect("Failed reading the scenes file");
    let all_scenes: Vec<Renderer> =
        serde_json::from_str(&data).expect("Failed deserializing the info json");
    let all_scenes: HashSet<String> = all_scenes
        .iter()
        .flat_map(|r| r.scenes.iter().map(|s| s.name.clone()))
        .collect();
    let missing_scenes: HashSet<&String> = all_scenes.difference(&tech_scenes).collect();
    if missing_scenes.is_empty() {
        return Ok(false);
    }
    match group {
        TechniqueType::DENOISER => config::gen_config(
            &tmp_workspace,
            &vec![&tech.short_name],
            &vec![],
            &missing_scenes,
        ),
        TechniqueType::SAMPLER => config::gen_config(
            &tmp_workspace,
            &vec![],
            &vec![&tech.short_name],
            &missing_scenes,
        ),
    }

    // copy binaries
    let src = paths::tech_install_path(group, &proj.id, &uuid).join("");
    let dest = tmp_workspace.join(group.as_str()).join(&proj.id);
    let status = Command::new("rsync")
        .args(&["-a", src.to_str().unwrap(), dest.to_str().unwrap()])
        .status();
    if status.is_err() || !status.unwrap().success() {
        return Err(Error::Unspecified);
    }
    Ok(true)
}

/// Creates a new temporary workspace configured with all scenes.
///
/// Optionally you can include all published techniques (results and binaries).
/// This function needs access to the "workspaces" folder and expects `fbksd` in the PATH.
pub fn create_tmp_workspace(include_published: bool) {
    // create temporary workspace
    let tmp_workspace = paths::tmp_workspace_path();
    if tmp_workspace.is_dir() {
        fs::remove_dir_all(&tmp_workspace).expect("Failed to clean temporary workspace dir");
    }
    fs::create_dir_all(&tmp_workspace).expect("Failed to create temporary workspace");
    let _cd = utils::CD::new(&tmp_workspace);

    // init
    unixfs::symlink(paths::renderers_path(), "renderers").expect("Failed to crate results link");
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

    // include published techniques
    if include_published {
        let reg = reg::Registry::load();
        for group in vec![reg::TechniqueType::DENOISER, reg::TechniqueType::SAMPLER] {
            fs::create_dir_all(PathBuf::from("results/.current").join(group.as_str())).unwrap();
            let published = reg.get_published(&group);
            for p in published {
                let base = paths::tech_workspace_path(&group, &p.0, &p.1);
                // binaries
                let src = base.join(paths::TECH_INSTALL_DIR).join("");
                let dest = Path::new(group.as_str()).join(&p.0);
                let status = Command::new("rsync")
                    .args(&["-a", src.to_str().unwrap(), dest.to_str().unwrap()])
                    .status();
                if status.is_err() || !status.unwrap().success() {
                    std::process::exit(1);
                }
                // results
                let src = base.join("results/");
                let tech = reg::Technique::read(
                    paths::tech_install_path(&group, &p.0, &p.1).join("info.json"),
                )
                .unwrap();
                let dest = PathBuf::from("results/.current/")
                    .join(group.as_str())
                    .join(&tech.short_name);
                let status = Command::new("rsync")
                    .args(&["-a", src.to_str().unwrap(), dest.to_str().unwrap()])
                    .status();
                if status.is_err() || !status.unwrap().success() {
                    std::process::exit(1);
                }
            }
        }
    }

    // config
    if config::fbksd_config().is_err() {
        std::process::exit(1);
    }
}

/// Save technique data from a temporary workspace to the permanent location.
///
/// Data can be copied or moved, and can include the executable or only the results.
pub fn save_technique_tmp_workspace(id: &str, uuid: &str, include_install: bool, mv: bool) {
    let tmp_workspace = paths::tmp_workspace_path();
    let group = reg::Registry::load().technique_type(id).unwrap();
    let tech = reg::Technique::read(
        tmp_workspace
            .join(group.as_str())
            .join(&id)
            .join("info.json"),
    )
    .unwrap();
    let src = tmp_workspace
        .join("results/.current")
        .join(group.as_str())
        .join(&tech.short_name)
        .join("");
    let dest = paths::tech_results_path(&group, &id, &uuid).join("");
    if mv {
        let status = Command::new("mv").args(&[&src, &dest]).status();
        if status.is_err() || !status.unwrap().success() {
            std::process::exit(1);
        }
    } else {
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

    if include_install {
        let src = tmp_workspace.join(group.as_str()).join(&id).join("");
        let dest = paths::tech_install_path(&group, &id, &uuid).join("");
        if mv {
            let status = Command::new("mv").args(&[&src, &dest]).status();
            if status.is_err() || !status.unwrap().success() {
                std::process::exit(1);
            }
        } else {
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
}

/// Saves data from the temporary workspace to the permanent location for all published techniques.
///
/// Args:
///  - include_install: also saves the techniques install ("<group>/*") folder
///  - mv: move instead of copy
pub fn save_tmp_workspace(include_install: bool, mv: bool) {
    let tmp_workspace = paths::tmp_workspace_path();
    if !tmp_workspace.is_dir() {
        return;
    }
    let _cd = utils::CD::new(&tmp_workspace);
    let reg = reg::Registry::load();
    for group in vec![reg::TechniqueType::DENOISER, reg::TechniqueType::SAMPLER] {
        let published = reg.get_published(&group);
        for p in published {
            save_technique_tmp_workspace(&p.0, &p.1, include_install, mv);
        }
    }
}

/// Export result images from a specific technique slot.
///
/// Args:
///  - src: `workspaces/<group>/<id>/<uuid>/results` directory
///  - dest: `<page>/data/<group>/<tech name>` directory
///  - ignore_existing: avoid transferring files that already exist in the destination
pub fn export_technique_images(src: &Path, dest: &Path, ignore_existing: bool) -> bool {
    let src = src.join("");
    let dest = dest.join("");
    let mut args = vec![
        "-a",
        "--include",
        "*.png",
        "--include",
        "*/",
        "--exclude",
        "*",
        "--delete",
    ];
    if ignore_existing {
        args.push("--ignore-existing");
    }
    args.push(src.to_str().unwrap());
    args.push(dest.to_str().unwrap());
    let status = Command::new("rsync").args(&args).status();
    if status.is_err() || !status.unwrap().success() {
        return false;
    }
    return true;
}

/// Export result images from all published techniques to the public page.
///
/// Old published images are overwritten.
pub fn export_images() {
    let reg = reg::Registry::load();
    for group in vec![reg::TechniqueType::DENOISER, reg::TechniqueType::SAMPLER] {
        let published = reg.get_published(&group);
        for p in published {
            let src = paths::tech_results_path(&group, &p.0, &p.1);
            let tech = reg::Technique::read(
                paths::tech_install_path(&group, &p.0, &p.1).join("info.json"),
            )
            .unwrap();
            let dest = paths::public_page_path()
                .join("data")
                .join(group.as_str())
                .join(&tech.short_name);
            export_technique_images(&src, &dest, false);
        }
    }
}

/// Re-scan the scenes folder and updates the cache file.
pub fn update_scenes() {
    let scenes_dir = paths::scenes_path();
    if !scenes_dir.is_dir() {
        eprintln!("scenes folder does not exist");
        std::process::exit(1);
    }

    fn fix_scene_paths(scene: &mut Scene, path: &Path) {
        scene.path = path
            .parent()
            .unwrap()
            .join(&scene.path)
            .to_str()
            .unwrap()
            .to_string();
        scene.ref_img = path
            .parent()
            .unwrap()
            .join(&scene.ref_img)
            .to_str()
            .unwrap()
            .to_string();
    }

    let mut renderers: Vec<Renderer> = Vec::new();
    for entry in fs::read_dir(&scenes_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            let _cd = utils::CD::new(&path);
            let mut scenes: Vec<Scene> = Vec::new();
            for scene_file in glob("**/fbksd-scene.json").expect("Failed to read glob pattern") {
                if let Ok(path) = scene_file {
                    let data =
                        fs::read_to_string(&path).expect("Failed reading the \"info.json\" file");
                    let mut scene: Scene =
                        serde_json::from_str(&data).expect("Failed to deserialize scene entry");
                    fix_scene_paths(&mut scene, &path);
                    scenes.push(scene);
                }
            }
            for scene_file in glob("**/fbksd-scenes.json").expect("Failed to read glob pattern") {
                if let Ok(path) = scene_file {
                    let data =
                        fs::read_to_string(&path).expect("Failed reading the \"info.json\" file");
                    let mut scenes_vec: Vec<Scene> =
                        serde_json::from_str(&data).expect("Failed to deserialize scene entry");
                    for mut scene in &mut scenes_vec {
                        fix_scene_paths(&mut scene, &path);
                    }
                    scenes.extend(scenes_vec);
                }
            }

            let renderer_name = path.file_name().unwrap().to_str().unwrap();
            renderers.push(Renderer {
                renderer: renderer_name.to_string(),
                scenes,
            });
        }
    }

    let data =
        serde_json::to_string_pretty(&renderers).expect("Failed to serialize scenes cache file");
    fs::write(scenes_dir.join(".fbksd-scenes-cache.json"), &data)
        .expect("Failed to save scenes cache file");
}

/// Deletes a technique's unpublished workspace (including results page).
pub fn delete_unpublished_workspace(id: &str, uuid: &str) -> WPResult<()> {
    let reg = reg::Registry::load();
    match reg.get_unpublished_wps(id).find(|i| i.as_str() == uuid) {
        Some(uuid) => {
            let mut reg = reg::Registry::load();
            let group = reg.technique_type(id).unwrap();
            if reg.remove_workspace(id, &uuid).is_ok() {
                let mut ok =
                    fs::remove_dir_all(paths::tech_workspace_path(&group, &id, &uuid)).is_ok();
                ok = ok && fs::remove_dir_all(paths::public_page_path().join(&uuid)).is_ok();
                if ok {
                    reg.save();
                    return Ok(());
                }
            }
        }
        None => {}
    };
    Err(Error::Unspecified)
}

/// deletes all unpublished workspaces that are older than the configured limit number of days.
pub fn trim_unpublished() {
    let config = config::SystemConfig::load();
    let reg = reg::Registry::load();
    let mut reg_new = reg.clone();
    for group in vec![reg::TechniqueType::DENOISER, reg::TechniqueType::SAMPLER] {
        let to_delete = reg.get_unpub_older_than(&group, config.unpublished_days_limit);
        for item in to_delete {
            fs::remove_dir_all(paths::tech_workspace_path(&group, &item.0, &item.1))
                .expect("failed to remove workspace");
            fs::remove_dir_all(paths::public_page_path().join(&item.1))
                .expect("failed to remove private page");
            reg_new.remove_workspace(&item.0, &item.1).unwrap();
            log::info!(
                "old workspace deleted: id = {}, uuid = {}",
                &item.0,
                &item.1
            );
        }
    }
    reg_new.save();
}

/// Unpublishes a technique, setting its workspace as "Finished".
pub fn unpublish_technique(id: i32) -> WPResult<()> {
    let id = id.to_string();
    let mut reg = reg::Registry::load();
    if let Ok((group, uuid)) = reg.unpublish_workspace(&id) {
        // delete "published" link
        if fs::remove_file(paths::tech_published_wp_path(&group, &id)).is_err() {
            return Err(Error::Unspecified);
        }
        // delete technique's results from the public page
        let tech = match reg::Technique::read(
            paths::tech_install_path(&group, &id, uuid).join("info.json"),
        ) {
            Ok(tech) => tech,
            _ => {
                eprintln!("failed to load technique");
                return Err(Error::Unspecified);
            }
        };
        if fs::remove_dir_all(
            paths::public_page_path()
                .join("data")
                .join(group.as_str())
                .join(&tech.short_name),
        )
        .is_err()
        {
            eprintln!("failed to remove public data");
            return Err(Error::Unspecified);
        }
        reg.save();
        // update public page data
        let wp = Workspace::load();
        wp.export_page(paths::public_page_path());
        return Ok(());
    }
    Err(Error::Unspecified)
}

fn www_ownership() -> (&'static String, &'static String) {
    const FBKSD_WWW_USER: &str = "FBKSD_WWW_USER";
    const FBKSD_WWW_GROUP: &str = "FBKSD_WWW_GROUP";
    lazy_static! {
        static ref USER: String =
            env::var(FBKSD_WWW_USER).expect(&format!("Evn var {} not defined", FBKSD_WWW_USER));
        static ref GROUP: String =
            env::var(FBKSD_WWW_GROUP).expect(&format!("Evn var {} not defined", FBKSD_WWW_GROUP));
    }
    (&USER, &GROUP)
}

pub fn set_public_page_permissions() -> WPResult<()> {
    let own = www_ownership();
    let status = Command::new("chown")
        .args(&[
            "-R",
            format!("{}:{}", own.0, own.1).as_ref(),
            paths::public_page_path().to_str().unwrap(),
        ])
        .status();
    if status.is_err() || !status.unwrap().success() {
        return Err(Error::Unspecified);
    }
    Ok(())
}
