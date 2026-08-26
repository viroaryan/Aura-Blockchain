# Technical Specification & Survey Report: PoS-BFT Consensus Engine & P2P Networking / Synchronization (R4 & R5)

- **Project**: Aura Cryptocurrency (`AUR`)
- **Working Directory**: `d:/cryptocurrency`
- **Subsystem Scope**: R4 (PoS-BFT Consensus Engine) & R5 (P2P Networking & Node Synchronization)
- **Status**: Exhaustive Technical Specification Complete

---

## Executive Summary

This report establishes the complete, production-grade architectural specification for the **Aura PoS-BFT Consensus Engine (R4)** and **P2P Networking & Node Synchronization Subsystem (R5)**.

### Core Architectural Guarantees:
1. **Safety & Instant Deterministic Finality**: A 2-phase Byzantine Fault Tolerant state machine (Propose -> PreVote -> PreCommit -> Commit) providing single-slot deterministic finality without probabilistic forks or chain reorganizations. Safety is mathematically guaranteed as long as Byzantine voting power $f < \frac{1}{3} W_{\text{total}}$.
2. **Deterministic Stake-Weighted Proposer Selection**: Deficit Round-Robin (Tendermint-style Proposer Priority) algorithm guaranteeing zero-grinding predictability, bounded priority spread ($-2W \le P_i \le 2W$), and exact stake-proportional block production without reliance on manipulate-able VRFs or random beacons.
3. **Economic Security & Slashing Protocol**: Immediate detection, verification, and slashing for Byzantine faults (equivocation / double-signing). Slashes 10% of validator stake, tombstones the validator key permanently, and allocates a 1% whistleblower reward to the evidence submitter within an unbonding horizon of $U = 10,000$ blocks.
4. **Resilient P2P Transport & Mesh Topology**: Built on `rust-libp2p` with authenticated Noise encryption (`Noise_XX_25519_ChaChaPoly_SHA256`) using Ed25519 node identities, Yamux stream multiplexing, Kademlia DHT (`/aura/kad/1.0.0`) for autonomous peer discovery, and GossipSub v1.1 with topic-level validation filters for low-latency block/transaction propagation.
5. **Tiered Node Synchronization**: Three dedicated Request-Response protocols:
   - **Full Chain Sync** (`/aura/sync/blocks/1.0.0`): Pipelined parallel batch block downloads with full transaction replay and authenticated state trie verification.
   - **Header Sync** (`/aura/sync/headers/1.0.0`): Rapid header verification with Quorum Certificates ($> 2/3$ stake signatures) and state snapshot pivoting.
   - **Light Client Verification** (`/aura/sync/light/1.0.0`): Cryptographic Merkle tree inclusion proofs for transactions and SMT proofs for account balances.

---

## 1. Traceability & Requirements Coverage Matrix

| Req ID | Component | Architectural Feature | Specification Summary | Source Input | Target Output / State | Invalidation / Error Mode | Section Reference |
|---|---|---|---|---|---|---|---|
| **R4.1** | Consensus | 2-Phase BFT State Machine | Round-based Propose -> PreVote -> PreCommit -> Commit cycle with explicit lock/valid round rules | Block proposals, signed votes, timers | Committed `FullBlock` with `QuorumCertificate` | $< 2/3$ quorum triggers round timeout & round skip | Section 2.2 |
| **R4.2** | Consensus | Proposer Selection | Deficit Round-Robin Proposer Priority algorithm allocating slots proportional to stake | `ValidatorSet`, `height`, `round` | Deterministic `Validator` leader | Empty validator set or zero total stake rejected | Section 2.3 |
| **R4.3** | Consensus | Quorum & Finality | Strict $> \frac{2}{3} W_{\text{total}}$ voting weight threshold for Polka and Commit Certificates | `Vec<SignedVote>` | Validated `QuorumCertificate` | Integer rounding error or $\le 2/3$ stake fails quorum | Section 2.2, 2.4 |
| **R4.4** | Consensus | Slashing & Evidence | Double-sign verification (conflicting votes at same height/round/step) and 10% stake burning | `DoubleSignEvidence` | Slashed stake, `Tombstoned` validator status | Invalid signature, matching hashes, or expired unbonding rejected | Section 2.5 |
| **R4.5** | Consensus | Validator Set Transitions | Epoch-based validator set updates, dynamic staking, and commission tracking | Staking transactions in block | Updated `ValidatorSet` committed at epoch boundary | Inactive validator selection or invalid power calculation rejected | Section 2.6 |
| **R5.1** | P2P | Transport & Encryption | libp2p TCP transport secured by Noise XX handshake with Ed25519 `PeerId` and Yamux | Socket multiaddrs, local Ed25519 keypair | Secure, authenticated bi-directional channel | Handshake failure drops peer connection | Section 3.1 |
| **R5.2** | P2P | Kademlia DHT Discovery | Autonomous peer discovery via `/aura/kad/1.0.0` with $k=20, \alpha=3$ routing buckets | Bootstrap nodes, XOR metric | Populated peer routing table | Unresponsive peers evicted from $k$-buckets | Section 3.2 |
| **R5.3** | P2P | GossipSub v1.1 Mesh | High-throughput pubsub with topic isolation (`/aura/tx`, `/aura/block`, `/aura/consensus`) | Serialized network payloads | Validated broadcasted messages | Malformed/spam messages dropped; peer score decremented | Section 3.3 |
| **R5.4** | P2P | Topic Pre-Validation | Cryptographic pre-validation before gossip forwarding to prevent network spam amplification | Wire messages (`Transaction`, `Block`, `ConsensusMessage`) | Validated message forwarded to mesh | Signature or format mismatch drops message without forwarding | Section 3.3.3 |
| **R5.5** | Sync | Full Chain Batch Sync | Request-Response parallel batch sync (`/aura/sync/blocks/1.0.0`) for fast historical catch-up | `BlockRequest(start_height, max_blocks)` | `BlockResponse(Vec<FullBlock>)` | Broken hash chain or state root mismatch halts sync | Section 3.4.1 |
| **R5.6** | Sync | Header Sync Protocol | Fast header download and Quorum Certificate validation (`/aura/sync/headers/1.0.0`) | `HeaderRequest(start_height, max_headers)` | `HeaderResponse(Vec<BlockHeader>, Vec<QuorumCertificate>)` | Invalid certificate signature aborts sync | Section 3.4.2 |
| **R5.7** | Sync | Light Client Proofs | Merkle proof verification for txs and SMT state proofs for account balances | `BlockHeader`, `MerkleProof`, target query | Boolean validity & authenticated state | Tampered proof yields `VerificationFailed` | Section 3.4.3 |
| **R5.8** | P2P | Reputation & Anti-DoS | Peer scoring, connection limits (max 50, min 15 outbound), and /16 CIDR grouping | Peer behavior event stream | Peer score updates, TCP disconnect, IP ban | Malicious peer banned on score threshold breach | Section 3.5 |

