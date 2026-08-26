use std::sync::Arc;
use aura_crypto::Hash;
use aura_primitives::{Block, MerkleProof, PrimitiveError, Transaction, TransactionType};
use aura_storage::{StateDB, StateOverlay, StorageError};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("primitive error: {0}")]
    Primitive(#[from] PrimitiveError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("invalid block link: height {height} prev_hash mismatch")]
    InvalidBlockLink { height: u64 },
    #[error("invalid state root after block application")]
    InvalidStateRoot,
}

pub struct ChainSyncer {
    state_db: Arc<StateDB>,
}

impl ChainSyncer {
    pub fn new(state_db: Arc<StateDB>) -> Self {
        Self { state_db }
    }

    /// Process and apply a batch of downloaded blocks from peers.
    pub fn apply_block_batch(&self, blocks: &[Block]) -> Result<u64, SyncError> {
        let mut latest_height = self.state_db.get_latest_height();

        for block in blocks {
            let height = block.header.height;
            if height != latest_height + 1 {
                continue; // Skip out-of-order blocks
            }

            // Verify basic block integrity
            block.validate_basic()?;

            // Verify parent hash link
            let prev_block = self
                .state_db
                .get_block_by_height(latest_height)
                .expect("previous block must exist");
            if block.header.prev_hash != prev_block.hash() {
                return Err(SyncError::InvalidBlockLink { height });
            }

            // Apply state transitions
            let mut overlay = StateOverlay::new();
            for tx in &block.transactions {
                let mut sender_acc = overlay.get_account(&tx.sender, &self.state_db);
                let total_deduct = tx.amount + tx.fee;
                sender_acc.balance = sender_acc.balance.saturating_sub(total_deduct);
                sender_acc.nonce += 1;

                match tx.tx_type {
                    TransactionType::Transfer => {
                        let mut recipient_acc =
                            overlay.get_account(&tx.recipient, &self.state_db);
                        recipient_acc.balance =
                            recipient_acc.balance.saturating_add(tx.amount);
                        overlay.set_account(tx.recipient, recipient_acc);
                    }
                    TransactionType::RegisterValidator => {
                        sender_acc.is_validator = true;
                        sender_acc.validator_pubkey = Some(tx.pubkey);
                        sender_acc.staked_amount =
                            sender_acc.staked_amount.saturating_add(tx.amount);
                    }
                    TransactionType::Stake => {
                        sender_acc.staked_amount =
                            sender_acc.staked_amount.saturating_add(tx.amount);
                    }
                    TransactionType::Unstake => {
                        sender_acc.staked_amount =
                            sender_acc.staked_amount.saturating_sub(tx.amount);
                        sender_acc.balance =
                            sender_acc.balance.saturating_add(tx.amount);
                    }
                }

                overlay.set_account(tx.sender, sender_acc);
            }

            self.state_db.commit_block(block, overlay)?;
            latest_height = height;
            info!(height = height, "Synced and applied block from peer");
        }

        Ok(latest_height)
    }

    /// Light client verification: verify a transaction's inclusion in a block without downloading the full block.
    pub fn verify_light_client_proof(
        block_header_merkle_root: &Hash,
        tx: &Transaction,
        proof: &MerkleProof,
    ) -> bool {
        let tx_hash = tx.tx_hash();
        let mut leaf_payload = Vec::new();
        leaf_payload.extend_from_slice(tx_hash.as_bytes());
        // Or hash leaf
        proof.verify(block_header_merkle_root, tx_hash.as_bytes())
    }
}
