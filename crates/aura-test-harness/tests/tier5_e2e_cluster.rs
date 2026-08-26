use aura_test_harness::TestCluster;

#[test]
fn test_multi_node_cluster_block_step() {
    let cluster = TestCluster::spawn_validators(2);
    assert_eq!(cluster.nodes.len(), 2);

    let block_hash = cluster.step_block_round();
    assert_ne!(block_hash, aura_crypto::Hash::ZERO);

    assert_eq!(cluster.nodes[0].state_db.get_latest_height(), 1);
    assert_eq!(cluster.nodes[1].state_db.get_latest_height(), 1);
}
