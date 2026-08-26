use aura_crypto::Address;
use aura_primitives::{PrimitiveError, Transaction, TransactionType};
use aura_storage::{Account, StateDB};
use thiserror::Error;

pub const MIN_TRANSACTION_FEE: u64 = 100;
pub const MAX_TRANSACTION_SIZE: usize = 128 * 1024; // 128 KB

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MempoolValidationError {
    #[error("primitive error: {0}")]
    Primitive(#[from] PrimitiveError),
    #[error("transaction exceeds max size limit of {limit} bytes (size: {size})")]
    ExceedsMaxSize { size: usize, limit: usize },
    #[error("insufficient fee: minimum fee is {min_fee}, provided {fee}")]
    InsufficientFee { fee: u64, min_fee: u64 },
    #[error("zero or overflow amount")]
    InvalidAmount,
    #[error("insufficient sender balance: balance {balance}, required {required}")]
    InsufficientBalance { balance: u64, required: u64 },
    #[error("nonce too low: expected >= {expected}, got {got}")]
    NonceTooLow { expected: u64, got: u64 },
    #[error("nonce gap too large: expected {expected}, got {got}")]
    NonceGapTooLarge { expected: u64, got: u64 },
    #[error("account is already registered as validator")]
    AlreadyValidator,
    #[error("account is not a validator")]
    NotValidator,
    #[error("insufficient stake amount")]
    InsufficientStake,
}

pub struct TransactionValidator;

impl TransactionValidator {
    /// Stage 1: Stateless validation (signature, format, basic limits).
    pub fn validate_stateless(tx: &Transaction) -> Result<(), MempoolValidationError> {
        let size = tx.size_bytes();
        if size > MAX_TRANSACTION_SIZE {
            return Err(MempoolValidationError::ExceedsMaxSize {
                size,
                limit: MAX_TRANSACTION_SIZE,
            });
        }

        if tx.fee < MIN_TRANSACTION_FEE {
            return Err(MempoolValidationError::InsufficientFee {
                fee: tx.fee,
                min_fee: MIN_TRANSACTION_FEE,
            });
        }

        if tx.amount == 0 && tx.tx_type == TransactionType::Transfer {
            return Err(MempoolValidationError::InvalidAmount);
        }

        // Verify cryptographic Ed25519 signature
        tx.verify()?;

        Ok(())
    }

    /// Stage 2: Stateful validation against current account state.
    pub fn validate_stateful(
        tx: &Transaction,
        account: &Account,
    ) -> Result<(), MempoolValidationError> {
        // Nonce check
        if tx.nonce <= account.nonce {
            return Err(MempoolValidationError::NonceTooLow {
                expected: account.nonce + 1,
                got: tx.nonce,
            });
        }

        if tx.nonce > account.nonce + 64 {
            return Err(MempoolValidationError::NonceGapTooLarge {
                expected: account.nonce + 1,
                got: tx.nonce,
            });
        }

        // Balance check: amount + fee
        let required = tx.amount.saturating_add(tx.fee);
        if account.balance < required {
            return Err(MempoolValidationError::InsufficientBalance {
                balance: account.balance,
                required,
            });
        }

        // Type-specific checks
        match tx.tx_type {
            TransactionType::RegisterValidator => {
                if account.is_validator {
                    return Err(MempoolValidationError::AlreadyValidator);
                }
                if tx.amount < 1_000_000 {
                    // Minimum validator stake
                    return Err(MempoolValidationError::InsufficientStake);
                }
            }
            TransactionType::Unstake => {
                if !account.is_validator || account.staked_amount < tx.amount {
                    return Err(MempoolValidationError::InsufficientStake);
                }
            }
            _ => {}
        }

        Ok(())
    }
}
