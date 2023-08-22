use tokio::sync::mpsc;
use super::message::Message;
use super::state_machine::{StateDriver, Instruction};

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
    role: T,
    node_tx: mpsc::UnboundedSender<Message>,
    state_tx: mpsc::UnboundedSender<Instruction>,
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
        node_tx: mpsc::UnboundedSender<Message>
    ) -> Self {
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();

        let driver = StateDriver::new(state_rx, node_tx.clone());
        tokio::spawn(driver.run());

        let node = Role::<Follower> {
            id: id.to_string(),
            peers,
            log,
            role: Follower::new(),
            state_tx,
            node_tx
        };

        return node.into();
    }
}

impl From<Role<Follower>> for Node {
    fn from(rn: Role<Follower>) -> Self {
        Node::Follower(rn)
    }
}
