use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;
use std::io;

use crate::ci::ProjectInfo;
use crate::registry::Technique;

#[derive(Serialize, Deserialize)]
pub enum Msg {
    Register(ProjectInfo, Technique),
    SaveResults(ProjectInfo, Technique),
    PublishPrivate(ProjectInfo, String),
    InitMissingScenesWP(ProjectInfo, String),
    UpdateResults(ProjectInfo, String),
    PublishPublic(ProjectInfo, String),
    CanRun(ProjectInfo),
    DeleteWorkspace(ProjectInfo, String),
    End,
    Invalid,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
    InvalidMessage,
    AlreadyPublished,
    MaxWorkspacesExceeded,
    Unspecified,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match *self {
            InvalidMessage => "invalid message".fmt(f),
            AlreadyPublished => "technique is already published".fmt(f),
            MaxWorkspacesExceeded => "maximum number of workspaces exceeded".fmt(f),
            Unspecified => "unspecified error".fmt(f),
        }
    }
}

impl error::Error for Error {}

pub type MsgResult = Result<String, Error>;

macro_rules! to_unspecified {
    ( $x:ty ) => {
        impl From<$x> for Error {
            fn from(_: $x) -> Self {
                Error::Unspecified
            }
        }
    };
}
to_unspecified!(io::Error);