---

### 1.1 System Architecture & Inter-Crate Topology

```
+---------------------------------------------------------------------------------------------------+
|                                        Aura Node Subsystems                                       |
+---------------------------------------------------------------------------------------------------+
|                                                                                                   |
|   +--------------------------+       Tx Inflow        +---------------------------------------+   |
|   |       aura-mempool       | ---------------------> |             aura-consensus            |   |
|   |  - Prioritized tx pool   |                        |  - 2-Phase BFT State Machine (H, R)   |   |
|   |  - Nonce & balance check | <--------------------- |  - Proposer Selection (Priority DRR)  |   |
|   +--------------------------+     Evict Committed    |  - Quorum Accumulator & QC Builder    |   |
|                 ^                                     |  - Slashing & Double-Sign Verification|   |
|                 | Gossip Txs                          +---------------------------------------+   |
|                 v                                                         |                       |
|   +--------------------------+     Gossip Consensus Messages              | Executed Block State  |
|   |         aura-p2p         | <------------------------------------------+                       |
|   |  - libp2p Swarm          |                                            v                       |
|   |  - Noise XX & Yamux      |       Block / Header Sync      +-----------------------------------+
|   |  - Kademlia DHT (/kad)   | <============================> |            aura-storage           |
|   |  - GossipSub v1.1        |                                |  - Authenticated SMT (state_root) |
|   +--------------------------+                                |  - RocksDB / Redb Key-Value Store |
|                 ^                                             |  - Crash-Resilient WAL Engine     |
|                 | Direct Requests                             +-----------------------------------+
|                 v                                                         ^                       |
|   +--------------------------+                                            | Historical Queries    |
|   |        aura-sync         | -------------------------------------------+                       |
|   |  - Full Chain Batch Sync |                                            v                       |
|   |  - Header Fast Sync      |                                +-----------------------------------+
|   |  - Light Client Proofs   |                                |              aura-rpc             |
|   +--------------------------+                                |  - JSON-RPC 2.0 & WebSocket Server|
|                                                               +-----------------------------------+
+---------------------------------------------------------------------------------------------------+
```

---
## 2. R4: PoS-BFT Consensus Engine Specification

### 2.1 Theoretical Foundations & Invariants

Aura PoS-BFT is a Byzantine Fault Tolerant state machine operating under the **partial synchrony model** (Dwork, Lynch, Stockmeyer 1988). The network has an unknown Global Stabilization Time ($\text{GST}$) after which message transmission delay is bounded by $\Delta$.

Let $V = \{v_1, v_2, \dots, v_n\}$ be the set of active validators at height $H$.
Each validator $v_i$ possesses voting power $w_i \in \mathbb{N}^+$.
The total active voting power is:
$$W_{\text{total}} = \sum_{i=1}^n w_i$$

#### Core Invariants:
1. **Quorum Threshold**: Any quorum decision (Polka or Commit) requires strictly greater than two-thirds of the total voting power:
   $$W_{\text{quorum}} \ge \left\lfloor \frac{2 \cdot W_{\text{total}}}{3} \right\rfloor + 1$$
2. **Safety Invariant**: Under partial synchrony, no two honest nodes commit distinct blocks at the same height $H$. 
   *Proof Sketch*: Suppose block $B$ is committed in round $R_1$ and block $B' \ne B$ is committed in round $R_2 \ge R_1$. Committing $B$ implies a quorum $Q_1$ ($> \frac{2}{3} W_{\text{total}}$) issued PreCommits for $B$ in $R_1$. Committing $B'$ implies a quorum $Q_2$ ($> \frac{2}{3} W_{\text{total}}$) issued PreCommits for $B'$ in $R_2$. The intersection $Q_1 \cap Q_2$ contains at least:
   $$|Q_1 \cap Q_2| > \frac{2}{3} W_{\text{total}} + \frac{2}{3} W_{\text{total}} - W_{\text{total}} = \frac{1}{3} W_{\text{total}}$$
   If Byzantine voting power $f < \frac{1}{3} W_{\text{total}}$, at least one honest validator $v_h \in Q_1 \cap Q_2$ must have pre-committed $B$ in $R_1$ and subsequently pre-committed $B'$ in $R_2$. However, by the locking rules, $v_h$ was locked on $B$ at round $R_1$ and could only pre-commit $B'$ if a valid Polka for $B'$ occurred at round $R' > R_1$. A Polka for $B'$ requires $> \frac{2}{3}$ PreVotes for $B'$, which is impossible without honest nodes violating their locks or Byzantine nodes double-signing. Hence, safety holds.
3. **Liveness Invariant**: If $f < \frac{1}{3} W_{\text{total}}$, after $\text{GST}$, consensus will finalize a new block within bounded rounds.
4. **Determinism**: Proposer selection, vote counting, state transitions, and slashing calculations are pure deterministic functions of the input messages and prior committed state.

---

### 2.2 Formal State Machine & Transitions

The consensus engine executes on a per-height basis. For height $H$, execution begins at round $R = 0$ and may advance through rounds $R = 1, 2, \dots$ until a block is finalized.

