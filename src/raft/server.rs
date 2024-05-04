use super::{
    message,
    message::Address::{Broadcast, Peer},
    node,
    node::log::Log
};
use crate::utils::config::CONFIG;
use std::io::prelude::*;
use std::net::TcpStream;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::{TcpListenerStream, UnboundedReceiverStream};
use tokio_stream::StreamExt as _;
use log::info;

pub struct Server {
    node: node::Node,
    peers: Vec<String>,
    node_rx: UnboundedReceiver<message::Message>,
}

impl Server {
    pub async fn new(id: &str, peers: Vec<String>, log: Log) -> Self {
        let (node_tx, node_rx) = unbounded_channel();
        info!(target: "raft", "raft server starting");
        Self {
            node: node::Node::new(id, peers.clone(), log, node_tx).await,
            peers,
            node_rx,
        }
    }

    pub async fn serve(self, tcp_listener: TcpListener) -> Result<(), &'static str> {
        let (tcp_inbound_tx, tcp_inbound_rx) = unbounded_channel::<message::Message>();
        tokio::spawn(Self::receiving_tcp(tcp_listener, tcp_inbound_tx));
        tokio::spawn(Self::sending_tcp(self.node_rx, self.peers));
        Self::event_loop(self.node, tcp_inbound_rx, CONFIG.raft.tick_millis_duration).await
    }

    async fn event_loop(
        mut node: node::Node,
        tcp_inbound_rx: UnboundedReceiver<message::Message>,
        ticks: u64,
    ) -> Result<(), &'static str> {
        let mut tcp_rx = UnboundedReceiverStream::new(tcp_inbound_rx);
        let mut ticker = tokio::time::interval(Duration::from_millis(ticks));

        loop {
            tokio::select! {
                _ = ticker.tick() => node = node.tick(),
                Some(msg) = tcp_rx.next() => node = node.step(msg)?
            }
        }
    }

    async fn receiving_tcp(
        listener: TcpListener,
        tcp_inbound_tr: UnboundedSender<message::Message>,
    ) -> Result<(), std::io::Error> {
        let mut listener = TcpListenerStream::new(listener);

        while let Some(socket) = listener.try_next().await? {
            let tcp_tr = tcp_inbound_tr.clone();
            let mut buffer = [0; 1024];
            let _ = &socket.readable().await;

            match socket.try_read(&mut buffer) {
                Ok(bytes_read) => {
                    let req_text = String::from_utf8(buffer[..bytes_read].to_vec()).unwrap();

                    // TODO: add validation to incoming messages
                    let req_body: Vec<&str> =
                        req_text.lines().skip_while(|x| !x.is_empty()).collect();

                    if req_body.get(1).is_none() {
                        info!(target: "raft", "message incorrect");
                        panic!("message incorrect");
                    };

                    // example payload: 
                    // "(term:1,from:Broadcast,to:Broadcast,event:AppendEntries(index:1,term:1))"
                    let parsed_msg: message::Message = ron::from_str(req_body[1]).unwrap();
                    tcp_tr.send(parsed_msg).unwrap();

                    let _ = &socket.writable();
                    let res = "HTTP/1.1 200 OK\r\n";
                    let _ = socket.try_write(res.as_bytes());
                }
                Err(e) => {
                    info!(target: "raft", "error on reading tcp data: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn sending_tcp(
        node_rx: UnboundedReceiver<message::Message>,
        peers: Vec<String>,
    ) -> Result<(), std::io::Error> {
        let mut listener = UnboundedReceiverStream::new(node_rx);

        while let Some(msg) = listener.next().await {
            let serialized_msg = ron::to_string(&msg).unwrap();
            let http_packet = format!("HTTP/1.1 200 OK \n\n{}", serialized_msg);

            info!(target: "raft", "tcp sending message to: {:?}", msg.to);

            match msg.to {
                Broadcast => {
                    peers.iter().for_each(|peer| {
                        info!(target: "raft", "tcp sending broadcast message to peer: {:?}", peer);
                        let _ = match TcpStream::connect(peer) {
                            Ok(mut stream) => stream.write(http_packet.as_bytes()),
                            _ => {
                                info!(target: "raft", "tcp connection refused (broadcast)");
                                return;
                            }
                        };
                    });
                }
                Peer(addr) => {
                    info!(target: "raft", "tcp sending individual message to peer: {:?}", addr);
                    let _ = match TcpStream::connect(addr) {
                        Ok(mut stream) => stream.write(http_packet.as_bytes()),
                        _ => {
                            info!(target: "raft", "tcp connection refused (individual)");
                            return Ok(());
                        }
                    };
                }
            }
        }

        Ok(())
    }
}
