use super::{Node, Role, log::Entry};
use super::super::{message::Message, message::Event, message::Address::{Peer, Broadcast}, 
state_machine::Instruction};
use std::collections::HashMap;
use log::info;

pub struct Leader {
    peer_last_index: HashMap<String, usize>,
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
            self.broadcast_heartbeat()
        } else {
            self.into()
        }
    }

    fn broadcast_heartbeat(self) -> Node {
        info!(target: "raft_leader", 
              "leader is broadcasting a heartbeat, leader term is: {}", self.log.last_term);

        self.node_tx.send(
            Message::new(
                self.log.last_term,
                Peer(self.id.clone()),
                Broadcast,
                Event::Heartbeat {}
        )).unwrap();

        self.into()
    }

    fn broadcast_append_entries(self) -> Node {
        info!(target: "raft_leader", "leader is broadcasting appendEntries to update peers logs");

        for (peer, peer_last_index) in &self.role.peer_last_index {
            if peer_last_index.clone() == self.log.last_index {
                continue;
            };

            let logs = self.log.entries.get(peer_last_index+0..).unwrap().to_vec();
            self.node_tx.send(
                Message::new(
                    self.log.last_term,
                    Peer(self.id.clone()),
                    Peer(String::from(peer)),
                    Event::AppendEntries { entries: logs }
                )
            ).unwrap();
        };

        self.into()
    }

    pub fn step(mut self, msg: Message) -> Result<Node, &'static str> {
        match msg.event {
            Event::AppendEntries { .. } => {
                info!(target: "raft_leader", "leader receiving an AppendEntries");
            }
            Event::Vote { voted_for: _ } => {
                info!(target: "raft_leader", "leader receiving an Vote");
            }
            Event::RequestVote {} => {
                info!(target: "raft_leader", "leader receiving an RequestVote");
            }
            Event::Heartbeat {} => {
                info!(target: "raft_leader", "leader receiving an Heartbeat");
            }
            Event::AckEntries { index } => {
                info!(target: "raft_leader", "leader receiving an AckEntries");

                let addr = match msg.from {
                    Broadcast => panic!("Expected msg sender to be a peer instead broadcast"),
                    Peer(peer) => peer
                };
                self.role.peer_last_index.entry(addr).and_modify(|e| *e = index);
            }
            Event::ClientRequest { command } => {
                info!(target: "raft_leader", "leader receiving an ClientRequest");
                info!(target: "raft_leader", "ClientRequest [ command: {:?} ]", command);

                self.log.append(self.log.last_term, vec!(command.clone()));
                
                // todo: refactor to remove the unwrap()
                // todo: create a function broadcastAppendEntries to repeat this procedure
                self.node_tx.send(Message::new(
                    self.log.last_term,
                    Peer(self.id.clone()),
                    Broadcast,
                    Event::AppendEntries { 
                        entries: vec!(Entry {
                            index: self.log.last_index, 
                            term: self.log.last_term,
                            command: command.clone()
                        })
                    }
                )).unwrap();

                // todo: should'n every node implement this funcion?
                self.state_tx.send(Instruction {
                    index: self.log.last_index+1,
                    term: self.log.last_term,
                    command
                }).unwrap();
            }
        }

        Ok(self.into())
    }
}

mod test {
    use super::*;
    use crate::raft::message::Message;
    use crate::raft::node::Log;
    use crate::raft::state_machine::Instruction;
    use tokio::sync::mpsc::UnboundedReceiver;

    fn setup() -> (
        Role<Leader>,
        UnboundedReceiver<Message>,
        UnboundedReceiver<Instruction>,
        ) {
        let (node_tx, node_rx) = tokio::sync::mpsc::unbounded_channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();

        let peers = vec!["a".into(), "b".into(), "c".into()];

        let leader = Role {
            id: "l".into(),
            peers: peers.clone(),
            log: Log::new(),
            node_tx,
            state_tx,
            role: Leader::new(peers, 2),
        };

        (leader, node_rx, state_rx)
    }

