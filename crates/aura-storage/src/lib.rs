pub mod account;
pub mod kv;
pub mod smt;
pub mod state;
pub mod wal;

pub use account::Account;
pub use kv::{BatchOp, KeyValueStore, StorageError, CF_ACCOUNTS, CF_BLOCKS, CF_HEIGHTS, CF_META, CF_TXS};
pub use smt::{empty_root, hash_smt_leaf, hash_smt_node, SparseMerkleTree, SMT_DEPTH};
pub use state::{StateDB, StateOverlay};
pub use wal::{WalError, WriteAheadLog};

#[cfg(test)]
mod tests {
    use super::*;
    use aura_crypto::{Address, Hash, KeyPair};
    use aura_primitives::{Block, BlockHeader, GenesisAccount, GenesisConfig, GenesisValidator};

    #[test]
    fn test_smt_deterministic_root_and_updates() {
        let mut smt = SparseMerkleTree::new();
        let initial_root = smt.root();
        assert_eq!(initial_root, empty_root(SMT_DEPTH));

        let key1 = Hash::new([1u8; 32]);
        let val1 = b"balance:1000";
        smt.update(key1, val1);
        let root1 = smt.root();
        assert_ne!(root1, initial_root);

        let key2 = Hash::new([2u8; 32]);
        let val2 = b"balance:5000";
        smt.update(key2, val2);
        let root2 = smt.root();
        assert_ne!(root2, root1);

        // Delete key2 should revert root back to root1
        smt.update(key2, b"");
        assert_eq!(smt.root(), root1);
    }

    #[test]
    fn test_state_db_genesis_and_commit_block() {
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
                    balance: 1_000_000_000,
                },
                GenesisAccount {
                    address: bob_addr,
                    balance: 500_000_000,
                },
            ],
            validators: vec![GenesisValidator {
                address: alice_addr,
                pubkey: alice_kp.public_key(),
                stake: 500_000_000,
            }],
            initial_state_root: Hash::ZERO,
        };

        let root_genesis = state_db.apply_genesis(&genesis).unwrap();
        assert_ne!(root_genesis, Hash::ZERO);

        let alice_acc = state_db.get_account(&alice_addr);
        assert_eq!(alice_acc.balance, 1_000_000_000);
        assert_eq!(alice_acc.staked_amount, 500_000_000);
        assert!(alice_acc.is_validator);

        // Execute a block transfer
        let mut overlay = StateOverlay::new();
        let mut alice_mod = state_db.get_account(&alice_addr);
        alice_mod.balance -= 100_000;
        alice_mod.nonce += 1;
        overlay.set_account(alice_addr, alice_mod);

        let mut bob_mod = state_db.get_account(&bob_addr);
        bob_mod.balance += 100_000;
        overlay.set_account(bob_addr, bob_mod);

        let block1 = Block::new(
            BlockHeader::new_unsigned(
                1,
                "aura-testnet".into(),
                1,
                0,
                genesis.to_genesis_block().hash(),
                Hash::ZERO,
                Hash::ZERO,
                genesis.validator_set_hash(),
                1700000005,
                alice_addr,
            ),
            vec![],
            None,
        );

        let root_b1 = state_db.commit_block(&block1, overlay).unwrap();
        assert_ne!(root_b1, root_genesis);
        assert_eq!(state_db.get_latest_height(), 1);

        assert_eq!(state_db.get_account(&alice_addr).balance, 999_900_000);
        assert_eq!(state_db.get_account(&bob_addr).balance, 500_100_000);
    }

    #[test]
    fn test_wal_write_and_recover() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        {
            let wal = WriteAheadLog::open(&wal_path).unwrap();
            wal.append(b"record_1").unwrap();
            wal.append(b"record_2").unwrap();
            wal.append(b"record_3").unwrap();
        }

        let wal = WriteAheadLog::open(&wal_path).unwrap();
        let recovered = wal.recover_and_truncate().unwrap();
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered[0], b"record_1");
        assert_eq!(recovered[1], b"record_2");
        assert_eq!(recovered[2], b"record_3");
    }
}
