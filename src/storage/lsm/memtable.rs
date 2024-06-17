use super::TableEntry;

#[derive(Debug)]
pub struct Memtable {
    pub size: usize,
    pub entries: Vec<TableEntry>
}

impl Memtable {
    pub fn new() -> Self {
        Self { 
            size: 0,
            entries: Vec::new() 
        }
    }

    fn get_index(&self, key: &[u8]) -> Result<usize, usize> {
        self.entries
            .binary_search_by_key(&key, |e| e.key.as_slice())
    }

    pub fn search(&self, key: &[u8]) -> Option<&TableEntry> {
        if let Ok(i) = self.get_index(key) {
            if &self.entries[i].deleted == &true {
                return Some(&self.entries[i]);
            };
            return Some(&self.entries[i])
        }
        None
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8], timestamp: i64) {
        let entry = TableEntry {
            key: key.to_owned(),
            value: Some(value.to_owned()),
            timestamp,
            deleted: false
        };

        match self.get_index(key) {
            Ok(i) => {
                if let Some(old) = self.entries[i].value.as_ref() {
                    if value.len() < old.len() {
                        self.size -= old.len() - value.len()
                    } else {
                        self.size += value.len() - old.len()
                    }
                    self.entries[i] = entry;
                }
            },
            Err(i) => {
                self.entries.insert(i, entry);
                self.size += key.len() + value.len() + 1 + 8
            }
        }
    }

    pub fn delete(&mut self, key: &[u8], timestamp: i64) {
        let entry = TableEntry {
            key: key.to_owned(),
            value: None,
            timestamp,
            deleted: true
        };

        match self.get_index(key) {
            Ok(i) => {
                if let Some(val) = self.entries[i].value.as_ref() {
                    self.size -= val.len()
                };
                self.entries[i] = entry;
            },
            Err(i) => {
                self.entries.insert(i, entry);
                self.size += key.len() + 1 + 8
            }
        }
    }
}
