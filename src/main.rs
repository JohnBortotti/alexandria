#[allow(dead_code)]
mod raft;
mod utils;

use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let env_addr = env::var("PEER_ADDR").unwrap();
    let env_peers = env::var("PEERS").unwrap();
    let peers: Vec<String> = env_peers.split(",").map(|x| x.to_string()).collect();

    let server = raft::server::Server::new(
        &env_addr,
        peers,
        raft::node::Log::new(),
        ).await;

    let tcp_listener = match TcpListener::bind("0.0.0.0:8080").await {
        Ok(listener) => listener,
        _ => panic!("TCPListener bind error")
    };

    let _ = server.serve(tcp_listener).await;
}
