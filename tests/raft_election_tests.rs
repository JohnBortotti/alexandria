use alexandria::raft::{node::Node, node::log::Log, message::Event};
use tokio::sync::mpsc::unbounded_channel;

#[tokio::test]
async fn raft_basic_election() {
    let (node_tx_1, mut node_rx_1) = unbounded_channel();
    let node1 = Node::new(
        "a",
        vec!("b".to_string(), "c".to_string()),
        Log::new(),
        node_tx_1
        ).await;

    let (node_tx_2, mut node_rx_2) = unbounded_channel();
    let node2 = Node::new(
        "b",
        vec!("a".to_string(), "c".to_string()),
        Log::new(),
        node_tx_2
        ).await;

    let (node_tx_3, mut node_rx_3) = unbounded_channel();
    let node3 = Node::new(
        "c",
        vec!("a".to_string(), "b".to_string()),
        Log::new(),
        node_tx_3
        ).await;

    // node1 should broadcast request_vote messages
    let node1 = node1.tick().tick().tick().tick().tick();
    let _request_vote_msg = node_rx_1.recv().await.unwrap();

    // node1 should broadcast a requestVote
    match _request_vote_msg.event {
        Event::RequestVote{} => {},
        _ => panic!("Expected event to be RequestVote")
    };

    // node2 should receive and reply with a vote
    node2.step(_request_vote_msg.clone()).unwrap();
    let _vote_msg_2 = node_rx_2.recv().await.unwrap();

    // node2 should vote for peer "a"
    match _vote_msg_2.clone().event {
        Event::Vote{ voted_for } => {
            assert_eq!(voted_for, "a");
        },
        _ => panic!("Expected event to be Vote")
    }

    // node3 should receive and reply with a vote
    node3.step(_request_vote_msg).unwrap();
    let _vote_msg_3 = node_rx_3.recv().await.unwrap();

    // node3 should vote for peer "a"
    match _vote_msg_3.clone().event {
        Event::Vote{ voted_for } => {
            assert_eq!(voted_for, "a");
        },
        _ => panic!("Expected event to be Vote")
    }

    // node1 should recive the votes and became the leader
    let node1 = node1.step(_vote_msg_2).unwrap().step(_vote_msg_3).unwrap();
    match node1 {
        Node::Leader(_) => {},
        _ => panic!("Expected node1 to be the leader")
    }
}
