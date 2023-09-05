use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub term: u64,
    pub from: Address,
    pub to: Address,
    pub event: Event
}

impl Message {
    pub fn new(term: u64, from: Address, to: Address, event: Event) -> Self {
        Self {
            term,
            from,
            to,
            event
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Address {
    Broadcast,
    Peer(String)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    AppendEntries { index: u64, term: u64 },
    RequestVote { term: u64 },
    Vote { term: u64, voted_for: String }
}
