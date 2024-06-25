use super::{
    message,
    message::Event,
    message::Address::{Broadcast, Peer},
    node,
    node::log::Log
};
use crate::utils::{config::CONFIG, log::{log_raft, RaftLogType}};
use std::{ 
    io::prelude::*, 
    collections::hash_map::HashMap, 
    sync::{Mutex, Arc},
    net::TcpStream,
    time::Duration
};
use tokio::{
    io::AsyncWriteExt,
    net::TcpListener,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}
};

use tokio_stream::wrappers::{TcpListenerStream, UnboundedReceiverStream};
use tokio_stream::StreamExt as _;

pub enum NodeResponseType {
    Redirect { address: String },
    Result { result: String },
    NoLeader
}

pub struct NodeResponse {
    pub request_id: u64,
    pub response_type: NodeResponseType
}

pub struct Server {
    node: node::Node,
    peers: Vec<String>,
    node_rx: UnboundedReceiver<message::Message>,
    outbound_rx: UnboundedReceiver<NodeResponse>,
    connection_table: Arc<Mutex<HashMap<u64, tokio::net::TcpStream>>>,
}

impl Server {
    pub async fn new(id: &str, peers: Vec<String>, log: Log) -> Self {
        let (node_tx, node_rx) = unbounded_channel();
        let (outbound_tx, outbound_rx) = unbounded_channel();

        let id = format!("{}:{}", id, CONFIG.server.raft_port);
        let peers: Vec<String> = peers
            .into_iter()
            .map(|peer| format!("{}:{}", peer, CONFIG.server.raft_port))
            .collect();

        Self {
            node: node::Node::new(&id, peers.clone(), log, node_tx, outbound_tx).await,
            peers,
            node_rx,
            outbound_rx,
            connection_table: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn serve(self, raft_tcp_listener: TcpListener, outbound_tcp_listener: TcpListener) 
        -> Result<(), &'static str> {
            let (tcp_inbound_tx, tcp_inbound_rx) = unbounded_channel::<message::Message>();
            let connection_table = Arc::clone(&self.connection_table);

            tokio::spawn(Self::receiving_tcp(raft_tcp_listener, tcp_inbound_tx.clone()));
            tokio::spawn(Self::sending_tcp(self.node_rx, self.peers));
            tokio::spawn(Self::receiving_outbound_tcp(outbound_tcp_listener, tcp_inbound_tx, self.connection_table));
            Self::event_loop(self.node, tcp_inbound_rx, self.outbound_rx, connection_table, CONFIG.raft.tick_millis_duration).await
        }


/*
 * Isn't this overcomplex?
 *
 * 1. user sends TCP request to port 5000
 *
 * 2. receiving_outbound_tcp() will handle this request:
 *   - register the socket on connection_table <HashMap<u64, TcpStream>>
 *   - send the message to node_inbound_tx <raft::Message>
 *
 * 3. this message will be handled by event_loop:
 *   - event loop will send the Message to node via on node.step(msg)
 *   - node will handle the message:
 *      - if the node is a leader:
 *        - append the command on log entry
 *        - replicate this entry and then commit with state_tx.send(entry)
 *        - state will run the query and return the result on node_tx.send(raft::StateResponse)
 *        - the node will handle the StateResponse and return to outbound_tx channel
 *        - the event loop will receive this message via response_rx, match the request_id with
 *          open requests and answer the user request
 *      - if the node is a follower:
 *        - if the command is "READ":
 *          - send the command to run on state_machine
 *          - state will run the query and return the result on node_tx.send(raft::StateResponse)
 *          - the node will handle the StateResponse and return to outbound_tx channel
 *        - if the command is "WRITE":
 *          - return a redirect message to outbound_tx channel
 *          - the event loop will receive this message and return a redirect response
 *
*/
    async fn event_loop(
        mut node: node::Node,
        tcp_inbound_rx: UnboundedReceiver<message::Message>,
        response_rx: UnboundedReceiver<NodeResponse>,
        connection_table: Arc<Mutex<HashMap<u64, tokio::net::TcpStream>>>,
        ticks: u64,
    ) -> Result<(), &'static str> {
        let mut tcp_rx = UnboundedReceiverStream::new(tcp_inbound_rx);
        let mut response_rx = UnboundedReceiverStream::new(response_rx);
        let mut ticker = tokio::time::interval(Duration::from_millis(ticks));

        loop {
            tokio::select! {
                _ = ticker.tick() => node = node.tick(),
                Some(msg) = tcp_rx.next() => node = node.step(msg)?,
                Some(response) = response_rx.next() => {
                            let socket = {
                                let mut map = connection_table.lock().unwrap();
                                map.remove(&response.request_id)
                            };

                            match response.response_type {
                                NodeResponseType::Redirect { address } => {
                                        let status_line = "HTTP/1.1 307 Temporary Redirect\r\n";
                                        let headers = format!("Location: http://{}\r\nContent-Length: 0\r\n\r\n", address);
                                        let response = format!("{}{}", status_line, headers);

                                        if let Some(mut socket) = socket {
                                            if let Err(err) = 
                                                socket.write_all(response.as_bytes()).await {
                                                    log_raft(RaftLogType::Error { 
                                                        message: format!("Failed to write response on socket: {:?}", err)
                                                    });
                                                }
                                        }

                                },
                                NodeResponseType::Result { result } => {
                                    let status_line = "HTTP/1.1 200 OK\r\n";
                                    let content_length = format!("Content-Length: {}\r\n", result.len());
                                    let headers = "Content-Type: text/plain\r\n\r\n";
                                    let response = format!("{}{}{}{}", status_line, content_length, headers, result);

                                    if let Some(mut socket) = socket {
                                        if let Err(err) = 
                                            socket.write_all(response.as_bytes()).await {
                                                log_raft(RaftLogType::Error { 
                                                    message: format!("Failed to write response on socket: {:?}", err)
                                                });
                                            }
                                    }

                                },
                                NodeResponseType::NoLeader => {
                                    let body = "Raft leader has not been elected yet";
                                    let status_line = "HTTP/1.1 503 Service Unavailable\r\n";
                                    let content_length = format!("Content-Length: {}\r\n", body.len());
                                    let headers = "Content-Type: text/plain\r\n\r\n";
                                    let response = format!("{}{}{}{}", status_line, content_length, headers, body);

                                    if let Some(mut socket) = socket {
                                        if let Err(err) = 
                                            socket.write_all(response.as_bytes()).await {
                                                log_raft(RaftLogType::Error { 
                                                    message: format!("Failed to write response on socket: {:?}", err)
                                                });
                                            }
                                    }

                                }
                            }
                }
            }
        }
    }

