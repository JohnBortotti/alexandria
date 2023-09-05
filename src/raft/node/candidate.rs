use super::{Role, Node, follower::Follower};
use super::super::{message::Event, message::Message, message::Address::Peer};

pub struct Candidate {
    election_ticks: u64,
    election_timeout: u64,
    pub votes: u64,
}

impl Candidate {
    pub fn new(election_timeout: u64, votes: u64) -> Self {
        Self {election_ticks: 0, election_timeout, votes}
    }
}

impl Role<Candidate> {
    pub fn step(self, msg: Message) -> Result<Node, &'static str> {
        match msg.event {
            Event::AppendEntries{index: _, term} => {
                if term >= self.log.last_term {
                    let address = match msg.from {
                        Peer(addr) => addr.to_string(),
                        _ => panic!("Unexpected Address")
                    };

                    return Ok(self.become_role(Follower::new(Some(address), None, 5)).into())
                } else {
                    return Ok(self.into())
                }

            },
            _ => panic!("Message event not defined"),
        };
    }
    
    pub fn tick(mut self) -> Node {
        self.role.election_ticks += 1;

        if self.role.election_ticks >= self.role.election_timeout {
            self.log.last_term += 1;
            self.role = Candidate::new(self.role.election_timeout, 1);
            self.into()
        } else {
            self.into()
        }

    }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::sync::mpsc::UnboundedReceiver;
  use crate::raft::state_machine::Instruction;
  use crate::raft::message::Message;
  use crate::raft::node::Log;

  fn setup() -> (Role<Candidate>, UnboundedReceiver<Message>, UnboundedReceiver<Instruction>) {
      let (node_tx, node_rx) = tokio::sync::mpsc::unbounded_channel();
      let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel();

      let candidate = Role {
          id: "d".into(),
          peers: vec!["a".into(), "b".into(), "c".into()],
          log: Log::new(),
          node_tx,
          state_tx,
          role: Candidate::new(2, 1),
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
          _ => panic!("Expected node to be candidate")
      }
  }

  #[test]
  fn candidate_election_timeout() {
      let (candidate, _, _) = setup();

      let node = candidate.tick().tick();

      match node {
          Node::Candidate(candidate) => {
              assert_eq!(candidate.role.election_ticks, 0);
              assert_eq!(candidate.role.votes, 1);
              assert_eq!(candidate.log.last_term, 1);
          }
          _ => panic!("Expected node to be Candidate")

      }
  }

  #[test]
  fn candidate_become_follower_by_heartbeat() {
      let (candidate, _, _) = setup();

      let msg = Message {
          event: Event::AppendEntries{index: 1, term: 2},
          term: 2,
          to: Peer("b".into()),
          from: Peer("c".into())
      };

      let node = candidate.step(msg);

      match node {
          Ok(Node::Follower(follower)) => {
              assert_eq!(follower.role.leader, Some("c".into()))
          },
          _ => panic!("Expected node to be Follower"),
      }
  }

  // #[test]
  // fn candidate_become_leader() {
  //     let (candidate, _,  _) = setup();
  // }

}