```
                                  +---------------------------------------+
                                  |            NewRound(H, R)             |
                                  | - Select Proposer p                   |
                                  | - Reset round accumulators            |
                                  +---------------------------------------+
                                                      |
                                                      v
                                  +---------------------------------------+
                                  |             Propose(H, R)             |
                                  | If p == self: broadcast Proposal      |
                                  | Start timer T_propose(R)              |
                                  +---------------------------------------+
                                                      |
                                    +-----------------+-----------------+
                                    | (Valid Proposal)                  | (Timeout T_propose expires
                                    v                                   |  or invalid proposal)
                     +-------------------------------+                  v
                     |  PreVote(H, R, BlockHash)     |   +-------------------------------+
                     |  Broadcast signed PreVote     |   |      PreVote(H, R, NIL)       |
                     +-------------------------------+   |   Broadcast signed NIL PreVote|
                                    \                    +-------------------------------+
                                     \                                  /
                                      \                                /
                                       v                              v
                                  +---------------------------------------+
                                  |             PreVote(H, R)             |
                                  | Collect PreVotes from Validator Set   |
                                  | Start timer T_prevote(R)              |
                                  +---------------------------------------+
                                                      |
                                    +-----------------+-----------------+
                                    | (>2/3 PreVotes for Block B)       | (>2/3 PreVotes for NIL
                                    | [Polka on Block B]                |  or Timeout T_prevote)
                                    v                                   v
                     +-------------------------------+   +-------------------------------+
                     | Set locked_block = B          |   |  PreCommit(H, R, NIL)         |
                     | Set locked_round = R          |   |  Broadcast signed NIL PreCommit|
                     | Broadcast PreCommit(B.hash)   |   +-------------------------------+
                     +-------------------------------+                  |
                                    \                                   /
                                     \                                 /
                                      v                               v
                                  +---------------------------------------+
                                  |            PreCommit(H, R)            |
                                  | Collect PreCommits from Validators    |
                                  | Start timer T_precommit(R)            |
                                  +---------------------------------------+
                                                      |
                                    +-----------------+-----------------+
                                    | (>2/3 PreCommits for Block B)     | (>2/3 PreCommits for NIL
                                    | [Commit Certificate Formed]       |  or Timeout T_precommit)
                                    v                                   v
                     +-------------------------------+   +-------------------------------+
                     |          Commit(H, R)         |   |          RoundSkip            |
                     | - Commit block B to Storage   |   | - Increment R <- R + 1        |
                     | - Commit WAL & State SMT      |   | - Go to NewRound(H, R+1)      |
                     | - H <- H + 1, R <- 0          |   +-------------------------------+
                     | - Go to NewRound(H+1, 0)      |
                     +-------------------------------+
```

#### Detailed Step Transition Rules:

1. **`NewRound(H, R)`**:
   - Set `step = Propose`.
   - Compute proposer $p = \text{SelectProposer}(V, H, R)$.
   - Reset round vote accumulators (`prevotes.clear()`, `precommits.clear()`).
   - If `self.address == p`:
     - If `valid_block` is `Some(B)` (from a prior round polka):
       - Construct `Proposal { height: H, round: R, block: B, valid_round: Some(valid_round), proposer: self.address }`.
     - Else:
       - Fetch pending transactions from Mempool.
       - Execute transactions against current authenticated state trie overlay to calculate `state_root`, `transactions_root`, and `receipts_root`.
       - Assemble `BlockHeader` and `Block`.
       - Construct `Proposal { height: H, round: R, block: B, valid_round: None, proposer: self.address }`.
     - Sign proposal with validator private key and broadcast `ConsensusMessage::Proposal` to `/aura/consensus/1.0.0`.
   - Arm proposal timer:
     $$T_{\text{propose}}(R) = T_{\text{propose\_init}} + R \cdot \Delta_{\text{timeout}}$$
     (Defaults: $T_{\text{propose\_init}} = 3000\text{ ms}$, $\Delta_{\text{timeout}} = 500\text{ ms}$).

2. **`Propose(H, R)` -> `PreVote(H, R)`**:
   - **On Proposal Received (`Proposal(H, R, B, VR)` from proposer $p$)**:
     - Verify $p == \text{SelectProposer}(V, H, R)$ and verify Ed25519 signature of $p$.
     - Verify $B.header.height == H$, $B.header.previous\_hash == \text{Hash}(H-1)$.
     - Verify all transactions in $B$ are valid (signatures, nonces, balances).
     - **Lock Rules**:
       - If `locked_round` is `None`: Block is acceptable.
       - If `locked_round` is `Some(LR)`:
         - If $B.hash == locked\_block.hash$: Acceptable (same block).
         - If $VR.is\_some() \land VR.unwrap() > LR$: Acceptable (higher polka round unlocks previous lock).
         - Else: Unacceptable (violates safety lock).
     - **Action**:
       - If acceptable: Broadcast `PreVote(H, R, Some(B.hash), Sign(B.hash))`.
       - If unacceptable: Broadcast `PreVote(H, R, None, Sign(NIL))`.
     - Disarm $T_{\text{propose}}$ timer; transition to `PreVote(H, R)`.
   - **On Timeout $T_{\text{propose}}(R)$ Expiry**:
     - Broadcast `PreVote(H, R, None, Sign(NIL))`.
     - Transition to `PreVote(H, R)`.

3. **`PreVote(H, R)` -> `PreCommit(H, R)`**:
   - Arm prevote timer $T_{\text{prevote}}(R) = 1000\text{ ms} + R \cdot 500\text{ ms}$.
   - **Condition A (Polka on Block)**:
     - If received `PreVote(H, R, Some(B.hash))` from a set of validators with total weight $W_{\text{prevote}}(B.hash) \ge W_{\text{quorum}}$:
       - Set $locked\_block \leftarrow \text{Some}(B)$.
       - Set $locked\_round \leftarrow \text{Some}(R)$.
       - Set $valid\_block \leftarrow \text{Some}(B)$.
       - Set $valid\_round \leftarrow \text{Some}(R)$.
       - Broadcast `PreCommit(H, R, Some(B.hash), Sign(B.hash))`.
       - Disarm $T_{\text{prevote}}$ timer; transition to `PreCommit(H, R)`.
   - **Condition B (Polka on NIL or PreVote Timeout)**:
     - If received `PreVote(H, R, None)` with weight $\ge W_{\text{quorum}}$ OR timer $T_{\text{prevote}}(R)$ expires:
       - Broadcast `PreCommit(H, R, None, Sign(NIL))`.
       - Transition to `PreCommit(H, R)`.

