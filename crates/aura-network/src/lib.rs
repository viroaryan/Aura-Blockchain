pub mod peer;
pub mod protocol;
pub mod sync;

pub use peer::{PeerId, PeerInfo, PeerManager};
pub use protocol::NetworkMessage;
pub use sync::{ChainSyncer, SyncError};

#[cfg(test)]
mod tests {
    use super::*;
    use aura_crypto::{Address, Hash, KeyPair};
    use aura_primitives::{
        Block, BlockHeader, GenesisAccount, GenesisConfig, GenesisValidator, MerkleTree,
        Transaction, TransactionType,
    };
    use aura_storage::StateDB;
    use std::sync::Arc;

    #[test]
    fn test_peer_manager_scoring_and_banning() {
        let pm = PeerManager::new();
        let peer_id = PeerId::random();

        let peer = PeerInfo {
            id: peer_id,
            addr: None,
            best_height: 50,
            best_hash: Hash::ZERO,
            reputation_score: 10,
            is_banned: false,
        };

        pm.add_peer(peer);
        assert_eq!(pm.active_peer_count(), 1);

        pm.reward(&peer_id, 20);
        assert_eq!(pm.get_highest_peer().unwrap().reputation_score, 30);

        pm.penalize(&peer_id, 150);
        assert_eq!(pm.active_peer_count(), 0); // Banned!
    }

    #[test]
    fn test_sync_apply_blocks_in_sequence() {
        let val_kp = KeyPair::generate();
        let val_addr = Address::from_pubkey(&val_kp.public_key());

        let state_db = Arc::new(StateDB::open_in_memory());
        let genesis = GenesisConfig {
            chain_id: "aura-testnet".into(),
            timestamp: 1700000000,
            accounts: vec![GenesisAccount {
                address: val_addr,
                balance: 10_000_000,
            }],
            validators: vec![GenesisValidator {
                address: val_addr,
                pubkey: val_kp.public_key(),
                stake: 1_000_000,
            }],
            initial_state_root: Hash::ZERO,
        };
        state_db.apply_genesis(&genesis).unwrap();

        let genesis_hash = genesis.to_genesis_block().hash();

        // Create block 1
        let mut block1 = Block::new(
            BlockHeader::new_unsigned(
                1,
                "aura-testnet".into(),
                1,
                0,
                genesis_hash,
                Hash::ZERO,
                Hash::ZERO,
                genesis.validator_set_hash(),
                1700000005,
                val_addr,
            ),
            vec![],
            None,
        );
        block1.header.sign(&val_kp).unwrap();

        let syncer = ChainSyncer::new(state_db.clone());
        let new_height = syncer.apply_block_batch(&[block1]).unwrap();

        assert_eq!(new_height, 1);
        assert_eq!(state_db.get_latest_height(), 1);
    }
}
