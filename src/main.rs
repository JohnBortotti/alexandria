#[allow(dead_code)]
mod raft;

use std::collections::HashMap;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let server = raft::server::Server::new("127.0.0.1:8080", HashMap::new(), raft::node::Log::new()).await;
    let tcp_listener = match TcpListener::bind("127.0.0.1:8080").await {
        Ok(listener) => listener,
        _ => panic!("TCPListener bind error")
    };

    let _ = server.serve(tcp_listener).await;
}
