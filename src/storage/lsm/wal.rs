use super::TableEntry;

use std::{
    io::{BufWriter, Write},
    fs::{File, OpenOptions},
    path::{Path, PathBuf}
};
// The WAL (write-ahead-log) component, is an on-disk copy of the Memtable,
// an append-only file that serves as a backup for node failures while 
// the memtable is not flushed to disk
// 
// WAL entry format:
// +---------------+---------------+-----------------+-----+-------+-----------------+
// | Tombstone(1B) | Key Size (8B) | Value Size (8B) | Key | Value | Timestamp (16B) |
// +---------------+---------------+-----------------+-----+-------+-----------------+
#[derive(Debug)]
pub struct WAL {
    pub path: PathBuf,
    file: BufWriter<File>
}

impl WAL {
    pub fn new(dir: &Path, timestamp: u128) -> Result<Self, std::io::Error> {
        let path = Path::new(dir).join(timestamp.to_string() + ".wal");
        let options = OpenOptions::new().append(true).create(true).open(&path)?;
        let file = BufWriter::new(options);

        Ok(WAL { path, file })
    }

    // this function is used to recover lost Memtables due to unexpected failures
    pub fn from_path(path: &Path) -> Result<Self, std::io::Error> {
        let options = OpenOptions::new().append(true).write(true).open(&path)?;
        let file = BufWriter::new(options);

        return Ok(WAL { path: path.to_owned(), file })
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.file.flush()
    }

    pub fn append(&mut self, entry: TableEntry) -> Result<(), std::io::Error> {
        self.file.write_all(&(entry.deleted as u8).to_le_bytes())?;
        self.file.write_all(&entry.key.len().to_le_bytes())?;
        self.file.write_all(&entry.value.clone().unwrap().len().to_le_bytes())?;
        self.file.write_all(&entry.key)?;
        self.file.write_all(&entry.value.unwrap())?;
        self.file.write_all(&entry.timestamp.to_le_bytes())?;

        self.flush()?;
        Ok(())
    }
}
