use aura_crypto::{Address, Hash, KeyPair};
use aura_mempool::{Mempool, MempoolConfig, MempoolError};
use aura_primitives::{GenesisAccount, GenesisConfig, Transaction, TransactionType};
use aura_storage::StateDB;

#[test]
fn test_mempool_stateless_and_stateful_validation() {
    let state_db = StateDB::open_in_memory();
    let alice_kp = KeyPair::generate();
    let alice_addr = Address::from_pubkey(&alice_kp.public_key());

    let bob_kp = KeyPair::generate();
    let bob_addr = Address::from_pubkey(&bob_kp.public_key());

    let genesis = GenesisConfig {
        chain_id: "aura-testnet".into(),
        timestamp: 1700000000,
        accounts: vec![GenesisAccount {
            address: alice_addr,
            balance: 50_000,
        }],
        validators: vec![],
        initial_state_root: Hash::ZERO,
    };
    state_db.apply_genesis(&genesis).unwrap();

    let mempool = Mempool::new(MempoolConfig::default());

    // Valid tx
    let tx_valid = Transaction::new_unsigned(
        alice_addr,
        bob_addr,
        10_000,
        1_000,
        1,
        TransactionType::Transfer,
        vec![],
        alice_kp.public_key(),
    )
    .sign(&alice_kp)
    .unwrap();

    assert!(mempool.add_transaction(tx_valid, &state_db).is_ok());

    // Tx exceeding balance
    let tx_excess = Transaction::new_unsigned(
        alice_addr,
        bob_addr,
        100_000,
        1_000,
        2,
        TransactionType::Transfer,
        vec![],
        alice_kp.public_key(),
    )
    .sign(&alice_kp)
    .unwrap();

    assert!(mempool.add_transaction(tx_excess, &state_db).is_err());
}
