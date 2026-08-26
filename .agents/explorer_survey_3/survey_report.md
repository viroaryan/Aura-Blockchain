# Technical Specification & Survey Report: RPC API, Observability, Block Explorer & Workspace Architecture

**Agent**: Survey Explorer 3  
**Project**: Aura Cryptocurrency (`AUR`)  
**Bech32 Prefix**: `aura`  
**Date**: 2026-08-26  
**Scope**: 
- **R6**: High-Performance JSON-RPC 2.0 & WebSocket Server, Real-Time Pub/Sub Subscriptions, and Prometheus Metrics Instrumentation
- **R7**: Modern Next.js/React Block Explorer Web Application (Dashboard, Search, Visualization, Real-Time Telemetry)
- **Workspace Architecture**: Cargo Workspace Layout, Crate Boundaries, Inter-Crate API Contracts, Next.js Project Structure
- **Multi-Node Local Network Test Harness & E2E Verification Engine**: In-process / Process-isolated cluster orchestration, Byzantine fault injection, automated verification scenarios

---

## Table of Contents

1. [Executive Summary & Architectural Invariants](#1-executive-summary--architectural-invariants)
2. [R6: High-Performance JSON-RPC 2.0 & WebSocket Wire Specification](#2-r6-high-performance-json-rpc-20--websocket-wire-specification)
   - 2.1 Protocol Framework & Server Architecture
   - 2.2 Comprehensive RPC Method Catalog & Request/Response JSON Schemas
   - 2.3 Real-Time WebSocket Pub/Sub Streaming Protocol
   - 2.4 Standard & Application-Specific Error Code Matrix
   - 2.5 Batch Requests, CORS, Rate Limiting & DoS Hardening
3. [R6: Observability & Prometheus Metrics Architecture](#3-r6-observability--prometheus-metrics-architecture)
   - 3.1 Exporter Design & `/metrics` Endpoint
   - 3.2 Exhaustive Metric Catalog (Consensus, Mempool, Storage, P2P, RPC, System)
   - 3.3 Prometheus Alerting Rules & Grafana Dashboard Specifications
4. [R7: Block Explorer Web Application Architecture](#4-r7-block-explorer-web-application-architecture)
   - 4.1 Next.js 14+ App Router Architecture & Directory Tree
   - 4.2 UI/UX Layout, Design System & Color Palette
   - 4.3 Page Specifications & Route Handlers
   - 4.4 Type-Safe TypeScript RPC Client & WebSocket Streaming Layer
   - 4.5 Universal Search Engine Implementation
5. [Overall Workspace Architecture & Crate Boundaries](#5-overall-workspace-architecture--crate-boundaries)
   - 5.1 Cargo Workspace Layout & Dependency DAG
   - 5.2 Crate-by-Crate Responsibilities & Public API Traits
   - 5.3 Shared Serialization Formats & Type Definitions
6. [Multi-Node Local Network Test Harness & E2E Test Architecture](#6-multi-node-local-network-test-harness--e2e-test-architecture)
   - 6.1 Cluster Orchestrator Architecture (`aura-test-harness`)
   - 6.2 Deterministic Genesis & Ephemeral Node Provisioning
   - 6.3 Fault Injection & Network Partitioning Capabilities
   - 6.4 E2E Test Suite Matrix (Tiers 1–4)
7. [Verification, Acceptance & Implementation Roadmap](#7-verification-acceptance--implementation-roadmap)

---

## 1. Executive Summary & Architectural Invariants

The Aura cryptocurrency ecosystem requires a robust, high-throughput, low-latency communication and visualization layer capable of supporting both automated light clients / external integrations and end-user interactive inspection.

### 1.1 Core Invariants
1. **Deterministic Wire Encoding**: All hexadecimal strings (hashes, public keys, signatures, state roots) are encoded as lowercase `0x`-prefixed 64-character (32-byte) or 128-character (64-byte) hex strings.
2. **Bech32 Uniformity**: All account addresses in RPC responses and Explorer views must strictly use the `aura` human-readable part (HRP) with Bech32 encoding (e.g., `aura1...`).
3. **Atomic Unit of Value**: 
   - Base currency unit: **AUR**.
   - Smallest sub-unit: **naura** (nano-Aura, $10^{-9}$ AUR) or **uaura** ($10^{-6}$ AUR). In this specification, we standardize on **naura** ($1\text{ AUR} = 10^9\text{ naura}$, 9 decimal places), represented on-chain as a `u128` to prevent any possibility of overflow.
4. **Zero-Overhead Serialization**: Serialization for the JSON-RPC interface utilizes `serde_json` with custom serializers for cryptographic types, ensuring zero allocations during high-frequency telemetry streaming.
5. **Real-Time Push Updates**: WebSocket connections leverage Tokio broadcast channels to push new block commits, transaction confirmations, and validator slashing events with sub-50ms latency from node finalization.

---

## 2. R6: High-Performance JSON-RPC 2.0 & WebSocket Wire Specification

### 2.1 Protocol Framework & Server Architecture

The RPC server crate (`aura-rpc`) implements the JSON-RPC 2.0 specification over both HTTP POST and WebSocket transports using `jsonrpsee` (or `axum` + `tokio-tungstenite` with custom JSON-RPC routing).

```
 +-------------------------------------------------------------------------+
 |                           Client Applications                           |
 |  (Next.js Explorer, CLI Wallets, Light Clients, Automated Testing SDK)  |
 +--------------------+-------------------------------+--------------------+
                      | HTTP POST                     | WebSocket
                      v                               v
 +--------------------+-------------------------------+--------------------+
 |                             aura-rpc                                    |
 |  +-------------------------------------------------------------------+  |
 |  | JSON-RPC 2.0 Dispatcher & WebSocket Subscription Engine           |  |
 |  | - Rate Limiter (Token Bucket per IP)                              |  |
 |  | - Request Parser & Schema Validator                              |  |
 |  | - Method Routing (Query Engine, Mempool Dispatch, Consensus State)|  |
 |  +---------------------------------+---------------------------------+  |
 +------------------------------------|------------------------------------+
                                      v
       +------------------------------+-------------------------------+
       |                              |                               |
       v                              v                               v
+--------------+              +---------------+              +-----------------+
| aura-storage |              | aura-mempool  |              | aura-consensus  |
| (State DB &  |              | (Tx Queue &   |              | (PoS Engine &   |
| Block Store) |              | Pre-flight)   |              | Validator Set)  |
+--------------+              +---------------+              +-----------------+
```

### 2.2 Comprehensive RPC Method Catalog & Request/Response JSON Schemas

All requests conform to JSON-RPC 2.0:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "METHOD_NAME",
  "params": []
}
```

#### 2.2.1 `aura_getBlockHeight`
Retrieves the current finalized block height of the node.
- **Parameters**: `[]` (Empty)
- **Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": 142058
}
```

#### 2.2.2 `aura_getBlockByHeight`
Retrieves block details by its height integer.
- **Parameters**: `[height: u64, verbose: bool]`
  - `height`: The integer block height (0 for Genesis).
  - `verbose`: If `true`, returns full transaction objects; if `false`, returns transaction hash strings.
- **Response** (Verbose):
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "height": 142058,
    "hash": "0x3f7b2c918e6e5a40a87642a69074d9e2b19280dff8c792193b2a8d3889bc4512",
    "parent_hash": "0x1a8c9034e4a682f9b15d298319e7cf87a209b5523a7891df42a19b08f4c19208",
    "timestamp": 1724671200,
    "proposer": "aura1valoper7xk3p9f...",
    "state_root": "0x89ab12cd34ef567890abcdef1234567890abcdef1234567890abcdef12345678",
    "transactions_root": "0xef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd",
    "validator_hash": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "round": 0,
    "commit_signatures": [
      {
        "validator": "aura1valoper7xk3p9f...",
        "signature": "0x7890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12"
      }
    ],
    "transactions_count": 2,
    "transactions": [
      {
        "hash": "0x4d9a1f2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f",
        "sender": "aura1qnr783k...",
        "recipient": "aura1z9y8x7w...",
        "amount": "5000000000",
        "fee": "100000",
        "nonce": 42,
        "signature": "0xabcdef...",
        "public_key": "0x123456...",
        "status": "confirmed",
        "block_height": 142058,
        "block_hash": "0x3f7b2c918e6e5a40a87642a69074d9e2b19280dff8c792193b2a8d3889bc4512",
        "timestamp": 1724671200
      }
    ],
    "size_bytes": 1048
  }
}
```

#### 2.2.3 `aura_getBlockByHash`
Retrieves block details by its 32-byte BLAKE3 block hash string.
- **Parameters**: `[hash: string, verbose: bool]`
- **Response**: Same structure as `aura_getBlockByHeight`.

#### 2.2.4 `aura_getTransaction`
Retrieves transaction details, execution status, and inclusion proof.
- **Parameters**: `[tx_hash: string]`
- **Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "hash": "0x4d9a1f2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f",
    "sender": "aura1qnr783k9d8q0x4yj5f6s7a8b9c0d1e2f3a4b5",
    "recipient": "aura1z9y8x7w6v5u4t3s2r1q0p9o8n7m6l5k4j3h2g",
    "amount": "5000000000",
    "fee": "100000",
    "nonce": 42,
    "signature": "0x98fbc...",
    "public_key": "0x3102a...",
    "status": "confirmed",
    "block_height": 142058,
    "block_hash": "0x3f7b2c918e6e5a40a87642a69074d9e2b19280dff8c792193b2a8d3889bc4512",
    "timestamp": 1724671200,
    "index_in_block": 0,
    "merkle_proof": {
      "leaf_index": 0,
      "total_leaves": 2,
      "audit_path": [
        "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
      ]
    }
  }
}
```

#### 2.2.5 `aura_getBalance`
Queries the confirmed balance and nonce of an address.
- **Parameters**: `[address: string, at_height?: u64]`
  - `address`: Bech32 `aura` encoded address.
  - `at_height` (Optional): Query historical balance at specific block height (defaults to latest state root).
- **Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "address": "aura1qnr783k9d8q0x4yj5f6s7a8b9c0d1e2f3a4b5",
    "balance": "24950000000",
    "balance_formatted": "24.950000000 AUR",
    "nonce": 42,
    "state_root": "0x89ab12cd34ef567890abcdef1234567890abcdef1234567890abcdef12345678",
    "is_validator": false
  }
}
```

#### 2.2.6 `aura_sendTransaction`
Submits a signed raw transaction to the node mempool.
- **Parameters**: `[raw_tx_hex_or_object: string | object]`
- **Pre-flight Checks**:
  1. Signature verification against `public_key` and derived sender address.
  2. Transaction size $\le \text{MAX\_TX\_SIZE}$ (e.g. 64 KB).
  3. Nonce matches `account.nonce` (or next contiguous mempool nonce).
  4. Balance $\ge \text{amount} + \text{fee}$.
  5. Fee $\ge \text{size\_in\_bytes} \times \text{MIN\_FEE\_PER\_BYTE}$.
- **Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "hash": "0x4d9a1f2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f",
    "status": "pending",
    "timestamp": 1724671205
  }
}
```

#### 2.2.7 `aura_getValidators`
Retrieves the active validator set with their stake weights, voting power, and status.
- **Parameters**: `[height?: u64]`
- **Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "height": 142058,
    "total_stake": "100000000000000",
    "total_voting_power": 10000,
    "validators": [
      {
        "address": "aura1valoper7xk3p9f0a2b4c6e8g0i2k4m6o8q0s2u4w",
        "public_key": "0x11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff",
        "stake": "40000000000000",
        "voting_power": 4000,
        "voting_power_percentage": 40.0,
        "is_jailed": false,
        "blocks_proposed": 5682,
        "uptime_percentage": 99.98
      },
      {
        "address": "aura1valoper8ym4q0g1b3d5e7f9h1j3l5n7p9r1t3v5x",
        "public_key": "0x223344556677889900aabbccddeeff11223344556677889900aabbccddeeff11",
        "stake": "35000000000000",
        "voting_power": 3500,
        "voting_power_percentage": 35.0,
        "is_jailed": false,
        "blocks_proposed": 4972,
        "uptime_percentage": 100.0
      },
      {
        "address": "aura1valoper9zn5r1h2c4e6g8h0j2k4l6m8n0p2q4r6t",
        "public_key": "0x3344556677889900aabbccddeeff11223344556677889900aabbccddeeff1122",
        "stake": "25000000000000",
        "voting_power": 2500,
        "voting_power_percentage": 25.0,
        "is_jailed": false,
        "blocks_proposed": 3550,
        "uptime_percentage": 99.95
      }
    ]
  }
}
```

#### 2.2.8 `aura_getNodeInfo` & `aura_getNetworkHealth`
Returns node runtime status, sync state, and live consensus telemetry.
- **Parameters**: `[]`
- **Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "node_version": "0.1.0",
    "network_id": "aura-mainnet-1",
    "peer_id": "12D3KooWDpJ7As7BWAbrkghWdC88kheP9w4LpMvV8P9N...",
    "peer_count": 24,
    "sync_status": {
      "is_syncing": false,
      "current_block": 142058,
      "highest_block": 142058
    },
    "consensus": {
      "state": "Commit",
      "height": 142058,
      "round": 0,
      "proposer": "aura1valoper7xk3p9f..."
    },
    "mempool": {
      "size": 18,
      "bytes_total": 4210
    },
    "live_tps": 42.5,
    "average_block_time_ms": 1980
  }
}
```

