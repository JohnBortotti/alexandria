use alexandria::raft::{
    node::log::Log, node::Role, node::Node, node::leader::Leader, 
    state_machine::StateMachine,
    node::follower::Follower,
    message::Message, message::Event, message::Address
};
use tokio::sync::mpsc::unbounded_channel;

#[tokio::test] 
async fn raft_basic_log_replication() {
    let (node_tx_1, mut node_rx_1) = tokio::sync::mpsc::unbounded_channel();
    let (state_tx_1, state_rx_1) = tokio::sync::mpsc::unbounded_channel();
    let (outbound_tx, _) = tokio::sync::mpsc::unbounded_channel();

    let state_machine = StateMachine::new(state_rx_1, node_tx_1.clone());
    tokio::spawn(state_machine.run("a".to_string()));

    let peers_1 = vec!["b".into(), "c".into()];
    let leader = Role {
        id: "a".into(),
        peers: peers_1.clone(),
        log: Log::new(),
        node_tx: node_tx_1,
        state_tx: state_tx_1,
        outbound_tx: outbound_tx.clone(),
        role: Leader::new(peers_1, 2),
    };

    let (node_tx_2, mut node_rx_2) = unbounded_channel();
    let (state_tx_2, _) = tokio::sync::mpsc::unbounded_channel();
    let node2 = Role {
        id: "b".into(),
        peers: vec!["a".into(), "c".into()],
        log: Log::new(),
        node_tx: node_tx_2,
        state_tx: state_tx_2,
        outbound_tx: outbound_tx.clone(),
        role: Follower::new(Some("a".to_string()), None, 4)
    };

    let (node_tx_3, mut node_rx_3) = unbounded_channel();
    let (state_tx_3, _) = tokio::sync::mpsc::unbounded_channel();
    let node3 = Role {
        id: "c".into(),
        peers: vec!["a".into(), "b".into()],
        log: Log::new(),
        node_tx: node_tx_3,
        state_tx: state_tx_3,
        outbound_tx: outbound_tx.clone(),
        role: Follower::new(Some("a".to_string()), None, 4)
    };

    // leader should handle a clientRequest
    let client_request = Message {
        event: Event::ClientRequest { 
            request_id: 0,
            command: String::from("command-test-1") 
        },
        term: 1,
        to: Address::Peer("l".to_string()),
        from: Address::Peer("".into()),
    };
    let leader = leader.step(client_request).unwrap();
    // then leader should broadcast_append_entries to replicate log entry on peers
    let _ = node_rx_1.recv().await.unwrap();
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
    let node2 = node2.step(append_entries_msg.clone()).unwrap();
    let node2 = match node2 {
        Node::Follower(follower) => {
            assert_eq!(follower.log.last_index, 1);
            assert_eq!(follower.log.commit_index, 0);
            assert_eq!(follower.log.entries[0].index, 1);
            assert_eq!(follower.log.entries[0].command, "command-test-1");
            follower
        },
        _ => panic!("expected node2 to be follower")
        
    };
    let ack_entries_msg_2 = node_rx_2.recv().await.unwrap();
    match ack_entries_msg_2.event {
        Event::AckEntries {index} => {
            assert_eq!(index, 1);
        },
        _ => panic!("Expected message event to be AckEntries")
    }
    
    // node3 should process the appendEntries and reply with a AckEntries message
    let node3 = node3.step(append_entries_msg).unwrap();
    let _node3 = match node3 {
        Node::Follower(follower) => {
            assert_eq!(follower.log.last_index, 1);
            assert_eq!(follower.log.commit_index, 0);
            assert_eq!(follower.log.entries[0].index, 1);
            assert_eq!(follower.log.entries[0].command, "command-test-1");
            follower
        },
        _ => panic!("expected node2 to be follower")
        
    };
    let ack_entries_msg_3 = node_rx_3.recv().await.unwrap();
    match ack_entries_msg_3.event {
        Event::AckEntries {index} => {
            assert_eq!(index, 1);
        },
        _ => panic!("Expected message event to be AckEntries")
    }

    let leader = leader.step(ack_entries_msg_2).unwrap();
    let leader = match leader {
        Node::Leader(leader) => {
            assert_eq!(leader.role.peer_last_index.get("b"), Some(1).as_ref());
            // peer c has not acknowledged yet, so the index is 0
            assert_eq!(leader.role.peer_last_index.get("c"), Some(0).as_ref());
            // leader has commited the new entry since it 
            // replicated in 2 peers from a total of 3 peers
            assert_eq!(leader.log.commit_index, 1);
            leader
        },
        _ => panic!("Expected node to be leader")
    };

    let _leader = leader.tick().tick().tick().tick();
    // now in the next leader appendEntries, he will include the new commit_index
    let commit_replication_msg = node_rx_1.recv().await.unwrap();
    match commit_replication_msg.clone().event {
        Event::AppendEntries { entries: _, commit_index } => {
            assert_eq!(commit_index, 1);
        },
        _ => panic!("Expected message event to be AppendEntries")
    };

    // when node2 receive the appendEntries with the new commit_index, 
    // the node will commit to the same index
    assert_eq!(node2.log.commit_index, 0);
    let node2 = node2.step(commit_replication_msg).unwrap();
    match node2 {
        Node::Follower(follower) => {
            assert_eq!(follower.log.commit_index, 1);
        },
        _ => panic!("Expected node to be follower")
    };
}
