pub mod server;
pub mod types;

pub use server::{build_app, RpcState};
pub use types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use aura_crypto::{Address, Hash, KeyPair};
    use aura_mempool::{Mempool, MempoolConfig};
    use aura_network::PeerManager;
    use aura_primitives::{GenesisAccount, GenesisConfig, GenesisValidator};
    use aura_storage::StateDB;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::broadcast;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_rpc_get_block_height_and_node_info() {
        let val_kp = KeyPair::generate();
        let val_addr = Address::from_pubkey(&val_kp.public_key());

        let state_db = Arc::new(StateDB::open_in_memory());
        let genesis = GenesisConfig {
            chain_id: "aura-testnet".into(),
            timestamp: 1700000000,
            accounts: vec![GenesisAccount {
                address: val_addr,
                balance: 10_000_000,
            }],
            validators: vec![GenesisValidator {
                address: val_addr,
                pubkey: val_kp.public_key(),
                stake: 1_000_000,
            }],
            initial_state_root: Hash::ZERO,
        };
        state_db.apply_genesis(&genesis).unwrap();

        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
        let peer_manager = Arc::new(PeerManager::new());
        let (tx_sender, _) = broadcast::channel(100);

        let state = Arc::new(RpcState {
            state_db,
            mempool,
            peer_manager,
            chain_id: "aura-testnet".into(),
            tx_broadcaster: tx_sender,
        });

        let app = build_app(state);

        let req = Request::builder()
            .method("POST")
            .uri("/rpc")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getNodeInfo",
                    "params": {}
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
