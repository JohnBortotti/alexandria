#[allow(dead_code)]
mod raft;
mod utils;
mod storage;

use std::env;
use tokio::net::TcpListener;
use raft::{node::log::Log, logging::log_raft, logging::RaftLogType};
use log::LevelFilter;
use utils::config::CONFIG;

#[tokio::main]
async fn main() {
    let _ = simple_logging::log_to_file("./logs/info.log", LevelFilter::Info);

    let env_addr = env::var("PEER_ADDR").expect("PEER_ADDR env var not found");
    let env_peers = env::var("PEERS").expect("PEERS env var not found");
    let _ = env_peers.len();
    let peers: Vec<String> = if env_peers.len() > 0 { 
        env_peers.split(',').map(|x| x.to_string()).collect()
     } else {
         vec!()
    };

    log_raft(
        RaftLogType::PeerStart { 
            id: env_addr.clone(), 
            peers: peers.clone()
        });

    let server = raft::server::Server::new(&env_addr, peers, Log::new()).await;
    let raft_listener = 
        match TcpListener::bind(format!("0.0.0.0:{}", CONFIG.server.raft_port)).await {
            Ok(listener) => listener,
            _ => panic!("TCPListener bind error"),
        };
    let outbound_listener = 
        match TcpListener::bind(format!("0.0.0.0:{}", CONFIG.server.outbound_port)).await {
            Ok(listener) => listener,
            _ => panic!("TCPListener bind error"),
        };

    let _ = server.serve(raft_listener, outbound_listener).await;

    // curl -X POST -d '(term:1, from: Peer("test"), to: Peer("test"), event: ClientRequest(command: "test"))' url
}
