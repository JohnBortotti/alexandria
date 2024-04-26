use super::super::{message::Address, message::Event, message::Message};
use super::{follower::Follower, leader::Leader, Node, Role};
use crate::utils::config::CONFIG;
use rand::Rng;
use log::info;

pub struct Candidate {
    election_ticks: u64,
    election_timeout: u64,
    election_timeout_rand: u64,
    pub votes: u64,
}

impl Candidate {
    pub fn new(election_timeout: u64, election_timeout_rand: u64, votes: u64) -> Self {
        let random_timeout = rand::thread_rng()
            .gen_range(election_timeout..election_timeout + election_timeout_rand);
        info!(target: "raft_candidate", 
              "new candidate created, random_timeout is {:?}", random_timeout);
        Self {
            election_ticks: 0,
            election_timeout: random_timeout,
            election_timeout_rand,
            votes,
        }
    }
}

impl Role<Candidate> {
    pub fn step(mut self, msg: Message) -> Result<Node, &'static str> {
        match msg.event {
            Event::AppendEntries { term, command: _, index: _ } => {
                info!(target: "raft_candidate", 
                      "candidate is receiving an appendEntries, checking message term...");
                if term >= self.log.last_term {
                    let address = match msg.from {
                        Address::Peer(addr) => addr.to_string(),
                        _ => panic!("Unexpected Address"),
                    };
                    info!(target: "raft_candidate", 
                          "candidate is becoming follwer");
                    Ok(self
                        .become_role(Follower::new(
                            Some(address),
                            None,
                            CONFIG.raft.leader_seen_timeout,
                        ))
                        .into())
                } else {
                    info!(target: "raft_candidate", 
                          "candidate ignored the appendEntries");
                    Ok(self.into())
                }
            }
            Event::RequestVote { term } => {
                info!(target: "raft_candidate", 
                      "candidate received a requestVote, checking message term...");
                if term > self.log.last_term {
                    info!(target: "raft_candidate", 
                          "candidate is voting on the other peer wich has a bigger term");
                    let from = match msg.from {
                        Address::Peer(addr) => addr.to_string(),
                        _ => panic!("Unexpected Address"),
                    };

                    self.node_tx
                        .send(Message::new(
                            term,
                            Address::Peer(self.id.clone()),
                            Address::Broadcast,
                            Event::Vote {
                                term,
                                voted_for: from.clone(),
                            },
                        ))
                        .unwrap();

                    Ok(self
                        .become_role(Follower::new(
                            Some(from),
                            None,
                            CONFIG.raft.leader_seen_timeout,
                        ))
                        .into())
                } else {
                    info!(target: "raft_candidate", 
                          "candidate is ignoring the requestVote");
                    Ok(self.into())
                }
            }
            Event::Vote { term, voted_for } => {
                let from = match msg.from {
                    Address::Peer(addr) => addr.to_string(),
                    _ => panic!("Unexpected Address"),
                };

                info!(target: "raft_candidate", 
                      "candidate is receiving a vote message, term: {}, voted_for: {}, from: {}", 
                      term, voted_for, from);

                if voted_for == self.id {
                    info!(target: "raft_candidate", "candidate received a vote");
                    self.role.votes += 1;

                    if self.role.votes >= self.peers.len() as u64 {
                        let peers = self.peers.clone();
                        Ok(self
                           .become_role(Leader::new(peers, CONFIG.raft.leader_idle_timeout))
                           .into())
                    } else {
                        Ok(self.into())
                    }
                } else {
                    info!(target: "raft_candidate", "the voting is for another candidate");
                    Ok(self.into())
                }
            },
            _ => { 
                info!(target: "raft_candidate", "receiving undefined message event");
                Ok(self.into()) 
            }
        }
    }

    pub fn tick(mut self) -> Node {
        info!(target: "raft_candidate", "candidate tick");
        self.role.election_ticks += 1;

        if self.role.election_ticks >= self.role.election_timeout {
            info!(target: "raft_candidate", "election timed out, starting a new election");
            self.log.last_term += 1;
            self.role = Candidate::new(
                self.role.election_timeout,
                self.role.election_timeout_rand,
                1,
            );

            if let Err(error) = self.node_tx.send(Message::new(
                self.log.last_term,
                Address::Peer(self.id.clone()),
                Address::Broadcast,
                Event::RequestVote {
                    term: self.log.last_term,
                },
            )) {
                panic!("{}", error);
            };

            self.into()
        } else {
            self.into()
        }
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
        Role<Candidate>,
        UnboundedReceiver<Message>,
        UnboundedReceiver<Instruction>,
    ) {
        let (node_tx, node_rx) = tokio::sync::mpsc::unbounded_channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();

        let candidate = Role {
            id: "d".into(),
            peers: vec!["a".into(), "b".into(), "c".into()],
            log: Log::new(),
            node_tx,
            state_tx,
            role: Candidate::new(2, 1, 1),
        };

        (candidate, node_rx, state_rx)
    }

    #[test]
    fn new_candidate() {
        let (candidate, _, _) = setup();

        assert_eq!(candidate.role.votes, 1);
        assert_eq!(candidate.role.election_ticks, 0);

        let node = candidate.tick();

        match node {
            Node::Candidate(candidate) => {
                assert_eq!(candidate.role.election_ticks, 1);
            }
            _ => panic!("Expected node to be candidate"),
        }
    }

    #[test]
    fn candidate_election_timeout() {
        let (candidate, _node_rx, _) = setup();

        let node = candidate.tick().tick();

        match node {
            Node::Candidate(candidate) => {
                assert_eq!(candidate.role.election_ticks, 0);
                assert_eq!(candidate.role.votes, 1);
                assert_eq!(candidate.log.last_term, 1);
            }
            _ => panic!("Expected node to be Candidate"),
        }
    }

    #[test]
    fn candidate_become_follower_by_heartbeat() {
        let (candidate, _, _) = setup();

        let msg = Message {
            event: Event::AppendEntries { term: 2, index: 1, command: String::from("") },
            term: 2,
            to: Address::Broadcast,
            from: Address::Peer("c".into()),
        };

        let node = candidate.step(msg);

        match node {
            Ok(Node::Follower(follower)) => {
                assert_eq!(follower.role.leader, Some("c".into()))
            }
            _ => panic!("Expected node to be Follower"),
        }
    }
}
