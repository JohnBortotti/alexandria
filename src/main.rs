// Ideas
// 
// description: create a simple relational database, storing data (normal tables) with some kind of Btree
// and Raft as consensus algorithm. Not inteended to be fully deployable, just simulate some
// operations, consensus, memory management, async and distributed computation.
//
// planning: -> API (cli, http, async server, dont know yet) 
//           -> SQL front-end (lex, parse, planner, etc) 
//           -> Raft backend (leader, replication, commit and log) 
//           -> Storage backend (SQL State machine) 
//
// Raft: -> base algorithm 
//       -> log database (append-file)
//
// DB engine: -> simple B-tree variation (B+tree, etc)
//            -> implement Store operations (set, get, delete, flush)
//
// SQL: -> data types (bool, int, float, string)
//      -> single x multiple databases??
//      -> schema operations (create, delete, read, update??)
//      -> implement transactions for row operations (ACID)

// ticks
// set tick from miliseconds
// set tick intervals (steps and elections)
// create tokio timer (on node eventLoop)
// implement each role tick

#[allow(dead_code)]
mod raft;

use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let server = raft::server::Server::new("a", HashMap::new(), raft::node::Log::new()).await;
    server.serve().await;
}
