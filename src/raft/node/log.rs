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

// todo: write tests
