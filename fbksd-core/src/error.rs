//! Error enum for the core library.

use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Technique with the same name already exists.
    NameAlreadyExists,
    /// No technique with the give id was found.
    NotRegistered,
    InvalidInfoFile,
    AlreadyPublished,
    MaxWorkspacesExceeded,
    Unspecified,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match *self {
            NameAlreadyExists => "technique with the same name already exists".fmt(f),
            NotRegistered => "no technique with the given id was found".fmt(f),
            InvalidInfoFile => "invalid info.json file".fmt(f),
            AlreadyPublished => "technique is already published".fmt(f),
            MaxWorkspacesExceeded => "maximum number of workspaces exceeded".fmt(f),
            Unspecified => "unspecified error".fmt(f),
        }
    }
}
impl error::Error for Error {}
