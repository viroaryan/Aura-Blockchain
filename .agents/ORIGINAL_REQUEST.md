# Original User Request

## 2026-08-26T05:45:51Z

<USER_REQUEST>
Build a production-grade, from-scratch modular cryptocurrency node and ecosystem for "Aura" (Native Token: AUR, Bech32 Prefix: `aura`) in Rust, featuring cryptographic primitives, PoS-BFT consensus with slashing, P2P networking, account state storage, priority mempool, JSON-RPC/WebSocket APIs, and a Next.js block explorer dashboard.

Working directory: d:/cryptocurrency
Integrity mode: development

## Requirements

### R1. Core Cryptographic Primitives & Blockchain Data Structures
Implement Ed25519 key generation and deterministic signing/verification, BLAKE3 cryptographic hashing, Bech32 address derivation with `aura` prefix, and HD wallet key derivation (BIP-39/44). Implement block headers, transaction data structures, a cryptographic Merkle Tree with proof generation and verification for light clients, and a deterministic genesis block configuration with initial validator and account balances.

### R2. Authenticated State Storage
Implement an authenticated Merkle-Patricia Trie / Sparse Merkle Tree backed by a fast, embedded persistent key-value store with crash-resilient write-ahead logging (WAL) that deterministically calculates and commits the global `state_root` upon applying valid blocks and transactions.

### R3. Mempool & Transaction Validation
Implement a pending transaction pool with fee-per-byte prioritization, strict account nonce sequencing, signature validity checks, balance verification, size limits, and eviction policies to prevent spam and DoS attacks.

### R4. PoS-BFT Consensus Engine
Implement a 2-phase BFT consensus engine (propose, pre-vote, pre-commit) featuring deterministic, stake-weighted proposer selection, 2/3+ stake weight finality threshold, and slashing logic for Byzantine faults (such as double-signing).

### R5. P2P Networking & Node Synchronization
Implement P2P communication (using libp2p) supporting node discovery (Kademlia DHT), GossipSub block and transaction propagation, and node synchronization protocols (full chain sync, header sync, and light client verification).

### R6. RPC API Server & Observability
Implement a high-performance JSON-RPC and WebSocket server exposing blockchain queries and transaction broadcasting endpoints (`getBlock`, `getTransaction`, `getBalance`, `sendTransaction`, `getBlockHeight`, and live block subscriptions) alongside Prometheus metrics instrumentation.

### R7. Block Explorer Web Application
Implement a modern Next.js/React web dashboard connected to node RPC to search and visualize blocks, transactions, addresses, validator set, live TPS, and network health.

## Acceptance Criteria

### Build & Compilation
- [ ] `cargo build --workspace --all-targets` compiles cleanly with zero errors.
- [ ] The Next.js block explorer builds and runs cleanly with zero runtime/compilation errors.

### Verification & Automated Testing
- [ ] `cargo test --workspace` passes all test suites.
- [ ] Unit tests verify tamper detection: any alteration to historical block data invalidates the cryptographic hash chain.
- [ ] Unit tests verify Merkle proof generation and inclusion verification for transactions.
- [ ] Unit tests verify signature authentication and rejection of malformed or replayed transactions.
- [ ] State storage tests confirm deterministic root hash computation and rollback safety.
- [ ] Consensus simulation tests confirm BFT safety and finality when > 2/3 of validator stake is honest.
- [ ] Multi-node local network test validates block production, peer-to-peer gossip propagation, and transaction confirmation.
</USER_REQUEST>
