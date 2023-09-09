use super::{node, message, message::Address::{Broadcast, Peer}};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::net::TcpListener;
use tokio_stream::wrappers::{UnboundedReceiverStream, TcpListenerStream};
use tokio_stream::StreamExt as _;
use std::time::Duration;
use std::net::TcpStream;
use std::io::prelude::*;

const TICK: Duration = Duration::from_millis(100);

pub struct Server {
    node: node::Node,
    peers: Vec<String>,
    node_rx: UnboundedReceiver<message::Message>,
}

impl Server {
    pub async fn new(
        id: &str,
        peers: Vec<String>,
        log: node::Log,
        ) -> Self {
        let (node_tx, node_rx) = unbounded_channel();
        Self {
            node: node::Node::new(id, peers.clone(), log, node_tx).await,
            peers,
            node_rx,
        }
    }

    pub async fn serve(self, tcp_listener: TcpListener) -> Result<(), &'static str> {
        // TODO: test single thread vs multithreading (spawn or forwarding messages on single runtime)
        let (tcp_inbound_tx, tcp_inbound_rx) = unbounded_channel::<message::Message>();
        tokio::spawn(Self::inbound_receiving_tcp(tcp_listener, tcp_inbound_tx));
        tokio::spawn(Self::inbound_sending_tcp(self.node_rx, self.peers));
        Self::event_loop(self.node, tcp_inbound_rx).await
    }

    async fn event_loop(
        mut node: node::Node, 
        tcp_inbound_rx: UnboundedReceiver<message::Message>
        ) -> Result<(), &'static str> {
        let mut tcp_rx = UnboundedReceiverStream::new(tcp_inbound_rx);
        let mut ticker = tokio::time::interval(TICK);

        loop {
            tokio::select! {
                _ = ticker.tick() => node = node.tick(),
                Some(msg) = tcp_rx.next() => node = node.step(msg)?
            }

            // TODO: code for peers sending messages (handle message routing)
        }
    }

    async fn inbound_receiving_tcp(
        listener: TcpListener, 
        tcp_inbound_tr: UnboundedSender<message::Message>
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
                    let req_body: Vec<&str> = req_text.lines()
                        .skip_while(|x| !x.is_empty())
                        .collect();

                    if let None = &req_body.get(1) {
                        panic!("message incorrect");
                    };

                    // example payload: "(term:1,from:Broadcast,to:Broadcast,event:AppendEntries(index:1,term:1))"
                    let parsed_msg: message::Message = ron::from_str(&req_body[1]).unwrap();
                    let _ = tcp_tr.send(parsed_msg).unwrap();

                    let _ = &socket.writable();
                    let res = "HTTP/1.1 200 OK\r\n";
                    let _ = socket.try_write(res.as_bytes());
                }
                Err(e) => {
                    eprintln!("Erro ao ler os dados: {}", e);
                }
            }
        };

        Ok(())
    }

    async fn inbound_sending_tcp(
        node_rx: UnboundedReceiver::<message::Message>,
        peers: Vec<String>
        ) -> Result<(), std::io::Error> {
        let mut listener = UnboundedReceiverStream::new(node_rx);

        while let Some(msg) = listener.next().await {
            let serialized_msg = ron::to_string(&msg).unwrap();
            let http_packet = format!("HTTP/1.1 200 OK \n\n{}", serialized_msg);

            match msg.to {
                Broadcast => {
                    peers.iter().for_each(|peer| {
                        let mut stream = TcpStream::connect(peer).unwrap();
                        stream.write(http_packet.as_bytes()).unwrap();
                    });
                },
                Peer(addr) => {
                    let mut stream = TcpStream::connect(addr).unwrap();
                    stream.write(http_packet.as_bytes()).unwrap();
                },
            }
        };

        Ok(())
    }
}
