# E2E Test Infra: Aura Cryptocurrency

## Test Philosophy
- Opaque-box, requirement-driven. Derives test cases strictly from `ORIGINAL_REQUEST.md` and user-facing specifications without dependence on internal private implementation details.
- Methodology: Category-Partition + Boundary Value Analysis (BVA) + Pairwise Combinatorial Testing + Real-World Workload Testing.
- Progressive testability: Tier 1 tests give pass/fail signals with early cryptographic and data structure milestones.

## Feature Inventory & Test Matrix
| # | Feature | Source | Tier 1 (Coverage) | Tier 2 (Boundary/Edge) | Tier 3 (Cross-Feature) |
|---|---|---|:---:|:---:|:---:|
| 1 | BLAKE3 256-bit Hashing & Domain Separation | R1 | 5 | 5 | ✓ |
| 2 | Ed25519 Deterministic Signatures & Strict Verification | R1 | 5 | 5 | ✓ |
| 3 | Bech32 Address Derivation (`aura` prefix) | R1 | 5 | 5 | ✓ |
| 4 | BIP-39 Mnemonic & SLIP-0010 HD Wallet Derivation | R1 | 5 | 5 | ✓ |
| 5 | RFC 6962 Binary Merkle Tree & Inclusion Proofs | R1 | 5 | 5 | ✓ |
| 6 | Block, Header, Transaction, Genesis Data Structures | R1 | 5 | 5 | ✓ |
| 7 | 256-bit Binary SMT & State Root Calculation | R2 | 5 | 5 | ✓ |
| 8 | Account State Storage & Nonce/Balance Persistence | R2 | 5 | 5 | ✓ |
| 9 | Crash-Resilient WAL & Crash Recovery Safety | R2 | 5 | 5 | ✓ |
| 10 | Atomic State Commit & Historical Rollback Safety | R2 | 5 | 5 | ✓ |
| 11 | Stateless & Stateful Validation Pipeline | R3 | 5 | 5 | ✓ |
| 12 | Fee-Per-Byte Priority Mempool & Anti-DoS Eviction | R3 | 5 | 5 | ✓ |
| 13 | Strict Nonce Sequencing & Replace-By-Fee (RBF) | R3 | 5 | 5 | ✓ |
| 14 | 2-Phase PoS-BFT Consensus Engine & Proposer Selection | R4 | 5 | 5 | ✓ |
| 15 | 2/3+ Stake Finality & QC Quorum Verification | R4 | 5 | 5 | ✓ |
| 16 | Byzantine Double-Signing Slashing & Jailing | R4 | 5 | 5 | ✓ |
| 17 | libp2p P2P Network Discovery (Kademlia DHT) | R5 | 5 | 5 | ✓ |
| 18 | GossipSub Block & Tx Propagation | R5 | 5 | 5 | ✓ |
| 19 | Full Chain Sync, Header Sync & Light Client Sync | R5 | 5 | 5 | ✓ |
| 20 | JSON-RPC 2.0 API Server & WebSocket Pub/Sub Streams | R6 | 5 | 5 | ✓ |
| 21 | Prometheus Metrics Registry (:9090/metrics) | R6 | 5 | 5 | ✓ |
| 22 | Next.js Block Explorer Dashboard & OmniSearch | R7 | 5 | 5 | ✓ |

## Test Architecture
- **Test Harness Crate**: `crates/aura-test-harness` (In-process and multi-process cluster orchestrator, mock nodes, chaos network injector, Byzantine fault generator).
- **Test Runner**: Invoked via standard `cargo test --workspace` and dedicated E2E integration runner `cargo test --test e2e_*`.
- **Next.js Explorer Testing**: Next.js build validation and API integration tests in `explorer/`.
- **Pass/Fail Semantics**: All assertions must pass with exit code 0, 0 test failures, 0 panics.

## Real-World Application Scenarios (Tier 4)
| # | Scenario | Features Exercised | Complexity |
|---|---|---|---|
| 1 | Multi-Validator 4-Node Network Bootstrap & Continuous Block Production | F6, F7, F14, F15, F17, F18, F20 | High |
| 2 | High-Throughput Burst Transaction Spam & Fee-Per-Byte Mempool Prioritization | F1, F2, F11, F12, F13, F20, F21 | High |
| 3 | Byzantine Proposer Equivocation / Double-Signing Detection & Automatic Slashing | F2, F6, F14, F15, F16, F20 | High |
| 4 | Sudden Validator Node Crash, WAL Recovery & Seamless Chain Resynchronization | F7, F8, F9, F10, F17, F18, F19 | High |
| 5 | End-to-End User Transaction Submission via JSON-RPC & Explorer Real-Time WS Verification | F2, F3, F11, F20, F21, F22 | High |
| 6 | Deep Chain Reorganization Simulation & Mempool Transaction Re-injection | F6, F7, F10, F12, F13, F14 | High |

## Coverage Thresholds
- **Tier 1 (Feature Coverage)**: $\ge 5$ test cases per feature ($22 \times 5 = 110$ tests minimum).
- **Tier 2 (Boundary & Corner)**: $\ge 5$ test cases per feature ($22 \times 5 = 110$ tests minimum).
- **Tier 3 (Cross-Feature Combinations)**: $\ge 22$ pairwise combinatorial integration tests.
- **Tier 4 (Real-World Workloads)**: $\ge 6$ multi-node end-to-end workload scenarios.
- **Total Minimum Test Count**: $\ge 248$ test cases.
