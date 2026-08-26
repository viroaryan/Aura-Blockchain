# Aura Cryptocurrency — Technical Specification Survey Report (R1, R2, R3)

**Author**: Survey Explorer 1  
**Target Architecture**: Modular Rust Workspace  
**Scope**: 
- **R1**: Core Cryptographic Primitives & Blockchain Data Structures
- **R2**: Authenticated State Storage & Crash-Resilient Persistence
- **R3**: Mempool Architecture & Transaction Validation Engine

---

## Table of Contents
1. [Executive Summary & Modular System Architecture](#1-executive-summary--modular-system-architecture)
2. [R1: Core Cryptographic Primitives & Data Structures](#2-r1-core-cryptographic-primitives--data-structures)
   - 2.1 Cryptographic Primitives & Algorithms
   - 2.2 Hierarchical Deterministic Key Derivation (BIP-39 / BIP-44)
   - 2.3 Bech32 Address Encoding Scheme
   - 2.4 Cryptographic Binary Merkle Tree & Inclusion Proofs
   - 2.5 Blockchain Data Structures (Transaction, BlockHeader, Block)
   - 2.6 Deterministic Genesis Block Configuration
3. [R2: Authenticated State Storage & Persistence](#3-r2-authenticated-state-storage--persistence)
   - 3.1 Authenticated State Trie: Sparse Merkle Tree (SMT) Architecture
   - 3.2 Account State Schema & Storage Layout
   - 3.3 Persistent Key-Value Storage & Column Family Design
   - 3.4 Crash-Resilient Write-Ahead Logging (WAL) & Recovery Protocol
   - 3.5 Atomic State Commit, Overlay DB & Rollback Safety
4. [R3: Mempool & Transaction Validation Engine](#4-r3-mempool--transaction-validation-engine)
   - 4.1 Two-Stage Validation Pipeline (Stateless & Stateful)
   - 4.2 Mempool Priority Queue & Fee-Per-Byte Mechanics
   - 4.3 Strict Account Nonce Sequencing & Gap Management
   - 4.4 Anti-DoS Eviction Policies & Replace-By-Fee (RBF)
   - 4.5 Block Inclusion, Chain Reorganization & Lifecycle Management
5. [Interface Contracts & Rust Traits](#5-interface-contracts--rust-traits)
6. [Error Handling & Error Hierarchies](#6-error-handling--error-hierarchies)
7. [Comprehensive Verification & Test Matrix](#7-comprehensive-verification--test-matrix)
8. [Features Discovered & Edge Cases](#8-features-discovered--edge-cases)

---

## 1. Executive Summary & Modular System Architecture

The Aura blockchain is a high-performance, modular, Proof-of-Stake Byzantine Fault Tolerant (PoS-BFT) cryptocurrency node designed from first principles in Rust. This survey report formalizes the technical specifications, byte-level data layouts, cryptographic operations, state storage protocols, and mempool mechanics for the foundational layers (Requirements R1, R2, and R3).

### Recommended Workspace Crate Layout
```
d:/cryptocurrency/
├── Cargo.toml                      # Workspace definition
├── crates/
│   ├── aura-crypto/                # R1: Cryptographic primitives (BLAKE3, Ed25519, Bech32, BIP-39/44, Merkle Tree)
│   ├── aura-core/                  # R1: Domain models (Address, Hash, Tx, Block, Header, Genesis)
│   ├── aura-storage/               # R2: SMT/Trie, Persistent KV engine, WAL, StateDB, OverlayDB
│   ├── aura-mempool/               # R3: Mempool, Validation pipeline, Prioritization, Eviction
│   ├── aura-consensus/             # R4: 2-phase PoS-BFT Engine, Slashing (Survey 2)
│   ├── aura-p2p/                   # R5: libp2p networking, sync, gossip (Survey 2)
│   ├── aura-rpc/                   # R6: JSON-RPC & WebSocket server (Survey 3)
│   └── aura-node/                  # Node CLI entrypoint, configuration, orchestration
└── apps/
    └── block-explorer/             # R7: Next.js block explorer (Survey 3)
```

---

## 2. R1: Core Cryptographic Primitives & Data Structures

### 2.1 Cryptographic Primitives & Algorithms

#### 2.1.1 BLAKE3 256-bit Cryptographic Hashing
- **Algorithm**: BLAKE3 (standard 256-bit output mode, tree-hash structure).
- **Digest Length**: 32 bytes (`[u8; 32]`).
- **Domain Separation**: All hashing routines must utilize explicit domain tags or context-separated prefixes to prevent cross-protocol and cross-structure collision attacks:
  - Transaction Digest: `BLAKE3("AURA_TX\x00" || canonical_tx_bytes)`
  - Block Header Digest: `BLAKE3("AURA_HEADER\x00" || canonical_header_bytes)`
  - Merkle Leaf: `BLAKE3("\x00" || leaf_data)` (RFC 6962 standard)
  - Merkle Internal Node: `BLAKE3("\x01" || left_child || right_child)`
  - SMT Leaf: `BLAKE3("AURA_SMT_LEAF\x00" || key || value_hash)`
  - SMT Internal: `BLAKE3("AURA_SMT_NODE\x00" || left_hash || right_hash)`
- **Data Type**:
```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub const ZERO: Self = Hash([0u8; 32]);
    
    pub fn of(data: &[u8]) -> Self {
        Hash(*blake3::hash(data).as_bytes())
    }
    
    pub fn keyed(key: &[u8; 32], data: &[u8]) -> Self {
        Hash(*blake3::keyed_hash(key, data).as_bytes())
    }
    
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    pub fn from_hex(s: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(s).map_err(|_| CryptoError::InvalidHexFormat)?;
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidHashLength(bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Hash(arr))
    }
}
```

#### 2.1.2 Ed25519 Digital Signatures
- **Curve**: Curve25519 in twisted Edwards form ($ -x^2 + y^2 = 1 - \frac{121665}{121666} x^2 y^2 $).
- **Public Key**: 32 bytes compressed Edwards $Y$-coordinate with sign bit.
- **Secret Key**: 32 bytes scalar seed (or 64-byte expanded secret key).
- **Signature**: 64 bytes $(R, s)$ where $R$ is an encoded point (32 bytes) and $s$ is a scalar modulo $L = 2^{252} + 27742317777372353535851937790883648493$ (32 bytes).
- **Malleability Protection**: Strict canonical scalar and point decoding (`ed25519-dalek` with `strict` mode enabled). Signatures with non-canonical $s \ge L$ or uncompressed points not on the curve must be rejected immediately during verification.
- **Data Types**:
```rust
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(pub [u8; 32]);

#[derive(Clone, Serialize, Deserialize)]
pub struct SecretKey(pub [u8; 32]);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub [u8; 64]);

pub struct KeyPair {
    pub public: PublicKey,
    pub secret: SecretKey,
}
```

### 2.2 Hierarchical Deterministic Key Derivation (BIP-39 / BIP-44)

#### 2.2.1 BIP-39 Mnemonic & Seed Generation
- **Entropy Sources**: 128 bits (12 words), 192 bits (18 words), or 256 bits (24 words).
- **Wordlist**: Standard English (2048 words).
- **Seed Derivation**: PBKDF2-HMAC-SHA512 with 2048 iterations:
  - Password: UTF-8 normalized mnemonic sentence.
  - Salt: `"mnemonic"` + optional user passphrase.
  - Output: 512-bit (64-byte) binary seed.

#### 2.2.2 SLIP-0010 / BIP-44 Derivation for Ed25519
Because standard BIP-32 unhardened derivation is mathematically impossible/insecure for Ed25519, Aura strictly implements **SLIP-0010** (Hardened Ed25519 HD Key Derivation).
- **Master Node Generation**:
  - `I = HMAC-SHA512(Key = b"ed25519 seed", Data = seed)`
  - `master_secret_key = I[0..32]`
  - `master_chain_code = I[32..64]`
- **BIP-44 Standard Derivation Path for Aura**:
  - `m / 44' / 1234' / account' / 0' / address_index'`
  - Purpose: `44'` (hardened BIP-44 standard)
  - Coin Type: `1234'` (registered coin type for Aura Native `AUR`)
  - Account: `0'`, `1'`, ...
  - Change: `0'` (external / receiving addresses only)
  - Index: `0'`, `1'`, `2'`, ...
  - Note: All levels in SLIP-0010 for Ed25519 MUST use hardened child index ($i \ge 2^{31}$, e.g. $i = \text{index} + 0x80000000$).

### 2.3 Bech32 Address Encoding Scheme

#### 2.3.1 Address Derivation
- **Raw Address Representation**: 20 bytes (`[u8; 20]`) computed as the first 20 bytes of the BLAKE3 hash of the 32-byte Ed25519 public key:
  $$\text{AddressBytes} = \text{BLAKE3}(\text{PublicKey})[0..20]$$
- **Human-Readable Part (HRP)**:
  - Mainnet: `"aura"`
  - Testnet: `"taura"`
  - Devnet / Regtest: `"daura"`
- **Bech32 Encoding Algorithm**:
  1. Convert the 20-byte payload from 8-bit groups to 5-bit groups using base32 radix conversion (yielding 32 base32 characters).
  2. Compute 6-character BCH checksum over `HRP` and 5-bit payload per BIP-173.
  3. Concatenate: `HRP + "1" + base32_chars + checksum`.
  - Example Mainnet Address: `aura1q629z6a5wqu58f3v3g0d8n83zvdqug2837r6` (42 characters).

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 20]);

impl Address {
    pub fn from_public_key(pk: &PublicKey) -> Self {
        let hash = blake3::hash(&pk.0);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash.as_bytes()[0..20]);
        Address(addr)
    }

    pub fn to_bech32(&self, hrp: &str) -> Result<String, CryptoError> {
        bech32::encode(hrp, self.0.to_base32(), bech32::Variant::Bech32)
            .map_err(|e| CryptoError::Bech32EncodingError(e.to_string()))
    }

    pub fn from_bech32(s: &str, expected_hrp: &str) -> Result<Self, CryptoError> {
        let (hrp, data, variant) = bech32::decode(s)
            .map_err(|e| CryptoError::Bech32DecodingError(e.to_string()))?;
        if hrp != expected_hrp {
            return Err(CryptoError::InvalidHrp { expected: expected_hrp.to_string(), found: hrp });
        }
        if variant != bech32::Variant::Bech32 {
            return Err(CryptoError::InvalidBech32Variant);
        }
        let bytes: Vec<u8> = bech32::FromBase32::from_base32(&data)
            .map_err(|e| CryptoError::Bech32DecodingError(e.to_string()))?;
        if bytes.len() != 20 {
            return Err(CryptoError::InvalidAddressLength(bytes.len()));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Address(arr))
    }
}
```

### 2.4 Cryptographic Binary Merkle Tree & Inclusion Proofs

Aura blocks utilize a binary Merkle tree with BLAKE3 domain separation (RFC 6962 compliance) for transactions and receipts.

#### 2.4.1 Merkle Tree Construction Rules
1. **Empty Tree**: If a block contains 0 transactions, `transactions_root = Hash::ZERO`.
2. **Leaf Node Hashing**: $\text{LeafHash}_i = \text{BLAKE3}(0x00 \ || \ \text{SerializedTx}_i)$.
3. **Internal Node Hashing**: $\text{ParentHash} = \text{BLAKE3}(0x01 \ || \ \text{LeftChildHash} \ || \ \text{RightChildHash})$.
4. **Odd Count Handling**: If a level has an odd number of nodes $2k+1$, duplicate the last node to form the pair $(N_{2k}, N_{2k})$.

#### 2.4.2 Merkle Inclusion Proof Specification
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofSide {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProofNode {
    pub hash: Hash,
    pub side: ProofSide,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: u32,
    pub total_leaves: u32,
    pub audit_path: Vec<MerkleProofNode>,
}

impl MerkleProof {
    pub fn verify(&self, root: &Hash, leaf_hash: &Hash) -> bool {
        if self.total_leaves == 0 {
            return false;
        }
        if self.total_leaves == 1 {
            return self.audit_path.is_empty() && root == leaf_hash;
        }
        let mut current = *leaf_hash;
        for node in &self.audit_path {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&[0x01]); // Domain tag for internal node
            match node.side {
                ProofSide::Left => {
                    hasher.update(&node.hash.0);
                    hasher.update(&current.0);
                }
                ProofSide::Right => {
                    hasher.update(&current.0);
                    hasher.update(&node.hash.0);
                }
            }
            current = Hash(*hasher.finalize().as_bytes());
        }
        &current == root
    }
}
```

### 2.5 Blockchain Data Structures

#### 2.5.1 Transaction Data Structure & Serialization
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionPayload {
    Transfer {
        memo: Option<String>,
    },
    Stake {
        amount: u128,
    },
    Unstake {
        amount: u128,
    },
    ClaimRewards,
    RegisterValidator {
        commission_bps: u16, // basis points (0-10000)
        consensus_pubkey: PublicKey,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub version: u8,               // Protocol version (currently 1)
    pub chain_id: u32,             // Network chain ID (prevent cross-chain replay)
    pub sender: Address,           // 20-byte sender address
    pub public_key: PublicKey,     // 32-byte Ed25519 signer public key
    pub nonce: u64,                // Monotonically increasing account nonce
    pub recipient: Address,        // 20-byte recipient address
    pub amount: u128,              // Amount in atomic units (1 AUR = 10^9 uAUR)
    pub fee: u128,                 // Transaction fee offered by sender
    pub payload: TransactionPayload,
    pub signature: Signature,      // 64-byte Ed25519 signature
}

impl Transaction {
    /// Compute the signing preimage hash (digest of all fields excluding signature).
    pub fn digest(&self) -> Hash {
        #[derive(Serialize)]
        struct SigningPreimage<'a> {
            version: u8,
            chain_id: u32,
            sender: &'a Address,
            public_key: &'a PublicKey,
            nonce: u64,
            recipient: &'a Address,
            amount: u128,
            fee: u128,
            payload: &'a TransactionPayload,
        }
        let preimage = SigningPreimage {
            version: self.version,
            chain_id: self.chain_id,
            sender: &self.sender,
            public_key: &self.public_key,
            nonce: self.nonce,
            recipient: &self.recipient,
            amount: self.amount,
            fee: self.fee,
            payload: &self.payload,
        };
        let encoded = bincode::serialize(&preimage).expect("Serialization must not fail");
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"AURA_TX_SIGN\x00");
        hasher.update(&encoded);
        Hash(*hasher.finalize().as_bytes())
    }

    /// Complete transaction hash including signature.
    pub fn tx_hash(&self) -> Hash {
        let encoded = bincode::serialize(self).expect("Serialization must not fail");
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"AURA_TX\x00");
        hasher.update(&encoded);
        Hash(*hasher.finalize().as_bytes())
    }

    /// Approximate or exact serialized byte size.
    pub fn size_bytes(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(256) as usize
    }
}
```

#### 2.5.2 BlockHeader & Block Structure
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,              // Block header version
    pub chain_id: u32,             // Chain identifier
    pub height: u64,               // Block height (Genesis = 0)
    pub previous_hash: Hash,       // Hash of parent block header
    pub timestamp: u64,            // UNIX timestamp in milliseconds
    pub state_root: Hash,          // Authenticated SMT state root after applying block
    pub transactions_root: Hash,   // Merkle root of transactions
    pub receipts_root: Hash,       // Merkle root of transaction receipts
    pub proposer: Address,         // Address of validator who authored block
    pub round: u32,                // BFT consensus round number
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let encoded = bincode::serialize(self).expect("Serialization must not fail");
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"AURA_BLOCK_HEADER\x00");
        hasher.update(&encoded);
        Hash(*hasher.finalize().as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub block_hash: Hash,
    pub height: u64,
    pub round: u32,
    pub signatures: Vec<(Address, Signature)>, // Signatures from >2/3 stake
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub last_commit_qc: Option<QuorumCertificate>,
}

impl Block {
    pub fn block_hash(&self) -> Hash {
        self.header.hash()
    }
}
```

### 2.6 Deterministic Genesis Block Configuration

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisAccount {
    pub address: Address,
    pub balance: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisValidator {
    pub address: Address,
    pub public_key: PublicKey,
    pub stake: u128,
    pub commission_bps: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusParams {
    pub block_time_target_ms: u64,    // e.g. 1000 ms (1s block time)
    pub max_block_bytes: usize,       // e.g. 2 * 1024 * 1024 (2 MB)
    pub max_txs_per_block: usize,     // e.g. 5,000 txs
    pub epoch_length_blocks: u64,     // e.g. 3600 blocks
    pub min_stake_amount: u128,       // e.g. 1,000 * 10^9 uAUR
    pub slash_double_sign_bps: u16,   // e.g. 500 (5%)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain_id: u32,
    pub genesis_time_ms: u64,
    pub initial_accounts: Vec<GenesisAccount>,
    pub initial_validators: Vec<GenesisValidator>,
    pub consensus_params: ConsensusParams,
}

impl GenesisConfig {
    pub fn generate_genesis_block(&self, state_root: Hash) -> Block {
        let header = BlockHeader {
            version: 1,
            chain_id: self.chain_id,
            height: 0,
            previous_hash: Hash::ZERO,
            timestamp: self.genesis_time_ms,
            state_root,
            transactions_root: Hash::ZERO,
            receipts_root: Hash::ZERO,
            proposer: Address([0u8; 20]),
            round: 0,
        };
        Block {
            header,
            transactions: Vec::new(),
            last_commit_qc: None,
        }
    }
}
```

---

## 3. R2: Authenticated State Storage & Persistence

### 3.1 Authenticated State Trie: Sparse Merkle Tree (SMT) Architecture

Aura specifies a **256-bit Binary Sparse Merkle Tree (SMT)** with leaf optimization and empty subtree pre-computation.

#### 3.1.1 Key & Value Mapping
- **Key**: 256-bit hash $K = \text{BLAKE3}(\text{AccountAddress})$ or $\text{BLAKE3}(\text{StorageKey})$.
- **Value**: Canonical `bincode` serialized `AccountState` (or raw value bytes).
- **Depth**: Fixed 256 levels (bits $0 \dots 255$ of $K$).

#### 3.1.2 Precomputed Empty Subtree Hashes
To ensure $O(\text{depth})$ efficiency without storing $2^{256}$ empty nodes:
- $\text{empty\_hash}[256] = \text{Hash::ZERO}$
- $\text{empty\_hash}[d] = \text{BLAKE3}(\text{"AURA\_SMT\_NODE\x00"} \ || \ \text{empty\_hash}[d+1] \ || \ \text{empty\_hash}[d+1])$ for $d \in [0..255]$.

#### 3.1.3 SMT Node Types & Compact Tree Representation
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmtNode {
    Empty,
    Leaf {
        key: Hash,          // 256-bit hashed key
        value_hash: Hash,   // BLAKE3 hash of serialized value
    },
    Internal {
        left: Hash,         // Left child node hash
        right: Hash,        // Right child node hash
    },
}

impl SmtNode {
    pub fn hash(&self, depth: usize, empty_hashes: &[Hash; 257]) -> Hash {
        match self {
            SmtNode::Empty => empty_hashes[depth],
            SmtNode::Leaf { key, value_hash } => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"AURA_SMT_LEAF\x00");
                hasher.update(&key.0);
                hasher.update(&value_hash.0);
                Hash(*hasher.finalize().as_bytes())
            }
            SmtNode::Internal { left, right } => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"AURA_SMT_NODE\x00");
                hasher.update(&left.0);
                hasher.update(&right.0);
                Hash(*hasher.finalize().as_bytes())
            }
        }
    }
}
```

#### 3.1.4 SMT Inclusion & Non-Inclusion Proofs
An SMT proof consists of:
1. `key: Hash`
2. `value: Option<Vec<u8>>` (None indicates proof of non-existence)
3. `siblings: Vec<Hash>` (list of 256 sibling hashes along the path from root to leaf)
4. Proof verification recalculates the root hash from leaf/empty up to root and matches `state_root`.

### 3.2 Account State Schema & Storage Layout

```rust
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AccountState {
    pub nonce: u64,                   // Monotonically increasing tx counter
    pub balance: u128,                 // Liquid spendable balance in uAUR
    pub staked_balance: u128,          // Bonded stake for validators/delegators
    pub unbonding_balance: u128,       // Stake in unbonding queue
    pub unbonding_release_height: u64, // Block height when unbonding unlocks
    pub is_validator: bool,            // Validator flag
    pub validator_pubkey: Option<PublicKey>,
}
```

### 3.3 Persistent Key-Value Storage & Column Family Design

The storage backend utilizes an embedded fast key-value store (e.g. RocksDB / Sled / pure-Rust KV engine like `redb`). Data is strictly partitioned into distinct Column Families (CFs) / Key Prefixes:

| Column Family / Prefix | Key Format | Value Format | Purpose |
|---|---|---|---|
| `CF_STATE_NODES` (`n:`) | `NodeHash (32B)` | `Serialized SmtNode` | Content-addressed SMT nodes |
| `CF_ACCOUNTS` (`a:`) | `Address (20B)` | `Serialized AccountState` | Direct account lookup by address |
| `CF_BLOCKS` (`b:`) | `BlockHash (32B)` | `Serialized Block` | Block data by hash |
| `CF_BLOCK_BY_HEIGHT` (`h:`) | `Height (8B big-endian)` | `BlockHash (32B)` | Canonical height-to-hash mapping |
| `CF_TX_LOOKUP` (`t:`) | `TxHash (32B)` | `(Height 8B, Index 4B, Receipt)` | Transaction index and receipt |
| `CF_CHAIN_META` (`m:`) | `ASCII Key (e.g. b"HEAD_HEIGHT")`| `Value Bytes` | Chain head, state root, genesis hash |

### 3.4 Crash-Resilient Write-Ahead Logging (WAL) & Recovery Protocol

To guarantee crash resilience against sudden process termination or power loss, all state transitions write to a dedicated append-only WAL before modifying disk indexes.

#### 3.4.1 WAL Record Wire Format
Each WAL record is formatted as follows:
```
+----------------+----------------+---------------+----------------+----------------+-------------------+----------------+
| Magic (4B)     | RecordLen (4B) | SeqNum (8B)   | OpType (1B)    | Payload (...)  | CRC32/BLAKE3 (4B) | Term (1B)      |
| 0x41 55 52 41  | u32 big-endian | u64 big-endian| Enum (Put/Del) | Variable       | Checksum          | 0x0A           |
+----------------+----------------+---------------+----------------+----------------+-------------------+----------------+
```

#### 3.4.2 WAL Operations
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WalOp {
    BeginBlock { height: u64, block_hash: Hash },
    PutAccount { address: Address, state: AccountState },
    PutNode { hash: Hash, node: SmtNode },
    PutBlock { height: u64, block: Block },
    CommitBlock { height: u64, state_root: Hash },
    RollbackBlock { height: u64 },
}
```

#### 3.4.3 Crash Recovery Protocol
1. On node boot, scan the WAL from the last persistent checkpoint.
2. Verify integrity of each record using the CRC32/BLAKE3 checksum. If a partial/corrupted record is found at the tail (due to mid-write crash), truncate the WAL at the last valid record boundary.
3. If a block has a `BeginBlock` but no matching `CommitBlock`, roll back any intermediate writes for that block.
4. Flush committed entries to persistent KV storage and sync disk.

### 3.5 Atomic State Commit, Overlay DB & Rollback Safety

#### 3.5.1 The `StateOverlay` (Working Buffer)
State changes during block execution occur strictly in an ephemeral memory overlay:
```rust
pub struct StateOverlay {
    pub account_mods: HashMap<Address, Option<AccountState>>, // None = deleted
    pub node_mods: HashMap<Hash, SmtNode>,
}
```

#### 3.5.2 Atomic Commit Workflow
1. Execute transactions sequentially on `StateOverlay`.
2. Compute new SMT root by recalculating node hashes for all modified keys.
3. Assert that computed SMT root matches `BlockHeader.state_root`.
4. Flush `StateOverlay` to WAL file and call `wal.sync_all()`.
5. Apply batch to persistent KV store atomically.
6. Advance canonical chain tip (`HEAD_HEIGHT`, `HEAD_HASH`).

#### 3.5.3 Reorg & Rollback Safety
- Because SMT nodes are content-addressed (`CF_STATE_NODES[NodeHash] -> SmtNode`), previous state roots remain intact in historical storage for $N$ retention blocks (pruning window).
- To rollback to block height $H - 1$:
  1. Load `state_root` from `BlockHeader(H - 1)`.
  2. Set active root to historical `state_root`.
  3. Update `HEAD_HEIGHT` and `HEAD_HASH`.
  4. Zero disk rewrite needed for historical nodes!

---

## 4. R3: Mempool & Transaction Validation Engine

### 4.1 Two-Stage Validation Pipeline

To protect node resources from CPU exhaustion and DoS spam, transaction validation is divided into two discrete stages:

```
Incoming Tx
    │
    ▼
┌────────────────────────────────────────────────────────┐
│ Stage 1: Stateless Validation (Parallel Multi-threaded) │
│ - Serialization integrity & size bounds (100B - 64KB)  │
│ - Chain ID verification (match active chain)           │
│ - Address derivation from public key                   │
│ - Ed25519 cryptographic signature verification         │
│ - Numerical bounds (amount > 0, fee >= min_fee, etc.)  │
└────────────────────────────────────────────────────────┘
    │ (Pass)
    ▼
┌────────────────────────────────────────────────────────┐
│ Stage 2: Stateful Validation (State DB Snapshot Lock)   │
│ - Balance check: Sender.balance >= Amount + Fee        │
│ - Nonce check: Tx.nonce >= Sender.current_nonce        │
│ - Nonce gap check: Tx.nonce < Sender.nonce + MAX_GAP   │
│ - Mempool quota checks (per-sender max tx limit)       │
└────────────────────────────────────────────────────────┘
    │ (Pass)
    ▼
Accepted into Mempool
```

### 4.2 Mempool Priority Queue & Fee-Per-Byte Mechanics

#### 4.2.1 Fee-Per-Byte Metric
Transactions are prioritized strictly by unit fee density:
$$\text{FeePerByte} = \frac{\text{Transaction.fee}}{\text{Transaction.size\_bytes()}}$$
- Unit: $\text{uAUR / Byte}$.
- Minimum Relay Fee: Default $1 \text{ uAUR / Byte}$. Transactions with $\text{FeePerByte} < \text{MIN\_RELAY\_FEE}$ are rejected immediately.

#### 4.2.2 Multi-Index Mempool Storage
The mempool maintains three synchronized indices for $O(1)$ lookups, $O(\log N)$ extraction, and sender-level ordering:
```rust
pub struct Mempool {
    /// Lookup by transaction hash
    pub by_hash: HashMap<Hash, Arc<Transaction>>,
    
    /// Lookup by sender, ordered strictly by nonce
    pub by_sender: HashMap<Address, BTreeMap<u64, Hash>>,
    
    /// Global priority queue ordered by Fee-Per-Byte (descending), then FIFO timestamp
    pub priority_index: BTreeSet<MempoolOrderKey>,
    
    /// Ready queue (transactions where nonce == account.current_nonce)
    pub ready_txs: HashSet<Hash>,
    
    /// Future queue (transactions where nonce > account.current_nonce)
    pub future_txs: HashSet<Hash>,
    
    pub total_bytes: usize,
    pub config: MempoolConfig,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MempoolOrderKey {
    pub fee_per_byte: u128,         // Primary sort key (inverted or descending)
    pub timestamp: u64,             // Secondary sort key (FIFO tie-breaker)
    pub tx_hash: Hash,              // Tertiary unique key
}
```

### 4.3 Strict Account Nonce Sequencing & Gap Management

1. **Sequential Nonce Requirement**:
   - For an account with on-chain nonce $N$:
   - A transaction with nonce $N$ is placed into `ready_txs`.
   - Transactions with nonces $N+1, N+2, \dots, N+K$ are placed into `future_txs` (where $K \le \text{MAX\_FUTURE\_NONCES}$, default 16).
   - If a transaction with nonce $< N$ arrives, it is rejected as a **stale/replayed nonce**.
   - If a transaction with nonce $> N + \text{MAX\_FUTURE\_NONCES}$ arrives, it is rejected as **excessive nonce gap**.
2. **Cascading Promotion**:
   - When transaction $N$ is executed/mined in a block, transaction $N+1$ in `future_txs` is automatically promoted to `ready_txs`.

### 4.4 Anti-DoS Eviction Policies & Replace-By-Fee (RBF)

#### 4.4.1 Global Capacity & DoS Thresholds
```rust
pub struct MempoolConfig {
    pub max_total_transactions: usize,   // Default: 10,000 txs
    pub max_total_bytes: usize,          // Default: 50 MB (52,428,800 bytes)
    pub max_txs_per_sender: usize,       // Default: 64 txs
    pub max_future_gap: u64,             // Default: 16 nonces
    pub min_relay_fee_per_byte: u128,    // Default: 1 uAUR / byte
    pub tx_ttl_seconds: u64,             // Default: 10,800 seconds (3 hours)
    pub rbf_min_fee_bump_bps: u16,       // Default: 1000 (10% fee increase required)
}
```

#### 4.4.2 Eviction on Mempool Saturation
When inserting a transaction would cause `total_bytes > max_total_bytes` or `by_hash.len() > max_total_transactions`:
1. Find the entry with the lowest `fee_per_byte` in the mempool (preferring `future_txs` first, then `ready_txs`).
2. If `incoming_tx.fee_per_byte <= lowest_tx.fee_per_byte`, the incoming transaction is **rejected** (mempool full, fee too low).
3. If `incoming_tx.fee_per_byte > lowest_tx.fee_per_byte`, evict `lowest_tx` along with any dependent future-nonce transactions from that sender, then insert `incoming_tx`.

#### 4.4.3 Replace-By-Fee (RBF) Specification
If an incoming transaction $T_{\text{new}}$ has the **same sender** and **same nonce** as an existing transaction $T_{\text{old}}$ in the pool:
1. $T_{\text{new}}$ must offer a total fee and fee-per-byte at least $(1 + \frac{\text{rbf\_min\_fee\_bump\_bps}}{10000})$ times greater than $T_{\text{old}}$ (e.g. $\ge 10\%$ fee bump).
2. If the fee bump criterion is met:
   - Evict $T_{\text{old}}$ from all indices.
   - Insert $T_{\text{new}}$ into `by_hash`, `by_sender`, and `priority_index`.
3. If the fee bump criterion is NOT met, reject $T_{\text{new}}$ with `MempoolError::FeeBumpTooLow`.

### 4.5 Block Inclusion, Chain Reorganization & Lifecycle Management

#### 4.5.1 Block Production Harvesting
When the consensus engine triggers block creation:
1. Proposer queries `mempool.select_transactions(max_bytes, max_count)`.
2. Transactions are harvested strictly from `ready_txs` in descending `fee_per_byte` order.
3. For each selected transaction from sender $S$, the next sequential nonce for sender $S$ in `future_txs` is speculatively promoted and eligible for inclusion in the same block.

#### 4.5.2 Post-Block Confirmation Cleanup
Upon committing block $B$:
1. For every transaction $T \in B$: remove $T$ from mempool if present.
2. For all senders affected by $B$: query updated account nonce from State DB, discard any remaining transactions with nonce $\le \text{account.nonce}$, and promote newly ready transactions.

#### 4.5.3 Chain Reorganization Handling
If a reorg of depth $D$ occurs:
1. Extract all transactions from the disconnected old fork blocks $B_{\text{old}, 1} \dots B_{\text{old}, D}$.
2. Remove transactions that were already included in the new fork blocks.
3. Re-inject remaining transactions into the mempool via the standard validation pipeline.

---

## 5. Interface Contracts & Rust Traits

To guarantee strict modularity and clean architectural boundaries, the following Rust traits define the formal interfaces for R1, R2, and R3.

```rust
// ==========================================
// R1: CRYPTOGRAPHIC ENGINE TRAIT
// ==========================================
pub trait CryptoEngine: Send + Sync {
    fn hash(data: &[u8]) -> Hash;
    fn hash_leaf(data: &[u8]) -> Hash;
    fn hash_internal(left: &Hash, right: &Hash) -> Hash;
    fn verify_signature(pk: &PublicKey, message_hash: &Hash, sig: &Signature) -> Result<(), CryptoError>;
    fn sign(sk: &SecretKey, message_hash: &Hash) -> Signature;
    fn derive_address(pk: &PublicKey) -> Address;
    fn derive_from_mnemonic(mnemonic: &str, passphrase: &str, derivation_path: &str) -> Result<KeyPair, CryptoError>;
}

// ==========================================
// R1: MERKLE TREE TRAIT
// ==========================================
pub trait MerkleTreeEngine {
    fn compute_root(leaves: &[Hash]) -> Hash;
    fn generate_proof(leaves: &[Hash], index: usize) -> Result<MerkleProof, CryptoError>;
    fn verify_proof(root: &Hash, leaf: &Hash, proof: &MerkleProof) -> bool;
}

// ==========================================
// R2: STATE STORAGE & AUTHENTICATED TRIE TRAIT
// ==========================================
pub trait StateStore: Send + Sync {
    fn get_account(&self, address: &Address) -> Result<Option<AccountState>, StorageError>;
    fn get_state_root(&self) -> Hash;
    fn insert_account(&mut self, address: Address, state: AccountState) -> Result<(), StorageError>;
    fn delete_account(&mut self, address: &Address) -> Result<(), StorageError>;
    fn commit_block(&mut self, height: u64, block_hash: Hash) -> Result<Hash, StorageError>;
    fn rollback_to(&mut self, height: u64, state_root: Hash) -> Result<(), StorageError>;
    fn get_smt_proof(&self, address: &Address) -> Result<SmtProof, StorageError>;
    fn verify_smt_proof(&self, root: Hash, address: &Address, proof: &SmtProof) -> bool;
}

// ==========================================
// R2: LOW-LEVEL PERSISTENT KV STORE & WAL TRAIT
// ==========================================
pub trait KeyValueStore: Send + Sync {
    fn get(&self, cf: &str, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    fn put(&self, cf: &str, key: &[u8], value: &[u8]) -> Result<(), StorageError>;
    fn delete(&self, cf: &str, key: &[u8]) -> Result<(), StorageError>;
    fn write_batch(&self, batch: StorageBatch) -> Result<(), StorageError>;
    fn flush(&self) -> Result<(), StorageError>;
}

// ==========================================
// R3: MEMPOOL & TRANSACTION VALIDATION TRAIT
// ==========================================
pub trait MempoolEngine: Send + Sync {
    fn validate_stateless(&self, tx: &Transaction) -> Result<(), ValidationError>;
    fn validate_stateful(&self, tx: &Transaction, state: &dyn StateStore) -> Result<(), ValidationError>;
    fn insert(&mut self, tx: Transaction, state: &dyn StateStore) -> Result<Hash, MempoolError>;
    fn remove(&mut self, tx_hash: &Hash) -> Option<Arc<Transaction>>;
    fn select_transactions(&mut self, max_bytes: usize, max_count: usize) -> Vec<Arc<Transaction>>;
    fn on_block_committed(&mut self, block: &Block, state: &dyn StateStore);
    fn on_chain_reorg(&mut self, disconnected_blocks: &[Block], state: &dyn StateStore);
    fn len(&self) -> usize;
    fn total_bytes(&self) -> usize;
}
```

---

## 6. Error Handling & Error Hierarchies

```rust
#[derive(thiserror::Error, Debug)]
pub enum CryptoError {
    #[error("Invalid hash length: expected 32 bytes, got {0}")]
    InvalidHashLength(usize),
    #[error("Invalid hex format")]
    InvalidHexFormat,
    #[error("Ed25519 signature verification failed")]
    SignatureVerificationFailed,
    #[error("Non-canonical or malformed Ed25519 signature")]
    MalformedSignature,
    #[error("Bech32 encoding error: {0}")]
    Bech32EncodingError(String),
    #[error("Bech32 decoding error: {0}")]
    Bech32DecodingError(String),
    #[error("Invalid HRP: expected {expected}, found {found}")]
    InvalidHrp { expected: String, found: String },
    #[error("Invalid Bech32 variant (expected RFC 173 Standard Bech32)")]
    InvalidBech32Variant,
    #[error("Invalid address length: expected 20 bytes, got {0}")]
    InvalidAddressLength(usize),
    #[error("BIP-39 mnemonic error: {0}")]
    MnemonicError(String),
    #[error("Key derivation path error: {0}")]
    DerivationPathError(String),
    #[error("Merkle proof generation error: {0}")]
    MerkleProofError(String),
}

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("Underlying KV store IO error: {0}")]
    IoError(String),
    #[error("Corrupted WAL record at sequence {seq}: {reason}")]
    WalCorrupted { seq: u64, reason: String },
    #[error("State root mismatch: expected {expected}, computed {computed}")]
    StateRootMismatch { expected: Hash, computed: Hash },
    #[error("Account not found: {0:?}")]
    AccountNotFound(Address),
    #[error("Trie node missing from database: {0}")]
    NodeNotFound(Hash),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Rollback failed: target height {height} state root {root} not found")]
    RollbackFailed { height: u64, root: Hash },
}

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("Invalid transaction version: {0}")]
    InvalidVersion(u8),
    #[error("Chain ID mismatch: expected {expected}, got {got}")]
    ChainIdMismatch { expected: u32, got: u32 },
    #[error("Transaction size out of bounds: {size} bytes (min: {min}, max: {max})")]
    SizeOutOfBounds { size: usize, min: usize, max: usize },
    #[error("Sender address does not match public key: addr {addr}, pk_addr {pk_addr}")]
    AddressMismatch { addr: Address, pk_addr: Address },
    #[error("Cryptographic signature invalid: {0}")]
    InvalidSignature(#[from] CryptoError),
    #[error("Amount is zero or invalid")]
    InvalidAmount,
    #[error("Fee {fee} is below minimum required {min_fee}")]
    FeeTooLow { fee: u128, min_fee: u128 },
    #[error("Arithmetic overflow in amount + fee")]
    ArithmeticOverflow,
    #[error("Account has insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u128, available: u128 },
    #[error("Invalid account nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },
    #[error("Nonce gap too large: current {current}, got {got}, max gap {max_gap}")]
    NonceGapTooLarge { current: u64, got: u64, max_gap: u64 },
}

#[derive(thiserror::Error, Debug)]
pub enum MempoolError {
    #[error("Transaction validation failed: {0}")]
    Validation(#[from] ValidationError),
    #[error("Transaction already exists in mempool: {0}")]
    AlreadyExists(Hash),
    #[error("Mempool is full and incoming fee {incoming_fee_per_byte} is too low for eviction")]
    MempoolFull { incoming_fee_per_byte: u128 },
    #[error("Sender exceeded maximum pending transactions limit ({0})")]
    SenderQuotaExceeded(usize),
    #[error("Replace-By-Fee rejected: fee bump {bump_bps} bps below required {required_bps} bps")]
    FeeBumpTooLow { bump_bps: u16, required_bps: u16 },
}
```

---

## 7. Comprehensive Verification & Test Matrix

The following test suites must be developed to verify 100% compliance across R1, R2, and R3:

### 7.1 R1: Cryptographic & Data Structure Test Suite
1. **BLAKE3 Test Vector Suite**:
   - Verify NIST-standard and empty input vectors.
   - Verify domain separation: `BLAKE3("AURA_TX\x00" || data) != BLAKE3("AURA_BLOCK\x00" || data)`.
2. **Ed25519 Canonical Verification**:
   - Valid keypair generation, message signing, and signature verification.
   - Tamper test: Altering single bit in message, public key, or signature must fail verification.
   - Malleability test: Non-canonical $S \ge L$ or high-order points must be rejected.
3. **BIP-39 & BIP-44 Derivation Vectors**:
   - Test against official BIP-39 test vectors (12, 18, 24 words).
   - Test SLIP-0010 derivation path `m/44'/1234'/0'/0'/0'` producing identical Ed25519 keypairs deterministically.
4. **Bech32 Encoding/Decoding**:
   - Valid conversions between `Address([u8; 20])` and `aura1...` Bech32 string.
   - Checksum error detection: Mutating any character in the Bech32 string triggers decode error.
   - HRP rejection: Supplying `"btc1..."` or `"cosmos1..."` string fails with `InvalidHrp`.
5. **Merkle Tree Inclusion & Tamper Tests**:
   - Generate Merkle tree for $1, 2, 3, 4, 7, 8, 15, 16, 100$ transactions.
   - Generate and verify inclusion proofs for all leaf indices.
   - Negative test: Altering transaction in leaf or proof node hash fails verification.
6. **Block Chain Tamper Detection**:
   - Construct a chain of 10 blocks where Block $N+1$ references `BlockHeader(N).hash()`.
   - Modifying a single transaction in Block 3 alters its `transactions_root`, which alters `BlockHeader(3).hash()`, breaking the chain at Block 4.

### 7.2 R2: Authenticated State Storage & Persistence Test Suite
1. **SMT Determinism Test**:
   - Insert 1000 accounts in random order vs sequential order; verify resulting `state_root` is **identical**.
2. **SMT Inclusion & Non-Inclusion Proofs**:
   - Verify cryptographic proofs for existing accounts.
   - Verify non-inclusion proofs for nonexistent addresses.
3. **Crash Recovery & WAL Replay Simulation**:
   - Execute 10 blocks with 50 txs each; simulate abrupt crash before flushing KV store.
   - Re-open storage engine; verify WAL automatically replays and recovers to the exact `state_root` of Block 10.
   - Inject simulated corrupted byte at tail of WAL file; verify recovery safely truncates corrupted tail without crashing.
4. **Atomic Commit & Rollback Safety**:
   - Test state rollback from Block 10 to Block 7.
   - Verify account balances and nonces at Block 7 match pre-Block 8 state exactly.
   - Verify new branch (Block 8', 9') can be appended seamlessly.

### 7.3 R3: Mempool & Transaction Validation Test Suite
1. **Fee-Per-Byte Priority Ordering**:
   - Insert 100 transactions with varying sizes (200B - 2000B) and fees (100 - 100,000 uAUR).
   - Call `select_transactions()`; assert returned list is sorted strictly descending by `fee / size`.
2. **Strict Nonce Sequencing & Gap Promotion**:
   - Insert txs for Sender A with nonces 0, 1, 2, 4.
   - Txs 0, 1, 2 must be marked ready; Tx 4 must be queued in future.
   - Insert Tx 3; Tx 3 and Tx 4 must automatically promote to ready.
3. **Replace-By-Fee (RBF)**:
   - Insert Tx(nonce=0, fee=1000).
   - Insert Tx(nonce=0, fee=1050) -> Rejected (`FeeBumpTooLow`, only +5%).
   - Insert Tx(nonce=0, fee=1200) -> Accepted, replacing old transaction.
4. **Anti-DoS Mempool Eviction**:
   - Configure mempool with `max_total_transactions = 100`.
   - Fill mempool with 100 txs with `fee_per_byte = 10`.
   - Insert tx with `fee_per_byte = 5` -> Rejected.
   - Insert tx with `fee_per_byte = 20` -> Accepted, evicts lowest fee tx.
5. **Reorg Re-injection**:
   - Commit 3 blocks; simulate 2-block reorg.
   - Verify unconfirmed transactions from discarded blocks return to mempool without data loss.

---

## 8. Features Discovered & Edge Cases

### 8.1 Features Discovered

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|----------|---------|-------------|--------|---------|----------------|----------------|
| 1 | Cryptography | SLIP-0010 Ed25519 HD Derivation | Deterministic hardened key derivation for Ed25519 (BIP-44 path) | Seed, Derivation Path `m/44'/1234'/0'/0'/0'` | KeyPair (SecretKey, PublicKey) | `CryptoError::DerivationPathError` | R1 Analysis |
| 2 | Cryptography | Bech32 Address Derivation | Standardized 20-byte address with `aura` HRP and BCH checksum | PublicKey | Bech32 String (`aura1...`) | `CryptoError::Bech32EncodingError` | R1 Analysis |
| 3 | Cryptography | RFC 6962 Binary Merkle Tree | Tree hashing with $0x00$ leaf and $0x01$ node domain tags | Slice of Transaction Hashes | Merkle Root Hash (`[u8; 32]`) | Empty slice yields `Hash::ZERO` | R1 Analysis |
| 4 | State Storage | 256-bit Binary Sparse Merkle Tree | Authenticated authenticated state root calculation with empty subtree pre-calculation | Key-Value pairs | `state_root: Hash`, SmtProof | `StorageError::StateRootMismatch` | R2 Analysis |
| 5 | State Storage | Crash-Resilient WAL | Append-only checksummed write-ahead log for durability | `WalOp` records | Disk persistence, crash recovery | `StorageError::WalCorrupted` | R2 Analysis |
| 6 | State Storage | Atomic Overlay DB & Time-Travel Rollback | Speculative state execution in overlay, instantaneous rollback via content-addressed node hashes | Target Block Height, State Root | Restored state | `StorageError::RollbackFailed` | R2 Analysis |
| 7 | Mempool | Two-Stage Parallel Validation | Stateless validation across CPU cores + stateful DB check | `Transaction`, `StateStore` | `Result<(), ValidationError>` | `ValidationError` enum | R3 Analysis |
| 8 | Mempool | Fee-Per-Byte Priority Queue | Multi-index transaction pool with $O(\log N)$ priority extraction | Transactions | Ordered iterator of transactions | `MempoolError` enum | R3 Analysis |
| 9 | Mempool | Strict Nonce Sequencing & Gap Queue | Nonce sequence enforcement with future gap queue and cascading promotions | `Transaction` with nonce $N$ | Categorized as Ready or Future | `ValidationError::InvalidNonce` | R3 Analysis |
| 10 | Mempool | Anti-DoS Saturation Eviction & RBF | Lowest fee density eviction when full + Replace-By-Fee (+10% bump) | Replacement Tx, High-fee Tx | Eviction of stale/low-fee entries | `MempoolError::FeeBumpTooLow` | R3 Analysis |

### 8.2 Edge Cases

| # | Feature | Input / Condition | Observed / Specified Behavior |
|---|---------|-------------------|-------------------------------|
| 1 | Merkle Tree | 0 transactions in block | Returns `Hash::ZERO`; proof verification returns false. |
| 2 | Merkle Tree | Odd number of leaves (e.g. 3 transactions) | Last leaf is duplicated to form a pair at each level per RFC 6962. |
| 3 | Ed25519 Signatures | Non-canonical scalar $S \ge L$ or point with high-order component | Verification fails with `CryptoError::MalformedSignature` (strict malleability rejection). |
| 4 | Bech32 Addresses | Mixed case or invalid checksum character | Bech32 decoding fails immediately; case-folding is enforced. |
| 5 | State Storage | Duplicate state insertion in different order | SMT produces bit-for-bit identical `state_root` regardless of insertion order. |
| 6 | State Storage | Crash during WAL write (partial record at tail) | Engine detects checksum mismatch, truncates partial record, and recovers clean state. |
| 7 | Mempool | Account submits nonce $N+5$ when current nonce is $N$ | Placed in `future_txs` queue; will not be included in block until nonces $N \dots N+4$ arrive. |
| 8 | Mempool | Account submits nonce $N+20$ (gap $> \text{MAX\_FUTURE\_GAP}$) | Rejected immediately with `ValidationError::NonceGapTooLarge`. |
| 9 | Mempool | Account balance $< \text{Amount} + \text{Fee}$ | Rejected during stateful validation with `ValidationError::InsufficientBalance`. |
| 10 | Mempool | Replacement tx submitted with identical nonce and identical fee | Rejected with `MempoolError::FeeBumpTooLow` (must exceed prior fee by $\ge 10\%$). |
| 11 | Mempool | Deep chain reorganization (fork switch) | Disconnected block transactions are re-validated and re-inserted into mempool without dropping valid txs. |

---

## 9. Conclusion & Next Steps

This survey establishes the complete mathematical, architectural, and interface specifications for Requirements R1, R2, and R3. All data structures are fully specified with Serde/Bincode serialization, zero ambiguity in binary representations, and clear error hierarchies.

The findings from this report can now be synthesized directly into the master `PROJECT.md` by the Orchestrator, enabling immediate modular milestone implementation by the specialist workers.
