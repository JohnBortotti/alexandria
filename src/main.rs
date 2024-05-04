#[allow(dead_code)]
mod raft;
mod utils;

use std::env;
use tokio::net::TcpListener;
use raft::node::log::Log;
use log::LevelFilter;
use log::info;

#[tokio::main]
async fn main() {
    // config log driver 
    let _ = simple_logging::log_to_file("./logs/info.log", LevelFilter::Info);

    let env_addr = env::var("PEER_ADDR").expect("PEER_ADDR env var not found");
    let env_peers = env::var("PEERS").expect("PEERS env var not found");

    info!(target: "main", "starting peer: {}", env_addr);

    let _ = env_peers.len();

    let peers: Vec<String> = if env_peers.len() > 0 { 
        env_peers.split(',').map(|x| x.to_string()).collect()
     } else {
         vec!()
    };

    let server = raft::server::Server::new(&env_addr, peers, Log::new()).await;

    let tcp_listener = match TcpListener::bind("0.0.0.0:8080").await {
        Ok(listener) => listener,
        _ => panic!("TCPListener bind error"),
    };

    let _ = server.serve(tcp_listener).await;

    // curl -X POST -d '(term:1, from: Peer("test"), to: Peer("test"), event: ClientRequest(command: "test"))' url
    // 
    // todo: create the client layer, responsible to create properly formatted messages to peers, and
    // redirect messages that were sent to peers that are not leaders. 
    //
    // user request/message -> raft/client -> raft/server -> peer/node
    //
}
