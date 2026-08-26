use aura_crypto::Hash;
use aura_consensus::ConsensusMessage;
use aura_primitives::{Block, BlockHeader, MerkleProof, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMessage {
    TxGossip(Transaction),
    BlockGossip(Block),
    Consensus(ConsensusMessage),
    StatusRequest {
        best_height: u64,
    },
    StatusResponse {
        best_height: u64,
        best_hash: Hash,
        state_root: Hash,
    },
    BlockRequest {
        start_height: u64,
        max_blocks: usize,
    },
    BlockResponse {
        blocks: Vec<Block>,
    },
    MerkleProofRequest {
        block_height: u64,
        leaf_index: usize,
    },
    MerkleProofResponse {
        block_hash: Hash,
        proof: Option<MerkleProof>,
    },
}
