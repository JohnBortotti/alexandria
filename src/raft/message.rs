#[derive(Debug)]
pub struct Message {
    pub term: u64,
    pub from: NodeAddr,
    pub to: NodeAddr,
    // pub event: Event
}

impl Message {
    pub fn new(term: u64, from: NodeAddr, to: NodeAddr) -> Self {
        Self {
            term,
            from,
            to
        }
    }


}

#[derive(Debug)]
pub enum NodeAddr {
    Broadcast,
    Peer(String)
}

#[derive(Debug)]
pub enum Event {
    Heartbeat,
    SoliciteVote
}
