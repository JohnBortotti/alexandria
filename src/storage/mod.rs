mod lsm;

use std::path::Path;
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

// todo:
// - choose the communication way
// - choose how to handle data locks
pub struct Engine {
    path: String,
    lsm: lsm::Lsm
}

impl Engine {
    pub fn new() -> Self {
        // todo:
        // config path, recover_mode and max_size
        Self { 
            path: "./.db-data".to_string(),
            lsm: lsm::Lsm::new(Path::new("./.db-data"), false, 128).unwrap()
        }
    }

    // todo:
    // parse query into a valid command
    pub fn run_command(&mut self, query: String) -> Result<Option<TableEntry>, std::io::Error> {
        let query = query
            .strip_suffix("\r\n")
            .or(query.strip_suffix("\n"))
            .unwrap_or(&query);

        if query.starts_with("get") {
            let key = query.replace("get ", "");

            return self.lsm.search(Path::new(&self.path), &Vec::try_from(key).unwrap())
        } else {
            let query: Vec<&str> = query.split(" ").collect();

            let entry = lsm::TableEntry {
                deleted: false,
                key: query[0].into(),
                value: Some(query[1].into()),
                timestamp: 1
            };

            self.lsm.write(Path::new(&self.path), entry).unwrap();

            return self.lsm.search(Path::new(&self.path), &Vec::try_from(query[0]).unwrap())
        }
    }
}
