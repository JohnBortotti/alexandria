use super::super::{
    message::Address, message::Event, message::Message,
};
use super::{follower::Follower, leader::Leader, Node, Role };
use crate::utils::{config::CONFIG, log::{log_raft, RaftLogType}};
use rand::Rng;

pub struct Candidate {
    election_ticks: u64,
    election_timeout: u64,
    election_timeout_rand: u64,
    pub votes: u64,
}

impl Candidate {
    pub fn new(election_timeout: u64, election_timeout_rand: u64, votes: u64) -> Self {
        log_raft(
            RaftLogType::NewRole { new_role: "cadidate".to_string() }
        );

        let random_timeout = rand::thread_rng()
            .gen_range(election_timeout..election_timeout + election_timeout_rand);

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
        log_raft(
            RaftLogType::ReceivingMessage { message: msg.clone() }
        );

        match msg.event {
            Event::AppendEntries { entries: _, commit_index: _ } => {
                if msg.term >= self.log.last_term {
                    let address = match msg.from {
                        Address::Peer(addr) => addr.to_string(),
                        addr => {
                            log_raft(RaftLogType::Error { 
                                message: format!("Receiving message from unexpected address: {:?}", addr)
                            });

                            return Ok(self.into())
                        } 
                    };

                    log_raft(
                        RaftLogType::NewRole{ new_role: "follower".to_string() }
                    );

                    Ok(self
                        .become_role(Follower::new(
                            Some(address),
                            CONFIG.raft.leader_seen_timeout,
                        ))
                        .into())
                } else {
                    Ok(self.into())
                }
            }
            Event::RequestVote {} => {
                if msg.term > self.log.last_term {
                    let from = match msg.from {
                        Address::Peer(addr) => addr.to_string(),
                        addr => {
                            log_raft(RaftLogType::Error { 
                                message: format!("Receiving message from unexpected address: {:?}", addr)
                            });

                            return Ok(self.into())

                        }
                    };

                    let vote_msg = Message::new(
                            msg.term,
                            Address::Peer(self.id.clone()),
                            Address::Broadcast,
                            Event::Vote {
                                voted_for: from.clone(),
                            },
                    );

                    log_raft(
                        RaftLogType::SendingMessage { message: vote_msg.clone() }
                    );

                    self.node_tx
                        .send(vote_msg)
                        .unwrap();

                    log_raft(
                        RaftLogType::NewRole { new_role: "follower".to_string() }
                    );

                    Ok(self
                        .become_role(Follower::new(
                            Some(from),
                            CONFIG.raft.leader_seen_timeout,
                        ))
                        .into())
                } else {
                    Ok(self.into())
                }
            }
            Event::Vote { voted_for } => {
                if voted_for == self.id {
                    self.role.votes += 1;
                    if self.role.votes >= (self.peers.len()/2) as u64 {
                        let peers = self.peers.clone();
                        Ok(self
                           .become_role(Leader::new(peers, CONFIG.raft.leader_idle_timeout))
                           .into())
                    } else {
                        Ok(self.into())
                    }
                } else {
                    Ok(self.into())
                }
            },
            msg => { 
                log_raft(RaftLogType::Error { 
                    message: format!("receiving undefined message event: {:?}", msg) 
                });

                Ok(self.into()) 
            }
        }
    }

    pub fn tick(mut self) -> Node {
        log_raft(
            RaftLogType::Tick 
        );

        self.role.election_ticks += 1;
        if self.role.election_ticks >= self.role.election_timeout {
            log_raft(
                RaftLogType::NewRole { new_role: "candidate".to_string() }
            );

            self.log.last_term += 1;
            self.role = Candidate::new(
                self.role.election_timeout,
                self.role.election_timeout_rand,
                1,
            );

            let election_msg = Message::new(
                self.log.last_term,
                Address::Peer(self.id.clone()),
                Address::Broadcast,
                Event::RequestVote {},
            );

            log_raft(
                RaftLogType::SendingMessage { message: election_msg.clone() }
            );

           self.node_tx.send(election_msg).unwrap();

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
    use crate::raft::node::{Log, log::Entry};
    use tokio::sync::mpsc::UnboundedReceiver;

    fn setup() -> (
        Role<Candidate>,
        UnboundedReceiver<Message>,
        UnboundedReceiver<Entry>,
    ) {
        let (node_tx, node_rx) = tokio::sync::mpsc::unbounded_channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();
        let (outbound_tx, _) = tokio::sync::mpsc::unbounded_channel();

        let candidate = Role {
            id: "d".into(),
            peers: vec!["a".into(), "b".into(), "c".into()],
            log: Log::new(),
            node_tx,
            state_tx,
            outbound_tx,
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
            event: Event::AppendEntries { 
                entries: Some(vec!(Entry { request_id: None, index: 1, term: 2, command: "".to_string()})),
                commit_index: 0
            },
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