#### 2.2.9 `aura_getGenesis`
Retrieves the static genesis state, initial account distribution, validator set, and chain parameters.
- **Parameters**: `[]`
- **Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "chain_id": "aura-mainnet-1",
    "genesis_time": 1724600000,
    "initial_validators": [ ... ],
    "initial_balances": [ ... ],
    "consensus_params": {
      "block_time_ms": 2000,
      "max_block_bytes": 2097152,
      "max_txs_per_block": 5000,
      "evidence_timeout_blocks": 10000,
      "slash_fraction_double_sign": "0.05"
    }
  }
}
```

---

### 2.3 Real-Time WebSocket Pub/Sub Streaming Protocol

WebSocket clients connect to `ws://<host>:<port>/ws` (or root `ws://`). The subscription model follows standard JSON-RPC Pub/Sub semantics.

#### 2.3.1 Subscription Request (`aura_subscribe`)
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "aura_subscribe",
  "params": ["newHeads"]
}
```
Supported subscription channels:
- `newHeads`: Emits on each committed block header.
- `newTransactions`: Emits on each transaction added to a committed block.
- `pendingTransactions`: Emits when a transaction passes pre-flight checks and enters the mempool.
- `validatorUpdates`: Emits on validator set updates, slashing events, or jail events.
- `networkHealth`: Emits periodic telemetry (TPS, peer count, mempool size) every 1 second.

#### 2.3.2 Subscription Acknowledgment
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "sub_9a8b7c6d5e4f3a2b"
}
```

