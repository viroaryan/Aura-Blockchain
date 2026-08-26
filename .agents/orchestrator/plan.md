# Aura Cryptocurrency Development Plan

## Mission & Scope
Build a production-grade, from-scratch modular cryptocurrency node and ecosystem for "Aura" (Native Token: AUR, Bech32 Prefix: `aura`) in Rust, featuring cryptographic primitives, PoS-BFT consensus with slashing, P2P networking, account state storage, priority mempool, JSON-RPC/WebSocket APIs, and a Next.js block explorer dashboard.

## Methodology & Architecture
1. **Phase 0: Comprehensive Survey**
   - Dispatch 3 parallel Explorers to dissect requirements (R1 - R7 and acceptance criteria), specify data structures, cryptographic formats, wire protocols, state transitions, consensus safety rules, RPC specifications, and Next.js frontend requirements.
   - Synthesize results into global `PROJECT.md` at root.

2. **Phase 1: Dual Track Execution**
   - **Track A: E2E Testing Orchestrator**
     - Design opaque-box test runner & harness.
     - Build Tier 1 (Feature Coverage), Tier 2 (Boundary & Corner), Tier 3 (Cross-Feature Combinations), Tier 4 (Real-World Application Scenarios).
     - Publish `TEST_READY.md`.
   - **Track B: Implementation Sub-Orchestrators (Modular Milestones)**
     - Milestone 1: Core Cryptographic Primitives, Key Derivation (BIP-39/44, Bech32, Ed25519, BLAKE3), Block & Tx Data Structures, Merkle Trees, Genesis Configuration.
     - Milestone 2: Authenticated State Storage (Merkle-Patricia / Sparse Merkle Tree backed by embedded persistent KV store with crash-resilient WAL, state_root computation & rollback safety).
     - Milestone 3: Mempool & Tx Validation (fee-per-byte prioritization, strict nonce sequencing, signature checks, balance verification, size limits, anti-DoS eviction).
     - Milestone 4: PoS-BFT Consensus Engine (2-phase BFT propose/pre-vote/pre-commit, stake-weighted deterministic proposer selection, 2/3+ finality threshold, Byzantine fault slashing & double-sign detection).
     - Milestone 5: P2P Networking & Node Synchronization (libp2p, Kademlia DHT discovery, GossipSub block/tx propagation, full/header sync, light client verification).
     - Milestone 6: High-Performance JSON-RPC & WebSocket API Server + Prometheus Metrics Instrumentation (all query/tx endpoints, live block/tx subscriptions, metrics).
     - Milestone 7: Block Explorer Web Application (Next.js/React dashboard connected to node RPC, block/tx/address/validator search, live TPS, network health).

3. **Phase 2: Final Verification & Adversarial Hardening**
   - Pass 100% of E2E test suite (Tiers 1-4).
   - Adversarial coverage hardening (Tier 5).
   - Full workspace test pass (`cargo test --workspace` + Next.js build clean).
   - Independent Forensic Audits across all modules.

4. **Phase 3: Completion & Victory Claim**
   - Transmit victory claim to Sentinel for independent Victory Audit.
