use std::net::SocketAddr;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub chain_id: String,
    pub data_dir: PathBuf,
    pub rpc_addr: SocketAddr,
    pub p2p_port: u16,
    pub validator_key: Option<String>, // Hex encoded secret key
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            chain_id: "aura-mainnet-1".into(),
            data_dir: PathBuf::from("./.aura-data"),
            rpc_addr: "127.0.0.1:8545".parse().unwrap(),
            p2p_port: 26656,
            validator_key: None,
        }
    }
}
