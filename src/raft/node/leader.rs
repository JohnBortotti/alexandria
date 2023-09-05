use super::{Role, Node};
use std::collections::HashMap;
use super::super::message::Message;

pub struct Leader {
    peer_last_index: HashMap<String, u64>
}


impl Leader {
    pub fn new(peers: Vec<String>) -> Self {
        let mut leader = Self {
            peer_last_index: HashMap::new() 
        };

        for peer in peers {
            leader.peer_last_index.insert(peer.clone(), 0);
        }

        leader
    }
}

impl Role<Leader> {
    // handle node step
    // - accepts logs from clients
    // - replicate logs across servers
    // pub fn step(mut self) -> Node {}

    pub fn tick(self) -> Node  {
        self.into()
    }

    pub fn step(self, msg: Message) -> Result<Node, &'static str> {
        Ok(self.into())
    } 
}