    async fn receiving_outbound_tcp(
        outbound_tcp_listener: TcpListener,
        tcp_inbound_tx: UnboundedSender<message::Message>,
        connection_table: Arc<Mutex<HashMap<u64, tokio::net::TcpStream>>>,
        ) -> Result<(), std::io::Error> {
        let mut listener = TcpListenerStream::new(outbound_tcp_listener);
        let mut request_id: u64 = 0;

        while let Some(mut socket) = listener.try_next().await? {
            let tcp_inbound_tx = tcp_inbound_tx.clone();
            let connection_table = Arc::clone(&connection_table);
            request_id += 1;

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                let _ = socket.readable().await;

                match socket.try_read(&mut buffer) {
                    Ok(bytes_read) => {
                        let req_text = String::from_utf8(buffer[..bytes_read].to_vec()).unwrap();

                        let req_body: Vec<&str> =
                            req_text.lines().skip_while(|x| !x.is_empty()).collect();

                        if req_body.get(1).is_none() {
                            let status_line = "HTTP/1.1 400 Bad Request\r\n";
                            let headers = "Content-Type: text/plain\r\n\r\n";
                            let body = "Empty request, please provide a valid command";
                            let res = format!("{}{}{}", status_line, headers, body);

                            if let Err(err) = 
                                socket.write_all(res.as_bytes()).await {
                                    log_raft(RaftLogType::Error { 
                                        message: format!("Failed to write response on socket: {:?}", err)
                                    });
                                }
                            
                            return
                        };

                        let user_command = format!("{}\n", req_body[1]);
                        let msg = message::Message::new(
                            0,
                            message::Address::Client,
                            message::Address::Peer("test".to_string()),
                            Event::ClientRequest { 
                                request_id, 
                                command: user_command.try_into().unwrap() 
                            }
                            );

                        let mut table = connection_table.lock().unwrap();
                        table.insert(request_id, socket);

                        tcp_inbound_tx.send(msg).unwrap();
                    },
                    Err(err) => {
                        log_raft(RaftLogType::Error { 
                            message: format!("Error while reading socket: {:?}", err)
                        });
                    }
                }
            });
        }
        Ok(())
    }

    async fn receiving_tcp(
        listener: TcpListener,
        tcp_inbound_tx: UnboundedSender<message::Message>,
    ) -> Result<(), std::io::Error> {
        let mut listener = TcpListenerStream::new(listener);

        while let Some(socket) = listener.try_next().await? {
            let tcp_tr = tcp_inbound_tx.clone();
            let mut buffer = [0; 1024];
            let _ = &socket.readable().await;

            match socket.try_read(&mut buffer) {
                Ok(bytes_read) => {
                    let req_text = match String::from_utf8(buffer[..bytes_read].to_vec()) {
                        Ok(text) => text,
                        Err(e) => {
                            log_raft(RaftLogType::Error { 
                                message: format!("Error decoding UTF-8: {:?}", e)
                            });

                            let res = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
                            let _ = socket.try_write(res.as_bytes());
                            return Ok(());
                        }
                    };

                    let req_body: Vec<&str> =
                        req_text.lines().skip_while(|x| !x.is_empty()).collect();

                    if req_body.get(1).is_none() {
                        log_raft(RaftLogType::Error { 
                            message: format!("Malformed incoming request: {:?}", req_body)
                        });

                        let res = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
                        let _ = socket.try_write(res.as_bytes());
                        return Ok(());
                    };

                    let parsed_msg: message::Message = match ron::from_str(req_body[1]) {
                        Ok(msg) => msg,
                        Err(e) => panic!("error on parsing message: {}, error: {}", req_body[1], e)
                    };
                    tcp_tr.send(parsed_msg).unwrap();

                    let _ = &socket.writable();
                    let res = "HTTP/1.1 200 OK\r\n";
                    let _ = socket.try_write(res.as_bytes());
                }
                Err(err) => {
                    log_raft(RaftLogType::Error { 
                        message: format!("Error while reading socket: {:?}", err)
                    });
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

            match msg.to {
                Broadcast => {
                    peers.iter().for_each(|peer| {
                        let _ = match TcpStream::connect(peer) {
                            Ok(mut stream) => stream.write(http_packet.as_bytes()),
                            _ => {
                                return;
                            }
                        };
                    });
                }
                Peer(addr) => {
                    let _ = match TcpStream::connect(addr) {
                        Ok(mut stream) => stream.write(http_packet.as_bytes()),
                        _ => {
                            return Ok(());
                        }
                    };
                },
                addr => {
                    log_raft(RaftLogType::Error { 
                        message: format!("Invalid message sender: {:?}", addr)
                    });
                }
            }
        }

        Ok(())
    }
}