#### 2.3.3 Live Push Notification Frame
```json
{
  "jsonrpc": "2.0",
  "method": "aura_subscription",
  "params": {
    "subscription": "sub_9a8b7c6d5e4f3a2b",
    "result": {
      "height": 142059,
      "hash": "0x5a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b",
      "parent_hash": "0x3f7b2c918e6e5a40a87642a69074d9e2b19280dff8c792193b2a8d3889bc4512",
      "timestamp": 1724671202,
      "proposer": "aura1valoper8ym4q0g...",
      "transactions_count": 14,
      "state_root": "0x9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba"
    }
  }
}
```

#### 2.3.4 Unsubscribe Request (`aura_unsubscribe`)
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "aura_unsubscribe",
  "params": ["sub_9a8b7c6d5e4f3a2b"]
}
```

---

### 2.4 Standard & Application-Specific Error Code Matrix

All errors return JSON-RPC 2.0 error payloads:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Insufficient balance for transaction",
    "data": {
      "required": "5000100000",
      "available": "2500000000"
    }
  }
}
```

| Code | Message | Description / Trigger Condition |
|:---|:---|:---|
| **-32700** | `Parse error` | Invalid JSON received by the server. |
| **-32600** | `Invalid Request` | The JSON sent is not a valid JSON-RPC 2.0 request. |
| **-32601** | `Method not found` | The requested method does not exist or is disabled. |
| **-32602** | `Invalid params` | Parameter types, arity, or bounds violated. |
| **-32603** | `Internal error` | Internal node execution error (sanitized; no panic traces). |
| **-32000** | `Invalid signature` | Transaction Ed25519 signature verification failed. |
| **-32001** | `Insufficient balance` | Account balance lower than `amount + fee`. |
| **-32002** | `Invalid nonce` | Transaction nonce mismatch (expected $N$, got $M$). |
| **-32003** | `Mempool fee too low` | Fee-per-byte is below minimum threshold or mempool is full. |
| **-32004** | `Block not found` | Block at requested height or hash does not exist. |
| **-32005** | `Transaction not found` | Transaction with requested hash not in chain or mempool. |
| **-32006** | `Account not found` | Queried address has no history or uninitialized balance. |
| **-32007** | `Duplicate transaction` | Transaction hash already exists in mempool or blockchain. |
| **-32008** | `Node is syncing` | Node is executing fast/header sync and cannot serve state queries. |
| **-32009** | `Subscription limit reached`| Client exceeded maximum active WebSocket subscriptions (e.g. 50). |

---

### 2.5 Batch Requests, CORS, Rate Limiting & DoS Hardening

1. **JSON-RPC Batch Execution**:
   - Supports array of requests up to 100 queries per batch.
   - Executes non-mutating queries concurrently using `futures::future::join_all`.
2. **CORS Policy**:
   - Configurable allowed origins (`*` in development; explicit domain list in production).
   - Allowed headers: `Content-Type`, `Authorization`, `Accept`.
   - Allowed methods: `POST`, `OPTIONS`.
3. **Rate Limiting**:
   - Token bucket algorithm per IP address: 1,000 requests/minute burst, 100 requests/second sustained.
   - HTTP 429 Too Many Requests response if token bucket is exhausted.
4. **Request Size Constraints**:
   - Max HTTP body payload: 2 MB (prevents memory exhaustion from giant payloads).
   - Max WebSocket frame size: 1 MB.
   - Max active WebSocket connections: 5,000 per node instance.

---

## 3. R6: Observability & Prometheus Metrics Architecture

### 3.1 Exporter Design & `/metrics` Endpoint

Observability is implemented in the `aura-metrics` crate using the `prometheus` registry crate. The node exposes an HTTP endpoint at `http://<host>:9090/metrics` (configurable via `--metrics-addr`).

