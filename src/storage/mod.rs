mod lsm;

use std::{path::Path, collections::HashMap, path::PathBuf, fs::read_dir, fs::create_dir_all};
use serde::{Deserialize, Serialize};
use lsm::TableEntry;

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
    List,
    GetEntry(String, String),
    Write { collection: String, key: String, value: String }
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
        // - config path, recover_mode and max_size
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
                                let collection = lsm::Lsm::new(path, 128).unwrap();
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

        let collection = lsm::Lsm::new(path, 128).unwrap();
        self.collections.insert(collection_name.to_string(), collection);

        Ok(())
    }

    // todo:
    // parse query into a valid command
    pub fn run_command(&mut self, query: String) -> Result<Option<String>, std::io::Error> {
        let query: Vec<&str> = query
            .strip_suffix("\r\n")
            .or(query.strip_suffix("\n"))
            .unwrap_or(&query)
            .split(" ").collect();

        if query[0] == "list" {
            return Ok(Some(format!("collections: {:?}", self.collections.keys().collect::<Vec<&String>>())))
        }

        if query[0] == "create" {
            self.new_collection(query[1])?;
            return Ok(Some(format!("collection created: {:?}", query[1])))
        }

        let collection: &mut lsm::Lsm = match self.collections.get_mut(query[0]) {
            Some(collection) => collection,
            None => return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("collection not found: {}", query[0])))
        };

        if query[1] == "get" {
            let key = query[2];

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
        } else {
            let entry = lsm::TableEntry {
                deleted: false,
                key: query[1].into(),
                value: Some(query[2].into()),
                // todo:
                // generate a valid timestamp, and add the field updated_at
                timestamp: 1
            };

            collection.write(entry).unwrap();
            match collection.search(&Vec::try_from(query[1]).unwrap()) {
                Err(msg) => Err(msg),
                Ok(res) => match res {
                    None => Ok(None),
                    Some(entry) => Ok(Some(format!("{{ key: {}, value: {}, timestamp: {}, deleted: {} }}", 
                                                   String::from_utf8(entry.key).unwrap(),
                                                   String::from_utf8(entry.value.unwrap()).unwrap(),
                                                   entry.timestamp, entry.deleted))),
                }
            }
        }
    }
}