    #[tokio::test] 
    async fn leader_broadcasting_heartbeats() {
        let (leader, mut node_rx, _) = setup();

        leader.tick().tick();
        let msg = node_rx.recv().await.unwrap();

        match msg {
           Message { term, from, to, event }  => {
               assert_eq!(term, 0);

               match from {
                   Peer(peer_id) => { assert_eq!(peer_id, "l") }
                   _ => panic!("Unexpected address")
               };

               match to {
                   Broadcast => {},
                   _ => panic!("Expected message to be broadcast")
               };

               match event {
                   Event::Heartbeat {} => { assert_eq!(term, 0) }
                   _ => panic!("Expected event to be a Heartbeat")
               };
           }

        }
    }

    #[tokio::test]
    async fn leader_receiving_client_command() {
        let (leader, _node_rx, _state_rx) = setup();

        let msg = Message {
            event: Event::ClientRequest { command: String::from("") },
            term: 2,
            to: Peer("l".to_string()),
            from: Peer("c".into()),
        };

        let node = leader.step(msg);

        match node {
            Ok(Node::Leader(nleader)) => {
                assert_eq!(nleader.log.last_term, 0);
                assert_eq!(nleader.log.last_index, 1);
            }
            _ => panic!("Expected node to be Leader")
        }
    }

    #[tokio::test]
    async fn leader_update_index_table_on_ack_entries() {
        let (leader, _, _) = setup();

        let msg = Message {
            event: Event::AckEntries { index: 2 },
            term: 0,
            to: Peer("l".to_string()),
            from: Peer("b".into())
        };

        let node = leader.step(msg);

        match node {
            Ok(Node::Leader(nleader)) => {
                assert_eq!(nleader.role.peer_last_index.get("a"), Some(0).as_ref());
                assert_eq!(nleader.role.peer_last_index.get("b"), Some(2).as_ref());
                assert_eq!(nleader.role.peer_last_index.get("c"), Some(0).as_ref());
            },
            _ => panic!("Expected node to be Leader")
        }

    }

    #[tokio::test]
    async fn test_leader_broadcast_append_entries_1() {
        let (mut leader, mut node_rx, _) = setup();

        leader.role.peer_last_index.clear();
        leader.role.peer_last_index.insert("a".to_string(), 1);

        leader.log.append(
            3, vec!(String::from("test1"), String::from("test2"), String::from("test3"))
        );
        leader.broadcast_append_entries();
        let msg = node_rx.recv().await.unwrap();

        match msg {
            Message { term, from, to, event } => {
               assert_eq!(term, 3);
               match from {
                   Peer(peer_id) => { assert_eq!(peer_id, "l") }
                   _ => panic!("Unexpected address")
               };
               match to {
                   Peer(peer_id) => { assert_eq!(peer_id, "a") }
                   _ => panic!("Unexpected address")
               }
               match event {
                   Event::AppendEntries { entries } => {
                       assert_eq!(entries.len(), 2);
                       assert_eq!(entries[0].command, "test2");
                       assert_eq!(entries[1].command, "test3");
                       assert_eq!(entries[0].index, 2);
                       assert_eq!(entries[1].index, 3);
                   }
                   _ => panic!("Expected event to be an AppendEntries")
               }
            }
        };
    }

    #[tokio::test]
    async fn test_leader_broadcast_append_entries_2() {
        let (mut leader, mut node_rx, _) = setup();

        leader.role.peer_last_index.clear();
        leader.role.peer_last_index.insert("b".to_string(), 0);

        leader.log.append(
            3, vec!(String::from("test1"), String::from("test2"), String::from("test3"))
        );
        leader.broadcast_append_entries();
        let msg = node_rx.recv().await.unwrap();

        match msg {
            Message { term, from, to, event } => {
               assert_eq!(term, 3);
               match from {
                   Peer(peer_id) => { assert_eq!(peer_id, "l") }
                   _ => panic!("Unexpected address")
               };
               match to {
                   Peer(peer_id) => { assert_eq!(peer_id, "b") }
                   _ => panic!("Unexpected address")
               }
               match event {
                   Event::AppendEntries { entries } => {
                       assert_eq!(entries.len(), 3);
                       assert_eq!(entries[0].command, "test1");
                       assert_eq!(entries[1].command, "test2");
                       assert_eq!(entries[2].command, "test3");
                       assert_eq!(entries[0].index, 1);
                       assert_eq!(entries[1].index, 2);
                       assert_eq!(entries[2].index, 3);
                   }
                   _ => panic!("Expected event to be an AppendEntries")
               }
            }
        };
    }
}
