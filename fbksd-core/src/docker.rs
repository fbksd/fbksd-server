use crate::paths;
use std::io;
use std::process::Command;
use std::process::ExitStatus;

/// Volume mapping from host to container:
/// - <data>/scenes -> /mnt/fbksd-data/scenes
/// - <data>/renderers -> /mnt/fbksd-data/renderers
/// - <data>/tmp/workspace -> /mnt/fbksd-data/tmp/workspace

pub fn run(command: &str, args: &[&str]) -> io::Result<ExitStatus> {
    // docker run --runtime=nvidia --rm nvidia/cuda:9.1-devel gcc --version
    // docker run -v <host_dir>:<cont_dir>:ro -w <working dir> -i -t <image> fbksd run
    let lock_file = String::from(paths::LOCK_FILE) + ":" + paths::LOCK_FILE;
    let workspace_mount = String::from(paths::tmp_workspace_path().to_str().unwrap())
        + ":/mnt/fbksd-data/tmp/workspace";
    let renderers_mount = String::from(paths::renderers_path().to_str().unwrap())
        + ":/mnt/fbksd-data/tmp/workspace/renderers:ro";
    let scenes_mount = String::from(paths::scenes_path().to_str().unwrap())
        + ":/mnt/fbksd-data/tmp/workspace/scenes:ro";
    let mut docker_args = vec![
        "run",
        "--runtime=nvidia",
        "-v",
        &lock_file,
        "-v",
        &scenes_mount,
        "-v",
        &renderers_mount,
        "-v",
        &workspace_mount,
        "-w",
        "/mnt/fbksd-data/tmp/workspace",
        "-i",
        "-t",
        "fbksd",
        command,
    ];
    for s in args {
        docker_args.push(s);
    }
    Command::new("docker").args(docker_args).status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run() {
        run("fbksd", &["--version"]).unwrap();
    }
}
