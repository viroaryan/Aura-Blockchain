pub mod engine;
pub mod proposer;
pub mod slashing;
pub mod types;

pub use engine::{ConsensusEngine, ConsensusError};
pub use proposer::select_proposer;
pub use slashing::{DoubleSignEvidence, SlashingError};
pub use types::{ConsensusMessage, ConsensusStep, TimeoutMessage, ValidatorInfo};

#[cfg(test)]
mod tests {
    use super::*;
    use aura_crypto::{Address, Hash, KeyPair};
    use aura_mempool::{Mempool, MempoolConfig};
    use aura_primitives::{
        GenesisAccount, GenesisConfig, GenesisValidator, Vote, VoteType,
    };
    use aura_storage::{StateDB, StateOverlay};
    use std::sync::Arc;

    #[test]
    fn test_proposer_election_deterministic_and_weighted() {
        let v1_kp = KeyPair::generate();
        let v1_addr = Address::from_pubkey(&v1_kp.public_key());

        let v2_kp = KeyPair::generate();
        let v2_addr = Address::from_pubkey(&v2_kp.public_key());

        let validators = vec![
            ValidatorInfo {
                address: v1_addr,
                pubkey: v1_kp.public_key(),
                voting_power: 100,
            },
            ValidatorInfo {
                address: v2_addr,
                pubkey: v2_kp.public_key(),
                voting_power: 900, // 9x weight
            },
        ];

        let p1 = select_proposer(1, 0, &validators).unwrap();
        let p1_again = select_proposer(1, 0, &validators).unwrap();
        assert_eq!(p1.address, p1_again.address);

        let mut v2_count = 0;
        for h in 1..=100 {
            let p = select_proposer(h, 0, &validators).unwrap();
            if p.address == v2_addr {
                v2_count += 1;
            }
        }
        // Heavily weighted validator v2 should win majority of rounds
        assert!(v2_count > 60);
    }

    #[test]
    fn test_double_sign_evidence_verification_and_slashing() {
        let val_kp = KeyPair::generate();
        let val_addr = Address::from_pubkey(&val_kp.public_key());

        let block_hash_1 = Hash::new([1u8; 32]);
        let block_hash_2 = Hash::new([2u8; 32]);

        let vote1 = Vote::new_signed(block_hash_1, 5, 0, VoteType::PreVote, &val_kp).unwrap();
        let vote2 = Vote::new_signed(block_hash_2, 5, 0, VoteType::PreVote, &val_kp).unwrap();

        let evidence = DoubleSignEvidence {
            vote_a: vote1,
            vote_b: vote2,
        };

        assert_eq!(evidence.verify().unwrap(), val_addr);

        let state_db = StateDB::open_in_memory();
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

        let mut overlay = StateOverlay::new();
        evidence.apply_slashing(&mut overlay, &state_db).unwrap();

        let slashed_acc = overlay.get_account(&val_addr, &state_db);
        assert_eq!(slashed_acc.staked_amount, 800_000); // 20% slashed
        assert!(!slashed_acc.is_validator); // Jailed
    }

    #[test]
    fn test_consensus_engine_proposal_and_commit_cycle() {
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

        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
        let engine = ConsensusEngine::new(
            val_kp.clone(),
            state_db.clone(),
            mempool,
            "aura-testnet".into(),
        );

        let proposal = engine.create_proposal().expect("should create proposal");
        assert_eq!(proposal.header.height, 1);

        let prevote = engine.handle_proposal(&proposal).expect("should handle proposal");
        assert_eq!(prevote.vote_type, VoteType::PreVote);

        // Single validator with 100% stake: precommit should trigger commit
        let precommit = engine
            .handle_prevote(&prevote)
            .expect("should trigger precommit");
        let qc = engine
            .handle_precommit(&precommit)
            .expect("should trigger commit and return QC");

        assert!(qc.is_valid_quorum());
        assert_eq!(state_db.get_latest_height(), 1);
    }
}
