use super::{message::{Message, Address, Event}, 
    node::log::Entry, 
    super::{
        storage::Engine,
        utils::log::{log_raft, RaftLogType}
    }
};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt as _;
use std::sync::Arc;

pub struct StateMachine {
    state_rx: UnboundedReceiverStream<Entry>,
    node_tx: UnboundedSender<Message>,
    storage_engine: Arc<Engine>,
    applied_index: usize,
}

impl StateMachine {
    pub fn new(
        state_rx: UnboundedReceiver<Entry>,
        node_tx: UnboundedSender<Message>,
        ) -> Self {
        Self {
            state_rx: UnboundedReceiverStream::new(state_rx),
            node_tx,
            storage_engine: Arc::new(Engine::new()),
            applied_index: 0,
        }
    }

    pub async fn run(mut self, self_addr: String) {
        while let Some(entry) = self.state_rx.next().await {
            self.applied_index = entry.index;

            let storage_engine = Arc::clone(&self.storage_engine);
            let self_addr = self_addr.clone();
            let node_tx = self.node_tx.clone();

            tokio::spawn(async move {
                let engine = storage_engine.clone();
                let result_entry = 
                    match engine.run_command(entry.command.clone()).await {
                        Err(msg) => msg.to_string(),
                        Ok(val) => match val {
                            Some(entry) => entry,
                            None => "key not found".to_string(),
                        }
                    };

                let message = Message::new(
                    entry.term,
                    Address::StateMachine,
                    Address::Peer(self_addr),
                    Event::StateResponse {
                        request_id: entry.request_id,
                        result: Ok(result_entry),
                    },
                    );
                
                if let Err(e) = node_tx.send(message) {
                    log_raft(RaftLogType::Error {
                        message: format!("State_machine failed to send message: {:?}", e)
                    });
                }
            });
        }
    }
}
