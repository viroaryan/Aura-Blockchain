use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use parking_lot::RwLock;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("bincode error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("key not found")]
    NotFound,
    #[error("state corrupted: {0}")]
    Corrupted(String),
}

pub const CF_ACCOUNTS: &str = "a:";
pub const CF_BLOCKS: &str = "b:";
pub const CF_HEIGHTS: &str = "h:";
pub const CF_TXS: &str = "t:";
pub const CF_META: &str = "m:";

#[derive(Debug, Clone)]
pub enum BatchOp {
    Put(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
}

/// Fast Key-Value storage engine with namespace column prefixes.
pub struct KeyValueStore {
    data: RwLock<BTreeMap<Vec<u8>, Vec<u8>>>,
    db_path: Option<PathBuf>,
}

impl KeyValueStore {
    /// Open in-memory store.
    pub fn open_in_memory() -> Self {
        Self {
            data: RwLock::new(BTreeMap::new()),
            db_path: None,
        }
    }

    /// Open or create on-disk key-value store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut data = BTreeMap::new();
        if path.exists() {
            let mut file = File::open(&path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            if !buf.is_empty() {
                data = bincode::deserialize(&buf)?;
            }
        }

        Ok(Self {
            data: RwLock::new(data),
            db_path: Some(path),
        })
    }

    /// Retrieve value by key.
    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.read().get(key).cloned()
    }

    /// Store a single key-value pair.
    pub fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StorageError> {
        self.data.write().insert(key, value);
        self.persist()?;
        Ok(())
    }

    /// Delete a key.
    pub fn delete(&self, key: &[u8]) -> Result<(), StorageError> {
        self.data.write().remove(key);
        self.persist()?;
        Ok(())
    }

    /// Apply an atomic batch of Puts and Deletes.
    pub fn apply_batch(&self, ops: Vec<BatchOp>) -> Result<(), StorageError> {
        {
            let mut guard = self.data.write();
            for op in ops {
                match op {
                    BatchOp::Put(k, v) => {
                        guard.insert(k, v);
                    }
                    BatchOp::Delete(k) => {
                        guard.remove(&k);
                    }
                }
            }
        }
        self.persist()?;
        Ok(())
    }

    /// Scan all entries with a given prefix.
    pub fn scan_prefix(&self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let guard = self.data.read();
        guard
            .range(prefix.to_vec()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn persist(&self) -> Result<(), StorageError> {
        if let Some(ref path) = self.db_path {
            let bytes = {
                let guard = self.data.read();
                bincode::serialize(&*guard)?
            };
            let temp_path = path.with_extension("tmp");
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temp_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(temp_path, path)?;
        }
        Ok(())
    }
}
