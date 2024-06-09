mod lsm;

use std::{
    path::Path
};
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
    lsm: lsm::Lsm
}

impl Engine {
    pub fn new() -> Self {
        // todo:
        // config path, recover_mode and max_size
        Self { 
            lsm: lsm::Lsm::new(Path::new("./.db-data"), false, 128).unwrap()
        }
    }
}
