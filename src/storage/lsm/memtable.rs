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
                } else {
                    self.size += value.len();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_memtable() {
        let memtable = Memtable::new();
        assert_eq!(memtable.size, 0);
        assert_eq!(memtable.entries.len(), 0);
        assert_eq!(memtable.entries.len(), 0);
    }

    #[test]
    fn test_insert() {
        let mut memtable = Memtable::new();
        memtable.insert(b"key1", b"value1", 1);

        assert_eq!(memtable.size, b"key1".len() + b"value1".len() + 1 + 8);
        assert_eq!(memtable.entries.len(), 1);
        assert_eq!(
            memtable.entries[0],
            TableEntry {
                key: b"key1".to_vec(),
                value: Some(b"value1".to_vec()),
                timestamp: 1,
                deleted: false
            });

        memtable.insert(b"key2", b"value2", 2);
        assert_eq!(memtable.entries.len(), 2);
        assert_eq!(
            memtable.size,
            b"key1".len() + b"value1".len() + b"key2".len() + b"value2".len() + 2 + 2 * 8);

    }

    #[test]
    fn test_search() {
        let mut memtable = Memtable::new();

        memtable.insert(b"key1", b"value1", 1);
        let entry = memtable.search(b"key1");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.key, b"key1");
        assert_eq!(entry.value.as_ref().unwrap(), b"value1");

        let missing_entry = memtable.search(b"key2");
        assert!(missing_entry.is_none());
    }

    #[test]
    fn test_delete() {
        let mut memtable = Memtable::new();

        memtable.insert(b"key1", b"value1", 1);
        assert_eq!(memtable.entries.len(), 1);
        memtable.delete(b"key1", 2);
        assert_eq!(memtable.entries.len(), 1);
        assert_eq!(memtable.entries[0].deleted, true);
        assert_eq!(memtable.entries[0].value, None);
    }

    #[test]
    fn test_update_existing_entry() {
        let mut memtable = Memtable::new();

        memtable.insert(b"key1", b"value1", 1);
        let original_size = memtable.size;
        memtable.insert(b"key1", b"value2", 2);

        assert_eq!(memtable.entries.len(), 1);
        assert_eq!(memtable.entries[0].value.as_ref().unwrap(), b"value2");
        assert_eq!(memtable.entries[0].timestamp, 2);
        assert_eq!(memtable.size, original_size - b"value1".len() + b"value2".len());
    }

    #[test]
    fn test_insert_and_delete_size() {
        let mut memtable = Memtable::new();

        memtable.insert(b"key1", b"value1", 1);

        let entry = memtable.search(b"key1");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.key, b"key1");
        assert_eq!(entry.value.as_ref().unwrap(), b"value1");

        memtable.delete(b"key1", 3);
        assert_eq!(memtable.size, b"key1".len() + 1 + 8);

        let deleted_entry = memtable.search(b"key1");
        assert!(deleted_entry.is_some());
        let deleted_entry = deleted_entry.unwrap();
        assert_eq!(deleted_entry.deleted, true);
        assert!(deleted_entry.value.is_none());
        assert_eq!(deleted_entry.timestamp, 3);
    }

}
