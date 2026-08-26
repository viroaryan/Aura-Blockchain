use aura_crypto::{Address, Hash, PublicKey, Signature};
use aura_primitives::{Block, Vote, VoteType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub address: Address,
    pub pubkey: PublicKey,
    pub voting_power: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusStep {
    NewRound,
    Propose,
    PreVote,
    PreCommit,
    Commit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutMessage {
    pub height: u64,
    pub round: u32,
    pub validator: Address,
    pub pubkey: PublicKey,
    pub signature: Signature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusMessage {
    Proposal(Block),
    PreVote(Vote),
    PreCommit(Vote),
    Timeout(TimeoutMessage),
}
