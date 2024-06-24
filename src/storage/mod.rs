mod lsm;

use std::{
    collections::HashMap,
    path::PathBuf, 
    fs::{read_dir, create_dir_all},
    sync::{Arc, RwLock}
};
use chrono::Utc;
use lsm::TableEntry;
use serde::{Deserialize, Serialize};
use crate::utils::config::CONFIG;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Command {
    ListCollections,
    CreateCollection { collection: String },
    GetEntry { collection: String, key: String },
    CreateEntry { collection: String, key: String, value: String },
    DeleteEntry { collection: String, key: String }
}

pub struct Engine {
    root_path: PathBuf,
    collections: Arc<RwLock<HashMap<String, Arc<RwLock<lsm::Lsm>>>>>,
}

impl Engine {
    pub fn new() -> Self {
        let root_path = PathBuf::from(CONFIG.storage.data_path.clone());
        let mut collections = HashMap::new();

        // this code scans root_path looking for folders,
        // each folder is a collection
        if let Ok(entries) = read_dir(root_path.clone()) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(folder_name) = path.clone().file_name() {
                            if let Some(folder_name_str) = folder_name.to_str() {
                                let collection = lsm::Lsm::new(
                                    path,
                                    CONFIG.storage.max_memtable_size)
                                    .unwrap();
                                collections.insert(
                                    folder_name_str.to_string(), 
                                    Arc::new(RwLock::new(collection))
                                    );
                            }
                        }
                    }
                }
            }
        }

        Self { 
            root_path: root_path.clone(),
            collections: Arc::new(RwLock::new(collections)),
        }
    }

    fn new_collection(&self, collection_name: &str) -> Result<(), std::io::Error> {
        let mut path = PathBuf::from(&self.root_path);
        path.push(collection_name);
        create_dir_all(&path)?;

        let collection = lsm::Lsm::new(path, CONFIG.storage.max_memtable_size).unwrap();
        let mut collections = self.collections.write().unwrap();
        collections.insert(collection_name.to_string(), Arc::new(RwLock::new(collection)));

        Ok(())
    }

    // todo: create a proper error struct
    fn parse_string_to_command(query: String) -> Result<Command, std::io::Error> {
        let _query: Vec<&str> = query
            .strip_suffix("\r\n")
            .or(query.strip_suffix("\n"))
            .unwrap_or(&query)
            .split(" ").collect();

        match _query.get(0) {
            Some(&"list") => {
                return Ok(Command::ListCollections);
            },
            Some(&"create") => {
                let collection = match _query.get(1) {
                    Some(value) => value.to_string(),
                    None => return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "Invalid command: please provide the collection name")
                                      )
                };
                return Ok(Command::CreateCollection { collection })
            }
            Some(&"get") => {
                let (collection, key) = match (_query.get(1), _query.get(2)) {
                    (Some(collection), Some(key)) => (collection.to_string(), key.to_string()),
                    _ =>  {
                        return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Invalid command: please provide collection name and key")
                                  )
                    }
                };
                return Ok(Command::GetEntry { collection, key })
            }
            Some(&"write") => {
                let (collection, key, value) = match (_query.get(1), _query.get(2), _query.get(3)) {
                    (Some(collection), Some(key), Some(value)) => (collection.to_string(), key.to_string(), value.to_string()),
                    _ =>  {
                        return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Invalid command: please provide collection name, key and value")
                                  )
                    }

                };
                return Ok(Command::CreateEntry { collection, key, value })
            }
            Some(&"delete") => {
                let (collection, key) = match (_query.get(1), _query.get(2)) {
                    (Some(collection), Some(key)) => (collection.to_string(), key.to_string()),
                    _ =>  {
                        return Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                "Invalid command: please provide collection name and key")
                                  )
                    }
                };
                return Ok(Command::DeleteEntry { collection, key })
            }
            | Some(..) | None => {
                return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Invalid command")
                          )
            }
        }
    }

    fn entry_to_string(entry: TableEntry) -> String {
        format!(
            "{{ key: {}, value: {}, timestamp: {}, deleted: {}}}",
            String::from_utf8(entry.key).unwrap_or("error".to_string()),
            String::from_utf8(entry.value.unwrap_or(vec!())).unwrap_or("error".to_string()),
            entry.timestamp, 
            entry.deleted
            )
    }

    pub async fn run_command(&self, query: String) -> Result<Option<String>, std::io::Error> {
        let command = match Engine::parse_string_to_command(query) {
            Ok(x) => x,
            Err(e) => return Err(e)
        };

        match command {
            Command::ListCollections => {
                let collections = self.collections.read().unwrap();
                return Ok(Some(
                        format!("Collections: {:?}",
                                collections.keys().collect::<Vec<&String>>())
                        ))
            },
            Command::CreateCollection { collection } => {
                let collections = self.collections.read().unwrap();
                if collections.contains_key(&collection) {
                    return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("Collection '{}' already exists", collection)))
                };
                drop(collections);

                self.new_collection(&collection)?;
                return Ok(Some(format!("Collection created: '{}'", collection)))
            },
            Command::GetEntry { collection, key } => {
                let collection = match self.collections.read().unwrap().get(&collection) {
                    Some(collection) => Arc::clone(collection),
                    None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid collection"))
                };

                let result = tokio::task::spawn_blocking(move || {
                    let collection = collection.read().unwrap();
                    collection.search(&Vec::try_from(key).unwrap())
                })
                .await??;

                match result {
                    None => Ok(None),
                    Some(entry) => Ok(Some(Engine::entry_to_string(entry))),
                }
            },
            Command::CreateEntry { collection, key, value } => {
                let collection = match self.collections.read().unwrap().get(&collection) {
                    Some(collection) => Arc::clone(collection),
                    None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid collection"))
                };

                let now = Utc::now();
                let timestamp = now.timestamp();
                let entry = lsm::TableEntry {
                    deleted: false,
                    key: key.clone().into(),
                    value: Some(value.into()),
                    timestamp
                };
                let _entry = entry.clone();

                let _ = tokio::task::spawn_blocking(move || {
                    let mut collection = collection.write().unwrap();
                    collection.write(entry)
                })
                .await??;

                return Ok(Some(Engine::entry_to_string(_entry)))
            },
            Command::DeleteEntry { collection, key } => {
                let collection = match self.collections.read().unwrap().get(&collection) {
                    Some(collection) => Arc::clone(collection),
                    None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid collection"))
                };

                let now = Utc::now();
                let timestamp = now.timestamp();
                let entry = lsm::TableEntry {
                    deleted: true,
                    key: key.clone().into(),
                    value: None,
                    timestamp
                };

                let _ = tokio::task::spawn_blocking(move || {
                    let mut collection = collection.write().unwrap();
                    collection.write(entry)
                })
                .await??;

                Ok(Some(format!("Key '{}' deleted", key)))
            }
        }
    }
}
