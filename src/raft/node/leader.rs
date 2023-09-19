use super::super::message::Address::{Broadcast, Peer};
use super::super::message::Event::AppendEntries;
use super::super::message::Message;
use super::{Node, Role};
use std::collections::HashMap;

pub struct Leader {
    peer_last_index: HashMap<String, u64>,
    idle_ticks: u64,
    idle_timeout: u64,
}

impl Leader {
    pub fn new(peers: Vec<String>, idle_timeout: u64) -> Self {
        println!("A new Leader ARISES");
        println!("leader_idle_timeout: {}", idle_timeout);
        let mut leader = Self {
            peer_last_index: HashMap::new(),
            idle_ticks: 0,
            idle_timeout,
        };
        for peer in peers {
            leader.peer_last_index.insert(peer.clone(), 0);
        }
        leader
    }
}

impl Role<Leader> {
    pub fn tick(mut self) -> Node {
        println!("leader tick");
        self.role.idle_ticks += 1;

        if self.role.idle_ticks >= self.role.idle_timeout {
            self.role.idle_ticks = 0;
            self.log.last_term += 1;
            self.broadcast_heartbeat()
        } else {
            self.into()
        }
    }

    pub fn step(self, _msg: Message) -> Result<Node, &'static str> {
        // let _ = self.node_tx.send(Message::new(1, Broadcast, Broadcast, AppendEntries{term:1,index:1}));
        Ok(self.into())
    }

    fn broadcast_heartbeat(self) -> Node {
        println!(
            "broadcast_heartbeat, leader term is: {}",
            self.log.last_term
        );
        for peer in self.peers.iter() {
            self.node_tx
                .send(Message::new(
                    self.log.last_term,
                    Peer(self.id.clone()),
                    Broadcast,
                    AppendEntries {
                        index: 0,
                        term: self.log.last_term,
                    },
                ))
                .unwrap();
            println!("peer: {}", peer);
        }

        self.into()
    }
}
