use std::collections::HashMap;
use std::net::SocketAddr;
use aura_crypto::Hash;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub [u8; 32]);

impl PeerId {
    pub fn random() -> Self {
        let mut arr = [0u8; 32];
        for b in &mut arr {
            *b = rand::random();
        }
        PeerId(arr)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub addr: Option<SocketAddr>,
    pub best_height: u64,
    pub best_hash: Hash,
    pub reputation_score: i32,
    pub is_banned: bool,
}

pub struct PeerManager {
    peers: RwLock<HashMap<PeerId, PeerInfo>>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
        }
    }

    pub fn add_peer(&self, peer: PeerInfo) {
        self.peers.write().insert(peer.id, peer);
    }

    pub fn remove_peer(&self, id: &PeerId) {
        self.peers.write().remove(id);
    }

    pub fn update_status(&self, id: &PeerId, height: u64, hash: Hash) {
        if let Some(peer) = self.peers.write().get_mut(id) {
            peer.best_height = height;
            peer.best_hash = hash;
        }
    }

    pub fn penalize(&self, id: &PeerId, penalty: i32) {
        let mut guard = self.peers.write();
        if let Some(peer) = guard.get_mut(id) {
            peer.reputation_score -= penalty;
            if peer.reputation_score < -100 {
                peer.is_banned = true;
            }
        }
    }

    pub fn reward(&self, id: &PeerId, bonus: i32) {
        let mut guard = self.peers.write();
        if let Some(peer) = guard.get_mut(id) {
            peer.reputation_score = (peer.reputation_score + bonus).min(100);
        }
    }

    pub fn get_highest_peer(&self) -> Option<PeerInfo> {
        let guard = self.peers.read();
        guard
            .values()
            .filter(|p| !p.is_banned)
            .max_by_key(|p| p.best_height)
            .cloned()
    }

    pub fn active_peer_count(&self) -> usize {
        self.peers.read().values().filter(|p| !p.is_banned).count()
    }
}
