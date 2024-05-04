use serde::Serialize;
use super::{message::Message, node::Node};

#[derive(Serialize)]
pub enum RaftLogType {
    Tick,
    RoleChange { new_role: String },
    ReceivingMessage { message: Message },
    SendingMessage { message: Message },
    Error { content: String },
}

#[derive(Serialize)]
pub struct RaftLog {
    node_id: String,
    role: String,
    log_type: RaftLogType, 
}

// this function must be use only for raft nodes
pub fn log_raft(node_id: String, role: &str, log_type: RaftLogType) {
    let _log = RaftLog { node_id, role: role.to_string(), log_type };
//     info!("{}", ron::to_string(&log).unwrap());
}