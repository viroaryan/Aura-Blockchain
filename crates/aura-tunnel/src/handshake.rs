use aura_crypto::{hash_with_domain, Hash, KeyPair, PublicKey};
use serde::{Deserialize, Serialize};

const DOMAIN_HANDSHAKE: &[u8] = b"AURA_TUNNEL_HANDSHAKE_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeInit {
    pub client_identity: PublicKey,
    pub client_ephemeral: PublicKey,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub host_identity: PublicKey,
    pub host_ephemeral: PublicKey,
    pub timestamp: u64,
}

pub struct TunnelHandshake;

impl TunnelHandshake {
    /// Derive 32-byte symmetric session key from ephemeral key exchange.
    pub fn derive_session_key(
        client_ephemeral: &PublicKey,
        host_ephemeral: &PublicKey,
    ) -> [u8; 32] {
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(client_ephemeral.as_bytes());
        data.extend_from_slice(host_ephemeral.as_bytes());
        let hash = hash_with_domain(DOMAIN_HANDSHAKE, &data);
        hash.to_bytes()
    }
}
