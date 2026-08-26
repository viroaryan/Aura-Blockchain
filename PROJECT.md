# Project: Aura Cryptocurrency

## Architecture

Aura is a high-performance, modular, Proof-of-Stake Byzantine Fault Tolerant (PoS-BFT) cryptocurrency node and ecosystem built in Rust with a Next.js block explorer dashboard.

```
+-----------------------------------------------------------------------------------------------+
|                                    Aura Ecosystem Architecture                                |
+-----------------------------------------------------------------------------------------------+
|                                                                                               |
|  [ User / Web ] <---> [ Next.js Block Explorer (explorer/) ]                                  |
|                                | (JSON-RPC 2.0 / WebSocket)                                   |
|                                v                                                              |
|  +-----------------------------------------------------------------------------------------+  |
|  | aura-node (Main Daemon Orchestrator & CLI)                                              |  |
|  |                                                                                         |  |
|  |  +-----------------------------------+  +--------------------------------------------+  |  |
|  |  | aura-rpc                          |  | aura-metrics                               |  |  |
|  |  | - JSON-RPC 2.0 Server             |  | - Prometheus Exporter (:9090/metrics)      |  |  |
|  |  | - WebSocket Pub/Sub Subscriptions|  | - 6 Subsystem Metrics Catalogs             |  |  |
|  |  +-----------------+-----------------+  +--------------------------------------------+  |  |
|  |                    |                                                                    |  |
|  |  +-----------------v-----------------+  +--------------------------------------------+  |  |
|  |  | aura-mempool                      |  | aura-p2p                                   |  |  |
|  |  | - Two-Stage Validation Pipeline   |  | - libp2p Networking (Noise + Yamux)        |  |  |
|  |  | - Fee-Per-Byte Priority Queue     |  | - Kademlia DHT Peer Discovery              |  |  |
|  |  | - Strict Nonce Sequencing (Ready/ |  | - GossipSub (blocks, txs, consensus)       |  |  |
|  |  |   Future Queues)                  |  | - Full Sync, Header Sync, Light Client     |  |  |
|  |  | - Anti-DoS Eviction & RBF (>=10%) |  +---------------------+----------------------+  |  |
|  |  +-----------------+-----------------+                        |                         |  |
|  |                    |                                          |                         |  |
|  |  +-----------------v------------------------------------------v----------------------+  |  |
|  |  | aura-consensus (2-Phase PoS-BFT Engine)                                           |  |  |
|  |  | - Propose -> Pre-vote -> Pre-commit -> Commit Steps                                   |  |  |
|  |  | - Deterministic Stake-Weighted Proposer Selection                                 |  |  |
|  |  | - >2/3 Stake Quorum Certificate (QC) Finality Threshold                           |  |  |
|  |  | - Byzantine Double-Signing Slashing & Jailing                                      |  |  |
|  |  +-----------------+-----------------------------------------------------------------+  |  |
|  |                    |                                                                    |  |
|  |  +-----------------v-----------------+  +--------------------------------------------+  |  |
|  |  | aura-storage                      |  | aura-core & aura-crypto                    |  |  |
|  |  | - 256-bit Binary SMT              |  | - BLAKE3 Hashing (Domain Separated)        |  |  |
|  |  | - Account State Schema            |  | - Ed25519 Deterministic Signatures         |  |  |
|  |  | - Persistent KV Engine (Column    |  | - Bech32 Address Derivation (`aura`)       |  |  |
|  |  |   Families: n:, a:, b:, h:, t:, m:)| - SLIP-0010 / BIP-44 HD Derivation           |  |  |
|  |  | - Crash-Resilient WAL & Rollback  |  | - RFC 6962 Binary Merkle Tree & Proofs     |  |  |
|  |  | - StateOverlay Atomic Commit     |  | - Block, Header, Tx, Genesis Types         |  |  |
|  |  +-----------------------------------+  +--------------------------------------------+  |  |
|  +-----------------------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------------------+
```

