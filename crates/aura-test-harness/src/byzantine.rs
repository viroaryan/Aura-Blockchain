use aura_crypto::{Hash, KeyPair};
use aura_consensus::DoubleSignEvidence;
use aura_primitives::{Vote, VoteType};

pub struct ByzantineInjector;

impl ByzantineInjector {
    /// Generate double-voting equivocation evidence for testing slashing logic.
    pub fn create_double_vote_evidence(
        keypair: &KeyPair,
        height: u64,
        round: u32,
    ) -> DoubleSignEvidence {
        let hash_a = Hash::new([0xAA; 32]);
        let hash_b = Hash::new([0xBB; 32]);

        let vote_a = Vote::new_signed(hash_a, height, round, VoteType::PreVote, keypair).unwrap();
        let vote_b = Vote::new_signed(hash_b, height, round, VoteType::PreVote, keypair).unwrap();

        DoubleSignEvidence { vote_a, vote_b }
    }
}
