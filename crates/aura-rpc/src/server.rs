use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use aura_crypto::{Address, Hash};
use aura_mempool::Mempool;
use aura_metrics::gather_metrics;
use aura_network::PeerManager;
use aura_primitives::Transaction;
use aura_storage::StateDB;
use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info};

use crate::types::{JsonRpcRequest, JsonRpcResponse};

pub struct RpcState {
    pub state_db: Arc<StateDB>,
    pub mempool: Arc<Mempool>,
    pub peer_manager: Arc<PeerManager>,
    pub chain_id: String,
    pub tx_broadcaster: broadcast::Sender<String>,
}

pub fn build_app(state: Arc<RpcState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", post(handle_rpc).get(health_check))
        .route("/rpc", post(handle_rpc))
        .route("/ws", get(handle_ws))
        .route("/metrics", get(handle_metrics))
        .layer(cors)
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "aura-rpc",
            "version": env!("CARGO_PKG_VERSION")
        })),
    )
}

async fn handle_metrics() -> impl IntoResponse {
    let metrics_text = gather_metrics();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        metrics_text,
    )
}

async fn handle_rpc(
    State(state): State<Arc<RpcState>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let id = req.id.clone();
    let method = req.method.as_str();

    let res = match method {
        "getBlockHeight" => {
            let height = state.state_db.get_latest_height();
            JsonRpcResponse::success(id, json!(height))
        }
        "getBlockByHeight" => {
            let height = req
                .params
                .get("height")
                .and_then(|v| v.as_u64())
                .or_else(|| req.params.as_array()?.first()?.as_u64());

            match height {
                Some(h) => match state.state_db.get_block_by_height(h) {
                    Some(block) => JsonRpcResponse::success(id, json!(block)),
                    None => JsonRpcResponse::error(id, -32602, format!("Block at height {h} not found")),
                },
                None => JsonRpcResponse::error(id, -32602, "Invalid parameters: missing height".into()),
            }
        }
        "getBlockByHash" => {
            let hash_str = req
                .params
                .get("hash")
                .and_then(|v| v.as_str())
                .or_else(|| req.params.as_array()?.first()?.as_str());

            match hash_str.and_then(|s| Hash::from_str(s).ok()) {
                Some(h) => match state.state_db.get_block_by_hash(&h) {
                    Some(block) => JsonRpcResponse::success(id, json!(block)),
                    None => JsonRpcResponse::error(id, -32602, format!("Block {h} not found")),
                },
                None => JsonRpcResponse::error(id, -32602, "Invalid parameters: invalid block hash".into()),
            }
        }
        "getBalance" => {
            let addr_str = req
                .params
                .get("address")
                .and_then(|v| v.as_str())
                .or_else(|| req.params.as_array()?.first()?.as_str());

            match addr_str.and_then(|s| Address::from_str(s).ok()) {
                Some(addr) => {
                    let account = state.state_db.get_account(&addr);
                    JsonRpcResponse::success(id, json!(account.balance))
                }
                None => JsonRpcResponse::error(id, -32602, "Invalid address parameter".into()),
            }
        }
        "getAccount" => {
            let addr_str = req
                .params
                .get("address")
                .and_then(|v| v.as_str())
                .or_else(|| req.params.as_array()?.first()?.as_str());

            match addr_str.and_then(|s| Address::from_str(s).ok()) {
                Some(addr) => {
                    let account = state.state_db.get_account(&addr);
                    JsonRpcResponse::success(id, json!(account))
                }
                None => JsonRpcResponse::error(id, -32602, "Invalid address parameter".into()),
            }
        }
        "sendTransaction" => {
            let tx_res: Result<Transaction, _> = serde_json::from_value(req.params.clone())
                .or_else(|_| {
                    if let Some(arr) = req.params.as_array() {
                        if let Some(first) = arr.first() {
                            return serde_json::from_value(first.clone());
                        }
                    }
                    Err(serde::de::Error::custom("invalid tx format"))
                });

            match tx_res {
                Ok(tx) => match state.mempool.add_transaction(tx.clone(), &state.state_db) {
                    Ok(tx_hash) => {
                        let _ = state
                            .tx_broadcaster
                            .send(json!({ "type": "newTransaction", "tx": tx, "hash": tx_hash }).to_string());
                        JsonRpcResponse::success(id, json!({ "tx_hash": tx_hash.to_hex() }))
                    }
                    Err(e) => JsonRpcResponse::error(id, -32000, format!("Mempool rejection: {e}")),
                },
                Err(e) => JsonRpcResponse::error(id, -32602, format!("Invalid transaction format: {e}")),
            }
        }
        "getValidators" => {
            let vals = state.state_db.get_validators();
            let json_vals: Vec<_> = vals
                .into_iter()
                .map(|(addr, acc)| {
                    json!({
                        "address": addr.to_string(),
                        "pubkey": acc.validator_pubkey.map(|pk| pk.to_hex()),
                        "staked_amount": acc.staked_amount,
                        "is_active": true
                    })
                })
                .collect();
            JsonRpcResponse::success(id, json!(json_vals))
        }
        "getNodeInfo" => {
            let latest_height = state.state_db.get_latest_height();
            let state_root = state.state_db.get_state_root();
            let peer_count = state.peer_manager.active_peer_count();
            let mempool_size = state.mempool.len();

            JsonRpcResponse::success(
                id,
                json!({
                    "chain_id": state.chain_id,
                    "latest_height": latest_height,
                    "state_root": state_root.to_hex(),
                    "peer_count": peer_count,
                    "mempool_size": mempool_size,
                    "version": env!("CARGO_PKG_VERSION")
                }),
            )
        }
        _ => JsonRpcResponse::error(id, -32601, format!("Method '{method}' not found")),
    };

    Json(res)
}

async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RpcState>>,
) -> Response {
    ws.on_upgrade(|socket| websocket_stream(socket, state))
}

async fn websocket_stream(mut socket: WebSocket, state: Arc<RpcState>) {
    let mut rx = state.tx_broadcaster.subscribe();

    while let Ok(msg) = rx.recv().await {
        if socket.send(WsMessage::Text(msg)).await.is_err() {
            break;
        }
    }
}
