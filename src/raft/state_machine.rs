use super::{message::{Message, Address, Event}, node::log::Entry};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt as _;

pub struct StateMachine {
    state_rx: UnboundedReceiverStream<Entry>,
    node_tx: UnboundedSender<Message>,
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
            applied_index: 0,
        }
    }

    pub async fn run(mut self, self_addr: String) {
        while let Some(entry) = self.state_rx.next().await {

            // todo: execute command on store engine
            self.applied_index = entry.index;

            // todo:
            // change the message struct of this channel and 
            // handle possible errors on sending message
            self.node_tx.send(Message::new(
                    entry.term,
                    Address::StateMachine,
                    Address::Peer(self_addr.clone()),
                    Event::StateResponse { 
                        request_id: entry.request_id,
                        result: Ok("instruction executed succesfully".to_string()) 
                    }
            )).unwrap();
        }
    }
}
