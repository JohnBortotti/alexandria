use super::{node, message};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
// use tokio_stream::StreamExt as _;
use std::time::Duration;

const TICK: Duration = Duration::from_millis(100);

pub struct Server {
    node: node::Node,
    peers: HashMap<String, String>,
    node_rx: mpsc::UnboundedReceiver<message::Message>,
}

impl Server {
    pub async fn new(
        id: &str,
        peers: HashMap<String, String>,
        log: node::Log,
        ) -> Self {
        let (node_tx, node_rx) = mpsc::unbounded_channel();
        Self {
            node: node::Node::new(id, peers.keys().map(|k| k.to_string()).collect(), log, node_tx).await,
            peers,
            node_rx,
        }
    }

    pub async fn serve(self) {
        // [] open tcp channel to receive requests
        // [] run eventLoop

        Self::event_loop(self.node, self.node_rx).await;
    }

    async fn event_loop(
        mut node: node::Node, 
        node_rx: mpsc::UnboundedReceiver<message::Message>,
        ) {
        let _ = UnboundedReceiverStream::new(node_rx);
        let mut ticker = tokio::time::interval(TICK);

        loop {
            tokio::select! {
                _ = ticker.tick() => node = node.tick(),
            }
        }


    }
}
