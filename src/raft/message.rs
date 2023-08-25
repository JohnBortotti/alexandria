#[derive(Debug)]
pub struct Message {
    pub term: u64,
    pub from: Address,
    pub to: Address,
    pub event: Event
}

impl Message {
    pub fn new(term: u64, from: Address, to: Address, event: Event) -> Self {
        Self {
            term,
            from,
            to,
            event
        }
    }


}

#[derive(Debug)]
pub enum Address {
    Broadcast,
    Peer(String)
}

#[derive(Debug)]
pub enum Event {
    Heartbeat { index: u64, term: u64 },
    SoliciteVote
}