4. **`PreCommit(H, R)` -> `Commit(H, R)` or `RoundSkip`**:
   - Arm precommit timer $T_{\text{precommit}}(R) = 1000\text{ ms} + R \cdot 500\text{ ms}$.
   - **Condition A (Commit Finality)**:
     - If received `PreCommit(H, R, Some(B.hash))` with total weight $W_{\text{precommit}}(B.hash) \ge W_{\text{quorum}}$:
       - Construct `QuorumCertificate`:
         ```rust
         pub struct QuorumCertificate {
             pub height: u64,
             pub round: u32,
             pub block_hash: Hash,
             pub signatures: Vec<(Address, Signature)>,
         }
         ```
       - Attach `QuorumCertificate` to $B$ to form `FullBlock`.
       - Commit block state transition to authenticated persistent SMT and write WAL commit record.
       - Remove committed transactions from Mempool.
       - Advance height: $H \leftarrow H + 1$, $R \leftarrow 0$.
       - Clear locks: $locked\_block \leftarrow \text{None}, locked\_round \leftarrow \text{None}, valid\_block \leftarrow \text{None}$.
       - Transition to `NewRound(H+1, 0)`.
   - **Condition B (Round Timeout / Nil PreCommits)**:
     - If received `PreCommit(H, R, None)` with weight $\ge W_{\text{quorum}}$ OR timer $T_{\text{precommit}}(R)$ expires:
       - Advance round: $R \leftarrow R + 1$.
       - Transition to `NewRound(H, R+1)`.

---

### 2.3 Deterministic Stake-Weighted Proposer Selection Algorithm

Aura uses the **Deficit Round-Robin Proposer Priority Algorithm** (Tendermint Proposer Priority). This algorithm provides deterministic, zero-grinding, stake-proportional proposer rotation.

#### Mathematical Algorithm:
Let $V = \{v_1, v_2, \dots, v_n\}$ be the sorted list of validators (sorted lexicographically by `Address`).
Each validator maintains a mutable integer priority $P_i \in \mathbb{Z}$, initialized to $0$.

```rust
pub fn compute_proposer(
    validators: &mut [Validator],
    height: u64,
    round: u32,
) -> Validator {
    let total_power: i64 = validators.iter().map(|v| v.voting_power as i64).sum();
    assert!(total_power > 0, "Total voting power must be positive");

    // Step 1: Add voting power to each validator's priority
    for v in validators.iter_mut() {
        v.proposer_priority = v.proposer_priority
            .checked_add(v.voting_power as i64)
            .expect("Proposer priority overflow");
    }

    // Step 2: Select validator with highest priority (tie-break by lowest Address bytes)
    let winner_idx = validators
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.proposer_priority
                .cmp(&b.proposer_priority)
                .then_with(|| b.address.0.cmp(&a.address.0)) // Lower address wins tie
        })
        .map(|(idx, _)| idx)
        .expect("Non-empty validator set");

    // Step 3: Subtract total_power from the winner
    validators[winner_idx].proposer_priority = validators[winner_idx]
        .proposer_priority
        .checked_sub(total_power)
        .expect("Proposer priority underflow");

    // Step 4: Scale centering to prevent priority drift
    center_priorities(validators);

    validators[winner_idx].clone()
}

fn center_priorities(validators: &mut [Validator]) {
    let total_power: i64 = validators.iter().map(|v| v.voting_power as i64).sum();
    let sum_priorities: i64 = validators.iter().map(|v| v.proposer_priority).sum();
    let avg = sum_priorities / (validators.len() as i64);
    
    for v in validators.iter_mut() {
        v.proposer_priority -= avg;
    }
}
```

#### Analytical Properties:
1. **Bounded Priority Spread**: For all $i$, $-2 W_{\text{total}} \le P_i \le 2 W_{\text{total}}$. Priorities never diverge.
2. **Exact Stake Proportionality**: In any window of $W_{\text{total}}$ blocks, validator $v_i$ is selected as proposer exactly $w_i$ times.
3. **Zero-Grinding Security**: The selection is strictly deterministic based on on-chain state and cannot be manipulated by transaction order or block hash grinding.

---

### 2.4 Quorum Calculation & Integer Arithmetic

To prevent floating-point inaccuracies or off-by-one quorum vulnerabilities across architectures:

```rust
pub fn calculate_quorum_threshold(total_voting_power: u64) -> u64 {
    // Strictly greater than 2/3 of total voting power:
    // floor((2 * total_power) / 3) + 1
    (2 * total_power) / 3 + 1
}

pub fn has_quorum(accumulated_power: u64, total_voting_power: u64) -> bool {
    accumulated_power >= calculate_quorum_threshold(total_voting_power)
}
```

#### Quorum Threshold Reference Table:

| Total Voting Power ($W_{\text{total}}$) | Quorum Threshold ($W_{\text{quorum}}$) | Max Tolerated Byzantine Power ($f < \frac{1}{3} W$) |
|---|---|---|
| 3 | 3 (100%) | 0 |
| 4 | 3 (75.0%) | 1 |
| 10 | 7 (70.0%) | 3 |
| 100 | 67 (67.0%) | 33 |
| 1,000 | 667 (66.7%) | 333 |
| 1,000,000 | 666,667 (66.6667%) | 333,333 |

---
### 2.5 Byzantine Fault Slashing Logic & Double-Sign Evidence

Aura enforces strict economic security through protocol-level slashing.

