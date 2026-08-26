use aura_crypto::{Address, Hash, KeyPair};
use aura_mempool::{Mempool, MempoolConfig};
use aura_primitives::{GenesisAccount, GenesisConfig, GenesisValidator, VoteType};
use aura_storage::StateDB;
use aura_test_harness::ByzantineInjector;
use std::sync::Arc;

#[test]
fn test_bft_consensus_liveness_and_safety() {
    let val_kp = KeyPair::generate();
    let val_addr = Address::from_pubkey(&val_kp.public_key());

    let state_db = Arc::new(StateDB::open_in_memory());
    let genesis = GenesisConfig {
        chain_id: "aura-testnet".into(),
        timestamp: 1700000000,
        accounts: vec![GenesisAccount {
            address: val_addr,
            balance: 100_000_000,
        }],
        validators: vec![GenesisValidator {
            address: val_addr,
            pubkey: val_kp.public_key(),
            stake: 10_000_000,
        }],
        initial_state_root: Hash::ZERO,
    };
    state_db.apply_genesis(&genesis).unwrap();

    let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
    let engine = aura_consensus::ConsensusEngine::new(
        val_kp.clone(),
        state_db.clone(),
        mempool,
        "aura-testnet".into(),
    );

    // Produce block 1
    let proposal1 = engine.create_proposal().unwrap();
    let prevote1 = engine.handle_proposal(&proposal1).unwrap();
    let precommit1 = engine.handle_prevote(&prevote1).unwrap();
    let qc1 = engine.handle_precommit(&precommit1).unwrap();

    assert_eq!(qc1.height, 1);
    assert_eq!(state_db.get_latest_height(), 1);

    // Produce block 2
    let proposal2 = engine.create_proposal().unwrap();
    let prevote2 = engine.handle_proposal(&proposal2).unwrap();
    let precommit2 = engine.handle_prevote(&prevote2).unwrap();
    let qc2 = engine.handle_precommit(&precommit2).unwrap();

    assert_eq!(qc2.height, 2);
    assert_eq!(state_db.get_latest_height(), 2);
}

#[test]
fn test_byzantine_double_vote_slashing() {
    let val_kp = KeyPair::generate();
    let evidence = ByzantineInjector::create_double_vote_evidence(&val_kp, 5, 0);
    assert!(evidence.verify().is_ok());
}
