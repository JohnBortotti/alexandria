use super::{Node, Role};
use super::super::{message::Message, message::Event, message::Address::{Peer, Broadcast}};
use std::collections::HashMap;
use log::info;

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
            self.broadcast_heartbeat()
        } else {
            self.into()
        }
    }

    // once a leader has been elected, it begins servicing
    // client requests. Each client request contains a command to
    // be executed by the replicated state machines.
    //
    // the leader appends the command to its log as a new entry, 
    // then issues AppendEntries RPCs in parallel to each of the other
    // servers to replicate the entry. When the entry has been
    // safely replicated, the leader applies
    // the entry to its state machine and returns the result of that
    // execution to the client.
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
            Event::ClientRequest { command } => {
                info!(target: "raft_leader", "leader receiving an ClientRequest");
                info!(target: "raft_leader", "ClientRequest [ command: {:?} ]", command);

                // set new last_term
                let new_term = self.log.last_term + 1;
                
                // append log entry
                self.log.append(new_term, command.clone());
                
                // replicate log
                self.node_tx.send(Message::new(
                    new_term,
                    Peer(self.id.clone()),
                    Broadcast,
                    Event::AppendEntries { term: new_term, command }
                ))
                .unwrap();

                // commit log
                // broadcast the term
                //
                // execute instruction
                // return response
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
}