#### 2.5.1 Double-Sign Evidence Data Structure
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleSignEvidence {
    pub validator_address: Address,
    pub height: u64,
    pub round: u32,
    pub vote_type: VoteType, // PreVote or PreCommit
    pub block_hash_a: Option<Hash>,
    pub signature_a: Signature,
    pub block_hash_b: Option<Hash>,
    pub signature_b: Signature,
}
```

#### 2.5.2 Verification Rules (`verify_double_sign_evidence`):
1. **Discrepancy Check**: `evidence.block_hash_a != evidence.block_hash_b`.
2. **Context Equivalence**: `height_a == height_b` and `round_a == round_b` and `vote_type_a == vote_type_b`.
3. **Validator Active Check**: `evidence.validator_address` was an active bonded validator at `evidence.height`.
4. **Signature Verification**:
   - Reconstruct Preimage A: `BLAKE3("AURA_VOTE\x00" || height || round || vote_type || block_hash_a)`.
   - Reconstruct Preimage B: `BLAKE3("AURA_VOTE\x00" || height || round || vote_type || block_hash_b)`.
   - `ed25519::verify(validator_pubkey, preimage_a, signature_a) == true`.
   - `ed25519::verify(validator_pubkey, preimage_b, signature_b) == true`.
5. **Unbonding Window Check**:
   $$\text{CurrentHeight} - \text{evidence.height} \le U_{\text{unbonding}} \quad (U_{\text{unbonding}} = 10,000\text{ blocks})$$

#### 2.5.3 Slashing Execution:
When valid `DoubleSignEvidence` is verified (either in a block or consensus reactor):
1. **Slash Ratio**: Slashes $\text{SlashPercentage} = 10\%$ of the validator's total bonded stake. Slashed tokens are permanently burned.
2. **Tombstoning**: Validator status transitions to `ValidatorStatus::Tombstoned`. The validator's public key is permanently blacklisted from ever validating again.
3. **Whistleblower Bounty**: 1% of the slashed stake ($0.1\%$ of original stake) is awarded to the coinbase address of the transaction submitter.
4. **Immediate Validator Set Recalculation**: Slashed voting power is immediately removed from $W_{\text{total}}$.

---

### 2.6 Dynamic Validator Set Management & Epoch Transitions

Validator sets update dynamically at **Epoch Boundaries** ($E = 1000\text{ blocks}$).

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorStatus {
    Active,
    Unbonding { completion_height: u64 },
    Tombstoned,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validator {
    pub address: Address,
    pub public_key: PublicKey,
    pub voting_power: u64,
    pub proposer_priority: i64,
    pub status: ValidatorStatus,
    pub commission_rate_bps: u16, // Basis points (e.g. 500 = 5%)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorSet {
    pub validators: Vec<Validator>,
    pub total_voting_power: u64,
    pub epoch: u64,
}

impl ValidatorSet {
    pub fn validator_set_hash(&self) -> Hash {
        let bytes = bincode::serialize(&self.validators).unwrap();
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"AURA_VALIDATOR_SET\x00");
        hasher.update(&bytes);
        Hash(*hasher.finalize().as_bytes())
    }
}
```

- **Staking Transactions**:
  - `StakeDeposit(amount)`: Increases bonded stake; activates at next epoch.
  - `StakeWithdraw(amount)`: Enters unbonding queue for $10,000$ blocks.
  - `RegisterValidator(pubkey, commission)`: Creates new validator entry.
- **Header Commitment**: Each `BlockHeader` commits `validator_set_hash`, enabling light clients to securely verify validator rotations across epoch boundaries.

---

## 3. R5: P2P Networking & Node Synchronization Specification

### 3.1 P2P Network Architecture & libp2p Stack

Aura implements a modular, high-performance P2P network using `rust-libp2p`.

```
+-------------------------------------------------------------------------------+
|                             Application Layer                                 |
|  +--------------------+  +--------------------+  +-------------------------+  |
|  | Mempool Reactor    |  | Consensus Engine   |  | Node Sync Manager       |  |
|  +--------------------+  +--------------------+  +-------------------------+  |
+-------------------------------------------------------------------------------+
                                       |
+-------------------------------------------------------------------------------+
|                               libp2p Protocols                                |
|  +-----------------------------+  +----------------------------------------+  |
|  | GossipSub v1.1 PubSub Mesh  |  | Kademlia DHT Discovery                 |  |
|  | - /aura/tx/1.0.0            |  | - /aura/kad/1.0.0                      |  |
|  | - /aura/block/1.0.0         |  | - k-bucket routing table (k=20, a=3)   |  |
|  | - /aura/consensus/1.0.0     |  +----------------------------------------+  |
|  +-----------------------------+  +----------------------------------------+  |
|  | Request-Response Sync       |  | Identify & Ping Protocol               |  |
|  | - /aura/sync/blocks/1.0.0   |  | - /ipfs/id/1.0.0                       |  |
|  | - /aura/sync/headers/1.0.0  |  | - /ipfs/ping/1.0.0                     |  |
|  +-----------------------------+  +----------------------------------------+  |
+-------------------------------------------------------------------------------+
                                       |
+-------------------------------------------------------------------------------+
|                           Multiplexing & Framing                              |
|   - Yamux (/yamux/1.0.0) Stream Multiplexing                                  |
|   - 4-Byte Big-Endian Length-Delimited Frame Codec                            |
+-------------------------------------------------------------------------------+
                                       |
+-------------------------------------------------------------------------------+
|                           Transport & Encryption                              |
|   - Noise Protocol (Noise_XX_25519_ChaChaPoly_SHA256)                         |
|   - PeerId derived from Ed25519 Node Identity Public Key                      |
|   - TCP Transport (tokio-based async I/O)                                     |
+-------------------------------------------------------------------------------+
```

---

### 3.2 Kademlia DHT Discovery Specification (`/aura/kad/1.0.0`)

1. **Protocol Identifier**: `/aura/kad/1.0.0`
2. **Node ID & Distance**:
   - `PeerId` derived deterministically from node's Ed25519 identity key.
   - Distance metric: XOR distance between 256-bit SHA-256 digests of `PeerId`s.
3. **Routing Table Parameters**:
   - Bucket size $k = 20$.
   - Concurrency parameter $\alpha = 3$.
   - Refresh interval: 5 minutes (triggering `kad.get_closest_peers(self_peer_id)`).
4. **Bootstrap Procedure**:
   - On node startup, parse configured bootstrap multiaddresses:
     `"/ip4/54.210.10.1/tcp/26656/p2p/12D3KooW..."`
   - Dial bootstrap nodes and initiate Kademlia bootstrap query to populate local $k$-buckets.

