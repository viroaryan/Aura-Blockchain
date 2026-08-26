use aura_crypto::PublicKey;
use serde::{Deserialize, Serialize};

/// Account state representing balance, nonce, staking and validator info.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Account {
    pub balance: u64,
    pub nonce: u64,
    pub staked_amount: u64,
    pub is_validator: bool,
    pub validator_pubkey: Option<PublicKey>,
}

impl Account {
    pub fn new(balance: u64, nonce: u64) -> Self {
        Self {
            balance,
            nonce,
            staked_amount: 0,
            is_validator: false,
            validator_pubkey: None,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("account serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