```
+----------------------------------------------------------------------------+
|                             aura-metrics                                   |
|                                                                            |
|  +----------------------------------------------------------------------+  |
|  | prometheus::Registry (Default Global Registry)                       |  |
|  | - Consensus Metrics                                                  |  |
|  | - Mempool Metrics                                                    |  |
|  | - Storage & Trie Metrics                                             |  |
|  | - P2P Network Metrics                                                |  |
|  | - RPC / WS Metrics                                                   |  |
|  | - Node Health & System Metrics                                       |  |
|  +-----------------------------------+----------------------------------+  |
|                                      | Scrapes                             |
|                                      v                                     |
|  +----------------------------------------------------------------------+  |
|  | Axum / Hyper HTTP Server on :9090/metrics                            |  |
|  | TextFormat::encode -> standard Prometheus text serialization         |  |
|  +----------------------------------------------------------------------+  |
+----------------------------------------------------------------------------+
```

---

### 3.2 Exhaustive Metric Catalog

#### 3.2.1 Consensus Metrics (`aura_consensus_*`)
| Metric Name | Type | Labels | Description |
|:---|:---|:---|:---|
| `aura_consensus_height` | Gauge | - | Current finalized blockchain height |
| `aura_consensus_round` | Gauge | - | Current consensus round for the active height |
| `aura_consensus_step` | Gauge | `step` (`propose`,`prevote`,`precommit`,`commit`) | Active consensus step indicator |
| `aura_consensus_block_processing_duration_seconds` | Histogram | - | Latency for executing and verifying a proposed block |
| `aura_consensus_validators_total` | Gauge | `status` (`active`,`jailed`) | Total number of registered validators |
| `aura_consensus_voting_power_total` | Gauge | - | Total active stake voting power participating in consensus |
| `aura_consensus_double_sign_slashes_total` | Counter | - | Total number of detected double-sign Byzantine events slashed |
| `aura_consensus_blocks_committed_total` | Counter | `proposer` | Total blocks committed, labeled by proposer address |

#### 3.2.2 Mempool Metrics (`aura_mempool_*`)
| Metric Name | Type | Labels | Description |
|:---|:---|:---|:---|
| `aura_mempool_size_transactions` | Gauge | - | Current count of pending transactions in the mempool |
| `aura_mempool_bytes_total` | Gauge | - | Total memory footprint (bytes) of all pending transactions |
| `aura_mempool_tx_admitted_total` | Counter | - | Total transactions successfully admitted to mempool |
| `aura_mempool_tx_rejected_total` | Counter | `reason` (`bad_sig`,`nonce_gap`,`low_fee`,`balance`) | Total transactions rejected at pre-flight |
| `aura_mempool_tx_evicted_total` | Counter | `reason` (`ttl_expired`,`overflow_low_fee`) | Transactions evicted without being mined |

#### 3.2.3 Storage & State Metrics (`aura_storage_*`)
| Metric Name | Type | Labels | Description |
|:---|:---|:---|:---|
| `aura_storage_state_root_commit_duration_seconds` | Histogram | - | Duration to recompute Merkle-Patricia Trie root & write batch |
| `aura_storage_db_read_duration_seconds` | Histogram | `operation` (`get_account`,`get_block`,`get_tx`) | Latency of key-value storage reads |
| `aura_storage_db_write_duration_seconds` | Histogram | `operation` (`commit_block`,`wal_flush`) | Latency of key-value storage writes |
| `aura_storage_wal_size_bytes` | Gauge | - | Current byte size of the uncompressed Write-Ahead Log |

#### 3.2.4 P2P Networking Metrics (`aura_p2p_*`)
| Metric Name | Type | Labels | Description |
|:---|:---|:---|:---|
| `aura_p2p_connected_peers` | Gauge | `direction` (`inbound`,`outbound`) | Number of active libp2p peer connections |
| `aura_p2p_gossip_messages_received_total` | Counter | `topic` (`blocks`,`transactions`,`consensus`) | Total GossipSub messages received |
| `aura_p2p_gossip_messages_sent_total` | Counter | `topic` (`blocks`,`transactions`,`consensus`) | Total GossipSub messages broadcast |
| `aura_p2p_bytes_received_total` | Counter | - | Total network bytes ingress |
| `aura_p2p_bytes_sent_total` | Counter | - | Total network bytes egress |

#### 3.2.5 RPC Server Metrics (`aura_rpc_*`)
| Metric Name | Type | Labels | Description |
|:---|:---|:---|:---|
| `aura_rpc_requests_total` | Counter | `method`, `status` (`success`,`error`) | Total JSON-RPC requests processed |
| `aura_rpc_request_duration_seconds` | Histogram | `method` | Execution duration histogram per method |
| `aura_ws_active_connections` | Gauge | - | Total currently connected WebSocket clients |
| `aura_ws_subscriptions_active` | Gauge | `channel` (`newHeads`,`newTransactions`, etc.) | Active real-time subscription streams |

#### 3.2.6 Node Health & System Metrics (`aura_node_*`)
| Metric Name | Type | Labels | Description |
|:---|:---|:---|:---|
| `aura_node_tps_current` | Gauge | - | Real-time computed transactions-per-second over last 60s |
| `aura_node_uptime_seconds` | Counter | - | Node process runtime in seconds |
| `aura_process_cpu_seconds_total` | Counter | - | Total user + system CPU time spent |
| `aura_process_resident_memory_bytes` | Gauge | - | Resident memory (RSS) allocated by node process |

---

## 4. R7: Block Explorer Web Application Architecture

### 4.1 Next.js 14+ App Router Architecture & Directory Tree

The Aura Block Explorer is built as a responsive, modern web application located at `d:/cryptocurrency/explorer` using **Next.js 14+ (App Router)**, **React 18+**, **TypeScript**, **Tailwind CSS**, and **Lucide React**.