---

### 3.3 GossipSub v1.1 PubSub Mesh Specification

Aura enforces strict message propagation rules over GossipSub v1.1.

#### 3.3.1 Topic Definitions

| Topic Name | Purpose | Max Payload Size | Validation Rule |
|---|---|---|---|
| `/aura/tx/1.0.0` | Mempool unconfirmed transactions | 64 KB | Valid signature, non-zero fee, valid nonce, balance $\ge \text{amount} + \text{fee}$ |
| `/aura/block/1.0.0` | Proposed and committed blocks | 4 MB | Valid header hash, valid Merkle roots, valid proposer signature |
| `/aura/consensus/1.0.0` | Low-latency consensus messages | 64 KB | Valid Ed25519 vote signature from active validator; $|H_{\text{msg}} - H_{\text{local}}| \le 1$ |

#### 3.3.2 GossipSub Mesh Configuration
- Target Mesh Degree ($D$): 6
- Low Watermark ($D_{\text{low}}$): 4
- High Watermark ($D_{\text{high}}$): 12
- Gossip Degree ($D_{\text{lazy}}$): 6
- Heartbeat Interval: $1000\text{ ms}$
- Message Cache Memory ($mcache\_len$): 5 heartbeats
- Duplicate Message Cache ($history\_gossip$): 3 heartbeats

#### 3.3.3 Topic Validation Filters (Pre-Gossip Validation)
All nodes install strict `libp2p::gossipsub::TopicSubscriptionFilter` hooks:
```rust
pub enum ValidationResult {
    Accept,  // Valid -> Deliver to app & forward to mesh
    Ignore,  // Duplicate or stale -> Drop without penalty
    Reject,  // Invalid / Malicious -> Drop & penalize peer score
}
```
1. **`/aura/tx/1.0.0` Filter**:
   - Decode bytes -> `Transaction`.
   - Verify `tx.verify_signature() == true`.
   - Reject if payload exceeds 64 KB or fee is lower than minimum relay fee.
2. **`/aura/block/1.0.0` Filter**:
   - Decode bytes -> `FullBlock`.
   - Verify `block.header.height > 0`.
   - Verify `block.header.transactions_root == MerkleTree::compute_root(&block.transactions)`.
3. **`/aura/consensus/1.0.0` Filter**:
   - Decode bytes -> `ConsensusMessage`.
   - Verify message height is within active consensus window ($H_{\text{current}} - 1 \le H \le H_{\text{current}} + 1$).
   - Verify signature against public key of validator in current `ValidatorSet`.

---
### 3.4 Synchronization Protocols & Direct Request-Response

When a node initializes or detects height lag ($H_{\text{peer}} > H_{\text{local}} + 2$), it transitions from gossip-following to the synchronization subsystem.

```
       +-----------------------------------------------------------------+
       |                         Node Sync Manager                       |
       |  - Periodic Status Ping (exchange latest height & block hash)   |
       +-----------------------------------------------------------------+
                                         |
                       +-----------------+-----------------+
                       |                                   |
                       v                                   v
        +-------------------------------+   +-------------------------------+
        |    Full Chain Batch Sync      |   |       Header Fast Sync        |
        |  (/aura/sync/blocks/1.0.0)    |   |   (/aura/sync/headers/1.0.0)  |
        +-------------------------------+   +-------------------------------+
                       |                                   |
        +-------------------------------+   +-------------------------------+
        | 1. Download 100-block batches |   | 1. Download 500-header batches|
        | 2. Verify parent hash links   |   | 2. Verify Quorum Certificates |
        | 3. Verify CommitCertificates  |   | 3. Pivot at target height     |
        | 4. Execute transactions       |   | 4. Download SMT state snapshot|
        | 5. Commit state trie roots    |   | 5. Switch to live GossipSub   |
        +-------------------------------+   +-------------------------------+
```

#### 3.4.1 Full Chain Sync Protocol (`/aura/sync/blocks/1.0.0`)
- **Request Format**:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub struct BlockRequest {
      pub start_height: u64,
      pub max_blocks: u32, // Maximum 100 blocks per batch
  }
  ```
- **Response Format**:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub struct BlockResponse {
      pub blocks: Vec<FullBlock>,
  }
  ```
- **Verification Pipeline**:
  For each block $B_k \in \text{blocks}$:
  1. $B_k.header.height == B_{k-1}.header.height + 1$.
  2. $B_k.header.previous\_hash == B_{k-1}.header.hash()$.
  3. Verify `QuorumCertificate` attached to $B_k$:
     $$\sum_{s \in \text{certificate.signatures}} \text{weight}(s.address) \ge W_{\text{quorum}}$$
  4. Execute all transactions in $B_k$ against persistent state storage.
  5. Verify `computed_state_root == B_k.header.state_root`.

#### 3.4.2 Header Sync Protocol (`/aura/sync/headers/1.0.0`)
- **Request Format**:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub struct HeaderRequest {
      pub start_height: u64,
      pub max_headers: u32, // Maximum 500 headers per batch
  }
  ```
- **Response Format**:
  ```rust
  #[derive(Clone, Debug, Serialize, Deserialize)]
  pub struct HeaderResponse {
      pub headers: Vec<BlockHeader>,
      pub certificates: Vec<QuorumCertificate>,
  }
  ```
- **Fast-Sync Mode**: Verifies header hash chain and Quorum Certificates rapidly, downloads SMT state snapshot at pivot height, and resumes live consensus.

#### 3.4.3 Light Client Verification Protocol
Light clients verify transaction inclusion and account states without downloading full blocks:
```rust
pub struct LightClientVerifier;

impl LightClientVerifier {
    pub fn verify_transaction_inclusion(
        header: &BlockHeader,
        tx_hash: &Hash,
        tx_index: usize,
        proof: &MerkleProof,
    ) -> bool {
        proof.verify(tx_hash, tx_index, &header.transactions_root)
    }

