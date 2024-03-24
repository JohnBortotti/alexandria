use super::{Node, Role};
use super::super::{message::Message, message::Event, message::Query, message::Address::{Peer, Broadcast}};
use std::collections::HashMap;
use log::info;
use ron::ser::to_string_pretty;

pub struct Leader {
    peer_last_index: HashMap<String, u64>,
    idle_ticks: u64,
    idle_timeout: u64,
}

impl Leader {
    pub fn new(peers: Vec<String>, idle_timeout: u64) -> Self {
        info!(target: "raft_leader", "a wild new leader appers");
        info!(target: "raft_leader", "leader idle timeout: {}", idle_timeout);
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
        info!(target: "raft_leader", "leader tick");
        self.role.idle_ticks += 1;

        if self.role.idle_ticks >= self.role.idle_timeout {
            self.role.idle_ticks = 0;
            self.log.last_term += 1;
            self.broadcast_heartbeat()
        } else {
            self.into()
        }
    }

    pub fn step(self, msg: Message) -> Result<Node, &'static str> {
        // let _ = self.node_tx.send(Message::new(1, Broadcast, Broadcast, AppendEntries{term:1,index:1}));
        // TODO: implement leader message handling
        match msg.event {
            Event::AppendEntries { index: _, term: _ } => {
                info!(target: "raft_leader", "leader receiving an AppendEntries");
            }
            Event::Vote { term: _, voted_for: _ } => {
                info!(target: "raft_leader", "leader receiving an Vote");
            }
            Event::RequestVote { term: _ } => {
                info!(target: "raft_leader", "leader receiving an RequestVote");
            }
            // when receiving ClientRequest... 
            Event::ClientRequest { test: a, query: b } => {
                info!(target: "raft_leader", "leader receiving an ClientRequest");
                info!(target: "raft_leader", "ClientRequest [ test: {:?}, query: {:?}  ]", a, b);
            }
        }

        Ok(self.into())
    }

    fn broadcast_heartbeat(self) -> Node {
        info!(target: "raft_leader", 
              "leader is broadcasting a heartbeat, leader term is: {}", self.log.last_term);

        self.node_tx.send(
            Message::new(
                self.log.last_term,
                Peer(self.id.clone()),
                Broadcast,
                Event::AppendEntries {
                    index: 0,
                    term: self.log.last_term
                }
                )).unwrap();

        self.into()
    }
}
