use crate::paths;
use std::io;
use std::process::Command;
use std::process::ExitStatus;

#[derive(Default)]
pub struct Docker {
    image: String,
    env_vars: Vec<String>,
    mounts: Vec<String>,
    cmd_args: Vec<String>,
    network: String,
}

impl Docker {
    pub fn new(image: &str) -> Docker {
        Docker {
            image: image.to_string(),
            ..Default::default()
        }
    }

    pub fn env_vars(&mut self, vars: &[&str]) -> &mut Self {
        for s in vars {
            self.env_vars.push("-e".to_string());
            self.env_vars.push(s.to_string());
        }
        self
    }

    pub fn mounts(&mut self, mounts: &[&str]) -> &mut Self {
        for s in mounts {
            self.mounts.push("-v".to_string());
            self.mounts.push(s.to_string());
        }
        self
    }

    pub fn cmd_args(&mut self, args: &[&str]) -> &mut Self {
        for s in args {
            self.cmd_args.push(s.to_string());
        }
        self
    }

    pub fn network(&mut self, net: &str) -> &mut Self {
        self.network = String::from(net);
        self
    }

    pub fn run(&self, cmd: &str) -> io::Result<ExitStatus> {
        let mut args = vec!["run", "--gpus", "all"];

        for s in self.mounts.iter() {
            args.push(&s);
        }

        for s in self.env_vars.iter() {
            args.push(&s);
        }

        args.push("--network");
        args.push(&self.network);

        args.push(&self.image);
        args.push(cmd);

        for s in self.cmd_args.iter() {
            args.push(&s);
        }

        Command::new("docker").args(args).status()
    }
}

/// Volume mapping from host to container:
/// - <data>/scenes -> /mnt/fbksd-data/scenes
/// - <data>/renderers -> /mnt/fbksd-data/renderers
/// - <data>/tmp/workspace -> /mnt/fbksd-data/tmp/workspace
pub fn run(
    image: &str,
    env_vars: &[&str],
    mounts: &[&str],
    command: &str,
    command_args: &[&str],
) -> io::Result<ExitStatus> {
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
        "--gpus",
        "all",
        // "-v",
        // &lock_file,
        // "-v",
        // &scenes_mount,
        // "-v",
        // &renderers_mount,
        // "-v",
        // &workspace_mount,
        "-w",
        "/mnt/fbksd-data/tmp/workspace",
        "-i",
        "-t",
    ];

    for s in mounts {
        docker_args.push("-v");
        docker_args.push(s);
    }

    for s in env_vars {
        docker_args.push("-e");
        docker_args.push(s);
    }

    docker_args.push(image);
    docker_args.push(command);

    // command args
    for s in command_args {
        docker_args.push(s);
    }

    Command::new("docker").args(docker_args).status()


    docker::Docker::new("fbksd-ci")
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
        .run("fbksd-ci")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run() {
        run("fbksd", &["--version"]).unwrap();
    }
}
