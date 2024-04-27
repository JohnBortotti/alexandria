use serde::{Deserialize, Serialize};

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

// todo: remove the term field (since every message already contains the current term)
#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    AppendEntries { index: u64, entries: Vec<String> },
    AckEntries { index: u64 },
    Heartbeat {},
    RequestVote {},
    Vote { voted_for: String },
    ClientRequest { command: String }
}
