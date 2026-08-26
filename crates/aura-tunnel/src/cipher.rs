use aura_crypto::{hash_with_domain, Hash};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DOMAIN_AEAD_STREAM: &[u8] = b"AURA_TUNNEL_STREAM_v1";
const DOMAIN_AEAD_TAG: &[u8] = b"AURA_TUNNEL_TAG_v1";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CipherError {
    #[error("authentication tag mismatch: packet corrupted or tampered")]
    TagMismatch,
    #[error("packet sequence out of order or replayed: expected {expected}, got {got}")]
    ReplayDetected { expected: u64, got: u64 },
    #[error("packet payload too large")]
    PayloadTooLarge,
}

/// End-to-End Authenticated Cipher Session for P2P data streaming.
pub struct TunnelCipher {
    session_key: [u8; 32],
    tx_seq: u64,
    rx_seq: u64,
}

impl TunnelCipher {
    pub fn new(session_key: [u8; 32]) -> Self {
        Self {
            session_key,
            tx_seq: 0,
            rx_seq: 0,
        }
    }

    /// Encrypt a payload into an authenticated ciphertext frame.
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let seq = self.tx_seq;
        self.tx_seq += 1;

        // Derive keystream block: BLAKE3(DOMAIN || session_key || seq)
        let mut key_data = Vec::with_capacity(32 + 8);
        key_data.extend_from_slice(&self.session_key);
        key_data.extend_from_slice(&seq.to_be_bytes());
        let stream_key = hash_with_domain(DOMAIN_AEAD_STREAM, &key_data);

        // Stream XOR encryption
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        let key_bytes = stream_key.as_bytes();
        for (i, &b) in plaintext.iter().enumerate() {
            ciphertext.push(b ^ key_bytes[i % 32]);
        }

        // Compute 32-byte Auth Tag: BLAKE3(TAG_DOMAIN || session_key || seq || ciphertext)
        let mut tag_data = Vec::with_capacity(32 + 8 + ciphertext.len());
        tag_data.extend_from_slice(&self.session_key);
        tag_data.extend_from_slice(&seq.to_be_bytes());
        tag_data.extend_from_slice(&ciphertext);
        let tag = hash_with_domain(DOMAIN_AEAD_TAG, &tag_data);

        // Frame: seq (8 bytes) + tag (32 bytes) + ciphertext
        let mut frame = Vec::with_capacity(8 + 32 + ciphertext.len());
        frame.extend_from_slice(&seq.to_be_bytes());
        frame.extend_from_slice(tag.as_bytes());
        frame.extend_from_slice(&ciphertext);
        frame
    }

    /// Decrypt and authenticate an incoming frame.
    pub fn decrypt(&mut self, frame: &[u8]) -> Result<Vec<u8>, CipherError> {
        if frame.len() < 40 {
            return Err(CipherError::TagMismatch);
        }

        let mut seq_bytes = [0u8; 8];
        seq_bytes.copy_from_slice(&frame[0..8]);
        let seq = u64::from_be_bytes(seq_bytes);

        // Anti-replay check
        if seq < self.rx_seq {
            return Err(CipherError::ReplayDetected {
                expected: self.rx_seq,
                got: seq,
            });
        }
        self.rx_seq = seq + 1;

        let tag_bytes = &frame[8..40];
        let ciphertext = &frame[40..];

        // Verify Tag
        let mut tag_data = Vec::with_capacity(32 + 8 + ciphertext.len());
        tag_data.extend_from_slice(&self.session_key);
        tag_data.extend_from_slice(&seq.to_be_bytes());
        tag_data.extend_from_slice(ciphertext);
        let expected_tag = hash_with_domain(DOMAIN_AEAD_TAG, &tag_data);

        if expected_tag.as_bytes() != tag_bytes {
            return Err(CipherError::TagMismatch);
        }

        // Stream XOR decryption
        let mut key_data = Vec::with_capacity(32 + 8);
        key_data.extend_from_slice(&self.session_key);
        key_data.extend_from_slice(&seq.to_be_bytes());
        let stream_key = hash_with_domain(DOMAIN_AEAD_STREAM, &key_data);

        let mut plaintext = Vec::with_capacity(ciphertext.len());
        let key_bytes = stream_key.as_bytes();
        for (i, &b) in ciphertext.iter().enumerate() {
            plaintext.push(b ^ key_bytes[i % 32]);
        }

        Ok(plaintext)
    }
}