```
d:/cryptocurrency/explorer/
├── package.json
├── tsconfig.json
├── tailwind.config.ts
├── postcss.config.js
├── next.config.mjs
├── public/
│   ├── favicon.ico
│   └── aura-logo.svg
├── src/
│   ├── app/
│   │   ├── layout.tsx               # Root layout: Navbar, Search, Footer, Web3 Providers
│   │   ├── page.tsx                 # Home Dashboard: TPS chart, metric cards, live feeds
│   │   ├── blocks/
│   │   │   ├── page.tsx             # Paginated Block Listing table
│   │   │   └── [heightOrHash]/
│   │   │       └── page.tsx         # Deep Block detail view & transaction list
│   │   ├── txs/
│   │   │   ├── page.tsx             # Paginated Transaction listing table
│   │   │   └── [hash]/
│   │   │       └── page.tsx         # Deep Transaction detail view & Merkle proof inspector
│   │   ├── address/
│   │   │   └── [address]/
│   │   │       └── page.tsx         # Account detail: Balance, Nonce, Tx History, QR code
│   │   ├── validators/
│   │   │   └── page.tsx             # Validator set table, voting power chart, slashing logs
│   │   └── network/
│   │       └── page.tsx             # Node telemetry, P2P peer map, consensus state monitor
│   ├── components/
│   │   ├── ui/                      # Reusable UI primitives (Card, Badge, Button, Table, Tabs)
│   │   ├── layout/                  # Navbar, Footer, MobileNav, SearchBar
│   │   ├── dashboard/               # MetricCards, LiveBlockFeed, LiveTxFeed, TPSChart
│   │   ├── blocks/                  # BlockTable, BlockSummaryCard, CommitSignaturesList
│   │   ├── txs/                     # TxTable, TxSummaryCard, MerkleProofViewer, RawJsonModal
│   │   ├── address/                 # AddressCard, BalanceDisplay, TxHistoryTable
│   │   └── validators/              # ValidatorTable, VotingPowerBar, SlashingBadge
│   ├── hooks/
│   │   ├── useAuraRpc.ts            # Hook for generic RPC queries with React Query / SWR
│   │   ├── useLiveBlocks.ts         # Hook subscribing to WebSocket 'newHeads'
│   │   ├── useLiveTransactions.ts   # Hook subscribing to WebSocket 'newTransactions'
│   │   ├── useNetworkHealth.ts      # Hook polling/subscribing to health & TPS telemetry
│   │   └── useOmniSearch.ts         # Search routing hook (height vs hash vs address vs tx)
│   ├── lib/
│   │   ├── rpc/
│   │   │   ├── client.ts            # Type-safe Aura JSON-RPC HTTP Client
│   │   │   ├── ws.ts                # Auto-reconnecting WebSocket Subscription Client
│   │   │   └── types.ts             # TypeScript interfaces for all RPC entities
│   │   ├── crypto/
│   │   │   ├── bech32.ts            # Bech32 validator & address formatter (`aura...`)
│   │   │   └── formatters.ts        # AUR unit formatter (naura -> AUR), timestamp formatter
│   │   └── constants.ts             # Default RPC URLs, chain constants, refresh intervals
│   └── styles/
│       └── globals.css              # Tailwind base, dark mode tokens, neon glow utility classes
```

---

### 4.2 UI/UX Layout, Design System & Color Palette

- **Theme**: Dark-first futuristic crypto terminal aesthetic with ultra-clean contrast.
- **Palette**:
  - Background: Obsidian `#0A0D14` and Deep Navy `#111827`
  - Cards / Surface: Translucent Charcoal `rgba(17, 24, 39, 0.7)` with `backdrop-blur-md` and 1px border `rgba(255, 255, 255, 0.08)`
  - Accent / Primary: Electric Cyan `#06B6D4` / `#22D3EE` (Aura Brand)
  - Secondary Accent: Deep Violet `#8B5CF6` (Consensus & Staking)
  - Success: Mint Emerald `#10B981` (Confirmed Txs, Honest Validators)
  - Warning / Alert: Amber `#F59E0B` (Pending, High Mempool)
  - Danger: Crimson `#EF4444` (Slashed, Reverted, Double-Sign)

---

### 4.3 Page Specifications & Route Handlers

#### 4.3.1 Home Dashboard (`/`)
- **Metric Cards Grid** (4 Cards):
  1. **Block Height**: Current height, finalized indicator, counter with pulse animation on new block.
  2. **Live TPS & Peak TPS**: Computed 60s sliding window TPS with micro-chart.
  3. **Active Validators & Total Stake**: Active count / total registered, total staked AUR.
  4. **Mempool Status**: Pending tx count and memory consumption.
- **Real-Time Split Feeds**:
  - **Left Pane (Latest Blocks)**: Streaming list of latest 10 blocks (Height, Proposer, Tx Count, Timestamp, Size).
  - **Right Pane (Latest Transactions)**: Streaming list of latest 10 transactions (Hash, Sender -> Recipient, Amount, Fee, Time).
- **Interactive TPS & Volume Area Chart**: 1-hour / 24-hour historical transaction throughput graph.

#### 4.3.2 Block Detail Page (`/blocks/[heightOrHash]`)
- Displays:
  - Header: Block Height, Hash, Status (`Finalized`), Proposer (Bech32 link), Age/Timestamp.
  - Cryptographic Roots: Parent Hash, State Root, Transactions Root, Validator Set Hash.
  - Consensus Details: Round, Commit Signatures count, 2/3+ threshold verification status.
  - Transactions Table: Embedded table of all transactions included in this block with click-through to transaction detail.

#### 4.3.3 Transaction Detail Page (`/txs/[hash]`)
- Displays:
  - Transaction Hash with copy-to-clipboard button.
  - Status Badge: `Confirmed in Block #142058` or `Pending in Mempool`.
  - From Address (Bech32) $\rightarrow$ To Address (Bech32).
  - Value: Formatted in `AUR` ($10^9$ naura) and raw `naura`.
  - Fee & Fee-per-byte rate.
  - Nonce integer.
  - Cryptographic Signature & Public Key (collapsible hex view).
  - **Merkle Proof Visualizer**: Interactive audit path tree verifying the transaction's inclusion in `transactions_root`.

#### 4.3.4 Address Page (`/address/[address]`)
- Displays:
  - Bech32 Address with QR Code modal.
  - Account Balance in AUR + USD approximation (mock/configurable).
  - Account Nonce (Total transactions sent).
  - Staking / Validator Profile (if address belongs to an active validator).
  - Paginated Transaction History (Inbound & Outbound filters).

#### 4.3.5 Validator Set Page (`/validators`)
- Displays:
  - Total Staked AUR and Staking Ratio progress bar.
  - Active Validator Table: Rank, Moniker / Address, Public Key, Stake Weight, Voting Power %, Blocks Proposed, Uptime Score %, Slashing History.
  - Jailed / Slashed Validator section displaying evidence (e.g., double-sign block height & round).

---

### 4.4 Type-Safe TypeScript RPC Client & WebSocket Streaming Layer

