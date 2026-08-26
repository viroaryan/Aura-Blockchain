use aura_crypto::{Address, Hash, PublicKey, Signature};
use serde::{Deserialize, Serialize};

use crate::block::Block;
use crate::header::BlockHeader;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub address: Address,
    pub balance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisValidator {
    pub address: Address,
    pub pubkey: PublicKey,
    pub stake: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain_id: String,
    pub timestamp: u64,
    pub accounts: Vec<GenesisAccount>,
    pub validators: Vec<GenesisValidator>,
    pub initial_state_root: Hash,
}

impl GenesisConfig {
    /// Generate deterministic validator set hash.
    pub fn validator_set_hash(&self) -> Hash {
        let mut raw = Vec::new();
        for v in &self.validators {
            raw.extend_from_slice(v.address.as_bytes());
            raw.extend_from_slice(v.pubkey.as_bytes());
            raw.extend_from_slice(&v.stake.to_be_bytes());
        }
        aura_crypto::hash_bytes(&raw)
    }

    /// Construct Genesis Block (Height 0, PrevHash = ZERO).
    pub fn to_genesis_block(&self) -> Block {
        let header = BlockHeader {
            version: 1,
            chain_id: self.chain_id.clone(),
            height: 0,
            round: 0,
            prev_hash: Hash::ZERO,
            merkle_root: Hash::ZERO,
            state_root: self.initial_state_root,
            validator_set_hash: self.validator_set_hash(),
            timestamp: self.timestamp,
            proposer: Address::ZERO,
            signature: Signature::from_bytes([0u8; 64]),
        };

        Block::new(header, vec![], None)
    }
}
