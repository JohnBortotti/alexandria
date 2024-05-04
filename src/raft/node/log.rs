use serde::{Deserialize, Serialize};
use log::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entry {
    pub index: usize,
    pub term: u64,
    pub command: String,
}

pub struct Log {
    pub last_index: usize,
    pub last_term: u64,
    pub commit_index: usize,
    // pub commit_term: u64,
    pub entries: Vec<Entry>,
}

impl Log {
    pub fn new() -> Self {
        Self {
            last_index: 0,
            last_term: 0,
            commit_index: 0,
            // commit_term: 0,
            entries: <Vec<Entry>>::default(),
        }
    }

    pub fn append(&mut self, entries: Vec<Entry>) {
        entries.iter().for_each(|entry| {
            self.last_index = entry.index;
            self.last_term = entry.term;
            self.entries.push(entry.clone());
        });
    }

    // todo: implement commit function
    pub fn commit(&mut self, index: usize) {
        self.commit_index = index;
        info!("commiting index: {}", index);
    }
}

mod test {
    use super::*;

    #[test]
    fn new_log() {
        let log = Log::new();
        assert_eq!(log.last_index, 0);
        assert_eq!(log.last_term, 0);
        assert_eq!(log.entries.len(), 0);
    }

    #[test]
    fn log_append() {
        let mut log = Log::new();
        log.append(vec!(Entry {
            index: 1,
            term: 0,
            command: "a".to_string()
        }));
        assert_eq!(log.last_index, 1);
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].command, "a");
    }
}
