use aura_crypto::{
    hash_with_domain, Address, CryptoError, Hash, KeyPair, PublicKey, Signature, DOMAIN_TX,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PrimitiveError {
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("invalid transaction signature")]
    InvalidSignature,
    #[error("sender address does not match public key")]
    AddressMismatch,
    #[error("amount overflow or zero amount")]
    InvalidAmount,
    #[error("insufficient fee: minimum fee required")]
    InsufficientFee,
    #[error("merkle root mismatch: expected {expected}, calculated {calculated}")]
    MerkleRootMismatch { expected: Hash, calculated: Hash },
    #[error("invalid block height: expected {expected}, found {found}")]
    InvalidBlockHeight { expected: u64, found: u64 },
    #[error("invalid prev hash: expected {expected}, found {found}")]
    InvalidPrevHash { expected: Hash, found: Hash },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer,
    Stake,
    Unstake,
    RegisterValidator,
}

/// An account-based Aura transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub sender: Address,
    pub recipient: Address,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub tx_type: TransactionType,
    pub payload: Vec<u8>,
    pub pubkey: PublicKey,
    pub signature: Signature,
}

impl Transaction {
    /// Construct an unsigned transaction payload.
    pub fn new_unsigned(
        sender: Address,
        recipient: Address,
        amount: u64,
        fee: u64,
        nonce: u64,
        tx_type: TransactionType,
        payload: Vec<u8>,
        pubkey: PublicKey,
    ) -> Self {
        Self {
            sender,
            recipient,
            amount,
            fee,
            nonce,
            tx_type,
            payload,
            pubkey,
            signature: Signature::from_bytes([0u8; 64]),
        }
    }

    /// Bytes to be signed by the sender.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.sender.as_bytes());
        bytes.extend_from_slice(self.recipient.as_bytes());
        bytes.extend_from_slice(&self.amount.to_be_bytes());
        bytes.extend_from_slice(&self.fee.to_be_bytes());
        bytes.extend_from_slice(&self.nonce.to_be_bytes());
        let type_byte = match self.tx_type {
            TransactionType::Transfer => 0u8,
            TransactionType::Stake => 1u8,
            TransactionType::Unstake => 2u8,
            TransactionType::RegisterValidator => 3u8,
        };
        bytes.push(type_byte);
        bytes.extend_from_slice(&(self.payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes.extend_from_slice(self.pubkey.as_bytes());
        bytes
    }

    /// Sign the transaction using sender's keypair.
    pub fn sign(mut self, keypair: &KeyPair) -> Result<Self, PrimitiveError> {
        if Address::from_pubkey(&keypair.public_key()) != self.sender {
            return Err(PrimitiveError::AddressMismatch);
        }
        self.pubkey = keypair.public_key();
        let signing_data = self.signing_bytes();
        self.signature = keypair.sign(&signing_data);
        Ok(self)
    }

    /// Calculate the unique BLAKE3 transaction hash.
    pub fn tx_hash(&self) -> Hash {
        let mut raw = self.signing_bytes();
        raw.extend_from_slice(self.signature.as_bytes());
        hash_with_domain(DOMAIN_TX, &raw)
    }

    /// Verify signature and sender address mapping.
    pub fn verify(&self) -> Result<(), PrimitiveError> {
        if Address::from_pubkey(&self.pubkey) != self.sender {
            return Err(PrimitiveError::AddressMismatch);
        }
        let signing_data = self.signing_bytes();
        self.pubkey
            .verify(&signing_data, &self.signature)
            .map_err(|_| PrimitiveError::InvalidSignature)
    }

    /// Estimated serialized size in bytes.
    pub fn size_bytes(&self) -> usize {
        // sender (20) + recipient (20) + amount (8) + fee (8) + nonce (8) + type (1) + payload_len (4) + payload + pubkey (32) + sig (64)
        165 + self.payload.len()
    }

    /// Fee density for priority mempool ranking.
    pub fn fee_per_byte(&self) -> u64 {
        let size = self.size_bytes();
        if size == 0 {
            0
        } else {
            self.fee / (size as u64)
        }
    }
}