## Feature Inventory

Every feature enumerated during the survey phase is categorized below and assigned to an explicit milestone.

| # | Feature | Description | Milestone | Source |
|---|---|---|---|---|
| 1 | BLAKE3 256-bit Hashing | Domain-separated hashing for transactions, headers, Merkle trees, and SMT | M1 | R1 |
| 2 | Ed25519 Signatures | Deterministic key generation, signing, and strict verification (malleability protection) | M1 | R1 |
| 3 | Bech32 Address Encoding | Base32 conversion and BIP-173 Bech32 checksum with `aura` HRP | M1 | R1 |
| 4 | BIP-39 / BIP-44 HD Wallet | Mnemonic generation and SLIP-0010 hardened Ed25519 derivation (`m/44'/1234'/...`) | M1 | R1 |
| 5 | RFC 6962 Binary Merkle Tree | Transaction tree with leaf domain `\x00` and internal domain `\x01` + inclusion proofs | M1 | R1 |
| 6 | Core Blockchain Domain Types | Hash, Address, Signature, PublicKey, Transaction, BlockHeader, Block, QC | M1 | R1 |
| 7 | Deterministic Genesis | Genesis accounts, initial validators, consensus parameters, and genesis block generation | M1 | R1 |
| 8 | 256-bit Binary SMT | Sparse Merkle Tree with precomputed empty hashes, leaf optimization & proofs | M2 | R2 |
| 9 | Account State Storage | Account balance, nonce, staked balance, unbonding queue, and validator flags | M2 | R2 |
| 10 | Persistent KV Engine | Pure-Rust embedded key-value storage with column families (`n:`, `a:`, `b:`, `h:`, `t:`, `m:`) | M2 | R2 |
| 11 | Crash-Resilient WAL | Append-only WAL with CRC32/BLAKE3 checksums, truncation on recovery, and sync | M2 | R2 |
| 12 | StateOverlay & Rollback | Ephemeral execution overlay, atomic disk commit, and zero-rewrite historical rollback | M2 | R2 |
| 13 | Stateless Validation | Stage 1 multi-threaded validation: sig checks, chain ID, size bounds, numerical checks | M3 | R3 |
| 14 | Stateful Validation | Stage 2 snapshot validation: balance verification and strict nonce sequencing | M3 | R3 |
| 15 | Fee-Per-Byte Mempool Queue | Multi-index priority queue ordered by `fee_per_byte` descending and FIFO tie-breaker | M3 | R3 |
| 16 | Nonce Gap & Ready/Future Queues | Ready queue (`nonce == account.nonce`) and future queue with cascading promotion | M3 | R3 |
| 17 | Anti-DoS Eviction & RBF | Byte capacity limits, TTL expiry, lowest-fee eviction, and $\ge 10\%$ fee bump RBF | M3 | R3 |
| 18 | Block Harvesting & Reorg Cleanup| FIFO/Priority transaction harvesting for proposers, post-commit cleanup & reorg re-injection | M3 | R3 |
| 19 | 2-Phase PoS-BFT Consensus | Propose $\rightarrow$ Pre-vote $\rightarrow$ Pre-commit $\rightarrow$ Commit state machine | M4 | R4 |
| 20 | Stake-Weighted Proposer Selection | Deterministic round proposer election weighted by validator active stake | M4 | R4 |
| 21 | 2/3+ Stake Finality & QC | Quorum Certificate generation and verification over $>2/3$ stake weight | M4 | R4 |
| 22 | Round Timeout & View Change | Pacemaker module with exponential backoff on stalled rounds | M4 | R4 |
| 23 | Slashing & Double-Sign Detection | Cryptographic double-sign evidence verification, stake slashing, and jailing | M4 | R4 |
| 24 | libp2p P2P Networking | TCP transport, Noise encryption, Yamux multiplexing, peer connection lifecycle | M5 | R5 |
| 25 | Kademlia DHT Peer Discovery | Distributed routing table, peer discovery, bootstrap node bootstrap | M5 | R5 |
| 26 | GossipSub Message Propagation | Pub/Sub topics for blocks (`aura/blocks`), txs (`aura/txs`), and consensus (`aura/consensus`) | M5 | R5 |
| 27 | Full Chain Sync Protocol | Batch historical block download, verification, and state application | M5 | R5 |
| 28 | Header Sync & Light Client Sync | Header-first validation and Merkle inclusion proof light client verification | M5 | R5 |
| 29 | JSON-RPC 2.0 API Server | `getBlockHeight`, `getBlockByHeight`, `getBlockByHash`, `getTransaction`, `getBalance`, `sendTransaction`, `getValidators`, `getNodeInfo`, `getGenesis` | M6 | R6 |
| 30 | WebSocket Pub/Sub Server | Live push streaming for `newHeads`, `newTransactions`, `pendingTransactions`, `validatorUpdates`, `networkHealth` | M6 | R6 |
| 31 | Prometheus Metrics Registry | Exporter on `:9090/metrics` covering consensus, mempool, storage, P2P, RPC, and system | M6 | R6 |
| 32 | Daemon CLI Orchestrator | `aura-node` CLI runtime combining storage, mempool, consensus, P2P, and RPC | M6 | R6 |
| 33 | Next.js Block Explorer UI | Next.js 14+ App Router dashboard with dark-mode crypto terminal design | M7 | R7 |
| 34 | Universal OmniSearch | Search router resolving block height, block hash, tx hash, and Bech32 address | M7 | R7 |
| 35 | Live Telemetry & Merkle Inspector | Real-time WebSocket streaming, live TPS charts, interactive Merkle proof visualizer | M7 | R7 |
| 36 | Multi-Node Test Harness | In-process multi-node cluster orchestrator (`aura-test-harness`) with Byzantine injection | Track A | E2E |
| 37 | E2E 4-Tier Test Suite | Comprehensive opaque-box test runner covering Tiers 1-4 | Track A | E2E |
| 38 | Adversarial Coverage Hardening | Tier 5 white-box stress testing, chaos testing, edge case mutations | M-Final | Final |