    pub fn verify_account_state(
        header: &BlockHeader,
        address: &Address,
        account_state: &AccountState,
        proof: &SmtProof,
    ) -> bool {
        let account_bytes = bincode::serialize(account_state).unwrap();
        let leaf_hash = blake3::hash(&account_bytes);
        proof.verify(&address.0, leaf_hash.as_bytes(), &header.state_root)
    }
}
```

---

### 3.5 Peer Reputation & Anti-DoS Architecture

To protect against network attacks (Sybil, Eclipse, flooding):
1. **Connection Manager**:
   - `max_connected_peers = 50`
   - `min_outbound_peers = 15`
   - Max 2 inbound connections per `/16` IPv4 subnet (or `/48` IPv6 subnet).
2. **GossipSub Peer Scoring Parameters**:
   - $P_1$ (Time in Mesh): $+0.1$ per heartbeat in mesh (cap $+10$).
   - $P_2$ (First Message Deliveries): $+1.0$ per valid first delivery (cap $+40$).
   - $P_3$ (Mesh Message Delivery Rate): Penalty if delivery falls below expected threshold.
   - $P_4$ (Invalid Messages): $-100.0$ per invalid message submitted.
3. **Thresholds**:
   - `GossipThreshold = -10.0`: Peer stopped from gossiping.
   - `PublishThreshold = -30.0`: Node ignores published messages from peer.
   - `GraylistThreshold = -50.0`: Node closes TCP connection; bans IP for 15 minutes.
   - `DoubleSignEvidence / Malicious Consensus Payload`: Immediate permanent IP ban.

---

## 4. Complete Rust Data Types & Serialization Formats

```rust
// =========================================================================
// Cryptographic & Identifier Types
// =========================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct Hash(pub [u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct PublicKey(pub [u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Signature(pub [u8; 64]);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub struct Address(pub [u8; 20]); // Bech32 "aura1..."

// =========================================================================
// Consensus Messages & Quorum Certificates
// =========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum VoteType {
    PreVote = 0x01,
    PreCommit = 0x02,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedVote {
    pub vote_type: VoteType,
    pub height: u64,
    pub round: u32,
    pub block_hash: Option<Hash>, // None = NIL vote
    pub validator_address: Address,
    pub signature: Signature,
}