```typescript
// lib/rpc/types.ts
export interface AuraBlockHeader {
  height: number;
  hash: string;
  parent_hash: string;
  timestamp: number;
  proposer: string;
  state_root: string;
  transactions_root: string;
  validator_hash: string;
  round: number;
  commit_signatures: { validator: string; signature: string }[];
  transactions_count: number;
  size_bytes: number;
}

export interface AuraTransaction {
  hash: string;
  sender: string;
  recipient: string;
  amount: string; // naura as string (u128)
  fee: string;
  nonce: number;
  signature: string;
  public_key: string;
  status: 'pending' | 'confirmed' | 'rejected';
  block_height?: number;
  block_hash?: string;
  timestamp?: number;
  index_in_block?: number;
  merkle_proof?: {
    leaf_index: number;
    total_leaves: number;
    audit_path: string[];
  };
}

export interface AuraAccount {
  address: string;
  balance: string;
  balance_formatted: string;
  nonce: number;
  state_root: string;
  is_validator: boolean;
}

export interface AuraValidator {
  address: string;
  public_key: string;
  stake: string;
  voting_power: number;
  voting_power_percentage: number;
  is_jailed: boolean;
  blocks_proposed: number;
  uptime_percentage: number;
}
```

```typescript
// lib/rpc/client.ts
export class AuraRpcClient {
  private rpcUrl: string;

  constructor(rpcUrl: string = process.env.NEXT_PUBLIC_AURA_RPC_URL || 'http://localhost:8545') {
    this.rpcUrl = rpcUrl;
  }

  private async request<T>(method: string, params: any[] = []): Promise<T> {
    const res = await fetch(this.rpcUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', id: Date.now(), method, params }),
      cache: 'no-store',
    });
    const json = await res.json();
    if (json.error) {
      throw new Error(`RPC Error [${json.error.code}]: ${json.error.message}`);
    }
    return json.result;
  }

  async getBlockHeight(): Promise<number> {
    return this.request<number>('aura_getBlockHeight');
  }

  async getBlock(heightOrHash: number | string, verbose = true): Promise<AuraBlockHeader> {
    const method = typeof heightOrHash === 'number' ? 'aura_getBlockByHeight' : 'aura_getBlockByHash';
    return this.request<AuraBlockHeader>(method, [heightOrHash, verbose]);
  }

  async getTransaction(hash: string): Promise<AuraTransaction> {
    return this.request<AuraTransaction>('aura_getTransaction', [hash]);
  }

  async getBalance(address: string): Promise<AuraAccount> {
    return this.request<AuraAccount>('aura_getBalance', [address]);
  }

  async getValidators(): Promise<{ validators: AuraValidator[]; total_stake: string }> {
    return this.request('aura_getValidators');
  }

  async getNodeInfo(): Promise<any> {
    return this.request('aura_getNodeInfo');
  }
}
```

---

### 4.5 Universal Search Engine Implementation

The OmniSearch bar routes user queries according to strict pattern matching:
```typescript
// lib/search.ts
export function parseSearchQuery(query: string): { type: 'block' | 'tx' | 'address' | 'validator' | 'unknown'; value: string } {
  const clean = query.trim();
  
  // 1. Bech32 Account / Validator Address
  if (clean.startsWith('aura1valoper') && clean.length >= 40) {
    return { type: 'validator', value: clean };
  }
  if (clean.startsWith('aura1') && clean.length >= 38) {
    return { type: 'address', value: clean };
  }

  // 2. Hex Hash (32-byte BLAKE3 = 64 hex chars, optional 0x prefix)
  const hexPattern = /^(0x)?[0-9a-fA-F]{64}$/;
  if (hexPattern.test(clean)) {
    // Check if it starts with 0x, normalize
    const normalized = clean.startsWith('0x') ? clean.toLowerCase() : `0x${clean.toLowerCase()}`;
    return { type: 'tx', value: normalized }; // Fallback to tx query, if 404 tries block
  }

  // 3. Integer Block Height
  if (/^\d+$/.test(clean)) {
    return { type: 'block', value: clean };
  }

  return { type: 'unknown', value: clean };
}
```

---

## 5. Overall Workspace Architecture & Crate Boundaries

### 5.1 Cargo Workspace Layout & Dependency DAG

The Rust codebase is organized as a multi-crate Cargo workspace rooted at `d:/cryptocurrency`.

```
d:/cryptocurrency/
├── Cargo.toml                      # Workspace definition & shared dependency versions
├── Cargo.lock
├── PROJECT.md                      # Synthesized specifications & milestones
├── README.md
├── crates/
│   ├── aura-crypto/                # R1: Ed25519, BLAKE3, Bech32, BIP-39/44
│   ├── aura-core/                  # R1: Block, Header, Tx, MerkleTree, Genesis, Account
│   ├── aura-storage/               # R2: SMT / MPT, WAL, RocksDB / KV Store, Atomic State
│   ├── aura-mempool/               # R3: Priority Tx Queue, Nonce Tracker, Anti-DoS
│   ├── aura-consensus/             # R4: 2-phase PoS-BFT, Proposer Selection, Slashing
│   ├── aura-p2p/                   # R5: libp2p Swarm, Kademlia DHT, GossipSub, Sync
│   ├── aura-rpc/                   # R6: JSON-RPC 2.0 & WebSocket Server, Pub/Sub
│   ├── aura-metrics/               # R6: Prometheus Registry, Metric Collectors, Exporter
│   ├── aura-node/                  # CLI Binary (`aurad`), Daemon Runner, Orchestration
│   └── aura-test-harness/          # Multi-Node Local Cluster, E2E Test Scenarios, Faults
└── explorer/                       # R7: Next.js 14+ React Block Explorer Web App
```

#### Dependency Flow Directed Acyclic Graph (DAG)
```
       [ aura-crypto ]
              |
              v
        [ aura-core ]
       /      |      \
      v       v       v
[storage] [mempool] [consensus]
      \       |       /    \
       v      v      v      v
      [   aura-p2p   ]   [ aura-metrics ]
              \                /
               v              v
               [   aura-rpc   ]
                      |
                      v
               [  aura-node   ]
                      |
                      v
           [  aura-test-harness  ]
```

---

### 5.2 Crate-by-Crate Responsibilities & Public API Traits

