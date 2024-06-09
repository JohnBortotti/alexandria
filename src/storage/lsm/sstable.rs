use super::{TableEntry, memtable::Memtable};

use std::{
    io::{BufWriter, Write, Read, BufReader, Seek, SeekFrom},
    fs::{File, OpenOptions},
    path::Path,
};

// SSTables are the final data storage, 
// when memtables are flushed, an SSTable is created to 
// store the flushed data on disk.
//
// They are immutable, if an entry is updated or deleted, the action
// is registered on a newer SSTable. SSTables are Periodically 
// merged to minimize disk usage and increase search speed
#[derive(Debug)]
pub struct SSTable {
    data_file: BufWriter<File>,
    index_file: BufWriter<File>,
    metadata_file: BufWriter<File>,
    pub timestamp: u128
}

// SSTable entry formats
// data_file
// +---------------+---------------+-----------------+-----+-------+-----------------+
// | Tombstone(1B) | Key Size (8B) | Value Size (8B) | Key | Value | Timestamp (16B) |
// +---------------+---------------+-----------------+-----+-------+-----------------+
//
// index_file
// +---------------+-----+-------------+
// | Key Size (8B) | Key | Offset (8B) |
// +---------------+-----+-------------+
impl SSTable {
    // todo:
    // - last entry of each table is lost
    // - add metadata file
    pub fn new(path: &Path, timestamp: u128) -> Result<Self, std::io::Error> {
        let _path = Path::new(&path).join(timestamp.to_string() + ".sst_data");
        let data_file = BufWriter::new(
            OpenOptions::new().append(true).create(true).open(_path)?
        );

        let _path = Path::new(&path).join(timestamp.to_string() + ".sst_index");
        let index_file = BufWriter::new(
            OpenOptions::new().append(true).create(true).open(_path)?
        );

        let _path = Path::new(&path).join(timestamp.to_string() + ".sst_meta");
        let metadata_file = BufWriter::new(
            OpenOptions::new().append(true).create(true).open(_path)?
        );

        Ok(Self { data_file, index_file, metadata_file, timestamp })
    }

    pub fn flush(&mut self, memtable: &Memtable) -> Result<(), std::io::Error> {
        let mut offset: u64 = 0;

        for entry in &memtable.entries {
            let key_len = entry.key.clone().len() as u64;
            let value_len = entry.value.as_ref().map_or(0, |v| v.len()) as u64;
            let entry_size = 1 + 8 + 8 + key_len + value_len + 16;

            self.data_file.write_all(&(entry.deleted as u8).to_le_bytes())?;
            self.data_file.write_all(&key_len.to_le_bytes())?;
            self.data_file.write_all(&value_len.to_le_bytes())?;
            self.data_file.write_all(&entry.key)?;
            if let Some(value) = &entry.value {
                self.data_file.write_all(value)?;
            }
            self.data_file.write_all(&entry.timestamp.to_le_bytes())?;

            self.index_file.write_all(&key_len.to_le_bytes())?;
            self.index_file.write_all(&entry.key)?;
            self.index_file.write_all(&offset.to_le_bytes())?;
            
            offset += entry_size;
        }
        self.data_file.flush()?;
        self.index_file.flush()?;

        self.metadata_file.flush()?;

        Ok(())
    }

    // todo:
    // [ ] improve the search algorithm (btree?)
    pub fn search(&self, path: &Path, key: &[u8]) -> Result<Option<u64>, std::io::Error> {
        let mut index_file = BufReader::new(
            File::open(path.join(self.timestamp.to_string() + ".sst_index"))?);

        let mut key_len_buf = [0; 8];
        let mut result_buffer = [0; 8];

        while index_file.read_exact(&mut key_len_buf).is_ok() {
            let key_len = usize::from_le_bytes(key_len_buf);
            let mut on_disk_key = vec![0u8; key_len as usize];
            index_file.read_exact(&mut on_disk_key)?;

            if on_disk_key == key {
                index_file.read_exact(&mut result_buffer)?;
                let offset = u64::from_le_bytes(result_buffer);
                return Ok(Some(offset))
            } else {
                index_file.seek(SeekFrom::Current(8))?;
            }
        }

        Ok(None)
    }

   pub fn get_entry(&self, path: &Path, offset: u64) 
       -> Result<Option<TableEntry>, std::io::Error> {
       let mut data_file = BufReader::new(
           File::open(path.join(self.timestamp.to_string() + ".sst_data"))?);

       data_file.seek(SeekFrom::Start(offset))?;
       let mut tombstone = [0u8; 1];
       data_file.read_exact(&mut tombstone)?;

       if tombstone[0] == 1 {
           return Ok(None);
       }

       let mut buffer = [0u8; 8];

       data_file.read_exact(&mut buffer)?;
       let _key_len = u64::from_le_bytes(buffer);

       data_file.read_exact(&mut buffer)?;
       let value_len = u64::from_le_bytes(buffer);

       let mut key_buffer = vec![0u8; _key_len as usize];
       data_file.read_exact(&mut key_buffer)?;

       let mut value_buffer = vec![0u8; value_len as usize];
       data_file.read_exact(&mut value_buffer)?;

       let mut timestamp_buffer = [0; 16];
       data_file.read_exact(&mut timestamp_buffer)?;
       let timestamp = u128::from_le_bytes(timestamp_buffer);

       let entry = TableEntry {
           key: key_buffer,
           value: Some(value_buffer),
           timestamp,
           deleted: u8::from_be_bytes(tombstone) > 0
       };

       Ok(Some(entry))
   }
}