## Milestones

| # | Name | Scope | Dependencies | Status |
|---|---|---|---|---|
| M1 | Core Cryptography & Data Structures | `aura-crypto`, `aura-core` (BLAKE3, Ed25519, Bech32, BIP-39/44, Merkle Tree, Block/Header/Tx/Genesis) | none | PLANNED |
| M2 | Authenticated State Storage | `aura-storage` (256-bit Binary SMT, Account state, KV engine, WAL, StateOverlay, Rollback) | M1 | PLANNED |
| M3 | Mempool & Transaction Validation | `aura-mempool` (Stateless/Stateful validation, Priority queue, Ready/Future nonces, DoS eviction, RBF, Harvesting) | M1, M2 | PLANNED |
| M4 | PoS-BFT Consensus Engine & Slashing | `aura-consensus` (2-phase BFT, Proposer election, 2/3+ QC finality, Pacemaker view change, Slashing) | M1, M2, M3 | PLANNED |
| M5 | P2P Networking & Node Synchronization | `aura-p2p` (libp2p, Noise, Yamux, Kademlia DHT, GossipSub, Full sync, Header sync, Light client) | M1, M2, M3, M4 | PLANNED |
| M6 | RPC Server, Metrics & Node Daemon | `aura-rpc`, `aura-metrics`, `aura-node` (JSON-RPC 2.0, WebSocket Pub/Sub, Prometheus metrics, Node CLI daemon) | M1, M2, M3, M4, M5 | PLANNED |
| M7 | Block Explorer Web Application | `explorer/` (Next.js 14+ dashboard, OmniSearch, Live feeds, Merkle visualizer, Validator dashboard) | M6 | PLANNED |
| M-Final | Final E2E Verification & Hardening | Pass 100% E2E test suite (Tiers 1-4) + Adversarial coverage hardening (Tier 5) | M1-M7, TEST_READY | PLANNED |

