use serde::{Deserialize, Serialize};
use super::node::log::Entry;

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub term: u64,
    pub from: Address,
    pub to: Address,
    pub event: Event,
}

impl Message {
    pub fn new(term: u64, from: Address, to: Address, event: Event) -> Self {
        Self {
            term,
            from,
            to,
            event,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Address {
    Broadcast,
    Peer(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    AppendEntries { entries: Vec<Entry> },
    AckEntries { index: usize },
    Heartbeat {},
    RequestVote {},
    Vote { voted_for: String },
    ClientRequest { command: String }
}
