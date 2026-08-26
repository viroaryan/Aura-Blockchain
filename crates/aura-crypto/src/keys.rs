use std::fmt;
use std::str::FromStr;
use ed25519_dalek::{
    Signature as DalekSignature, Signer, SigningKey, Verifier, VerifyingKey,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CryptoError {
    #[error("invalid key length")]
    InvalidKeyLength,
    #[error("invalid signature length")]
    InvalidSignatureLength,
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),
    #[error("hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}

/// 32-byte Ed25519 Public Key.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
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

    pub fn to_verifying_key(&self) -> Result<VerifyingKey, CryptoError> {
        VerifyingKey::from_bytes(&self.0)
            .map_err(|e| CryptoError::InvalidPublicKey(e.to_string()))
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        let vk = self.to_verifying_key()?;
        let dalek_sig = DalekSignature::from_bytes(&signature.0);
        vk.verify_strict(message, &dalek_sig)
            .map_err(|_| CryptoError::SignatureVerificationFailed)
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for PublicKey {
    fn from(arr: [u8; 32]) -> Self {
        PublicKey(arr)
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", hex::encode(self.0))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for PublicKey {
    type Err = CryptoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.strip_prefix("0x").unwrap_or(s);
        if clean.len() != 64 {
            return Err(CryptoError::InvalidKeyLength);
        }
        let bytes = hex::decode(clean)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(PublicKey(arr))
    }
}

impl Serialize for PublicKey {
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

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            PublicKey::from_str(&s).map_err(serde::de::Error::custom)
        } else {
            let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
            if bytes.len() != 32 {
                return Err(serde::de::Error::custom("expected 32 bytes for PublicKey"));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Ok(PublicKey(arr))
        }
    }
}

/// 64-byte Ed25519 Signature.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signature([u8; 64]);

impl Signature {
    pub const fn from_bytes(bytes: [u8; 64]) -> Self {
        Signature(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 64]> for Signature {
    fn from(arr: [u8; 64]) -> Self {
        Signature(arr)
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", hex::encode(self.0))
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for Signature {
    type Err = CryptoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.strip_prefix("0x").unwrap_or(s);
        if clean.len() != 128 {
            return Err(CryptoError::InvalidSignatureLength);
        }
        let bytes = hex::decode(clean)?;
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Signature(arr))
    }
}

impl Serialize for Signature {
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

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            Signature::from_str(&s).map_err(serde::de::Error::custom)
        } else {
            let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
            if bytes.len() != 64 {
                return Err(serde::de::Error::custom("expected 64 bytes for Signature"));
            }
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&bytes);
            Ok(Signature(arr))
        }
    }
}

/// Ed25519 KeyPair for signing.
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    public_key: PublicKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let public_key = PublicKey(signing_key.verifying_key().to_bytes());
        KeyPair {
            signing_key,
            public_key,
        }
    }

    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let public_key = PublicKey(signing_key.verifying_key().to_bytes());
        KeyPair {
            signing_key,
            public_key,
        }
    }

    pub fn public_key(&self) -> PublicKey {
        self.public_key
    }

    pub fn secret_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        let dalek_sig = self.signing_key.sign(message);
        Signature(dalek_sig.to_bytes())
    }
}
