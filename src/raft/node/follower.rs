use super::{Role, Node, candidate::Candidate};
use super::super::{message::Event, message::Message, message::Address};

pub struct Follower {
    pub leader: Option<String>,
    voted: Option<String>,
    leader_seen_ticks: u64,
    leader_seen_timeout: u64
}

impl Follower {
    pub fn new(leader: Option<String>, voted: Option<String>, leader_seen_timeout: u64) -> Self { 
        Self { leader, voted, leader_seen_ticks: 0, leader_seen_timeout }
    }
}

impl Role<Follower> {
    pub fn step(mut self, msg: Message) -> Result<Node, &'static str> {
        if self.is_leader(&msg.from) {
            self.role.leader_seen_ticks = 0;
        }

        if self.role.leader == None {
            return Ok(self.follow(msg.from))
        }
        
        match msg.event {
            Event::AppendEntries { index: _, term } => {
                if self.is_leader(&msg.from) {
                    self.log.append(term, None);
                }
            },
            Event::RequestVote { term } => {
                if term > self.log.last_term {
                    let vote_msg = match msg.from {
                        Address::Peer(sender) => {
                            Message::new(
                                term, 
                                Address::Peer(self.id.clone()), 
                                Address::Broadcast, 
                                Event::Vote{term, voted_for: sender}
                                );
                        },
                        _ => panic!("Unexpected sender address"),
                    };

                    // TODO: broadcast vote_msg

                    //     Message::new(
                    //     term, 
                    //     Address::Peer(self.id), 
                    //     Address::Broadcast, 
                    //     Event::Vote{term, voted_for: msg.from}
                    //     );
                    // println!("Vote for");
                }
            },
            _ => panic!("Unexpected event message")
        };

        Ok(self.into())
    }

    pub fn tick(mut self) -> Node {
        self.role.leader_seen_ticks += 1;

        if self.role.leader_seen_ticks >= self.role.leader_seen_timeout {
            self.log.last_term += 1;
            self.become_role(Candidate::new(5, 1)).into()
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

        let follower = self.become_role(Follower::new(Some(address), None, 5));
        follower.into()
    }

}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::raft::node::Log;
  use tokio::sync::mpsc::UnboundedReceiver;
  use crate::raft::state_machine::Instruction;
  use crate::raft::message::Message;

  fn setup() -> (Role<Follower>, UnboundedReceiver<Message>, UnboundedReceiver<Instruction>) {
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
          _ => panic!("Expected node to be follower")
      }
  }

  #[tokio::test]
  async fn follower_become_candidate() {
      let (follower, _, _) = setup();

      let node = follower.tick().tick();

      match node {
          Node::Candidate(candidate) => {
              assert_eq!(candidate.role.votes, 1);
              assert_eq!(candidate.log.last_term, 1);
          },
          _ => panic!("Expected node to become candidate after seen ticks timeout")
      }
  }

  #[test]
  fn follower_step_reset_seen_ticks() {
      let (follower, _, _) = setup();

      let node = follower.tick();

      match node {
          Node::Follower(follower) => {
              let msg = Message {
                  event: Event::AppendEntries{index: 1, term: 1},
                  term: 1,
                  to: Address::Peer("b".into()),
                  from: Address::Peer("a".into())

              };

              let follower = follower.step(msg);
              match follower {
                  Ok(Node::Follower(follower)) => {
                      assert_eq!(follower.role.leader_seen_ticks, 0);
                      assert_eq!(follower.role.leader, Some("a".into()))
                  },
                  _ => panic!("Expected node to be follower")
              };

          },
          _ => panic!("Expected node to be follower")

      }

  }

}