impl SignedVote {
    pub fn digest(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"AURA_VOTE\x00");
        hasher.update(&(self.vote_type as u8).to_le_bytes());
        hasher.update(&self.height.to_le_bytes());
        hasher.update(&self.round.to_le_bytes());
        match &self.block_hash {
            Some(h) => {
                hasher.update(&[1u8]);
                hasher.update(&h.0);
            }
            None => {
                hasher.update(&[0u8]);
            }
        }
        Hash(*hasher.finalize().as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub height: u64,
    pub round: u32,
    pub block: Block,
    pub valid_round: Option<u32>,
    pub proposer: Address,
    pub signature: Signature,
}

impl Proposal {
    pub fn digest(&self) -> Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"AURA_PROPOSAL\x00");
        hasher.update(&self.height.to_le_bytes());
        hasher.update(&self.round.to_le_bytes());
        let block_hash = self.block.header.hash();
        hasher.update(&block_hash.0);
        match self.valid_round {
            Some(vr) => {
                hasher.update(&[1u8]);
                hasher.update(&vr.to_le_bytes());
            }
            None => {
                hasher.update(&[0u8]);
            }
        }
        Hash(*hasher.finalize().as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub height: u64,
    pub round: u32,
    pub block_hash: Hash,
    pub signatures: Vec<(Address, Signature)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusMessage {
    Proposal(Proposal),
    PreVote(SignedVote),
    PreCommit(SignedVote),
    SlashEvidence(DoubleSignEvidence),
}

// =========================================================================
// Block & Block Header Structures
// =========================================================================

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub chain_id: u32,
    pub height: u64,
    pub previous_hash: Hash,
    pub timestamp: u64,
    pub state_root: Hash,
    pub transactions_root: Hash,
    pub receipts_root: Hash,
    pub proposer: Address,
    pub round: u32,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let encoded = bincode::serialize(self).expect("Serialization failed");
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"AURA_BLOCK_HEADER\x00");
        hasher.update(&encoded);
        Hash(*hasher.finalize().as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub last_commit_qc: Option<QuorumCertificate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullBlock {
    pub block: Block,
    pub commit_certificate: QuorumCertificate,
}
```

---
### 4.1 Rust Trait Specifications

```rust
// Trait for the PoS-BFT Consensus Reactor
#[async_trait::async_trait]
pub trait ConsensusEngine: Send + Sync {
    async fn handle_proposal(&mut self, proposal: Proposal) -> Result<Option<SignedVote>, ConsensusError>;
    async fn handle_prevote(&mut self, vote: SignedVote) -> Result<Option<SignedVote>, ConsensusError>;
    async fn handle_precommit(&mut self, vote: SignedVote) -> Result<Option<FullBlock>, ConsensusError>;
    async fn handle_slash_evidence(&mut self, evidence: DoubleSignEvidence) -> Result<bool, ConsensusError>;
    async fn handle_timeout(&mut self, step: ConsensusStep, height: u64, round: u32) -> Result<Option<SignedVote>, ConsensusError>;
}

// Trait for Validator Set Management
pub trait ValidatorSetManager: Send + Sync {
    fn current_set(&self) -> &ValidatorSet;
    fn get_validator(&self, address: &Address) -> Option<&Validator>;
    fn select_proposer(&mut self, height: u64, round: u32) -> Validator;
    fn process_double_sign(&mut self, evidence: &DoubleSignEvidence) -> Result<u64, ConsensusError>;
    fn apply_epoch_transition(&mut self, new_validators: Vec<Validator>) -> Result<(), ConsensusError>;
}

// Trait for P2P Network Service
#[async_trait::async_trait]
pub trait NetworkService: Send + Sync {
    async fn broadcast_transaction(&self, tx: Transaction) -> Result<(), NetworkError>;
    async fn broadcast_block(&self, block: FullBlock) -> Result<(), NetworkError>;
    async fn broadcast_consensus_message(&self, msg: ConsensusMessage) -> Result<(), NetworkError>;
    async fn request_blocks(&self, peer: libp2p::PeerId, request: BlockRequest) -> Result<BlockResponse, NetworkError>;
    async fn request_headers(&self, peer: libp2p::PeerId, request: HeaderRequest) -> Result<HeaderResponse, NetworkError>;
}
```

---

## 5. Error Handling & Failure Matrix

| Component | Error Variant | Root Cause | Handling Strategy |
|---|---|---|---|
| **Consensus (R4)** | `InvalidProposerSignature` | Received proposal signature does not match expected proposer key | Drop proposal immediately; penalize peer score; wait for $T_{\text{propose}}$ to issue NIL PreVote |
| **Consensus (R4)** | `LockViolation` | Proposer offered block conflicting with locked block without valid polka proof | Reject block; broadcast `PreVote(H, R, NIL)` |
| **Consensus (R4)** | `InsufficientQuorum` | Cumulative vote weight $< W_{\text{quorum}}$ at timeout expiry | Step timeout fires; broadcast NIL vote for next step; advance round if necessary |
| **Consensus (R4)** | `DoubleSignDetected` | Two conflicting votes from same validator at same height/round | Build `DoubleSignEvidence`, slash 10% stake, tombstone validator, broadcast evidence |
| **Consensus (R4)** | `InvalidStateRootCommit` | Proposed block state root does not match local transaction execution output | Reject proposal; broadcast `PreVote(H, R, NIL)` |
| **P2P (R5)** | `NoiseHandshakeFailed` | Incompatible crypto parameters or bad identity signature | Terminate TCP connection; log warning |
| **P2P (R5)** | `GossipSubValidationReject` | Transaction has invalid signature, negative balance, or bad nonce | Drop message; decrement peer score; do not forward to gossip mesh |
| **P2P (R5)** | `SubnetPeerLimitExceeded` | More than 2 inbound peers from same `/16` IPv4 subnet | Reject incoming TCP connection to mitigate eclipse attacks |
| **Sync (R5)** | `BrokenChainContinuity` | Downloaded batch block $B_k.prev\_hash \ne Hash(B_{k-1})$ | Abort sync session; disconnect peer; retry sync from alternative peer |
| **Sync (R5)** | `InvalidQuorumCertificate` | Commit certificate has insufficient voting power or forged signatures | Disconnect peer; ban peer for 15 minutes; purge unverified batch |

---

## 6. Security Invariants & Threat Mitigations

| Threat Vector | Attack Mechanism | Aura Architectural Defense |
|---|---|---|
| **Equivocation / Double-Signing** | Malicious validator signs conflicting blocks/votes at same height/round | BFT 2-phase lock requires $> 2/3$ stake on single block before commit; conflicting votes produce valid `DoubleSignEvidence` triggering immediate 10% slashing + permanent tombstoning. |
| **Long-Range Attack** | Attacker buys old unbonded validator private keys to forge historical fork | Unbonding horizon ($U = 10,000$ blocks) combined with light client checkpointing and BFT deterministic finality disallows historical rewrites older than unbonding window. |
| **GossipSub Flooding / Spam** | Attacker floods network with invalid transactions/votes | Topic-level pre-validation filters drop invalid messages before forwarding; GossipSub peer scoring demotes and disconnects abusive peers. |
| **Sybil Network Isolation** | Attacker surrounds target node with malicious peers (Eclipse Attack) | Kademlia DHT maintains diverse $k$-buckets; minimum outbound peer quota (15 peers) enforced across distinct IP subnets (/16 CIDR grouping). |
| **Proposer Censorship / Withholding** | Malicious proposer refuses to broadcast block | Timeout $T_{\text{propose}}$ triggers honest NIL PreVotes, advancing round to next deterministic proposer without halting chain progress. |
| **Split-Brain Network Partition** | Network splits into 50-50 disconnected partitions | Consensus halts safely on both sides (neither reaches $> 2/3$ finality threshold); zero fork occurs; resumes automatically upon reconnection. |

---

## 7. Test Harness & Verification Architecture

To validate consensus correctness and P2P robustness, Aura specifies a multi-layer testing harness:

### 7.1 Virtual Consensus Test Harness (Deterministic In-Memory Simulator)
- Simulates $N$ validator nodes in a single process using virtual message channels (`tokio::sync::mpsc`).
- **Fault Injection Scenarios**:
  1. *Honest Majority*: 4 nodes, 100 blocks produced, 0 round timeouts.
  2. *Crash Fault*: 1 node crashes ($f = 1$ in $N = 4$ cluster, where $W_{\text{honest}} = 75\% > 66.7\%$). Block production continues smoothly.
  3. *Liveness Halt*: 2 nodes crash ($f = 2$ in $N = 4$, $W_{\text{honest}} = 50\% < 66.7\%$). Engine enters timeout loop, produces zero forks, resumes when 1 node recovers.
  4. *Byzantine Equivocator*: Node 1 sends dual conflicting `PreVote` messages. Harness verifies `DoubleSignEvidence` generation, signature validation, and stake slashing.
  5. *Lock & Unlock Safety*: Validates that honest validator locked on Block A in Round 0 safely unlocks and commits Block B in Round 1 upon receiving proposal with higher polka proof.

### 7.2 Multi-Node Integration Harness
- Spawns $N$ independent node processes bound to local TCP loopback ports (`127.0.0.1:26656..26660`).
- Verifies:
  - Dynamic Kademlia DHT discovery from a single seed bootstrap node.
  - GossipSub transaction propagation and mempool synchronization.
  - Fast block sync for a newly joined late node (syncing from height 0 to tip).

---

## 8. Next Steps for Module Implementation

1. **Create crate `crates/aura-consensus`**:
   - State machine reactor (`ConsensusEngine`)
   - Proposer priority deficit round-robin algorithm
   - Vote accumulator and `QuorumCertificate` builder
   - Slashing evidence verifier and double-sign proof generator
2. **Create crate `crates/aura-p2p` & `crates/aura-sync`**:
   - libp2p Swarm configuration (Noise XX, Yamux, Kademlia DHT, GossipSub v1.1)
   - Protocol handlers for `/aura/sync/blocks/1.0.0` and `/aura/sync/headers/1.0.0`
   - Light client Merkle and SMT verification routines
