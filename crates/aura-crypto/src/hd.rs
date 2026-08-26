use bip39::{Language, Mnemonic, MnemonicType, Seed};
use hmac::{Hmac, Mac};
use sha2::Sha512;
use thiserror::Error;

use crate::keys::KeyPair;

type HmacSha512 = Hmac<Sha512>;

pub const AURA_COIN_TYPE: u32 = 1234; // Registered coin type for Aura
const ED25519_KEY: &[u8] = b"ed25519 seed";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum HdWalletError {
    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("invalid derivation path: {0}")]
    InvalidDerivationPath(String),
    #[error("hmac error")]
    HmacError,
}

pub struct HdWallet;

impl HdWallet {
    /// Generate a fresh 12-word or 24-word BIP-39 mnemonic phrase.
    pub fn generate_mnemonic(word_count: usize) -> Result<String, HdWalletError> {
        let mtype = match word_count {
            12 => MnemonicType::Words12,
            24 => MnemonicType::Words24,
            _ => {
                return Err(HdWalletError::InvalidMnemonic(
                    "supported word counts are 12 and 24".into(),
                ))
            }
        };
        let mnemonic = Mnemonic::new(mtype, Language::English);
        Ok(mnemonic.into_phrase())
    }

    /// Derive master key & chain code using SLIP-0010 from a BIP-39 seed.
    pub fn seed_from_mnemonic(mnemonic_str: &str, passphrase: &str) -> Result<[u8; 64], HdWalletError> {
        let mnemonic = Mnemonic::from_phrase(mnemonic_str, Language::English)
            .map_err(|e| HdWalletError::InvalidMnemonic(e.to_string()))?;
        let seed = Seed::new(&mnemonic, passphrase);
        let mut out = [0u8; 64];
        out.copy_from_slice(seed.as_bytes());
        Ok(out)
    }

    /// Derive an Ed25519 KeyPair for path m/44'/1234'/account'/change'/index' using SLIP-0010.
    pub fn derive_keypair(
        seed: &[u8; 64],
        account: u32,
        change: u32,
        index: u32,
    ) -> Result<KeyPair, HdWalletError> {
        // Master key derivation: HMAC-SHA512(Key = "ed25519 seed", Data = Seed)
        let mut mac = HmacSha512::new_from_slice(ED25519_KEY)
            .map_err(|_| HdWalletError::HmacError)?;
        mac.update(seed);
        let master = mac.finalize().into_bytes();

        let mut key = [0u8; 32];
        let mut chain_code = [0u8; 32];
        key.copy_from_slice(&master[0..32]);
        chain_code.copy_from_slice(&master[32..64]);

        // SLIP-0010 hardened path: 44' / 1234' / account' / change' / index'
        let path = [
            44 | 0x8000_0000,
            AURA_COIN_TYPE | 0x8000_0000,
            account | 0x8000_0000,
            change | 0x8000_0000,
            index | 0x8000_0000,
        ];

        for &child_index in &path {
            let mut data = Vec::with_capacity(37);
            data.push(0x00);
            data.extend_from_slice(&key);
            data.extend_from_slice(&child_index.to_be_bytes());

            let mut child_mac = HmacSha512::new_from_slice(&chain_code)
                .map_err(|_| HdWalletError::HmacError)?;
            child_mac.update(&data);
            let child_out = child_mac.finalize().into_bytes();

            key.copy_from_slice(&child_out[0..32]);
            chain_code.copy_from_slice(&child_out[32..64]);
        }

        Ok(KeyPair::from_secret_bytes(&key))
    }
}
