use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use aura_crypto::{Address, Hash};
use aura_primitives::Transaction;
use aura_storage::StateDB;
use parking_lot::RwLock;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::validation::{MempoolValidationError, TransactionValidator};

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MempoolError {
    #[error("validation error: {0}")]
    Validation(#[from] MempoolValidationError),
    #[error("transaction already exists in mempool: {0}")]
    Duplicate(Hash),
    #[error("mempool full and transaction fee too low to evict")]
    PoolFull,
    #[error("insufficient fee bump for RBF (required at least +{min_percent}%)")]
    InsufficientRbfBump { min_percent: u32 },
}

#[derive(Debug, Clone)]
pub struct MempoolConfig {
    pub max_transactions: usize,
    pub max_bytes: usize,
    pub min_rbf_bump_percent: u32,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_transactions: 10_000,
            max_bytes: 10 * 1024 * 1024, // 10 MB
            min_rbf_bump_percent: 10,    // 10%
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PriorityKey {
    fee_per_byte: u64,
    nonce: u64,
    tx_hash: Hash,
}

/// Thread-safe fee-prioritized transaction mempool.
pub struct Mempool {
    config: MempoolConfig,
    by_hash: RwLock<HashMap<Hash, Transaction>>,
    by_sender: RwLock<HashMap<Address, BTreeMap<u64, Transaction>>>,
    priority_index: RwLock<BTreeSet<PriorityKey>>,
    total_bytes: RwLock<usize>,
}

impl Mempool {
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            config,
            by_hash: RwLock::new(HashMap::new()),
            by_sender: RwLock::new(HashMap::new()),
            priority_index: RwLock::new(BTreeSet::new()),
            total_bytes: RwLock::new(0),
        }
    }

    /// Add a signed transaction into the mempool.
    pub fn add_transaction(
        &self,
        tx: Transaction,
        state_db: &StateDB,
    ) -> Result<Hash, MempoolError> {
        // Stage 1: Stateless Validation
        TransactionValidator::validate_stateless(&tx)?;

        let tx_hash = tx.tx_hash();
        let tx_size = tx.size_bytes();

        // Duplicate check
        if self.by_hash.read().contains_key(&tx_hash) {
            return Err(MempoolError::Duplicate(tx_hash));
        }

        // Stage 2: Stateful Validation against account
        let account = state_db.get_account(&tx.sender);
        TransactionValidator::validate_stateful(&tx, &account)?;

        // RBF / Nonce collision check
        {
            let mut by_sender = self.by_sender.write();
            let sender_txs = by_sender.entry(tx.sender).or_default();

            if let Some(existing_tx) = sender_txs.get(&tx.nonce) {
                // Check Replace-by-Fee
                let min_required_fee = existing_tx.fee + (existing_tx.fee * self.config.min_rbf_bump_percent as u64) / 100;
                if tx.fee < min_required_fee {
                    return Err(MempoolError::InsufficientRbfBump {
                        min_percent: self.config.min_rbf_bump_percent,
                    });
                }

                // Evict old transaction
                let old_hash = existing_tx.tx_hash();
                let old_fee_per_byte = existing_tx.fee_per_byte();
                let old_nonce = existing_tx.nonce;
                let old_size = existing_tx.size_bytes();

                self.by_hash.write().remove(&old_hash);
                self.priority_index.write().remove(&PriorityKey {
                    fee_per_byte: old_fee_per_byte,
                    nonce: old_nonce,
                    tx_hash: old_hash,
                });
                *self.total_bytes.write() = self.total_bytes.read().saturating_sub(old_size);
                sender_txs.remove(&tx.nonce);
            }
        }

        // Anti-DoS capacity check & eviction
        while self.by_hash.read().len() >= self.config.max_transactions
            || *self.total_bytes.read() + tx_size > self.config.max_bytes
        {
            let lowest_key = {
                let prio = self.priority_index.read();
                prio.iter().next().cloned()
            };

            if let Some(key) = lowest_key {
                if key.fee_per_byte >= tx.fee_per_byte() {
                    return Err(MempoolError::PoolFull);
                }
                self.evict_transaction(&key.tx_hash);
            } else {
                return Err(MempoolError::PoolFull);
            }
        }

        // Insert new transaction
        let priority_key = PriorityKey {
            fee_per_byte: tx.fee_per_byte(),
            nonce: tx.nonce,
            tx_hash,
        };

        self.by_hash.write().insert(tx_hash, tx.clone());
        self.by_sender
            .write()
            .entry(tx.sender)
            .or_default()
            .insert(tx.nonce, tx.clone());
        self.priority_index.write().insert(priority_key);
        *self.total_bytes.write() += tx_size;

        debug!(hash = %tx_hash, sender = %tx.sender, "Transaction accepted into mempool");
        Ok(tx_hash)
    }

    /// Harvest top-priority transactions for block inclusion up to `max_bytes`.
    pub fn harvest(&self, max_bytes: usize) -> Vec<Transaction> {
        let mut harvested = Vec::new();
        let mut current_bytes = 0;

        let prio_guard = self.priority_index.read();
        let by_hash = self.by_hash.read();

        // Iterate descending by fee-per-byte
        for key in prio_guard.iter().rev() {
            if let Some(tx) = by_hash.get(&key.tx_hash) {
                let tx_size = tx.size_bytes();
                if current_bytes + tx_size <= max_bytes {
                    harvested.push(tx.clone());
                    current_bytes += tx_size;
                }
            }
        }

        harvested
    }

    /// Remove committed transactions after block finalization.
    pub fn remove_committed(&self, txs: &[Transaction]) {
        for tx in txs {
            let hash = tx.tx_hash();
            self.evict_transaction(&hash);
        }
    }

    /// Evict a transaction by hash.
    pub fn evict_transaction(&self, tx_hash: &Hash) -> Option<Transaction> {
        let tx = self.by_hash.write().remove(tx_hash)?;
        let size = tx.size_bytes();

        let key = PriorityKey {
            fee_per_byte: tx.fee_per_byte(),
            nonce: tx.nonce,
            tx_hash: *tx_hash,
        };
        self.priority_index.write().remove(&key);

        if let Some(sender_map) = self.by_sender.write().get_mut(&tx.sender) {
            sender_map.remove(&tx.nonce);
        }

        *self.total_bytes.write() = self.total_bytes.read().saturating_sub(size);
        Some(tx)
    }

    /// Get transaction by hash.
    pub fn get_transaction(&self, hash: &Hash) -> Option<Transaction> {
        self.by_hash.read().get(hash).cloned()
    }

    pub fn len(&self) -> usize {
        self.by_hash.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn total_bytes(&self) -> usize {
        *self.total_bytes.read()
    }
}
