//! Runs processes inside docker containers.

use std::io;
use std::process::Command;
use std::process::ExitStatus;

#[derive(Default)]
pub struct Docker {
    image: String,
    args: Vec<String>,
    env_vars: Vec<String>,
    mounts: Vec<String>,
    cmd_args: Vec<String>,
    network: String,
    wd: String,
}

impl Docker {
    pub fn new(image: &str) -> Docker {
        Docker {
            image: image.to_string(),
            ..Default::default()
        }
    }

    pub fn args(&mut self, args: &[&str]) -> &mut Self {
        for s in args {
            self.args.push(s.to_string());
        }
        self
    }

    pub fn env_vars(&mut self, vars: &[&str]) -> &mut Self {
        for s in vars {
            self.env_vars.push("-e".to_string());
            self.env_vars.push(s.to_string());
        }
        self
    }

    pub fn mounts(&mut self, mounts: &[(&str, &str)]) -> &mut Self {
        for s in mounts {
            self.mounts.push("-v".to_string());
            self.mounts
                .push(format!("{}:{}", s.0.to_string(), s.1.to_string()));
        }
        self
    }

    pub fn mounts_ro(&mut self, mounts: &[(&str, &str)]) -> &mut Self {
        for s in mounts {
            self.mounts.push("-v".to_string());
            self.mounts
                .push(format!("{}:{}:ro", s.0.to_string(), s.1.to_string()));
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

    pub fn working_dir(&mut self, wd: &str) -> &mut Self {
        self.wd = String::from(wd);
        self
    }

    pub fn run(&self, cmd: &str, cmd_args: &[&str]) -> io::Result<ExitStatus> {
        let mut args = vec!["run", "--gpus", "all"];

        for s in self.args.iter() {
            args.push(&s);
        }

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
        for s in cmd_args {
            args.push(&s);
        }

        Command::new("docker").args(args).status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run() {
        run("fbksd", &["--version"]).unwrap();
    }
}
