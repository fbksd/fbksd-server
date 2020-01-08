use std::fmt;
use std::process::Command;

pub enum BuildType {
    Debug,
    Release,
}
impl Default for BuildType {
    fn default() -> Self {
        BuildType::Debug
    }
}
impl fmt::Display for BuildType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use BuildType::*;
        match *self {
            Debug => "Debug".fmt(f),
            Release => "Release".fmt(f),
        }
    }
}

/// Runs `cmake` with the given parameters on the current directory.
pub fn config(
    build_type: BuildType,
    install_prefix: &str,
    source_path: &str,
) -> std::result::Result<std::process::ExitStatus, std::io::Error> {
    Command::new("cmake")
        .args(&[
            format!("-DCMAKE_BUILD_TYPE={}", build_type),
            format!("-DCMAKE_INSTALL_PREFIX={}", install_prefix),
            String::from(source_path),
        ])
        .status()
}

/// Runs `cmake --build . --target install` on the current directory.
pub fn install() -> std::result::Result<std::process::ExitStatus, std::io::Error> {
    Command::new("cmake")
        .args(&["--build", ".", "--target", "install"])
        .status()
}
