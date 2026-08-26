use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum HashError {
    #[error("invalid hex string length: expected 64 characters")]
    InvalidLength,
    #[error("hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}

/// A 32-byte cryptographic hash value (Blake3 based).
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash([u8; 32]);

pub const DOMAIN_TX: &[u8] = b"AURA_TX_v1";
pub const DOMAIN_BLOCK_HEADER: &[u8] = b"AURA_HEADER_v1";
pub const DOMAIN_MERKLE_LEAF: &[u8] = b"AURA_LEAF_v1";
pub const DOMAIN_MERKLE_NODE: &[u8] = b"AURA_NODE_v1";
pub const DOMAIN_SMT_LEAF: &[u8] = b"AURA_SMT_LEAF_v1";
pub const DOMAIN_SMT_NODE: &[u8] = b"AURA_SMT_NODE_v1";
pub const DOMAIN_VOTE: &[u8] = b"AURA_VOTE_v1";

impl Hash {
    pub const ZERO: Hash = Hash([0u8; 32]);

    pub const fn new(bytes: [u8; 32]) -> Self {
        Hash(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HashError> {
        if bytes.len() != 32 {
            return Err(HashError::InvalidLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Hash(arr))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for Hash {
    fn from(arr: [u8; 32]) -> Self {
        Hash(arr)
    }
}

impl From<Hash> for [u8; 32] {
    fn from(h: Hash) -> Self {
        h.0
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", hex::encode(self.0))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for Hash {
    type Err = HashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.strip_prefix("0x").unwrap_or(s);
        if clean.len() != 64 {
            return Err(HashError::InvalidLength);
        }
        let bytes = hex::decode(clean)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_hex())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Hash::from_str(&s).map_err(serde::de::Error::custom)
        } else {
            let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
            if bytes.len() != 32 {
                return Err(serde::de::Error::custom("expected 32 bytes for Hash"));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Ok(Hash(arr))
        }
    }
}

/// Compute standard BLAKE3 hash over bytes.
pub fn hash_bytes(data: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    let out = hasher.finalize();
    Hash(*out.as_bytes())
}

/// Compute domain-separated BLAKE3 hash.
pub fn hash_with_domain(domain: &[u8], data: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(domain.len() as u64).to_le_bytes());
    hasher.update(domain);
    hasher.update(data);
    let out = hasher.finalize();
    Hash(*out.as_bytes())
}

/// Compute binary internal node hash with domain separation.
pub fn hash_two(domain: &[u8], left: &Hash, right: &Hash) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(domain.len() as u64).to_le_bytes());
    hasher.update(domain);
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    let out = hasher.finalize();
    Hash(*out.as_bytes())
}
