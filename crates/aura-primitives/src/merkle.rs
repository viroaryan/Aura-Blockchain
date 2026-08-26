use aura_crypto::{hash_with_domain, Hash, DOMAIN_MERKLE_LEAF, DOMAIN_MERKLE_NODE};
use serde::{Deserialize, Serialize};

/// RFC 6962 compliant Merkle Proof component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub total_leaves: usize,
    /// Vector of (sibling_hash, is_sibling_right)
    pub audit_path: Vec<(Hash, bool)>,
}

impl MerkleProof {
    /// Verify that `leaf_data` is included at `leaf_index` in the tree with root `expected_root`.
    pub fn verify(&self, expected_root: &Hash, leaf_data: &[u8]) -> bool {
        let mut current_hash = hash_leaf(leaf_data);

        for (sibling_hash, is_sibling_right) in &self.audit_path {
            current_hash = if *is_sibling_right {
                hash_internal_node(&current_hash, sibling_hash)
            } else {
                hash_internal_node(sibling_hash, &current_hash)
            };
        }

        &current_hash == expected_root
    }
}

/// Compute RFC 6962 leaf hash: BLAKE3(DOMAIN_MERKLE_LEAF || 0x00 || data)
pub fn hash_leaf(data: &[u8]) -> Hash {
    let mut payload = Vec::with_capacity(1 + data.len());
    payload.push(0x00);
    payload.extend_from_slice(data);
    hash_with_domain(DOMAIN_MERKLE_LEAF, &payload)
}

/// Compute RFC 6962 internal node hash: BLAKE3(DOMAIN_MERKLE_NODE || 0x01 || left || right)
pub fn hash_internal_node(left: &Hash, right: &Hash) -> Hash {
    let mut payload = Vec::with_capacity(1 + 64);
    payload.push(0x01);
    payload.extend_from_slice(left.as_bytes());
    payload.extend_from_slice(right.as_bytes());
    hash_with_domain(DOMAIN_MERKLE_NODE, &payload)
}

/// A complete RFC 6962 Binary Merkle Tree.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<Hash>,
    levels: Vec<Vec<Hash>>,
}

impl MerkleTree {
    /// Build Merkle tree from raw leaf byte slices.
    pub fn from_leaf_payloads(payloads: &[Vec<u8>]) -> Self {
        let leaf_hashes: Vec<Hash> = payloads.iter().map(|p| hash_leaf(p)).collect();
        Self::from_leaf_hashes(leaf_hashes)
    }

    /// Build Merkle tree from precomputed leaf hashes.
    pub fn from_leaf_hashes(leaves: Vec<Hash>) -> Self {
        if leaves.is_empty() {
            return Self {
                leaves: vec![],
                levels: vec![vec![Hash::ZERO]],
            };
        }

        let mut levels = Vec::new();
        levels.push(leaves.clone());

        let mut current_level = leaves.clone();
        while current_level.len() > 1 {
            let mut next_level = Vec::with_capacity((current_level.len() + 1) / 2);
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    next_level.push(hash_internal_node(&chunk[0], &chunk[1]));
                } else {
                    // RFC 6962 rule: duplicate odd leaf or promote (promote odd element)
                    next_level.push(hash_internal_node(&chunk[0], &chunk[0]));
                }
            }
            levels.push(next_level.clone());
            current_level = next_level;
        }

        Self { leaves, levels }
    }

    /// Returns the root hash of the tree.
    pub fn root(&self) -> Hash {
        self.levels
            .last()
            .and_then(|lvl| lvl.first().copied())
            .unwrap_or(Hash::ZERO)
    }

    /// Generate an inclusion proof for a leaf at `leaf_index`.
    pub fn generate_proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return None;
        }

        let mut audit_path = Vec::new();
        let mut index = leaf_index;

        for level in &self.levels[0..self.levels.len() - 1] {
            let is_right = index % 2 == 0;
            let sibling_index = if is_right {
                if index + 1 < level.len() {
                    index + 1
                } else {
                    index // Sibling is self if odd
                }
            } else {
                index - 1
            };

            let sibling_hash = level[sibling_index];
            audit_path.push((sibling_hash, is_right));
            index /= 2;
        }

        Some(MerkleProof {
            leaf_index,
            total_leaves: self.leaves.len(),
            audit_path,
        })
    }
}
