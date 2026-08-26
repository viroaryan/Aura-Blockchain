use std::fmt;
use std::str::FromStr;
use bech32::{Bech32, Hrp};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::hash::{hash_with_domain, Hash};
use crate::keys::PublicKey;

pub const DEFAULT_HRP: &str = "aura";
const DOMAIN_ADDRESS: &[u8] = b"AURA_ADDRESS_v1";

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AddressError {
    #[error("invalid address length: expected 20 bytes payload")]
    InvalidPayloadLength,
    #[error("invalid bech32 encoding: {0}")]
    Bech32Error(String),
    #[error("invalid prefix: expected '{expected}', found '{found}'")]
    InvalidPrefix { expected: String, found: String },
}

/// 20-byte Account Address derived from PublicKey via BLAKE3 domain hash.
#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address([u8; 20]);

impl Address {
    pub const ZERO: Address = Address([0u8; 20]);

    pub const fn from_bytes(bytes: [u8; 20]) -> Self {
        Address(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        self.0
    }

    /// Derive an address from a PublicKey using domain-separated BLAKE3 truncated to 20 bytes.
    pub fn from_pubkey(pubkey: &PublicKey) -> Self {
        let h = hash_with_domain(DOMAIN_ADDRESS, pubkey.as_bytes());
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&h.as_bytes()[0..20]);
        Address(arr)
    }

    /// Encode address as Bech32 string with given HRP (default "aura").
    pub fn to_bech32(&self, hrp_str: &str) -> Result<String, AddressError> {
        let hrp = Hrp::parse(hrp_str)
            .map_err(|e| AddressError::Bech32Error(e.to_string()))?;
        bech32::encode::<Bech32>(hrp, &self.0)
            .map_err(|e| AddressError::Bech32Error(e.to_string()))
    }

    /// Decode Bech32 string into Address with prefix verification.
    pub fn from_bech32(s: &str, expected_hrp: &str) -> Result<Self, AddressError> {
        let (hrp, data) = bech32::decode(s)
            .map_err(|e| AddressError::Bech32Error(e.to_string()))?;
        
        if hrp.as_str() != expected_hrp {
            return Err(AddressError::InvalidPrefix {
                expected: expected_hrp.to_string(),
                found: hrp.as_str().to_string(),
            });
        }

        if data.len() != 20 {
            return Err(AddressError::InvalidPayloadLength);
        }

        let mut arr = [0u8; 20];
        arr.copy_from_slice(&data);
        Ok(Address(arr))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 20]> for Address {
    fn from(arr: [u8; 20]) -> Self {
        Address(arr)
    }
}

impl From<Address> for [u8; 20] {
    fn from(a: Address) -> Self {
        a.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bech = self.to_bech32(DEFAULT_HRP).unwrap_or_else(|_| hex::encode(self.0));
        write!(f, "Address({})", bech)
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bech = self.to_bech32(DEFAULT_HRP).unwrap_or_else(|_| hex::encode(self.0));
        write!(f, "{}", bech)
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with(DEFAULT_HRP) {
            Address::from_bech32(s, DEFAULT_HRP)
        } else {
            let clean = s.strip_prefix("0x").unwrap_or(s);
            if clean.len() != 40 {
                return Err(AddressError::InvalidPayloadLength);
            }
            let bytes = hex::decode(clean).map_err(|e| AddressError::Bech32Error(e.to_string()))?;
            let mut arr = [0u8; 20];
            arr.copy_from_slice(&bytes);
            Ok(Address(arr))
        }
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let s = self.to_bech32(DEFAULT_HRP)
                .map_err(serde::ser::Error::custom)?;
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Address::from_str(&s).map_err(serde::de::Error::custom)
        } else {
            let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
            if bytes.len() != 20 {
                return Err(serde::de::Error::custom("expected 20 bytes for Address"));
            }
            let mut arr = [0u8; 20];
            arr.copy_from_slice(&bytes);
            Ok(Address(arr))
        }
    }
}
