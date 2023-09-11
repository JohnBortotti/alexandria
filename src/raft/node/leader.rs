use super::{Role, Node};
use std::collections::HashMap;
use super::super::message::Message;
use super::super::message::Address::Broadcast;
use super::super::message::Event::AppendEntries;

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
    // TODO: accept logs from peers
    // TODO: replicate logs across peers
    // TODO: send heartBeats to keep authority
    pub fn tick(self) -> Node  {
        self.into()
    }

    pub fn step(self, msg: Message) -> Result<Node, &'static str> {
        let _ = self.node_tx.send(Message::new(1, Broadcast, Broadcast, AppendEntries{term:1,index:1}));
        Ok(self.into())
    } 
}
