use super::{node, message};
use std::collections::HashMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::net::TcpListener;
use tokio_stream::wrappers::{UnboundedReceiverStream, TcpListenerStream};
use tokio_stream::StreamExt as _;
use std::time::Duration;

const TICK: Duration = Duration::from_millis(100);

pub struct Server {
    node: node::Node,
    peers: HashMap<String, String>,
    node_rx: UnboundedReceiver<message::Message>,
}

impl Server {
    pub async fn new(
        id: &str,
        peers: HashMap<String, String>,
        log: node::Log,
        ) -> Self {
        let (node_tx, node_rx) = unbounded_channel();
        Self {
            node: node::Node::new(id, peers.keys().map(|k| k.to_string()).collect(), log, node_tx).await,
            peers,
            node_rx,
        }
    }

    pub async fn serve(self, tcp_listener: TcpListener) -> Result<(), &'static str> {
        // TODO: test single thread vs multithreading (spawn or forwarding messages on single runtime)
        let (tcp_inbound_tx, tcp_inbound_rx) = unbounded_channel::<message::Message>();
        tokio::spawn(Self::handle_inbound_tcp(tcp_listener, tcp_inbound_tx));
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

            // TODO: match node sending message (node_rx -> another peer)
        }
    }

    async fn handle_inbound_tcp(
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

                    // example payload: "(term:1,from:Broadcast,to:Broadcast,event:AppendEntries(index:1,term:1))"
                    let parsed_msg: message::Message = ron::from_str(&req_body[1]).unwrap();
                    let _ = tcp_tr.send(parsed_msg).unwrap();

                    let _ = &socket.writable();
                    let res = "HTTP/1.1 201 OK\r\n";
                    let _ = socket.try_write(res.as_bytes());
                }
                Err(e) => {
                    eprintln!("Erro ao ler os dados: {}", e);
                }
            }
        };

        Ok(())
    }
}
