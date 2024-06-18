mod memtable;
mod wal;
mod sstable;


use std::{
    fs::{read_dir, remove_file},
    path::Path,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH}
};
/*
 *   LSM Pipeline  
 *
 * - When data is commited, the database writes in parallel at:
 *   1. Memtable (in memory cache structure)
 *   2. Commit log (WAL/write-ahead-log)
 * - Periodically the memtable is flushed to persistent storage:  
 *   1. An SSTable is created
 *   2. The Commit Log is empty
 *
 * - When searching for data:
 *  1. Search on Memtable (wich is fast), if the key is not present, go to step 2
 *  2. Use the Bloom filter to determine if the key might be present on SSTable
 *  3. Search on SSTable
*/
#[derive(Debug, Clone)]
pub struct TableEntry {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    pub timestamp: i64,
    pub deleted: bool
}

#[derive(Debug)]
pub struct Lsm {
    path: PathBuf,
    pub memtable: memtable::Memtable,
    memtable_size: usize,
    wal: wal::WAL,
    tables: Vec<sstable::SSTable>,
}

impl Lsm {
    pub fn new(path: PathBuf,memtable_size: usize) 
        -> Result<Self, std::io::Error> {
            let mut memtable = memtable::Memtable::new();
            let wal = match Lsm::search_for_wal_file(&path) {
                Ok(wal) => { 
                    let wal_iterator = wal::WALIterator::new(&wal.path)?.into_iter();
                    for entry in wal_iterator {
                        if entry.deleted == true {
                            memtable.delete(&entry.key, entry.timestamp);
                        } else {
                            memtable.insert(&entry.key, &entry.value.unwrap(), entry.timestamp);
                        }
                    }
                    wal 
                },
                Err(..) => {
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_micros();
                    wal::WAL::new(&path, timestamp)? 
                }
            };

            let tables = Lsm::search_for_table_files(&path)?;

            Ok(Self { path, memtable, memtable_size, wal, tables })
        }

    // todo: 
    // fix bug, idk why but sometimes there is a strange .wal file outside collection folder
    pub fn write(&mut self, data: TableEntry) -> Result<(), std::io::Error> {
        if self.memtable.size < self.memtable_size {
            if data.deleted == false {
                if let Some(val) = &data.value {
                    self.memtable.insert(&data.key, val, data.timestamp)
                } else {
                    return Err
                        (std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid data"))
                }
            } else {
                self.memtable.delete(&data.key, data.timestamp)
            }
            self.wal.append(data)?;
        } else {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros();

            let mut new_table = sstable::SSTable::new(&self.path, timestamp)?;
            new_table.flush(&self.memtable)?;
            self.tables.push(new_table);

            self.memtable = memtable::Memtable::new();

            remove_file(&self.wal.path)?;
            self.wal = wal::WAL::new(&self.path, timestamp)?;

            if data.deleted == false {
                if let Some(val) = &data.value {
                    self.memtable.insert(&data.key, val, data.timestamp)
                } else {
                    return Err
                        (std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid data"))
                }
            } else {
                self.memtable.delete(&data.key, data.timestamp)
            }
            self.wal.append(data)?;
        }

        Ok(())
    }

    // todo:
    // [ ] implement Bloom filter
    // [ ] create integration test to test storage engine
    pub fn search(&self, key: &[u8]) 
        -> Result<Option<TableEntry>, std::io::Error> {
            if let Some(entry) = self.memtable.search(key) {
                return Ok(Some(entry.clone()));
            };

            for table in self.tables.iter().rev() {
                if let Some(res) = table.search(&self.path, key)? {
                    if let Some(entry) = table.get_entry(&self.path, res)? {
                        return Ok(Some(entry))
                    }
                }
            }
            Ok(None)
        }

    fn search_for_wal_file(path: &Path) -> Result<wal::WAL, std::io::Error> {
        if path.is_dir() {
            for entry in read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "wal" {
                            let wal = wal::WAL::from_path(&path.to_owned())?;
                            return Ok(wal)
                        }
                    }
                }
            }
        } 

        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "WAL recovery file not found"))
    }

    fn get_timestamp_from_filename(file: &Path) -> Option<u128> {
        file.file_stem()?.to_str()?.parse().ok()
    }

    fn search_for_table_files(path: &Path) -> Result<Vec<sstable::SSTable>, std::io::Error> {
        let mut sstables: Vec<sstable::SSTable> = Vec::new();
        if path.is_dir() {
            for entry in read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "sst_index" {
                            let parent_dir = path.parent().unwrap();
                            let index_file = path.clone();
                            let metadata_file = 
                                parent_dir.join(path.file_stem().unwrap())
                                .with_extension("sst_meta");
                            let data_file = 
                                parent_dir.join(path.file_stem().unwrap())
                                .with_extension("sst_data");

                            if metadata_file.exists() && data_file.exists() {
                                if let Some(timestamp) = 
                                    Lsm::get_timestamp_from_filename(&index_file) {
                                        sstables.push(sstable::SSTable::new(
                                                &path.parent().unwrap(),
                                                timestamp)?
                                        );
                                    }
                            }
                        }
                    }
                }
            }


            sstables.sort_by_key(|table| table.timestamp);
        }

        Ok(sstables)
    }
}