## Interface Contracts

### 1. `aura-crypto` $\leftrightarrow$ All Crates
- `Hash`: `[u8; 32]` with `Hash::of(&[u8]) -> Hash`, `Hash::keyed(&[u8; 32], &[u8]) -> Hash`.
- `PublicKey`: `[u8; 32]`, `SecretKey`: `[u8; 32]`, `Signature`: `[u8; 64]`.
- `CryptoEngine::sign(&SecretKey, &Hash) -> Signature`.
- `CryptoEngine::verify_signature(&PublicKey, &Hash, &Signature) -> Result<(), CryptoError>`.
- `Address::from_public_key(&PublicKey) -> Address`, `Address::to_bech32(&self, hrp: &str) -> Result<String, CryptoError>`.
- `MerkleTreeEngine::compute_root(&[Hash]) -> Hash`, `MerkleTreeEngine::generate_proof(&[Hash], usize) -> Result<MerkleProof, CryptoError>`.

### 2. `aura-core` $\leftrightarrow$ `aura-storage`, `aura-mempool`, `aura-consensus`
- `Transaction`: `version: u8, chain_id: u32, sender: Address, public_key: PublicKey, nonce: u64, recipient: Address, amount: u128, fee: u128, payload: TransactionPayload, signature: Signature`.
- `Transaction::digest(&self) -> Hash`, `Transaction::tx_hash(&self) -> Hash`, `Transaction::size_bytes(&self) -> usize`.
- `BlockHeader`: `version: u32, chain_id: u32, height: u64, previous_hash: Hash, timestamp: u64, state_root: Hash, transactions_root: Hash, receipts_root: Hash, proposer: Address, round: u32`.
- `Block`: `header: BlockHeader, transactions: Vec<Transaction>, last_commit_qc: Option<QuorumCertificate>`.

### 3. `aura-storage` $\leftrightarrow$ `aura-mempool`, `aura-consensus`, `aura-rpc`
- `StateStore::get_account(&self, &Address) -> Result<Option<AccountState>, StorageError>`.
- `StateStore::get_state_root(&self) -> Hash`.
- `StateStore::insert_account(&mut self, Address, AccountState) -> Result<(), StorageError>`.
- `StateStore::commit_block(&mut self, height: u64, block: &Block) -> Result<Hash, StorageError>`.
- `StateStore::rollback_to(&mut self, height: u64) -> Result<(), StorageError>`.
- `StateStore::get_block_by_height(&self, height: u64) -> Result<Option<Block>, StorageError>`.
- `StateStore::get_block_by_hash(&self, hash: &Hash) -> Result<Option<Block>, StorageError>`.
- `StateStore::get_transaction(&self, hash: &Hash) -> Result<Option<(u64, u32, Transaction)>, StorageError>`.

### 4. `aura-mempool` $\leftrightarrow$ `aura-consensus`, `aura-rpc`
- `Mempool::insert(&mut self, Arc<Transaction>, &dyn StateStore) -> Result<Hash, MempoolError>`.
- `Mempool::select_transactions(&self, max_bytes: usize, max_count: usize) -> Vec<Arc<Transaction>>`.
- `Mempool::on_block_committed(&mut self, block: &Block, state: &dyn StateStore)`.
- `Mempool::get_pending(&self) -> Vec<Arc<Transaction>>`.
- `Mempool::get_by_hash(&self, hash: &Hash) -> Option<Arc<Transaction>>`.

