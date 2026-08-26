use aura_crypto::Hash;
use serde::{Deserialize, Serialize};

use crate::header::BlockHeader;
use crate::merkle::MerkleTree;
use crate::qc::QuorumCertificate;
use crate::transaction::{PrimitiveError, Transaction};

/// Full Block containing header, body transactions, and last commit QC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub last_commit_qc: Option<QuorumCertificate>,
}

impl Block {
    pub fn new(
        header: BlockHeader,
        transactions: Vec<Transaction>,
        last_commit_qc: Option<QuorumCertificate>,
    ) -> Self {
        Self {
            header,
            transactions,
            last_commit_qc,
        }
    }

    /// Compute Merkle root of transactions.
    pub fn compute_merkle_root(&self) -> Hash {
        let tx_hashes: Vec<Hash> = self.transactions.iter().map(|tx| tx.tx_hash()).collect();
        let tree = MerkleTree::from_leaf_hashes(tx_hashes);
        tree.root()
    }

    /// Block hash equals the header hash.
    pub fn hash(&self) -> Hash {
        self.header.header_hash()
    }

    /// Validate structural integrity (merkle root, transaction signatures).
    pub fn validate_basic(&self) -> Result<(), PrimitiveError> {
        let calculated_root = self.compute_merkle_root();
        if calculated_root != self.header.merkle_root {
            return Err(PrimitiveError::MerkleRootMismatch {
                expected: self.header.merkle_root,
                calculated: calculated_root,
            });
        }

        for tx in &self.transactions {
            tx.verify()?;
        }

        Ok(())
    }
}
