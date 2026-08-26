use aura_crypto::{hash_with_domain, Address};
use crate::types::ValidatorInfo;

const DOMAIN_PROPOSER_SEED: &[u8] = b"AURA_PROPOSER_SEED_v1";

/// Select proposer deterministically weighted by voting power.
pub fn select_proposer(
    height: u64,
    round: u32,
    validators: &[ValidatorInfo],
) -> Option<ValidatorInfo> {
    if validators.is_empty() {
        return None;
    }

    let total_power: u64 = validators.iter().map(|v| v.voting_power).sum();
    if total_power == 0 {
        return None;
    }

    // Seed = BLAKE3(height || round)
    let mut seed_data = Vec::with_capacity(12);
    seed_data.extend_from_slice(&height.to_be_bytes());
    seed_data.extend_from_slice(&round.to_be_bytes());
    let seed_hash = hash_with_domain(DOMAIN_PROPOSER_SEED, &seed_data);

    // Convert first 8 bytes of hash into u64
    let mut num_bytes = [0u8; 8];
    num_bytes.copy_from_slice(&seed_hash.as_bytes()[0..8]);
    let random_val = u64::from_be_bytes(num_bytes);

    let target_power = random_val % total_power;

    let mut accumulated = 0u64;
    for val in validators {
        accumulated += val.voting_power;
        if accumulated > target_power {
            return Some(val.clone());
        }
    }

    validators.first().cloned()
}
