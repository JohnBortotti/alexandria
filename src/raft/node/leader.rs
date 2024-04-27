use super::{Node, Role};
use super::super::{message::Message, message::Event, message::Address::{Peer, Broadcast}, 
state_machine::Instruction};
use std::collections::HashMap;
use log::info;

pub struct Leader {
    peer_last_index: HashMap<String, u64>,
    idle_ticks: u64,
    idle_timeout: u64,
}

// todo: leader must keep track of peers and wich is the last commit index from each peer, 
// this way it can resend lost entries and keep log consistency
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

    pub fn step(mut self, msg: Message) -> Result<Node, &'static str> {
        match msg.event {
            Event::AppendEntries { .. } => {
                info!(target: "raft_leader", "leader receiving an AppendEntries");
            }
            Event::Vote { term: _, voted_for: _ } => {
                info!(target: "raft_leader", "leader receiving an Vote");
            }
            Event::RequestVote { term: _ } => {
                info!(target: "raft_leader", "leader receiving an RequestVote");
            }
            Event::Heartbeat { term: _ } => {
                info!(target: "raft_leader", "leader receiving an Heartbeat");
            }
            Event::AckEntries { .. } => {
                info!(target: "raft_leader", "leader receiving an AckEntries");
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
                        term: self.log.last_term, 
                        index: self.log.last_index, 
                        entries: vec!(command.clone())
                    }
                )).unwrap();

                self.state_tx.send(Instruction {
                    index: self.log.last_index+1,
                    term: self.log.last_index,
                    command
                }).unwrap();
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
                Event::Heartbeat {
                    term: self.log.last_term
                }
        )).unwrap();

        self.into()
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
                   Event::Heartbeat { term, .. } => { assert_eq!(term, 0) }
                   _ => panic!("Expected event to be an AppendEntries")
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
}