#### 1. `aura-crypto`
- **Dependencies**: `ed25519-dalek`, `blake3`, `bech32`, `bip39`, `tiny-bip44`, `rand`, `subtle`.
- **Exposed Types/Traits**:
  - `KeyPair`, `PublicKey`, `PrivateKey`, `Signature`
  - `Hash256` (BLAKE3 32-byte hash wrapper)
  - `Address` (Bech32 `aura` prefix encoding/decoding)
  - `KeyDerivation` (BIP-39 mnemonic phrase to BIP-44 path `m/44'/1234'/0'/0/0`)

#### 2. `aura-core`
- **Dependencies**: `aura-crypto`, `serde`, `bincode`, `serde_json`, `thiserror`.
- **Exposed Types/Traits**:
  - `Transaction`, `TransactionBody`, `SignedTransaction`, `TransactionReceipt`
  - `Block`, `BlockHeader`, `BlockCommitProof`
  - `MerkleTree`, `MerkleProof`, `MerkleAuditPath`
  - `Account`, `AccountState`, `GenesisConfig`, `ChainParameters`

#### 3. `aura-storage`
- **Dependencies**: `aura-core`, `aura-crypto`, `rocksdb` (or `sled` / `heed`), `parking_lot`, `tokio`.
- **Exposed Types/Traits**:
  - `StateDb` trait: `get_account(&Address) -> Result<Option<Account>>`
  - `commit_block(&Block, &StateBatch) -> Result<Hash256>` (Returns deterministic `state_root`)
  - `rollback_to(&Hash256) -> Result<()>`
  - `MerklePatriciaTrie` / `SparseMerkleTree` with verifiable cryptographic root proofs
  - `WriteAheadLog` (WAL) with automatic crash recovery

#### 4. `aura-mempool`
- **Dependencies**: `aura-core`, `aura-crypto`, `aura-storage`, `parking_lot`.
- **Exposed Types/Traits**:
  - `Mempool`: `insert_transaction(SignedTransaction) -> Result<(), MempoolError>`
  - `get_prioritized_batch(max_bytes: usize, max_count: usize) -> Vec<SignedTransaction>`
  - `evict_committed(&[Hash256])`
  - Fee-per-byte priority queue + per-account nonce contiguous index

#### 5. `aura-consensus`
- **Dependencies**: `aura-core`, `aura-crypto`, `aura-storage`, `aura-mempool`.
- **Exposed Types/Traits**:
  - `ConsensusEngine`: State machine handling `Propose`, `PreVote`, `PreCommit`, `Commit`
  - `ProposerSelector`: Deterministic stake-weighted round-robin proposer selection
  - `SlashingEngine`: Double-sign detection (`Evidence`) and stake reduction/jailing logic

#### 6. `aura-p2p`
- **Dependencies**: `aura-core`, `libp2p`, `tokio`, `futures`, `tracing`.
- **Exposed Types/Traits**:
  - `P2pService`: GossipSub topic pub/sub (`blocks`, `txs`, `consensus`)
  - `KademliaDHT` peer routing & peer exchange
  - `ChainSyncService`: Full chain sync, header-first sync, and light client proof verification

#### 7. `aura-rpc`
- **Dependencies**: `aura-core`, `aura-storage`, `aura-mempool`, `aura-consensus`, `aura-metrics`, `jsonrpsee` / `axum`, `tokio`, `tower-http`.
- **Exposed Types/Traits**:
  - `RpcServer`: Initializes JSON-RPC 2.0 HTTP & WebSocket listeners
  - `SubscriptionManager`: Tokio broadcast stream router for real-time pub/sub

#### 8. `aura-metrics`
- **Dependencies**: `prometheus`, `lazy_static`, `axum`.
- **Exposed Types/Traits**:
  - `Registry`, `init_metrics()`, `start_metrics_server(addr: SocketAddr)`
  - Standardized metric static vectors (`CONSENSUS_HEIGHT`, `MEMPOOL_SIZE`, etc.)

#### 9. `aura-node`
- **Dependencies**: All workspace crates + `clap` (CLI argument parsing), `tracing-subscriber`.
- **Binary**: `aurad`
  - `aurad init --chain-id aura-testnet-1`
  - `aurad start --config ./config.toml --validator-key ./validator.key`
  - `aurad keys new / import / show`

#### 10. `aura-test-harness`
- **Dependencies**: All workspace crates + `tempfile`, `async-trait`.
- **Exposed Types/Traits**:
  - `LocalCluster`: Multi-node cluster manager
  - `NodeHandle`: Individual node controller (stop, resume, query, inspect)
  - `ScenarioRunner`: Automated execution of E2E verification suites

---

### 5.3 Shared Serialization Formats & Type Definitions

```rust
// crates/aura-core/src/types.rs
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address(pub [u8; 20]); // Encodes to "aura1..." via Bech32

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash256(pub [u8; 32]); // Hex encoded as "0x..."

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub sender: Address,
    pub recipient: Address,
    pub amount: u128,         // in naura
    pub fee: u128,            // in naura
    pub nonce: u64,
    pub signature: [u8; 64],  // Ed25519 signature
    pub public_key: [u8; 32], // Ed25519 public key
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub height: u64,
    pub parent_hash: Hash256,
    pub timestamp: u64,
    pub proposer: Address,
    pub state_root: Hash256,
    pub transactions_root: Hash256,
    pub validator_hash: Hash256,
    pub round: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub commit_proof: Vec<CommitSignature>,
}
```

---

## 6. Multi-Node Local Network Test Harness & E2E Test Architecture

### 6.1 Cluster Orchestrator Architecture (`aura-test-harness`)

The `aura-test-harness` provides a completely automated, programmatic in-process and subprocess multi-node test environment. It allows E2E test suites to spin up an arbitrary topology of validator and full nodes, simulate network latency and partitions, broadcast transactions, and assert consensus invariants.

