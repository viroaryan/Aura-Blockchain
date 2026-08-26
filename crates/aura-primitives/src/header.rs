use aura_crypto::{
    hash_with_domain, Address, Hash, KeyPair, PublicKey, Signature, DOMAIN_BLOCK_HEADER,
};
use serde::{Deserialize, Serialize};

use crate::transaction::PrimitiveError;

/// Fixed-field Block Header for Aura blockchain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub chain_id: String,
    pub height: u64,
    pub round: u32,
    pub prev_hash: Hash,
    pub merkle_root: Hash,
    pub state_root: Hash,
    pub validator_set_hash: Hash,
    pub timestamp: u64,
    pub proposer: Address,
    pub signature: Signature,
}

impl BlockHeader {
    /// Construct unsigned block header.
    pub fn new_unsigned(
        version: u32,
        chain_id: String,
        height: u64,
        round: u32,
        prev_hash: Hash,
        merkle_root: Hash,
        state_root: Hash,
        validator_set_hash: Hash,
        timestamp: u64,
        proposer: Address,
    ) -> Self {
        Self {
            version,
            chain_id,
            height,
            round,
            prev_hash,
            merkle_root,
            state_root,
            validator_set_hash,
            timestamp,
            proposer,
            signature: Signature::from_bytes([0u8; 64]),
        }
    }

    /// Serialization for signature verification.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_be_bytes());
        bytes.extend_from_slice(&(self.chain_id.len() as u32).to_be_bytes());
        bytes.extend_from_slice(self.chain_id.as_bytes());
        bytes.extend_from_slice(&self.height.to_be_bytes());
        bytes.extend_from_slice(&self.round.to_be_bytes());
        bytes.extend_from_slice(self.prev_hash.as_bytes());
        bytes.extend_from_slice(self.merkle_root.as_bytes());
        bytes.extend_from_slice(self.state_root.as_bytes());
        bytes.extend_from_slice(self.validator_set_hash.as_bytes());
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());
        bytes.extend_from_slice(self.proposer.as_bytes());
        bytes
    }

    /// Proposer signs the header.
    pub fn sign(&mut self, keypair: &KeyPair) -> Result<(), PrimitiveError> {
        if Address::from_pubkey(&keypair.public_key()) != self.proposer {
            return Err(PrimitiveError::AddressMismatch);
        }
        let data = self.signing_bytes();
        self.signature = keypair.sign(&data);
        Ok(())
    }

    /// Calculate the canonical block header hash.
    pub fn header_hash(&self) -> Hash {
        let mut raw = self.signing_bytes();
        raw.extend_from_slice(self.signature.as_bytes());
        hash_with_domain(DOMAIN_BLOCK_HEADER, &raw)
    }

    /// Verify signature with proposer's public key.
    pub fn verify_signature(&self, pubkey: &PublicKey) -> Result<(), PrimitiveError> {
        if Address::from_pubkey(pubkey) != self.proposer {
            return Err(PrimitiveError::AddressMismatch);
        }
        let data = self.signing_bytes();
        pubkey
            .verify(&data, &self.signature)
            .map_err(|_| PrimitiveError::InvalidSignature)
    }
}
