use super::server::Server;

pub struct Client {
    server: Server
}

// the raft external interface, clients will handle 
// query requests and parse to Event::ClientRequest,
// redirect the query if the peer is not a leader, or execute if 
// the query is not a transaction
// todo: implement the message routing, if the query is a transaction, redirect to leader,
// otherwise, run in the current node
impl Client {
    pub async fn new(server: Server) -> Self {
        Client { server }
    }

}
