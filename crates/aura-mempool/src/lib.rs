pub mod pool;
pub mod validation;

pub use pool::{Mempool, MempoolConfig, MempoolError};
pub use validation::{
    MempoolValidationError, TransactionValidator, MAX_TRANSACTION_SIZE, MIN_TRANSACTION_FEE,
};

#[cfg(test)]
mod tests {
    use super::*;
    use aura_crypto::{Address, Hash, KeyPair};
    use aura_primitives::{GenesisAccount, GenesisConfig, Transaction, TransactionType};
    use aura_storage::StateDB;

    fn setup_test_state() -> (StateDB, KeyPair, KeyPair) {
        let state_db = StateDB::open_in_memory();
        let alice_kp = KeyPair::generate();
        let alice_addr = Address::from_pubkey(&alice_kp.public_key());

        let bob_kp = KeyPair::generate();
        let bob_addr = Address::from_pubkey(&bob_kp.public_key());

        let genesis = GenesisConfig {
            chain_id: "aura-testnet".into(),
            timestamp: 1700000000,
            accounts: vec![
                GenesisAccount {
                    address: alice_addr,
                    balance: 10_000_000,
                },
                GenesisAccount {
                    address: bob_addr,
                    balance: 5_000_000,
                },
            ],
            validators: vec![],
            initial_state_root: Hash::ZERO,
        };

        state_db.apply_genesis(&genesis).unwrap();
        (state_db, alice_kp, bob_kp)
    }

    #[test]
    fn test_mempool_add_and_harvest_by_priority() {
        let (state_db, alice_kp, bob_kp) = setup_test_state();
        let alice_addr = Address::from_pubkey(&alice_kp.public_key());
        let bob_addr = Address::from_pubkey(&bob_kp.public_key());

        let mempool = Mempool::new(MempoolConfig::default());

        // Low fee tx
        let tx1 = Transaction::new_unsigned(
            alice_addr,
            bob_addr,
            1_000,
            100, // min fee
            1,
            TransactionType::Transfer,
            vec![],
            alice_kp.public_key(),
        )
        .sign(&alice_kp)
        .unwrap();

        // High fee tx from bob
        let tx2 = Transaction::new_unsigned(
            bob_addr,
            alice_addr,
            2_000,
            10_000, // high fee
            1,
            TransactionType::Transfer,
            vec![],
            bob_kp.public_key(),
        )
        .sign(&bob_kp)
        .unwrap();

        mempool.add_transaction(tx1, &state_db).unwrap();
        mempool.add_transaction(tx2.clone(), &state_db).unwrap();

        assert_eq!(mempool.len(), 2);

        // Harvest should return high fee tx first
        let harvested = mempool.harvest(1024 * 1024);
        assert_eq!(harvested.len(), 2);
        assert_eq!(harvested[0].tx_hash(), tx2.tx_hash());
    }

    #[test]
    fn test_mempool_rbf_fee_bump() {
        let (state_db, alice_kp, bob_kp) = setup_test_state();
        let alice_addr = Address::from_pubkey(&alice_kp.public_key());
        let bob_addr = Address::from_pubkey(&bob_kp.public_key());

        let mempool = Mempool::new(MempoolConfig::default());

        let tx_orig = Transaction::new_unsigned(
            alice_addr,
            bob_addr,
            1_000,
            1_000,
            1,
            TransactionType::Transfer,
            vec![],
            alice_kp.public_key(),
        )
        .sign(&alice_kp)
        .unwrap();

        mempool.add_transaction(tx_orig, &state_db).unwrap();

        // Try replace with only 5% bump -> should fail (needs >= 10%)
        let tx_insufficient = Transaction::new_unsigned(
            alice_addr,
            bob_addr,
            1_000,
            1_050,
            1,
            TransactionType::Transfer,
            vec![],
            alice_kp.public_key(),
        )
        .sign(&alice_kp)
        .unwrap();

        assert!(matches!(
            mempool.add_transaction(tx_insufficient, &state_db),
            Err(MempoolError::InsufficientRbfBump { .. })
        ));

        // Replace with 20% bump -> should succeed
        let tx_valid_rbf = Transaction::new_unsigned(
            alice_addr,
            bob_addr,
            1_000,
            1_200,
            1,
            TransactionType::Transfer,
            vec![],
            alice_kp.public_key(),
        )
        .sign(&alice_kp)
        .unwrap();

        assert!(mempool.add_transaction(tx_valid_rbf, &state_db).is_ok());
        assert_eq!(mempool.len(), 1);
    }
}
