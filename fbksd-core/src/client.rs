use crate::ci::ProjectInfo;
use crate::msgs::{Msg, MsgResult};
use crate::registry::Technique;

use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json;
use std::env;
use std::net::TcpStream;

fn server_addr() -> &'static String {
    const VAR: &str = "FBKSD_SERVER_ADDR";
    lazy_static! {
        static ref VALUE: String = env::var(VAR).expect(&format!("Evn var {} not defined", VAR));
    }
    &VALUE
}

pub struct Client {
    stream: TcpStream,
}

impl Client {
    /// Creates a new connection with the server.
    /// The connection is closed when the value is dropped.
    pub fn new() -> Client {
        // let stream = TcpStream::connect("127.0.0.1:8096").expect("Failed to connect to server");
        let stream = TcpStream::connect(&server_addr()).expect("Failed to connect to server");
        Client { stream }
    }

    /// Register a technique.
    ///
    /// Trying to register a new id with a name already used by other technique causes error.
    /// This method can also be used to change the current name of a technique.
    /// Multiple technique versions are not allowed (the info.json file can have only the default version).
    pub fn register(&self, proj: ProjectInfo, tech: Technique) {
        serde_json::to_writer(&self.stream, &Msg::Register(proj, tech))
            .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        if let Err(err) = res {
            eprintln!("{}", &err);
            std::process::exit(1);
        }
    }

    /// Check if the technique is allowed to run.
    pub fn can_run(&self, info: ProjectInfo) {
        serde_json::to_writer(&self.stream, &Msg::CanRun(info))
            .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        if let Err(err) = res {
            eprintln!("{}", &err);
            std::process::exit(1);
        }
    }

    /// Save results from the temporary workspace and returns the key (uuid).
    ///
    /// They key is used to publish the results.
    pub fn save_results(&self, info: ProjectInfo, tech: Technique) -> String {
        serde_json::to_writer(&self.stream, &Msg::SaveResults(info, tech))
            .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        match res {
            Ok(uuid) => uuid,
            Err(err) => {
                eprintln!("{}", &err);
                std::process::exit(1);
            }
        }
    }

    /// Publish results in a hidden location given the workspace uuid.
    pub fn publish_results_private(&self, proj: ProjectInfo, uuid: &str) {
        serde_json::to_writer(&self.stream, &Msg::PublishPrivate(proj, String::from(uuid)))
            .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        if let Err(err) = res {
            eprintln!("{}", &err);
            std::process::exit(1);
        }
    }

    /// Creates a temporary workspace with the missing scenes that need to be run.
    /// Returns Some() if there are such scenes.
    pub fn init_missing_scenes_workspace(&self, proj: ProjectInfo, uuid: &str) -> Option<()> {
        serde_json::to_writer(
            &self.stream,
            &Msg::InitMissingScenesWP(proj, String::from(uuid)),
        )
        .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        match res {
            Ok(msg) => {
                if msg == "NO_SCENE" {
                    return None;
                }
                Some(())
            }
            Err(err) => {
                eprintln!("{}", &err);
                std::process::exit(1);
            }
        }
    }

    /// Updates results from the temporary workspace.
    pub fn update_results(&self, proj: ProjectInfo, uuid: &str) {
        serde_json::to_writer(&self.stream, &Msg::UpdateResults(proj, String::from(uuid)))
            .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        if let Err(err) = res {
            eprintln!("{}", &err);
            std::process::exit(1);
        }
    }

    /// Publish results in the public page.
    pub fn publish_results_public(&self, proj: ProjectInfo, uuid: &str) {
        serde_json::to_writer(&self.stream, &Msg::PublishPublic(proj, String::from(uuid)))
            .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        if let Err(err) = res {
            eprintln!("{}", &err);
            std::process::exit(1);
        }
    }

    pub fn delete_workspace(&self, proj: ProjectInfo, uuid: &str) {
        serde_json::to_writer(
            &self.stream,
            &Msg::DeleteWorkspace(proj, String::from(uuid)),
        )
        .expect("Failed to send message to server");
        let mut de = serde_json::Deserializer::from_reader(&self.stream);
        let res = MsgResult::deserialize(&mut de).expect("Failed to receive response from server");
        if let Err(err) = res {
            eprintln!("{}", &err);
            std::process::exit(1);
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        serde_json::to_writer(&self.stream, &Msg::End).expect("Failed to send message to server");
    }
}