### 5. `aura-consensus` $\leftrightarrow$ `aura-p2p`, `aura-node`
- `ConsensusEngine::handle_proposal(&mut self, Proposal) -> Result<Option<Vote>, ConsensusError>`.
- `ConsensusEngine::handle_vote(&mut self, Vote) -> Result<Option<QuorumCertificate>, ConsensusError>`.
- `ConsensusEngine::handle_slash_evidence(&mut self, SlashEvidence) -> Result<SlashReceipt, ConsensusError>`.
- `ConsensusEngine::on_round_timeout(&mut self) -> Result<Option<Vote>, ConsensusError>`.

### 6. `aura-rpc` $\leftrightarrow$ External Clients / Explorer
- JSON-RPC 2.0 endpoints over HTTP and WebSocket at `/` and `/ws`.
- Standard responses conforming to schemas specified in Survey Report 3.

## Code Layout

```
d:/cryptocurrency/
├── Cargo.toml                       # Root Workspace Manifest
├── crates/
│   ├── aura-crypto/                 # R1: Cryptographic Primitives & Merkle Tree
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── blake3.rs
│   │       ├── ed25519.rs
│   │       ├── bech32.rs
│   │       ├── hd_wallet.rs
│   │       └── merkle.rs
│   ├── aura-core/                   # R1: Core Domain Models & Serialization
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── hash.rs
│   │       ├── address.rs
│   │       ├── transaction.rs
│   │       ├── block.rs
│   │       ├── genesis.rs
│   │       └── error.rs
│   ├── aura-storage/                # R2: SMT, Persistent KV Engine, WAL & StateDB
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── smt.rs
│   │       ├── account.rs
│   │       ├── kv.rs
│   │       ├── wal.rs
│   │       ├── state_db.rs
│   │       └── overlay.rs
│   ├── aura-mempool/                # R3: Priority Mempool & Validation Pipeline
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── validation.rs
│   │       ├── pool.rs
│   │       ├── priority.rs
│   │       ├── nonce_tracker.rs
│   │       └── eviction.rs
│   ├── aura-consensus/              # R4: 2-Phase PoS-BFT Consensus Engine & Slashing
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state_machine.rs
│   │       ├── proposer.rs
│   │       ├── voting.rs
│   │       ├── quorum.rs
│   │       ├── pacemaker.rs
│   │       └── slashing.rs
│   ├── aura-p2p/                    # R5: libp2p Networking, GossipSub & Node Sync
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── transport.rs
│   │       ├── discovery.rs
│   │       ├── gossip.rs
│   │       ├── sync.rs
│   │       └── light_client.rs
│   ├── aura-metrics/                # R6: Prometheus Metrics Instrumentation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── registry.rs
│   ├── aura-rpc/                    # R6: High-Performance JSON-RPC & WebSocket Server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       ├── handlers.rs
│   │       ├── ws.rs
│   │       └── types.rs
│   ├── aura-node/                   # R6: Node CLI Runtime Daemon
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── config.rs
│   │       ├── node.rs
│   │       └── runner.rs
│   └── aura-test-harness/           # Track A: Cluster Orchestrator & E2E Test Runner
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── cluster.rs
│           ├── mock_node.rs
│           ├── chaos.rs
│           └── assertions.rs
├── explorer/                        # R7: Next.js Block Explorer Dashboard
│   ├── package.json
│   ├── tsconfig.json
│   ├── tailwind.config.ts
│   ├── next.config.mjs
│   └── src/
│       ├── app/
│       │   ├── layout.tsx
│       │   ├── page.tsx
│       │   ├── blocks/
│       │   ├── txs/
│       │   ├── address/
│       │   ├── validators/
│       │   └── network/
│       ├── components/
│       ├── hooks/
│       ├── lib/
│       └── styles/
└── tests/                           # Workspace Integration & E2E Tests
    ├── e2e_tier1_features.rs
    ├── e2e_tier2_boundaries.rs
    ├── e2e_tier3_pairwise.rs
    └── e2e_tier4_workloads.rs
```
