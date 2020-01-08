use crate::paths;
use fs2::FileExt;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct CD {
    prev: PathBuf,
}

impl CD {
    pub fn new<T: AsRef<Path>>(path: T) -> Self {
        let prev = env::current_dir().expect("Error getting current directory");
        env::set_current_dir(path.as_ref()).expect("Failed to change current directory");
        CD { prev }
    }
}

impl Drop for CD {
    fn drop(&mut self) {
        env::set_current_dir(&self.prev).expect("Failed to change current directory");
    }
}

/// Executes a block in a new working directory.
/// Panics if the working directory can not be set.
#[macro_export]
macro_rules! cd {
    ( $p:expr, $x:block ) => {
        let _cd = $crate::utils::CD::new($p);
        $x
    };
}

pub struct FLock {
    file: File,
}

impl FLock {
    /// Acquires an exclusive lock, blocking until it succeeds.
    ///
    /// The lock is released when the returned object is dropped.
    /// Panics an error occurs.
    pub fn new() -> FLock {
        let file = File::open(paths::LOCK_FILE).expect("Failed to open lock file");
        let flock = FLock { file };
        flock.file.lock_exclusive().unwrap();
        flock
    }

    /// Try to acquire exclusive lock, returning None if not possible.
    ///
    /// The lock is released when the returned object is dropped.
    /// Panics an error occurs.
    pub fn try_new() -> Option<FLock> {
        let file = File::open(paths::LOCK_FILE).expect("Failed to open lock file");
        let flock = FLock { file };
        if flock.file.try_lock_exclusive().is_ok() {
            return Some(flock);
        }
        None
    }
}

impl Drop for FLock {
    fn drop(&mut self) {
        self.file.unlock().unwrap();
    }
}

/// Acquires a flock and executes the block before releasing it.
#[macro_export]
macro_rules! flock {
    ( $($x:tt)* ) => {{
        let _flock = $crate::utils::FLock::new();
        { $($x)* }
    }};
}

/// Tries to acquires a flock, if succeeds, executes the block before releasing it.
///
/// If a second block is given, it is executed if the flock is not acquired.
#[macro_export]
macro_rules! try_flock {
    ( $x:block ) => {
        let _flock = $crate::utils::FLock::try_new();
        if _flock.is_some() {
            $x
        }
    };
    ( $x:block, $y:block ) => {
        let _flock = $crate::utils::FLock::try_new();
        if _flock.is_some() {
            $x
        } else {
            $y
        }
    };
    ( $x:expr, $y:expr ) => {
        let _flock = $crate::utils::FLock::try_new();
        if _flock.is_some() {
            $x
        } else {
            $y
        }
    };
}

/// Returns the relative form of `path` as seen from `from`.
pub fn relative_from(path: &Path, from: &Path) -> Option<PathBuf> {
    // This routine is adapted from the *old* Path's `path_relative_from`
    // function, which works differently from the new `relative_from` function.
    // In particular, this handles the case on unix where both paths are
    // absolute but with only the root as the common directory.
    use std::path::Component;

    if path.is_absolute() != from.is_absolute() {
        if path.is_absolute() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        let mut ita = path.components();
        let mut itb = from.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
                (Some(_), Some(b)) if b == Component::ParentDir => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_from() {
        cd!("/home/jonas/fbksd-data/public/data", {
            println!(
                "{:?}",
                relative_from(
                    Path::new("/home/jonas/fbksd-data/public/data/denoisers/Box"),
                    Path::new("/home/jonas/.")
                )
            );
        });
    }
}
