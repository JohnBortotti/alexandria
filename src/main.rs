#[allow(dead_code)]
mod raft;
mod utils;

use std::env;
use tokio::net::TcpListener;
use log::LevelFilter;
use log::info;

#[tokio::main]
async fn main() {
    let _ = simple_logging::log_to_file("./test.log", LevelFilter::Info);

    // let env_addr = env::var("PEER_ADDR").expect("PEER_ADDR env var not found");
    // let env_peers = env::var("PEERS").expect("PEERS env var not found");
    let env_addr = "0.0.0.0";
    let env_peers = "";

    let _ = env_peers.len();

    let peers: Vec<String> = if env_peers.len() > 0 { 
        env_peers.split(',').map(|x| x.to_string()).collect()
     } else {
         vec!()
    };

    let server = raft::server::Server::new(&env_addr, peers, raft::node::Log::new()).await;

    let tcp_listener = match TcpListener::bind("0.0.0.0:8080").await {
        Ok(listener) => listener,
        _ => panic!("TCPListener bind error"),
    };

    let _ = server.serve(tcp_listener).await;
}
