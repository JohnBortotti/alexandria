use super::super::{
    message::Address, message::Event, message::Message, 
};
use super::{candidate::Candidate, Node, Role, log::Entry};
use crate::utils::{config::CONFIG, log::{log_raft, RaftLogType}};
use crate::raft::server::{NodeResponse, NodeResponseType};

pub struct Follower {
    pub leader: Option<String>,
    leader_seen_ticks: u64,
    leader_seen_timeout: u64,
}

impl Follower {
    pub fn new(leader: Option<String>, leader_seen_timeout: u64) -> Self {
        log_raft(
            RaftLogType::NewRole { new_role: "follower".to_string() }
        );
        Self {
            leader,
            leader_seen_ticks: 0,
            leader_seen_timeout,
        }
    }
}

impl Role<Follower> {
    pub fn step(mut self, msg: Message) -> Result<Node, &'static str> {
        log_raft(
            RaftLogType::ReceivingMessage { message: msg.clone() }
        );

        if self.is_leader(&msg.from) {
            self.role.leader_seen_ticks = 0;
        }
        
        match msg.event {
            Event::AppendEntries { entries, commit_index } => {
                if self.is_leader(&msg.from) {
                    match entries {
                        None => {},
                        Some(entries) => { 
                            log_raft(
                                RaftLogType::LogAppend { entry: entries.clone() }
                            );

                            let last_index = self.log.entries.last().map_or(0, |entry| entry.index);
                            let valid_entries: Vec<Entry> = entries.iter()
                                .filter_map(|new_entry| {
                                    if new_entry.index > last_index {
                                        Some(Entry {
                                            request_id: None,
                                            command: new_entry.command.clone(),
                                            index: new_entry.index,
                                            term: new_entry.term
                                        })
                                    } else {
                                        log_raft(RaftLogType::Error { 
                                            message: format!("Duplicate or out of order entry detected: {:?}", new_entry)
                                        });
                                        None
                                    }
                                }).collect();

                            self.log.append(valid_entries)
                        }
                    };

                    // todo: implement safe commiting, 
                    // verify if the index are stored in Log to avoid
                    // commiting entries that are not present in Log.entries
                    if commit_index > self.log.commit_index {
                        log_raft(
                            RaftLogType::LogCommit { index: commit_index }
                        );

                        for i in (self.log.commit_index)..=commit_index {
                            if let Some(entry) = self.log.entries.get(i) {
                                self.state_tx.send(entry.clone()).unwrap();
                                self.log.commit(entry.index);
                            }
                        }
                    };

                    let leader = match msg.from {
                        Address::Peer(id) => id,
                        addr => {
                            log_raft(RaftLogType::Error { 
                                message: format!("Unexpected leader address: {:?}", addr)
                            });

                            return Ok(self.into())
                        }
                    };
                    // todo: fix this, send ack only if entries != None
                    let ack = Message::new(
                        msg.term,
                        Address::Peer(self.id.clone()),
                        Address::Peer(leader),
                        Event::AckEntries { index: self.log.last_index }
                    );

                    log_raft(
                        RaftLogType::SendingMessage { message: ack.clone() }
                    );

                    self.node_tx.send(ack).unwrap();
                }
            }
            Event::RequestVote {} => {
                if msg.term > self.log.last_term {
                    match msg.from {
                        Address::Peer(sender) => {
                            let res = Message::new(
                                msg.term,
                                Address::Peer(self.id.clone()),
                                Address::Broadcast,
                                Event::Vote {
                                    voted_for: sender.clone(),
                                },
                            );

                            log_raft(
                                RaftLogType::SendingMessage { message: res.clone() }
                            );

                            self.node_tx.send(res).unwrap();
                            self.role.leader_seen_ticks = 0;

                            return Ok(self.follow(Address::Peer(sender)))
                        }
                        addr => {
                            log_raft(RaftLogType::Error { 
                                message: format!("Receiving message from unexpected address: {:?}", addr)
                            });

                            return Ok(self.into())

                        } 
                    };
                }
            },
            Event::Vote { voted_for: _ } => {},
            Event::StateResponse { request_id, result } => {
                if let Some(request_id) = request_id {
                    let result = match result {
                        Ok(r) => r,
                        Err(_) => "error on state_machine".to_string()
                    };

                    let response = NodeResponse {
                        request_id,
                        response_type: NodeResponseType::Result { result }
                    };

                    self.outbound_tx.send(response).unwrap();

                    return Ok(self.into())
                };
            },
            Event::ClientRequest { request_id, command } => {
                let _command: Vec<&str> = command
                    .strip_suffix("\r\n")
                    .or(command.strip_suffix("\n"))
                    .unwrap_or(&command)
                    .split(" ").collect();

                if _command[0] == "list" || _command[0] == "get" {
                    let entry = Entry { 
                        request_id: Some(request_id),
                        index: self.log.last_index,
                        term: self.log.last_term, 
                        command 
                    };

                    self.state_tx.send(entry.clone()).unwrap();
                } else {
                    match self.role.leader {
                        None => {
                            let response = NodeResponse {
                                request_id,
                                response_type: NodeResponseType::NoLeader
                            };
                            self.outbound_tx.send(response).unwrap();
                        },
                        Some(ref leader) => {
                            let _addr: Vec<&str> = leader.split(":").collect();
                            let _addr = format!("{}:{}", _addr[0], CONFIG.server.outbound_port);
                            let response = NodeResponse {
                                request_id,
                                response_type: NodeResponseType::Redirect { address: _addr }
                            };
                            self.outbound_tx.send(response).unwrap();
                        }
                    }
                }
            }
            msg => { 
                log_raft(
                    RaftLogType::Error 
                        { message: format!("Receiving undefined message event: {:?}", msg) }
                );
            }
        };

