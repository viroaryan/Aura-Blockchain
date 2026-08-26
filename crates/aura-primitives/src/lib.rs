pub mod block;
pub mod genesis;
pub mod header;
pub mod merkle;
pub mod qc;
pub mod transaction;

pub use block::Block;
pub use genesis::{GenesisAccount, GenesisConfig, GenesisValidator};
pub use header::BlockHeader;
pub use merkle::{hash_internal_node, hash_leaf, MerkleProof, MerkleTree};
pub use qc::{QuorumCertificate, Vote, VoteType};
pub use transaction::{PrimitiveError, Transaction, TransactionType};

#[cfg(test)]
mod tests {
    use super::*;
    use aura_crypto::{Address, Hash, KeyPair, DEFAULT_HRP};

    #[test]
    fn test_transaction_signing_and_verification() {
        let sender_kp = KeyPair::generate();
        let sender_addr = Address::from_pubkey(&sender_kp.public_key());

        let receiver_kp = KeyPair::generate();
        let receiver_addr = Address::from_pubkey(&receiver_kp.public_key());

        let tx = Transaction::new_unsigned(
            sender_addr,
            receiver_addr,
            500_000_000,
            1_000,
            1,
            TransactionType::Transfer,
            vec![],
            sender_kp.public_key(),
        )
        .sign(&sender_kp)
        .unwrap();

        assert!(tx.verify().is_ok());

        // Tamper with transaction amount
        let mut tampered_tx = tx.clone();
        tampered_tx.amount = 999_999_999;
        assert!(tampered_tx.verify().is_err());
    }

    #[test]
    fn test_merkle_tree_root_and_inclusion_proof() {
        let payloads = vec![
            b"tx_001_transfer_alice_bob".to_vec(),
            b"tx_002_stake_validator_1".to_vec(),
            b"tx_003_transfer_carol_dave".to_vec(),
            b"tx_004_register_validator_2".to_vec(),
            b"tx_005_transfer_eve_frank".to_vec(),
        ];

        let tree = MerkleTree::from_leaf_payloads(&payloads);
        let root = tree.root();
        assert_ne!(root, Hash::ZERO);

        for (i, payload) in payloads.iter().enumerate() {
            let proof = tree.generate_proof(i).expect("proof generation should succeed");
            assert!(proof.verify(&root, payload));

            // Tamper test: modified payload should fail verification
            let bad_payload = b"tampered_payload_data";
            assert!(!proof.verify(&root, bad_payload));
        }
    }

    #[test]
    fn test_tamper_detection_in_block() {
        let proposer_kp = KeyPair::generate();
        let proposer_addr = Address::from_pubkey(&proposer_kp.public_key());

        let sender_kp = KeyPair::generate();
        let sender_addr = Address::from_pubkey(&sender_kp.public_key());

        let tx1 = Transaction::new_unsigned(
            sender_addr,
            proposer_addr,
            100,
            10,
            1,
            TransactionType::Transfer,
            vec![],
            sender_kp.public_key(),
        )
        .sign(&sender_kp)
        .unwrap();

        let mut block = Block::new(
            BlockHeader::new_unsigned(
                1,
                "aura-testnet-1".into(),
                1,
                0,
                Hash::ZERO,
                Hash::ZERO,
                Hash::ZERO,
                Hash::ZERO,
                1700000000,
                proposer_addr,
            ),
            vec![tx1],
            None,
        );

        block.header.merkle_root = block.compute_merkle_root();
        block.header.sign(&proposer_kp).unwrap();

        // Valid block validates
        assert!(block.validate_basic().is_ok());

        // Tamper with transaction in body: Merkle root mismatch detected!
        let mut tampered_block = block.clone();
        tampered_block.transactions[0].amount = 50000;
        assert!(tampered_block.validate_basic().is_err());
    }

    #[test]
    fn test_bft_quorum_certificate() {
        let qc_valid = QuorumCertificate {
            block_hash: Hash::ZERO,
            height: 10,
            round: 0,
            votes: vec![],
            total_voting_power: 100,
            signers_voting_power: 67, // > 2/3 (66.66)
        };
        assert!(qc_valid.is_valid_quorum());

        let qc_invalid = QuorumCertificate {
            block_hash: Hash::ZERO,
            height: 10,
            round: 0,
            votes: vec![],
            total_voting_power: 100,
            signers_voting_power: 66, // <= 2/3
        };
        assert!(!qc_invalid.is_valid_quorum());
    }
}
