use aura_crypto::{
    hash_with_domain, Address, Hash, KeyPair, PublicKey, Signature, DOMAIN_VOTE,
};
use serde::{Deserialize, Serialize};

use crate::transaction::PrimitiveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    PreVote,
    PreCommit,
}

/// A BFT Consensus Vote cast by an active validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub block_hash: Hash,
    pub height: u64,
    pub round: u32,
    pub vote_type: VoteType,
    pub validator: Address,
    pub pubkey: PublicKey,
    pub signature: Signature,
}

impl Vote {
    pub fn signing_bytes(
        block_hash: &Hash,
        height: u64,
        round: u32,
        vote_type: VoteType,
        validator: &Address,
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(block_hash.as_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&round.to_be_bytes());
        let type_byte = match vote_type {
            VoteType::PreVote => 0u8,
            VoteType::PreCommit => 1u8,
        };
        bytes.push(type_byte);
        bytes.extend_from_slice(validator.as_bytes());
        bytes
    }

    pub fn new_signed(
        block_hash: Hash,
        height: u64,
        round: u32,
        vote_type: VoteType,
        keypair: &KeyPair,
    ) -> Result<Self, PrimitiveError> {
        let validator = Address::from_pubkey(&keypair.public_key());
        let signing_data = Self::signing_bytes(&block_hash, height, round, vote_type, &validator);
        let signature = keypair.sign(&signing_data);

        Ok(Self {
            block_hash,
            height,
            round,
            vote_type,
            validator,
            pubkey: keypair.public_key(),
            signature,
        })
    }

    pub fn verify(&self) -> Result<(), PrimitiveError> {
        if Address::from_pubkey(&self.pubkey) != self.validator {
            return Err(PrimitiveError::AddressMismatch);
        }
        let signing_data =
            Self::signing_bytes(&self.block_hash, self.height, self.round, self.vote_type, &self.validator);
        self.pubkey
            .verify(&signing_data, &self.signature)
            .map_err(|_| PrimitiveError::InvalidSignature)
    }

    pub fn vote_hash(&self) -> Hash {
        let signing_data =
            Self::signing_bytes(&self.block_hash, self.height, self.round, self.vote_type, &self.validator);
        hash_with_domain(DOMAIN_VOTE, &signing_data)
    }
}

/// A BFT Quorum Certificate aggregating > 2/3 pre-commits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub block_hash: Hash,
    pub height: u64,
    pub round: u32,
    pub votes: Vec<Vote>,
    pub total_voting_power: u64,
    pub signers_voting_power: u64,
}

impl QuorumCertificate {
    /// Check if the aggregated voting power meets or exceeds the 2/3 BFT threshold (> 2/3).
    pub fn is_valid_quorum(&self) -> bool {
        if self.total_voting_power == 0 {
            return false;
        }
        // 3 * signers > 2 * total
        (self.signers_voting_power as u128) * 3 > (self.total_voting_power as u128) * 2
    }
}
