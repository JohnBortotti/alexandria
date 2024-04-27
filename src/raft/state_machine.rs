use super::message::Message;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt as _;
use log::info;

// todo: update the command data structure
#[derive(Debug)]
pub struct Instruction {
    pub index: u64,
    pub term: u64,
    pub command: String,
}

pub struct StateMachine {
    state_rx: UnboundedReceiverStream<Instruction>,
    node_tx: UnboundedSender<Message>,
    applied_index: u64,
}

impl StateMachine {
    pub fn new(
        state_rx: UnboundedReceiver<Instruction>,
        node_tx: UnboundedSender<Message>,
    ) -> Self {
        Self {
            state_rx: UnboundedReceiverStream::new(state_rx),
            node_tx,
            applied_index: 0,
        }
    }

    pub async fn run(mut self) {
        info!(target: "state_machine", "running state_driver");

        while let Some(msg) = self.state_rx.next().await {
            println!("\nInstruction received by state_driver:");
            println!("{:?}", msg);

            // todo: execute command 
            
            self.applied_index = msg.index;

            // todo: return response
        }
    }
}
