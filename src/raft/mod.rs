pub mod message;
mod client;
mod server;

use tokio::sync::mpsc;
use message::{Message, NodeAddr};

pub struct Log {
    last_index: u64,
    last_term: u64,
    commit_index: u64,
    commit_term: u64,
    entries: Vec<(u64, u64)>
}

impl Log {
    pub fn new() -> Self {
        Self {
            last_index: 0, 
            last_term: 0, 
            commit_index: 0, 
            commit_term: 0, 
            entries: vec!()
        }

    }
}

pub enum Node {
    Follower(Role<Follower>),
    Candidate(Role<Candidate>),
    Leader(Role<Leader>),

}

pub struct Role<T> {
    id: String,
    peers: Vec<String>,
    log: Log,
    role: T
}

pub struct Follower {
    test: u64
}

impl Follower {
    pub fn new() -> Self { 
        Self { test: 12 }
    }
}

pub struct Candidate {}
impl Role<Candidate> {}

pub struct Leader {}
impl Role<Leader> {}

impl Node {
    pub async fn new(
        id: &str,
        peers: Vec<String>,
        log: Log,
        node_tx: mpsc::UnboundedSender<message::Message>
    ) -> Self {

        let node = Role::<Follower> {
            id: id.to_string(),
            peers,
            log,
            role: Follower::new()
        };

        let msg = message::Message::new(23, NodeAddr::Peer("0".to_string()), NodeAddr::Broadcast);
        let _ = node_tx.send(msg);

        return node.into();
    }
}

impl From<Role<Follower>> for Node {
    fn from(rn: Role<Follower>) -> Self {
        Node::Follower(rn)
    }
}
