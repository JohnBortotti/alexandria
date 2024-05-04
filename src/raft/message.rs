use serde::{Deserialize, Serialize};
use super::node::log::Entry;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

// todo: add Client address option
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Address {
    Broadcast,
    Peer(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    AppendEntries { entries: Option<Vec<Entry>>, commit_index: usize },
    AckEntries { index: usize },
    RequestVote {},
    Vote { voted_for: String },
    ClientRequest { command: String }
}
