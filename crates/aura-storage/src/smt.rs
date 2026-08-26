use std::collections::HashMap;
use aura_crypto::{hash_with_domain, Hash, DOMAIN_SMT_LEAF, DOMAIN_SMT_NODE};
use serde::{Deserialize, Serialize};

pub const SMT_DEPTH: usize = 256;

/// Precomputed default empty hashes for all 256 levels of the Sparse Merkle Tree.
pub struct SmtEmptyRoots {
    roots: [Hash; SMT_DEPTH + 1],
}

impl SmtEmptyRoots {
    pub fn new() -> Self {
        let mut roots = [Hash::ZERO; SMT_DEPTH + 1];
        // Base leaf level 0 is ZERO
        roots[0] = Hash::ZERO;
        for i in 1..=SMT_DEPTH {
            let prev = &roots[i - 1];
            roots[i] = hash_smt_node(prev, prev);
        }
        Self { roots }
    }

    pub fn get(&self, level: usize) -> Hash {
        if level <= SMT_DEPTH {
            self.roots[level]
        } else {
            Hash::ZERO
        }
    }
}

lazy_static_empty_roots!();

macro_rules! lazy_static_empty_roots {
    () => {
        pub fn empty_root(level: usize) -> Hash {
            // Compute or lookup
            EMPTY_ROOTS.get(level)
        }
    };
}

use parking_lot::RwLock;
use std::sync::LazyLock;

static EMPTY_ROOTS: LazyLock<SmtEmptyRoots> = LazyLock::new(SmtEmptyRoots::new);

pub fn hash_smt_leaf(key: &Hash, value: &[u8]) -> Hash {
    let mut payload = Vec::with_capacity(32 + value.len());
    payload.extend_from_slice(key.as_bytes());
    payload.extend_from_slice(value);
    hash_with_domain(DOMAIN_SMT_LEAF, &payload)
}

pub fn hash_smt_node(left: &Hash, right: &Hash) -> Hash {
    let mut payload = Vec::with_capacity(64);
    payload.extend_from_slice(left.as_bytes());
    payload.extend_from_slice(right.as_bytes());
    hash_with_domain(DOMAIN_SMT_NODE, &payload)
}

/// A Sparse Merkle Tree backed by an in-memory or persisted map of updated leaves.
#[derive(Debug, Clone, Default)]
pub struct SparseMerkleTree {
    leaves: HashMap<Hash, Hash>, // key -> leaf_hash
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        Self {
            leaves: HashMap::new(),
        }
    }

    /// Insert or update a key with given value.
    pub fn update(&mut self, key: Hash, value: &[u8]) {
        if value.is_empty() {
            self.leaves.remove(&key);
        } else {
            let leaf_hash = hash_smt_leaf(&key, value);
            self.leaves.insert(key, leaf_hash);
        }
    }

    /// Delete a key from the SMT.
    pub fn delete(&mut self, key: &Hash) {
        self.leaves.remove(key);
    }

    /// Compute the 256-bit Sparse Merkle Tree root deterministically.
    pub fn root(&self) -> Hash {
        if self.leaves.is_empty() {
            return empty_root(SMT_DEPTH);
        }

        // Build tree recursively for non-empty subtrees
        self.compute_subtree_root(&self.leaves, 0)
    }

    fn compute_subtree_root(&self, nodes: &HashMap<Hash, Hash>, depth: usize) -> Hash {
        if nodes.is_empty() {
            return empty_root(SMT_DEPTH - depth);
        }
        if nodes.len() == 1 && depth == SMT_DEPTH {
            let (_, &val) = nodes.iter().next().unwrap();
            return val;
        }
        if depth == SMT_DEPTH {
            let (_, &val) = nodes.iter().next().unwrap();
            return val;
        }

        let mut left_group = HashMap::new();
        let mut right_group = HashMap::new();

        let byte_idx = depth / 8;
        let bit_idx = 7 - (depth % 8);

        for (key, leaf) in nodes {
            let bit = (key.as_bytes()[byte_idx] >> bit_idx) & 1;
            if bit == 0 {
                left_group.insert(*key, *leaf);
            } else {
                right_group.insert(*key, *leaf);
            }
        }

        let left_hash = self.compute_subtree_root(&left_group, depth + 1);
        let right_hash = self.compute_subtree_root(&right_group, depth + 1);

        if left_hash == empty_root(SMT_DEPTH - depth - 1)
            && right_hash == empty_root(SMT_DEPTH - depth - 1)
        {
            empty_root(SMT_DEPTH - depth)
        } else {
            hash_smt_node(&left_hash, &right_hash)
        }
    }
}
