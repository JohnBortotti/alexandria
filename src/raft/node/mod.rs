use super::{
    message::Message,
    node::log::Entry,
    state_machine::StateMachine,
};
use crate::utils::{config::CONFIG, log::{log_raft, RaftLogType}};
use crate::raft::server::NodeResponse;

use tokio::sync::mpsc;
use self::log::Log;

pub mod candidate;
pub mod follower;
pub mod leader;
pub mod log;

pub enum Node {
    Follower(Role<follower::Follower>),
    Candidate(Role<candidate::Candidate>),
    Leader(Role<leader::Leader>),
}

impl Node {
    pub async fn new(
        id: &str,
        peers: Vec<String>,
        log: Log,
        node_tx: mpsc::UnboundedSender<Message>,
        outbound_tx: mpsc::UnboundedSender<NodeResponse>
    ) -> Self {
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();
        let state_machine = StateMachine::new(state_rx, node_tx.clone());
        tokio::spawn(state_machine.run(id.to_string()));

        let node = Role::<follower::Follower> {
            id: id.to_string(),
            peers,
            log,
            role: follower::Follower::new(None, CONFIG.raft.leader_seen_timeout),
            state_tx,
            node_tx,
            outbound_tx
        };

        if node.peers.is_empty() {
            log_raft(
                RaftLogType::NewRole { new_role: "leader".to_string() }
            );

            node.become_role(leader::Leader::new(vec![], CONFIG.raft.leader_idle_timeout))
                .into()
        } else {
            node.into()
        }
    }

    pub fn tick(self) -> Self {
        match self {
            Node::Candidate(n) => n.tick(),
            Node::Follower(n) => n.tick(),
            Node::Leader(n) => n.tick(),
        }
    }

    pub fn step(self, msg: Message) -> Result<Node, &'static str> {
        match self {
            Node::Candidate(n) => n.step(msg),
            Node::Follower(n) => n.step(msg),
            Node::Leader(n) => n.step(msg),
        }
    }
}

pub struct Role<T> {
    pub id: String,
    pub peers: Vec<String>,
    pub log: Log,
    pub role: T,
    pub node_tx: mpsc::UnboundedSender<Message>,
    pub state_tx: mpsc::UnboundedSender<Entry>,
    pub outbound_tx: mpsc::UnboundedSender<NodeResponse>
}

impl<R> Role<R> {
    fn become_role<T>(self, role: T) -> Role<T> {
        Role {
            id: self.id,
            peers: self.peers,
            log: self.log,
            node_tx: self.node_tx,
            state_tx: self.state_tx,
            outbound_tx: self.outbound_tx,
            role,
        }
    }
}

impl From<Role<follower::Follower>> for Node {
    fn from(r: Role<follower::Follower>) -> Self {
        Node::Follower(r)
    }
}

impl From<Role<leader::Leader>> for Node {
    fn from(r: Role<leader::Leader>) -> Self {
        Node::Leader(r)
    }
}

impl From<Role<candidate::Candidate>> for Node {
    fn from(r: Role<candidate::Candidate>) -> Self {
        Node::Candidate(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_node() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let (otx, _) = tokio::sync::mpsc::unbounded_channel();
        let node = Node::new(
            "a",
            vec!["a".to_string(), "b".to_string()],
            Log::new(),
            tx.clone(),
            otx.clone()
        )
        .await;

        match node {
            Node::Follower(node) => {
                assert_eq!(node.id, "a".to_owned());
                assert_eq!(node.peers, vec!("a".to_string(), "b".to_string()));
            }
            _ => panic!("Expected node to start as follower"),
        }
    }

    #[tokio::test]
    async fn new_node_become_leader() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel();
        let (otx, _) = tokio::sync::mpsc::unbounded_channel();
        let node = Node::new("a", vec![], Log::new(), tx.clone(), otx.clone()).await;

        match node {
            Node::Leader(node) => {
                assert_eq!(node.id, "a".to_owned());
                assert_eq!(node.peers.is_empty(), true);
            }
            _ => panic!("Expected node to become leader"),
        }
    }
}
