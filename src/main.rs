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

mod raft;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt as _;

#[tokio::main]
async fn main() {

    let (node_tx, node_rx) = mpsc::unbounded_channel::<raft::message::Message>();

    let _ = raft::Node::new(
        "test",
        vec!(),
        raft::Log::new(),
        node_tx.clone()
    ).await;


    let mut rx_stream = UnboundedReceiverStream::new(node_rx);

    loop {
        while let Some(msg) = rx_stream.next().await {
            println!("{:?}", msg);
        }
    }
}
