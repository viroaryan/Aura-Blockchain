use std::collections::HashMap;
use std::sync::Arc;
use aura_crypto::{hash_with_domain, Address, Hash, DOMAIN_SMT_LEAF};
use aura_primitives::{Block, GenesisConfig, Transaction, TransactionType};
use parking_lot::RwLock;

use crate::account::Account;
use crate::kv::{BatchOp, KeyValueStore, StorageError, CF_ACCOUNTS, CF_BLOCKS, CF_HEIGHTS, CF_META};
use crate::smt::SparseMerkleTree;

/// Ephemeral memory overlay for transaction/block execution.
#[derive(Debug, Clone, Default)]
pub struct StateOverlay {
    pub accounts: HashMap<Address, Account>,
}

impl StateOverlay {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn get_account(&self, address: &Address, base_db: &StateDB) -> Account {
        if let Some(acc) = self.accounts.get(address) {
            acc.clone()
        } else {
            base_db.get_account(address)
        }
    }

    pub fn set_account(&mut self, address: Address, account: Account) {
        self.accounts.insert(address, account);
    }
}

/// StateDB encapsulates the blockchain state, SMT, and KV store.
pub struct StateDB {
    kv: Arc<KeyValueStore>,
    smt: RwLock<SparseMerkleTree>,
}

impl StateDB {
    pub fn new(kv: Arc<KeyValueStore>) -> Self {
        let mut smt = SparseMerkleTree::new();
        // Load all existing accounts into SMT
        let prefix = CF_ACCOUNTS.as_bytes();
        for (k, v) in kv.scan_prefix(prefix) {
            if let Ok(addr_slice) = k[prefix.len()..].try_into() {
                let addr = Address::from_bytes(addr_slice);
                let key_hash = hash_with_domain(DOMAIN_SMT_LEAF, addr.as_bytes());
                smt.update(key_hash, &v);
            }
        }

        Self {
            kv,
            smt: RwLock::new(smt),
        }
    }

    pub fn open_in_memory() -> Self {
        let kv = Arc::new(KeyValueStore::open_in_memory());
        Self::new(kv)
    }

    /// Retrieve an account's state.
    pub fn get_account(&self, address: &Address) -> Account {
        let mut key = CF_ACCOUNTS.as_bytes().to_vec();
        key.extend_from_slice(address.as_bytes());
        if let Some(bytes) = self.kv.get(&key) {
            Account::from_bytes(&bytes).unwrap_or_default()
        } else {
            Account::default()
        }
    }

    /// Get current state root from SMT.
    pub fn get_state_root(&self) -> Hash {
        self.smt.read().root()
    }

    /// Get latest committed block height.
    pub fn get_latest_height(&self) -> u64 {
        let key = format!("{}latest_height", CF_META).into_bytes();
        if let Some(bytes) = self.kv.get(&key) {
            if bytes.len() == 8 {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(&bytes);
                return u64::from_be_bytes(arr);
            }
        }
        0
    }

    /// Get block by height.
    pub fn get_block_by_height(&self, height: u64) -> Option<Block> {
        let mut height_key = CF_HEIGHTS.as_bytes().to_vec();
        height_key.extend_from_slice(&height.to_be_bytes());
        let hash_bytes = self.kv.get(&height_key)?;
        let mut block_key = CF_BLOCKS.as_bytes().to_vec();
        block_key.extend_from_slice(&hash_bytes);
        let block_bytes = self.kv.get(&block_key)?;
        bincode::deserialize(&block_bytes).ok()
    }

    /// Get block by hash.
    pub fn get_block_by_hash(&self, hash: &Hash) -> Option<Block> {
        let mut block_key = CF_BLOCKS.as_bytes().to_vec();
        block_key.extend_from_slice(hash.as_bytes());
        let block_bytes = self.kv.get(&block_key)?;
        bincode::deserialize(&block_bytes).ok()
    }