```
 +-------------------------------------------------------------------------+
 |                           E2E Test Runner                               |
 |      (cargo test -p aura-test-harness --test multi_node_scenarios)      |
 +------------------------------------+------------------------------------+
                                      |
                                      v
 +-------------------------------------------------------------------------+
 |                     LocalCluster (aura-test-harness)                    |
 |                                                                         |
 |  +--------------------+  +--------------------+  +--------------------+ |
 |  |    Node 0 (Val 1)  |  |    Node 1 (Val 2)  |  |    Node 2 (Val 3)  | |
 |  | - State DB (tmp)   |  | - State DB (tmp)   |  | - State DB (tmp)   | |
 |  | - RPC on :18545    |  | - RPC on :18546    |  | - RPC on :18547    | |
 |  | - P2P on :19000    |  | - P2P on :19001    |  | - P2P on :19002    | |
 |  +---------^----------+  +---------^----------+  +---------^----------+ |
 |            |                       |                       |            |
 |  +---------v-----------------------v-----------------------v----------+ |
 |  |                   Virtual Network Interconnect                      | |
 |  |       (P2P Message Filter / Drop / Delay / Partition Engine)        | |
 |  +--------------------------------------------------------------------+ |
 +-------------------------------------------------------------------------+
```

---

### 6.2 Deterministic Genesis & Ephemeral Node Provisioning

`LocalCluster::builder()` supports:
1. Generating $N$ deterministic Ed25519 validator keypairs.
2. Generating $M$ pre-funded user keypairs from BIP-39 mnemonic seeds.
3. Constructing a valid `GenesisConfig` with equal or weighted stake distributions.
4. Spawning each node in a dedicated Tokio task with ephemeral storage directories (`tempfile::TempDir`).
5. Auto-binding ephemeral OS ports for P2P, RPC, and Metrics to prevent port collisions during parallel test runs.

```rust
// Example Test Harness Usage
let mut cluster = LocalCluster::builder()
    .validator_count(4)
    .initial_balance_per_account(1_000_000_000_000_000) // 1M AUR
    .block_time(Duration::from_millis(500))
    .build()
    .await?;

cluster.start().await?;

// Wait for all 4 nodes to finalize block 5
cluster.wait_for_height(5, Duration::from_secs(10)).await?;

// Assert state root consistency across all nodes
let roots: Vec<Hash256> = cluster.get_all_state_roots(5).await?;
assert!(roots.windows(2).all(|w| w[0] == w[1]), "State roots diverged across nodes!");
```

---

### 6.3 Fault Injection & Network Partitioning Capabilities

The test harness exposes programmatic fault injection mechanisms:
1. **`cluster.stop_node(node_index)`**: Simulates crash fault (process kill / power loss).
2. **`cluster.restart_node(node_index)`**: Verifies WAL recovery and catch-up synchronization.
3. **`cluster.partition(group_a, group_b)`**: Blocks P2P packets between two node sets.
4. **`cluster.heal_partition()`**: Restores full network connectivity and asserts convergence.
5. **`cluster.inject_byzantine_double_sign(validator_index)`**: Triggers the node to sign two conflicting proposal blocks at the same height/round and verifies that honest nodes detect the evidence and apply slashing.

---

### 6.4 E2E Test Suite Matrix (Tiers 1–4)

#### Tier 1: Core Feature Coverage
- **T1.1: Genesis Boot & Steady Block Production**: 4 validator nodes boot from genesis and produce 10 consecutive finalized blocks.
- **T1.2: Single Transaction Lifecycle**: Fund user A $\rightarrow$ sign transfer to user B $\rightarrow$ submit via RPC $\rightarrow$ verify in mempool $\rightarrow$ verify block inclusion $\rightarrow$ verify balance update of A and B.
- **T1.3: Cryptographic Merkle Proof Verification**: Query Merkle proof for a confirmed transaction and independently verify inclusion against block's `transactions_root`.

#### Tier 2: Boundary & Corner Cases
- **T2.1: Insufficient Balance Rejection**: Attempt transaction with amount + fee > account balance; assert immediate pre-flight rejection with error `-32001`.
- **T2.2: Nonce Gap & Out-of-Order Nonces**: Submit transactions with nonces $[0, 2, 1]$; assert nonce 2 waits in mempool until nonce 1 arrives, then both execute in order $[0, 1, 2]$.
- **T2.3: Signature Tamper / Corrupted Payload**: Mutate 1 bit in signature; assert rejection with `-32000`.
- **T2.4: Historical Tamper Detection**: Corrupt 1 byte in block #2 on disk; assert node fails startup hash-chain validation and halts safely.

#### Tier 3: Cross-Feature Combinations & Resiliency
- **T3.1: 1-Node Crash Resiliency ($N=4$, 1 Failed Node = 25% Stake)**: Kill 1 node; remaining 3 nodes (> 66.7% stake) continue producing blocks without interruption.
- **T3.2: Byzantine Double-Signing & Slashing ($N=4$)**: Malicious validator produces 2 conflicting proposals; honest nodes construct `DoubleSignEvidence`, slash validator's stake, jail the validator, and finalize honest block.
- **T3.3: Network Split & Recovery**: Partition 4 nodes into 2 vs 2 (50% each); assert block production halts (no $2/3+$ quorum); heal partition; assert consensus resumes seamlessly.
- **T3.4: Catch-Up Synchronization**: Take Node 3 offline for 20 blocks; bring Node 3 back online; assert Node 3 downloads missing blocks, validates state roots, and re-joins active consensus.

#### Tier 4: Real-World Application & Stress Scenarios
- **T4.1: High-Throughput Burst**: Flood cluster with 1,000 concurrent transactions; verify fee-per-byte prioritization, zero mempool leaks, and 100% confirmation.
- **T4.2: Explorer WebSocket Live Stream**: Connect simulated Next.js Explorer WebSocket client to Node 0; assert client receives real-time `newHeads` notifications for every finalized block.

---

## 7. Verification, Acceptance & Implementation Roadmap

### 7.1 Automated Verification Commands
```bash
# 1. Workspace Build Verification
cargo build --workspace --all-targets

# 2. Complete Unit & Integration Test Suite
cargo test --workspace

# 3. Multi-Node Local Network E2E Test Suite
cargo test -p aura-test-harness --test e2e_cluster_tests

# 4. Next.js Block Explorer Build & Lint
cd explorer && npm install && npm run build && npm run lint
```

### 7.2 Milestone Integration Plan
- **Milestone 6 (RPC & Metrics)**: Implements `aura-rpc` and `aura-metrics` using the exact JSON-RPC schemas and Prometheus catalogs specified in Sections 2 & 3.
- **Milestone 7 (Block Explorer)**: Implements `explorer/` Next.js frontend using the route tree, UI components, and TypeScript RPC client specified in Section 4.
- **Track A (E2E Test Runner)**: Implements `aura-test-harness` using the multi-node cluster design and test tiers specified in Section 6.
