use alexandria::raft::{
    node::log::Log, node::Role, node::Node, node::leader::Leader, 
    node::follower::Follower,
    message::Message, message::Event, message::Address
};
use tokio::sync::mpsc::unbounded_channel;

#[tokio::test] 
async fn raft_basic_log_replication() {
    let (node_tx_1, mut node_rx_1) = tokio::sync::mpsc::unbounded_channel();
    let (state_tx_1, state_rx_1) = tokio::sync::mpsc::unbounded_channel();
    let peers_1 = vec!["b".into(), "c".into()];
    let leader = Role {
        id: "a".into(),
        peers: peers_1.clone(),
        log: Log::new(),
        node_tx: node_tx_1,
        state_tx: state_tx_1,
        role: Leader::new(peers_1, 2),
    };

    let (node_tx_2, mut node_rx_2) = unbounded_channel();
    let (state_tx_2, state_rx_2) = tokio::sync::mpsc::unbounded_channel();
    let node2 = Role {
        id: "b".into(),
        peers: vec!["a".into(), "c".into()],
        log: Log::new(),
        node_tx: node_tx_2,
        state_tx: state_tx_2,
        role: Follower::new(Some("a".to_string()), None, 4)
    };

    let (node_tx_3, mut node_rx_3) = unbounded_channel();
    let (state_tx_3, state_rx_3) = tokio::sync::mpsc::unbounded_channel();
    let node3 = Role {
        id: "c".into(),
        peers: vec!["a".into(), "b".into()],
        log: Log::new(),
        node_tx: node_tx_3,
        state_tx: state_tx_3,
        role: Follower::new(Some("a".to_string()), None, 4)
    };

    // leader should handle a clientRequest
    let client_request = Message {
        event: Event::ClientRequest { command: String::from("command-test-1") },
        term: 1,
        to: Address::Peer("l".to_string()),
        from: Address::Peer("".into()),
    };
    let leader = leader.step(client_request);
    // then leader should broadcast_append_entries to replicate log entry on peers
    let append_entries_msg = node_rx_1.recv().await.unwrap();
    match append_entries_msg.clone().event {
        Event::AppendEntries { entries, commit_index: _ } => {
            match entries {
                Some(entries) => {
                    assert_eq!(entries[0].index, 1);
                    assert_eq!(entries[0].command, "command-test-1");
                },
                None => panic!("appendEntries should contain entries")
            }
        },
        _ => panic!("Expected message event to be AppendEntries")
    };

    // node2 should process the appendEntries and reply with a AckEntries message
    let node2 = node2.step(append_entries_msg.clone());
    let ack_entries_msg_2 = node_rx_2.recv().await.unwrap();
    match ack_entries_msg_2.event {
        Event::AckEntries {index} => {
            assert_eq!(index, 1);
        },
        _ => panic!("Expected message event to be AckEntries")
    }
    
    // node3 should process the appendEntries and reply with a AckEntries message
    let node3 = node3.step(append_entries_msg);
    let ack_entries_msg_3 = node_rx_3.recv().await.unwrap();
    match ack_entries_msg_3.event {
        Event::AckEntries {index} => {
            assert_eq!(index, 1);
        },
        _ => panic!("Expected message event to be AckEntries")
    }

    // todo:
    // after receiving the ackEntries from nodes, leader must commit the log entry, 
    // and send commitEntries with the new commit_index,
    // the nodes should commit the entries as well
}
