//! Messages used to communicate between `fbksd-ci` and `fbksd-server`.

use crate::ci::ProjectInfo;
use crate::error::Error as Core_error;
use crate::info::TechniqueInfo;

use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;

#[derive(Serialize, Deserialize)]
pub enum Msg {
    Register(ProjectInfo, TechniqueInfo),
    SaveResults(ProjectInfo, TechniqueInfo),
    PublishPrivate(ProjectInfo, String),
    InitMissingScenesWP(ProjectInfo, String),
    UpdateResults(ProjectInfo, String),
    PublishPublic(ProjectInfo, String),
    CanRun(ProjectInfo),
    DeleteWorkspace(ProjectInfo, String),
    End,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
    InvalidMessage,
    Logic(String),
    Internal,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            InvalidMessage => "invalid message".fmt(f),
            Logic(msg) => f.write_fmt(format_args!("logic error: {}", msg)),
            Internal => "internal error".fmt(f),
        }
    }
}

impl error::Error for Error {}

impl From<Core_error> for Error {
    fn from(err: Core_error) -> Self {
        match err {
            Core_error::NameAlreadyExists
            | Core_error::NotRegistered
            | Core_error::InvalidInfoFile
            | Core_error::AlreadyPublished
            | Core_error::MaxWorkspacesExceeded => Error::Logic(err.to_string()),
            Core_error::Unspecified => Error::Internal,
        }
    }
}

pub type MsgResult = Result<String, Error>;
