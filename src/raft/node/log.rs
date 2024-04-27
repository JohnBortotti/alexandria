use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    index: u64,
    term: u64,
    command: String,
}

pub struct Log {
    pub last_index: u64,
    pub last_term: u64,
    pub commit_index: u64,
    pub commit_term: u64,
    pub entries: Vec<Entry>,
}

impl Log {
    pub fn new() -> Self {
        Self {
            last_index: 0,
            last_term: 0,
            commit_index: 0,
            commit_term: 0,
            entries: <Vec<Entry>>::default(),
        }
    }

    pub fn append(&mut self, term: u64, entries: Vec<String>) {
        let _ = entries.iter().for_each(|command| {
            let entry = Entry {
                index: self.last_index + 1,
                term,
                command: command.to_string(),
            };

            self.last_index += 1;
            self.last_term = term;
            self.entries.push(entry);
        });
    }

    // TODO
    //
    // an entry is considered committed if it is safe for that
    // entry to be applied to state machines
    // pub fn commit(mut self, entry: Entry) {
    //
    // }
}
