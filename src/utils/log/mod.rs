use serde::Serialize;
use crate::raft::{message::Message, node::log::Entry};
use log::info;

#[derive(Serialize)]
pub enum RaftLogType {
    PeerStart { id: String, peers: Vec<String> },
    NewRole { new_role: String },
    Tick,
    ReceivingMessage { message: Message },
    SendingMessage { message: Message },
    LogAppend { entry: Vec<Entry> },
    LogCommit { index: usize },
    Error { message: String },
}

#[derive(Serialize)]
pub struct RaftLog {
    log_type: RaftLogType 
}

pub fn log_raft(log_type: RaftLogType) {
    let log_entry = RaftLog { log_type };
    info!("{}", ron::to_string(&log_entry).unwrap());
}
