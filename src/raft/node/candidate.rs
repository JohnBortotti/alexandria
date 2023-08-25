use super::{Role, Node};

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
    pub fn step(self) -> Node {
        self.into()
    }
    
    pub fn tick(mut self) -> Node {
        self.role.election_ticks += 1;

        if self.role.election_ticks >= self.role.election_timeout {
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
          }
          _ => panic!("Expected node to be Candidate")

      }
  }



}
