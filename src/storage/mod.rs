mod lsm;

use std::{collections::HashMap, path::PathBuf, fs::read_dir, fs::create_dir_all};
use chrono::{Utc, DateTime};
use serde::{Deserialize, Serialize};

/*
 *
 * 1. initialization:
 *      the database will start on a base path, in this path will be saved the 
 *      available databases (each database is a folder), inside each database folder
 *      we have the tables/collections (each table/collection represented by a folder),
 *      inside the database folder, we have files storing the data, index, etc...
 *
 * 2. creating databases and collections:
 *      pretty straightforward, for databases create folders on base path, and for 
 *      tables/collections create a folder inside the proper database
 *
 * 3. communication:
 *      ???
 *
 * 4. data lock model:
 *      ???
*/
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Command {
    ListCollections,
    CreateCollection { collection: String },
    GetEntries { collection: String },
    GetEntry { collection: String, key: String },
    CreateEntry { collection: String, key: String, value: String },
    Delete { collection: String, key: String }
}

// todo:
// - choose the communication way
// - choose how to handle data locks
pub struct Engine {
    root_path: PathBuf,
    collections: HashMap<String, lsm::Lsm>,
}

impl Engine {
    pub fn new() -> Self {
        // todo:
        // - config path and max memtable size
        let root_path = PathBuf::from("./db-data");
        let mut collections = HashMap::new();

        // scan root path looking for folders (each folder is a collection)
        if let Ok(entries) = read_dir(root_path.clone()) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(folder_name) = path.clone().file_name() {
                            if let Some(folder_name_str) = folder_name.to_str() {
                                let collection = lsm::Lsm::new(path, 64).unwrap();
                                collections.insert(folder_name_str.to_string(), collection);
                            }
                        }
                    }
                }
            }
        }

        Self { 
            root_path: root_path.clone(),
            collections,
        }
    }

    fn new_collection(&mut self, collection_name: &str) -> Result<(), std::io::Error> {
        let mut path = PathBuf::from(&self.root_path);
        path.push(collection_name);
        create_dir_all(&path)?;

        // todo: set memtable_max from config file
        let collection = lsm::Lsm::new(path, 128).unwrap();
        self.collections.insert(collection_name.to_string(), collection);

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
                return Ok(Command::Delete{ collection, key })
            }
            | Some(..) | None => {
                return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Invalid command")
                          )
            }
        }
    }

    pub fn run_command(&mut self, query: String) -> Result<Option<String>, std::io::Error> {
        let command = match Engine::parse_string_to_command(query) {
            Ok(x) => x,
            Err(e) => return Err(e)
        };

        match command {
            Command::ListCollections => {
                return Ok(Some(
                        format!("collections: {:?}",
                                self.collections.keys().collect::<Vec<&String>>())
                        ))
            },
            Command::CreateCollection { collection } => {
                    self.new_collection(&collection)?;
                    return Ok(Some(format!("collection created: {:?}", collection)))
            },
            Command::GetEntries { collection: _ } => todo!("get entries no implemented"),
            Command::GetEntry { collection, key } => {
                let collection: &mut lsm::Lsm = match self.collections.get_mut(&collection) {
                    Some(collection) => collection,
                    None => todo!("invalid collection")
                };
                match collection.search(&Vec::try_from(key).unwrap()) {
                    Err(msg) => Err(msg),
                    Ok(res) => match res {
                        None => Ok(None),
                        // todo: improve here
                        Some(entry) => Ok(Some(format!("{{ key: {}, value: {}, timestamp: {}, deleted: {} }}", 
                                                       String::from_utf8(entry.key).unwrap(),
                                                       String::from_utf8(entry.value.unwrap()).unwrap(),
                                                       entry.timestamp, entry.deleted))),
                    }
                }
            },
            Command::CreateEntry { collection, key, value } => {
                let collection: &mut lsm::Lsm = match self.collections.get_mut(&collection) {
                    Some(collection) => collection,
                    None => todo!("invalid collection")
                };

                let now = Utc::now();
                let timestamp_i64 = now.timestamp();
                let timestamp_u128 = timestamp_i64 as u128;
                let datetime: DateTime<Utc> = DateTime::from_utc(now.naive_utc(), Utc);

                println!("u128 timestamp: {}", timestamp_u128);
                println!("date: {}", datetime.format("%Y-%m-%d %H:%M:%S"));

                let entry = lsm::TableEntry {
                    deleted: false,
                    key: key.clone().into(),
                    value: Some(value.into()),
                    // todo:
                    // change LSM timestamp from u128 to i64
                    timestamp: timestamp_u128
                };

                collection.write(entry).unwrap();
                match collection.search(&Vec::try_from(key).unwrap()) {
                    Err(msg) => Err(msg),
                    Ok(res) => match res {
                        None => Ok(None),
                        Some(entry) => Ok(Some(format!("{{ key: {}, value: {}, timestamp: {}, deleted: {} }}", 
                                                       String::from_utf8(entry.key).unwrap(),
                                                       String::from_utf8(entry.value.unwrap()).unwrap(),
                                                       entry.timestamp, entry.deleted))),
                    }
                }
            },
            Command::Delete { collection, key } => {
                let collection: &mut lsm::Lsm = match self.collections.get_mut(&collection) {
                    Some(collection) => collection,
                    None => todo!("invalid collection")
                };

                let entry = lsm::TableEntry {
                    deleted: true,
                    key: key.clone().into(),
                    value: None,
                    // todo:
                    // generate a valid timestamp, and add the field updated_at
                    timestamp: 1
                };

                collection.write(entry).unwrap();

                Ok(Some(format!("key '{}' deleted", key)))
            }
        }
    }
}
