use super::super::{message::Address, message::Event, message::Message};
use super::{follower::Follower, leader::Leader, Node, Role};
use crate::utils::config::CONFIG;
use rand::Rng;

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
        println!("new candidate here, rand_timeout is {:?}", random_timeout);
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
            Event::AppendEntries { index: _, term } => {
                println!("candidate receiving a appendEntries");
                if term >= self.log.last_term {
                    let address = match msg.from {
                        Address::Peer(addr) => addr.to_string(),
                        _ => panic!("Unexpected Address"),
                    };

                    Ok(self
                        .become_role(Follower::new(
                            Some(address),
                            None,
                            CONFIG.raft.leader_seen_timeout,
                        ))
                        .into())
                } else {
                    Ok(self.into())
                }
            }
            Event::RequestVote { term } => {
                println!("candidate receiving a requestVote");
                if term > self.log.last_term {
                    println!("voting on the other peer wich has a bigger term");
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
                    println!("ignoring the requestVote, my term is bigger");
                    Ok(self.into())
                }
            }
            Event::Vote { term, voted_for } => {
                println!(
                    "candidate receiving a vote, term: {}, voted_for: {}",
                    term, voted_for
                );
                self.role.votes += 1;

                if self.role.votes >= self.peers.len() as u64 {
                    let peers = self.peers.clone();
                    Ok(self
                        .become_role(Leader::new(peers, CONFIG.raft.leader_idle_timeout))
                        .into())
                } else {
                    Ok(self.into())
                }
            } // TODO: become leader when receive majority of votes
        }
    }

    pub fn tick(mut self) -> Node {
        println!("candidate tick");
        self.role.election_ticks += 1;

        if self.role.election_ticks >= self.role.election_timeout {
            println!("election timed out, starting new election");
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
            event: Event::AppendEntries { index: 1, term: 2 },
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
