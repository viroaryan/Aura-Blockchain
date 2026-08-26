use aura_crypto::Address;
use aura_primitives::{PrimitiveError, Vote};
use aura_storage::{Account, StateDB, StateOverlay};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SlashingError {
    #[error("invalid signature on vote: {0}")]
    InvalidSignature(#[from] PrimitiveError),
    #[error("evidence does not represent double signing: mismatched height or round")]
    InvalidEvidenceParams,
    #[error("evidence votes have identical block hashes")]
    IdenticalVotes,
    #[error("validators on votes do not match")]
    ValidatorMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleSignEvidence {
    pub vote_a: Vote,
    pub vote_b: Vote,
}

impl DoubleSignEvidence {
    /// Verify that two votes prove malicious double-signing by the same validator.
    pub fn verify(&self) -> Result<Address, SlashingError> {
        if self.vote_a.validator != self.vote_b.validator {
            return Err(SlashingError::ValidatorMismatch);
        }

        if self.vote_a.height != self.vote_b.height
            || self.vote_a.round != self.vote_b.round
            || self.vote_a.vote_type != self.vote_b.vote_type
        {
            return Err(SlashingError::InvalidEvidenceParams);
        }

        if self.vote_a.block_hash == self.vote_b.block_hash {
            return Err(SlashingError::IdenticalVotes);
        }

        // Verify signatures of both votes
        self.vote_a.verify()?;
        self.vote_b.verify()?;

        Ok(self.vote_a.validator)
    }

    /// Apply slashing penalty to validator in state overlay (e.g. slash 20% and jail/unregister).
    pub fn apply_slashing(
        &self,
        overlay: &mut StateOverlay,
        state_db: &StateDB,
    ) -> Result<Address, SlashingError> {
        let malicious_validator = self.verify()?;
        let mut account = overlay.get_account(&malicious_validator, state_db);

        // Slash 20% of staked amount
        let slashed_amount = account.staked_amount / 5;
        account.staked_amount = account.staked_amount.saturating_sub(slashed_amount);
        account.is_validator = false; // Jailed / unseated

        overlay.set_account(malicious_validator, account);
        Ok(malicious_validator)
    }
}