        Ok(self.into())
    }

    pub fn tick(mut self) -> Node {
        log_raft(
            RaftLogType::Tick
        );

        self.role.leader_seen_ticks += 1;
        if self.role.leader_seen_ticks >= self.role.leader_seen_timeout {
            log_raft(
                RaftLogType::NewRole { new_role: "candidate".to_string() }
            );

            self.log.last_term += 1;
            let candidate = self.become_role(Candidate::new(
                CONFIG.raft.candidate_election_timeout,
                CONFIG.raft.candidate_election_timeout_rand,
                1,
            ));

            let election_msg = Message::new(
                candidate.log.last_term,
                Address::Peer(candidate.id.clone()),
                Address::Broadcast,
                Event::RequestVote {},
            );

            log_raft(
                RaftLogType::SendingMessage {message: election_msg.clone()}
            );

            candidate.node_tx.send(election_msg).unwrap();
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
            addrs => {
                log_raft(RaftLogType::Error { 
                    message: format!("Trying to follow an invalid peer address: {:?}", addrs)
                });

                return self.into()
            } 
        };

        log_raft(
            RaftLogType::NewRole { new_role: "follower".to_string() }
        );

        let follower = self.become_role(Follower::new(
            Some(address),
            CONFIG.raft.leader_seen_timeout,
        ));
        follower.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::message::Message;
    use crate::raft::node::{Log, log::Entry};
    use tokio::sync::mpsc::UnboundedReceiver;

    fn setup() -> (
        Role<Follower>,
        UnboundedReceiver<Message>,
        UnboundedReceiver<Entry>,
    ) {
        let (node_tx, node_rx) = tokio::sync::mpsc::unbounded_channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();
        let (outbound_tx, _) = tokio::sync::mpsc::unbounded_channel();

        let follower = Role {
            id: "d".into(),
            peers: vec!["a".into(), "b".into(), "c".into()],
            log: Log::new(),
            node_tx,
            state_tx,
            outbound_tx,
            role: Follower::new(Some("a".into()), 2),
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
                    event: Event::AppendEntries { entries: None, commit_index: 0 },
                    term: 1,
                    to: Address::Peer("b".into()),
                    from: Address::Peer("a".into()),
                };

                let follower = follower.step(msg);
                match follower {
                    Ok(Node::Follower(follower)) => {
                        assert_eq!(follower.role.leader_seen_ticks, 0);
                        assert_eq!(follower.log.entries.len(), 0);
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
                request_id: None,
                command: "command1".to_string(),
                index: 1,
                term: 1
            },
            Entry {
                request_id: None,
                command: "command2".to_string(),
                index: 2,
                term: 1
            }
        );
        let append_entries = Message {
            term: 1,
            from: Address::Peer("a".to_string()),
            to: Address::Broadcast,
            event: Event::AppendEntries { entries: Some(entries), commit_index: 0 }
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

    #[tokio::test]
    async fn follower_must_append_logs_then_update_commit_index() {
        let (mut follower, _node_rx, _state_rx) = setup();
        follower.role.leader = Some(String::from("a"));

        let entries = vec!(
            Entry { 
                request_id: None,
                command: "command1".to_string(),
                index: 1,
                term: 1
            },
            Entry {
                request_id: None,
                command: "command2".to_string(),
                index: 2,
                term: 1
            }
        );
        let append_entries = Message {
            term: 1,
            from: Address::Peer("a".to_string()),
            to: Address::Broadcast,
            event: Event::AppendEntries { entries: Some(entries), commit_index: 1 }
        };
        let follower = follower.step(append_entries).unwrap();

        let commit_update = Message {
            term: 1,
            from: Address::Peer("a".to_string()),
            to: Address::Broadcast,
            event: Event::AppendEntries { entries: None, commit_index: 1 }
        };
        let follower = follower.step(commit_update).unwrap();
        match follower {
            Node::Follower(follower) => {
                assert_eq!(follower.log.last_term, 1);
                assert_eq!(follower.log.entries[0].command, "command1");
                assert_eq!(follower.log.entries[1].command, "command2");
                assert_eq!(follower.log.commit_index, 2);
            },
            _ => panic!("Expected node to be Follower")
        };
    }

}