    /// Initialize state from Genesis Config.
    pub fn apply_genesis(&self, genesis: &GenesisConfig) -> Result<Hash, StorageError> {
        let mut ops = Vec::new();
        let mut smt = self.smt.write();

        for acc in &genesis.accounts {
            let account_obj = Account::new(acc.balance, 0);
            let bytes = account_obj.to_bytes();

            let mut key = CF_ACCOUNTS.as_bytes().to_vec();
            key.extend_from_slice(acc.address.as_bytes());
            ops.push(BatchOp::Put(key, bytes.clone()));

            let key_hash = hash_with_domain(DOMAIN_SMT_LEAF, acc.address.as_bytes());
            smt.update(key_hash, &bytes);
        }

        for val in &genesis.validators {
            let mut acc = self.get_account(&val.address);
            acc.is_validator = true;
            acc.validator_pubkey = Some(val.pubkey);
            acc.staked_amount = val.stake;
            let bytes = acc.to_bytes();

            let mut key = CF_ACCOUNTS.as_bytes().to_vec();
            key.extend_from_slice(val.address.as_bytes());
            ops.push(BatchOp::Put(key, bytes.clone()));

            let key_hash = hash_with_domain(DOMAIN_SMT_LEAF, val.address.as_bytes());
            smt.update(key_hash, &bytes);
        }

        let genesis_block = genesis.to_genesis_block();
        let genesis_hash = genesis_block.hash();

        let mut block_key = CF_BLOCKS.as_bytes().to_vec();
        block_key.extend_from_slice(genesis_hash.as_bytes());
        let block_bytes = bincode::serialize(&genesis_block)?;
        ops.push(BatchOp::Put(block_key, block_bytes));

        let mut height_key = CF_HEIGHTS.as_bytes().to_vec();
        height_key.extend_from_slice(&0u64.to_be_bytes());
        ops.push(BatchOp::Put(height_key, genesis_hash.as_bytes().to_vec()));

        let height_meta_key = format!("{}latest_height", CF_META).into_bytes();
        ops.push(BatchOp::Put(height_meta_key, 0u64.to_be_bytes().to_vec()));

        self.kv.apply_batch(ops)?;
        Ok(smt.root())
    }

    /// Apply a committed block and update account states and SMT state_root atomically.
    pub fn commit_block(&self, block: &Block, overlay: StateOverlay) -> Result<Hash, StorageError> {
        let mut ops = Vec::new();
        let mut smt = self.smt.write();

        for (addr, account) in overlay.accounts {
            let bytes = account.to_bytes();
            let mut key = CF_ACCOUNTS.as_bytes().to_vec();
            key.extend_from_slice(addr.as_bytes());
            ops.push(BatchOp::Put(key, bytes.clone()));

            let key_hash = hash_with_domain(DOMAIN_SMT_LEAF, addr.as_bytes());
            smt.update(key_hash, &bytes);
        }

        let block_hash = block.hash();
        let mut block_key = CF_BLOCKS.as_bytes().to_vec();
        block_key.extend_from_slice(block_hash.as_bytes());
        let block_bytes = bincode::serialize(block)?;
        ops.push(BatchOp::Put(block_key, block_bytes));

        let mut height_key = CF_HEIGHTS.as_bytes().to_vec();
        height_key.extend_from_slice(&block.header.height.to_be_bytes());
        ops.push(BatchOp::Put(height_key, block_hash.as_bytes().to_vec()));

        let height_meta_key = format!("{}latest_height", CF_META).into_bytes();
        ops.push(BatchOp::Put(
            height_meta_key,
            block.header.height.to_be_bytes().to_vec(),
        ));

        self.kv.apply_batch(ops)?;
        Ok(smt.root())
    }

    /// Get all active registered validators.
    pub fn get_validators(&self) -> Vec<(Address, Account)> {
        let prefix = CF_ACCOUNTS.as_bytes();
        let mut vals = Vec::new();
        for (k, v) in self.kv.scan_prefix(prefix) {
            if let Ok(addr_slice) = k[prefix.len()..].try_into() {
                let addr = Address::from_bytes(addr_slice);
                if let Ok(acc) = Account::from_bytes(&v) {
                    if acc.is_validator && acc.staked_amount > 0 {
                        vals.push((addr, acc));
                    }
                }
            }
        }
        vals
    }
}
