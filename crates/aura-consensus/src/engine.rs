use std::collections::HashMap;
use std::sync::Arc;
use aura_crypto::{Address, Hash, KeyPair};
use aura_mempool::Mempool;
use aura_primitives::{
    Block, BlockHeader, QuorumCertificate, Transaction, TransactionType, Vote, VoteType,
};
use aura_storage::{StateDB, StateOverlay};
use parking_lot::RwLock;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::proposer::select_proposer;
use crate::types::{ConsensusMessage, ConsensusStep, ValidatorInfo};

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("not elected proposer for height {height}, round {round}")]
    NotProposer { height: u64, round: u32 },
    #[error("invalid proposal header or signature")]
    InvalidProposal,
    #[error("storage error: {0}")]
    Storage(#[from] aura_storage::StorageError),
    #[error("mempool error: {0}")]
    Mempool(#[from] aura_mempool::MempoolError),
}

pub struct ConsensusEngine {
    keypair: KeyPair,
    validator_address: Address,
    state_db: Arc<StateDB>,
    mempool: Arc<Mempool>,
    chain_id: String,

    // Consensus state
    current_height: RwLock<u64>,
    current_round: RwLock<u32>,
    step: RwLock<ConsensusStep>,

    // Round data
    proposed_block: RwLock<Option<Block>>,
    prevotes: RwLock<HashMap<Hash, Vec<Vote>>>,
    precommits: RwLock<HashMap<Hash, Vec<Vote>>>,
    last_qc: RwLock<Option<QuorumCertificate>>,
}

impl ConsensusEngine {
    pub fn new(
        keypair: KeyPair,
        state_db: Arc<StateDB>,
        mempool: Arc<Mempool>,
        chain_id: String,
    ) -> Self {
        let validator_address = Address::from_pubkey(&keypair.public_key());
        let latest_height = state_db.get_latest_height();

        Self {
            keypair,
            validator_address,
            state_db,
            mempool,
            chain_id,
            current_height: RwLock::new(latest_height + 1),
            current_round: RwLock::new(0),
            step: RwLock::new(ConsensusStep::NewRound),
            proposed_block: RwLock::new(None),
            prevotes: RwLock::new(HashMap::new()),
            precommits: RwLock::new(HashMap::new()),
            last_qc: RwLock::new(None),
        }
    }

    /// Retrieve active validator set from StateDB.
    pub fn get_active_validators(&self) -> Vec<ValidatorInfo> {
        self.state_db
            .get_validators()
            .into_iter()
            .filter_map(|(addr, acc)| {
                acc.validator_pubkey.map(|pk| ValidatorInfo {
                    address: addr,
                    pubkey: pk,
                    voting_power: acc.staked_amount,
                })
            })
            .collect()
    }

    /// Check if local node is elected proposer.
    pub fn is_proposer(&self, height: u64, round: u32) -> bool {
        let validators = self.get_active_validators();
        if let Some(proposer) = select_proposer(height, round, &validators) {
            proposer.address == self.validator_address
        } else {
            false
        }
    }

    /// Propose a new block if elected proposer.
    pub fn create_proposal(&self) -> Result<Block, ConsensusError> {
        let height = *self.current_height.read();
        let round = *self.current_round.read();

        let validators = self.get_active_validators();
        let proposer = select_proposer(height, round, &validators)
            .ok_or(ConsensusError::NotProposer { height, round })?;

        if proposer.address != self.validator_address {
            return Err(ConsensusError::NotProposer { height, round });
        }

        let prev_block = self
            .state_db
            .get_block_by_height(height - 1)
            .expect("previous block must exist");
        let prev_hash = prev_block.hash();

        // Harvest transactions from mempool (up to 1 MB)
        let txs = self.mempool.harvest(1024 * 1024);

        // Speculative execution to compute state_root
        let mut overlay = StateOverlay::new();
        self.execute_transactions_speculative(&txs, &mut overlay);

        // Compute validator set hash
        let mut val_bytes = Vec::new();
        for v in &validators {
            val_bytes.extend_from_slice(v.address.as_bytes());
            val_bytes.extend_from_slice(v.pubkey.as_bytes());
            val_bytes.extend_from_slice(&v.voting_power.to_be_bytes());
        }
        let val_set_hash = aura_crypto::hash_bytes(&val_bytes);

        let mut block = Block::new(
            BlockHeader::new_unsigned(
                1,
                self.chain_id.clone(),
                height,
                round,
                prev_hash,
                Hash::ZERO,
                self.state_db.get_state_root(),
                val_set_hash,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                self.validator_address,
            ),
            txs,
            self.last_qc.read().clone(),
        );

        block.header.merkle_root = block.compute_merkle_root();
        block
            .header
            .sign(&self.keypair)
            .map_err(|_| ConsensusError::InvalidProposal)?;

        *self.proposed_block.write() = Some(block.clone());
        *self.step.write() = ConsensusStep::Propose;

        info!(height = height, round = round, "Created block proposal");
        Ok(block)
    }

    /// Process received block proposal and generate PreVote.
    pub fn handle_proposal(&self, block: &Block) -> Result<Vote, ConsensusError> {
        let height = *self.current_height.read();
        let round = *self.current_round.read();

        if block.header.height != height || block.header.round != round {
            return Err(ConsensusError::InvalidProposal);
        }

        block
            .validate_basic()
            .map_err(|_| ConsensusError::InvalidProposal)?;

        *self.proposed_block.write() = Some(block.clone());
        *self.step.write() = ConsensusStep::PreVote;

        let vote = Vote::new_signed(
            block.hash(),
            height,
            round,
            VoteType::PreVote,
            &self.keypair,
        )
        .map_err(|_| ConsensusError::InvalidProposal)?;

        self.handle_prevote(&vote);
        Ok(vote)
    }

    /// Process received PreVote.
    pub fn handle_prevote(&self, vote: &Vote) -> Option<Vote> {
        let mut prevotes = self.prevotes.write();
        let list = prevotes.entry(vote.block_hash).or_default();
        if !list.iter().any(|v| v.validator == vote.validator) {
            list.push(vote.clone());
        }

        let validators = self.get_active_validators();
        let total_power: u64 = validators.iter().map(|v| v.voting_power).sum();
        let vote_power: u64 = list
            .iter()
            .filter_map(|v| validators.iter().find(|val| val.address == v.validator))
            .map(|val| val.voting_power)
            .sum();

        // If > 2/3 stake pre-voted and step is PreVote -> cast PreCommit
        if total_power > 0
            && (vote_power as u128) * 3 > (total_power as u128) * 2
            && *self.step.read() == ConsensusStep::PreVote
        {
            *self.step.write() = ConsensusStep::PreCommit;
            let height = *self.current_height.read();
            let round = *self.current_round.read();

            let precommit = Vote::new_signed(
                vote.block_hash,
                height,
                round,
                VoteType::PreCommit,
                &self.keypair,
            )
            .ok()?;

            self.handle_precommit(&precommit);
            return Some(precommit);
        }

        None
    }

    /// Process received PreCommit.
    pub fn handle_precommit(&self, vote: &Vote) -> Option<QuorumCertificate> {
        let mut precommits = self.precommits.write();
        let list = precommits.entry(vote.block_hash).or_default();
        if !list.iter().any(|v| v.validator == vote.validator) {
            list.push(vote.clone());
        }

        let validators = self.get_active_validators();
        let total_power: u64 = validators.iter().map(|v| v.voting_power).sum();
        let vote_power: u64 = list
            .iter()
            .filter_map(|v| validators.iter().find(|val| val.address == v.validator))
            .map(|val| val.voting_power)
            .sum();

        // If > 2/3 stake pre-committed -> Commit Block!
        if total_power > 0
            && (vote_power as u128) * 3 > (total_power as u128) * 2
            && *self.step.read() != ConsensusStep::Commit
        {
            *self.step.write() = ConsensusStep::Commit;

            let qc = QuorumCertificate {
                block_hash: vote.block_hash,
                height: *self.current_height.read(),
                round: *self.current_round.read(),
                votes: list.clone(),
                total_voting_power: total_power,
                signers_voting_power: vote_power,
            };

            if let Some(ref block) = *self.proposed_block.read() {
                if block.hash() == vote.block_hash {
                    // Commit to StateDB
                    let mut overlay = StateOverlay::new();
                    self.execute_transactions_speculative(&block.transactions, &mut overlay);
                    self.state_db
                        .commit_block(block, overlay)
                        .expect("block commit must succeed");

                    // Clean mempool
                    self.mempool.remove_committed(&block.transactions);

                    info!(height = block.header.height, "Committed block to state!");
                }
            }

            *self.last_qc.write() = Some(qc.clone());

            // Advance to next height
            *self.current_height.write() += 1;
            *self.current_round.write() = 0;
            *self.step.write() = ConsensusStep::NewRound;
            *self.proposed_block.write() = None;
            self.prevotes.write().clear();
            self.precommits.write().clear();

            return Some(qc);
        }

        None
    }

    fn execute_transactions_speculative(&self, txs: &[Transaction], overlay: &mut StateOverlay) {
        for tx in txs {
            let mut sender_acc = overlay.get_account(&tx.sender, &self.state_db);
            let total_deduct = tx.amount + tx.fee;
            sender_acc.balance = sender_acc.balance.saturating_sub(total_deduct);
            sender_acc.nonce += 1;

            match tx.tx_type {
                TransactionType::Transfer => {
                    let mut recipient_acc = overlay.get_account(&tx.recipient, &self.state_db);
                    recipient_acc.balance = recipient_acc.balance.saturating_add(tx.amount);
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
                    sender_acc.balance = sender_acc.balance.saturating_add(tx.amount);
                }
            }

            overlay.set_account(tx.sender, sender_acc);
        }
    }
}
