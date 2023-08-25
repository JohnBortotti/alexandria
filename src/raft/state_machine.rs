use tokio::sync::mpsc::{ UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt as _;
use super::message::{Message, Address};

#[derive(Clone, Debug)]
pub struct Entry {
    pub index: u64,
    pub term: u64,
    pub command: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum Instruction {
    Abort,
    Append { entry: Entry },
    Notify { id: Vec<u8>, address: Address},
    Vote { term: u64, address: Address}
}

pub struct StateDriver {
    state_rx: UnboundedReceiverStream<Instruction>,
    node_tx: UnboundedSender<Message>,
    applied_index: u64
}

impl StateDriver {
    pub fn new(
        state_rx: UnboundedReceiver<Instruction>,
        node_tx: UnboundedSender<Message>,
        ) -> Self {
        Self {
            state_rx: UnboundedReceiverStream::new(state_rx),
            node_tx,
            applied_index: 0
        }
    }

    pub async fn run(mut self) {
        println!("running state_driver");

        while let Some(msg) = self.state_rx.next().await {
            println!("\nInstruction received by state_driver:");
            println!("{:?}", msg);
        } 
    }
}
