use super::{Role, Node};
use std::collections::HashMap;

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
        println!("alo");

        self.into()
    }
}
