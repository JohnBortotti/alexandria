use super::super::{message::Address, message::Event, message::Message, node::log::Entry};
use super::{candidate::Candidate, Node, Role};
use crate::utils::config::CONFIG;
use log::info;

pub struct Follower {
    pub leader: Option<String>,
    voted: Option<String>,
    leader_seen_ticks: u64,
    leader_seen_timeout: u64,
}

impl Follower {
    pub fn new(leader: Option<String>, voted: Option<String>, leader_seen_timeout: u64) -> Self {
        Self {
            leader,
            voted,
            leader_seen_ticks: 0,
            leader_seen_timeout,
        }
    }
}

impl Role<Follower> {
    pub fn step(mut self, msg: Message) -> Result<Node, &'static str> {
        info!(target: "raft_follower", "the follower is receiving a message from {:?}", &msg.from);
        if self.is_leader(&msg.from) {
            info!(target: "raft_follower", "message from leader, reseting seen ticks");
            self.role.leader_seen_ticks = 0;
        }
        
        match msg.event {
            Event::AppendEntries { entries } => {
                if self.is_leader(&msg.from) {
                    info!(target: "raft_follower", "receiving appendEntries from leader");
                    self.log.append(entries);

                    let leader = match msg.from {
                        Address::Peer(id) => id,
                        _ => panic!("Unexpected msg.from value")
                    };
                    let ack = Message::new(
                        msg.term,
                        Address::Peer(self.id.clone()),
                        Address::Peer(leader),
                        Event::AckEntries { index: self.log.last_index }
                    );

                    info!(target: "raft_follower", "sending ackEntries to leader, last_index: {}", self.log.last_index);
                    self.node_tx.send(ack).unwrap();
                }
            }
            Event::RequestVote {} => {
                info!(target: "raft_follower", "follower is receiving a requestVote");
                if msg.term > self.log.last_term {
                    match msg.from {
                        Address::Peer(sender) => {
                            info!(target: "raft_follower",
                                "the follower is voting for peer {:?}", sender);

                            let res = Message::new(
                                msg.term,
                                Address::Peer(self.id.clone()),
                                Address::Broadcast,
                                Event::Vote {
                                    voted_for: sender.clone(),
                                },
                            );

                            self.node_tx.send(res).unwrap();
                            info!(target: "raft_follower", 
                                  "follower granted a vote, reseting leader_seen_ticks");
                            self.role.leader_seen_ticks = 0;
                            info!(target: "raft_follower", "following the peer i voted for");

                            return Ok(self.follow(Address::Peer(sender)))
                        }
                        _ => panic!("Unexpected sender address"),
                    };
                }
            },
            Event::Vote { voted_for } => {
                info!(target: "raft_follower", 
                      "follower is receiving a vote messge, term: {}, voted_for: {}, from: {:?}", 
                      msg.term, voted_for, &msg.from);
            },
            _ => { info!(target: "raft_candidate", "receiving undefined message event"); }
        };

        Ok(self.into())
    }

    pub fn tick(mut self) -> Node {
        info!(target: "raft_follower", "follower tick");
        self.role.leader_seen_ticks += 1;

        if self.role.leader_seen_ticks >= self.role.leader_seen_timeout {
            info!(target: "raft_follower", "follower starting an election");
            self.log.last_term += 1;
            let candidate = self.become_role(Candidate::new(
                CONFIG.raft.candidate_election_timeout,
                CONFIG.raft.candidate_election_timeout_rand,
                1,
            ));

            if let Err(error) = candidate.node_tx.send(Message::new(
                candidate.log.last_term,
                Address::Peer(candidate.id.clone()),
                Address::Broadcast,
                Event::RequestVote {},
            )) {
                panic!("{}", error);
            }

            info!(target: "raft_follower", "follower sent election request");
            candidate.into()
        } else {
            self.into()
        }
    }

    fn is_leader(&self, from: &Address) -> bool {
        matches!((&self.role.leader, from), (Some(leader), Address::Peer(from)) if leader == from)
    }

    fn follow(self, leader: Address) -> Node {
        let address = match leader {
            Address::Peer(addr) => addr,
            _ => panic!("Expected leader to be an Peer Address"),
        };

        let follower = self.become_role(Follower::new(
            Some(address),
            None,
            CONFIG.raft.leader_seen_timeout,
        ));
        follower.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::message::Message;
    use crate::raft::node::Log;
    use crate::raft::state_machine::Instruction;
    use tokio::sync::mpsc::UnboundedReceiver;

    fn setup() -> (
        Role<Follower>,
        UnboundedReceiver<Message>,
        UnboundedReceiver<Instruction>,
    ) {
        let (node_tx, node_rx) = tokio::sync::mpsc::unbounded_channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();

        let follower = Role {
            id: "d".into(),
            peers: vec!["a".into(), "b".into(), "c".into()],
            log: Log::new(),
            node_tx,
            state_tx,
            role: Follower::new(Some("a".into()), None, 2),
        };

        (follower, node_rx, state_rx)
    }

    #[test]
    fn new_follower() {
        let (follower, _, _) = setup();

        assert_eq!(follower.role.leader_seen_ticks, 0);
        assert_eq!(follower.role.leader, Some("a".into()));

        let node = follower.tick();

        match node {
            Node::Follower(follower) => {
                assert_eq!(follower.role.leader_seen_ticks, 1);
            }
            _ => panic!("Expected node to be follower"),
        }
    }

    #[tokio::test]
    async fn follower_become_candidate() {
        let (follower, _node_rx, _) = setup();

        let node = follower.tick().tick();

        match node {
            Node::Candidate(candidate) => {
                assert_eq!(candidate.role.votes, 1);
                assert_eq!(candidate.log.last_term, 1);
            }
            _ => panic!("Expected node to become candidate after seen ticks timeout"),
        }
    }

    #[test]
    fn follower_step_reset_seen_ticks() {
        let (follower, _node_rx, _) = setup();

        let node = follower.tick();

        match node {
            Node::Follower(follower) => {
                let msg = Message {
                    event: Event::AppendEntries { entries: vec!() },
                    term: 1,
                    to: Address::Peer("b".into()),
                    from: Address::Peer("a".into()),
                };

                let follower = follower.step(msg);
                match follower {
                    Ok(Node::Follower(follower)) => {
                        assert_eq!(follower.role.leader_seen_ticks, 0);
                        assert_eq!(follower.role.leader, Some("a".into()))
                    }
                    _ => panic!("Expected node to be follower"),
                };
            }
            _ => panic!("Expected node to be follower"),
        }
    }

    #[tokio::test]
    async fn follower_must_append_log_on_append_entries() {
        let (mut follower, _node_rx, _) = setup();
        follower.role.leader = Some(String::from("a"));

        let entries = vec!(
            Entry { 
                command: "command1".to_string(),
                index: 1,
                term: 1
            },
            Entry {
                command: "command2".to_string(),
                index: 2,
                term: 1
            }
        );
        let append_entries = Message {
            term: 1,
            from: Address::Peer("a".to_string()),
            to: Address::Broadcast,
            event: Event::AppendEntries { entries }
        };

        let follower = follower.step(append_entries).unwrap();
        match follower {
            Node::Follower(follower) => {
                assert_eq!(follower.log.last_term, 1);
                assert_eq!(follower.log.last_index, 2);
                assert_eq!(follower.log.entries[0].index, 1);
                assert_eq!(follower.log.entries[0].command, "command1");
                assert_eq!(follower.log.entries[1].index, 2);
                assert_eq!(follower.log.entries[1].command, "command2");
            },
            _ => panic!("Expected node to be Follower")
        };
        

    }

}
