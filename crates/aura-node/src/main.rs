mod cli;
mod config;

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use clap::Parser;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use aura_crypto::{Address, Hash, HdWallet, KeyPair, DEFAULT_HRP};
use aura_mempool::{Mempool, MempoolConfig};
use aura_metrics::{BLOCK_HEIGHT, MEMPOOL_TXS};
use aura_network::PeerManager;
use aura_primitives::{GenesisAccount, GenesisConfig, GenesisValidator, Transaction, TransactionType};
use aura_rpc::{build_app, RpcState};
use aura_storage::{Account, KeyValueStore, StateDB};

use cli::{Cli, Commands, TxCommands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { words } => {
            let mnemonic = HdWallet::generate_mnemonic(words)?;
            let seed = HdWallet::seed_from_mnemonic(&mnemonic, "")?;
            let keypair = HdWallet::derive_keypair(&seed, 0, 0, 0)?;
            let pubkey = keypair.public_key();
            let addr = Address::from_pubkey(&pubkey);
            let bech32 = addr.to_bech32(DEFAULT_HRP)?;

            println!("============================================================");
            println!("   Aura Blockchain - Cryptographic Key Generation");
            println!("============================================================");
            println!("Mnemonic:    {}", mnemonic);
            println!("Address:     {}", bech32);
            println!("Public Key:  {}", pubkey.to_hex());
            println!("Secret Key:  {}", hex::encode(keypair.secret_bytes()));
            println!("============================================================");
        }
        Commands::Init { data_dir, chain_id } => {
            std::fs::create_dir_all(&data_dir)?;
            let kv_path = data_dir.join("state.db");
            let kv = Arc::new(KeyValueStore::open(&kv_path)?);
            let state_db = StateDB::new(kv);

            let val_kp = KeyPair::generate();
            let val_addr = Address::from_pubkey(&val_kp.public_key());

            let genesis = GenesisConfig {
                chain_id: chain_id.clone(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                accounts: vec![GenesisAccount {
                    address: val_addr,
                    balance: 10_000_000_000,
                }],
                validators: vec![GenesisValidator {
                    address: val_addr,
                    pubkey: val_kp.public_key(),
                    stake: 1_000_000_000,
                }],
                initial_state_root: Hash::ZERO,
            };

            let state_root = state_db.apply_genesis(&genesis)?;
            info!(
                chain_id = %chain_id,
                state_root = %state_root,
                genesis_validator = %val_addr.to_bech32(DEFAULT_HRP)?,
                "Initialized Aura blockchain genesis state successfully"
            );
        }
        Commands::Run {
            data_dir,
            rpc_addr,
            validator_key,
            chain_id,
        } => {
            std::fs::create_dir_all(&data_dir)?;
            let kv_path = data_dir.join("state.db");
            let kv = Arc::new(KeyValueStore::open(&kv_path)?);
            let state_db = Arc::new(StateDB::new(kv));

            // Apply default testnet genesis if new database
            if state_db.get_latest_height() == 0 && state_db.get_block_by_height(0).is_none() {
                let val_kp = validator_key
                    .as_ref()
                    .and_then(|k| hex::decode(k).ok())
                    .and_then(|bytes| bytes.try_into().ok())
                    .map(|b| KeyPair::from_secret_bytes(&b))
                    .unwrap_or_else(KeyPair::generate);

                let val_addr = Address::from_pubkey(&val_kp.public_key());

                let genesis = GenesisConfig {
                    chain_id: chain_id.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    accounts: vec![GenesisAccount {
                        address: val_addr,
                        balance: 100_000_000_000, // 100,000 AUR
                    }],
                    validators: vec![GenesisValidator {
                        address: val_addr,
                        pubkey: val_kp.public_key(),
                        stake: 10_000_000_000, // 10,000 AUR
                    }],
                    initial_state_root: Hash::ZERO,
                };

                let root = state_db.apply_genesis(&genesis)?;
                info!(
                    state_root = %root,
                    validator = %val_addr.to_bech32(DEFAULT_HRP)?,
                    "Genesis block initialized"
                );
            }

            let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
            let peer_manager = Arc::new(PeerManager::new());
            let (tx_sender, _) = broadcast::channel(1024);

            let rpc_state = Arc::new(RpcState {
                state_db: state_db.clone(),
                mempool: mempool.clone(),
                peer_manager: peer_manager.clone(),
                chain_id: chain_id.clone(),
                tx_broadcaster: tx_sender.clone(),
            });

            // Start JSON-RPC & WebSocket Server
            let app = build_app(rpc_state);
            info!("Starting JSON-RPC / WebSocket Server on http://{}", rpc_addr);
            let listener = tokio::net::TcpListener::bind(rpc_addr).await?;

            tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, app).await {
                    error!("RPC Server error: {}", e);
                }
            });

            // If running as validator, start BFT block production loop
            if let Some(ref sec_hex) = validator_key {
                if let Ok(sec_bytes) = hex::decode(sec_hex) {
                    if let Ok(arr) = sec_bytes.as_slice().try_into() {
                        let val_keypair = KeyPair::from_secret_bytes(arr);
                        let val_engine = aura_consensus::ConsensusEngine::new(
                            val_keypair,
                            state_db.clone(),
                            mempool.clone(),
                            chain_id.clone(),
                        );

                        info!("Validator node active — starting block production loop");
                        let tx_bc = tx_sender.clone();

                        tokio::spawn(async move {
                            let mut interval = tokio::time::interval(Duration::from_secs(2));
                            loop {
                                interval.tick().await;

                                if let Ok(proposal) = val_engine.create_proposal() {
                                    if let Ok(prevote) = val_engine.handle_proposal(&proposal) {
                                        if let Some(precommit) = val_engine.handle_prevote(&prevote) {
                                            if let Some(qc) = val_engine.handle_precommit(&precommit) {
                                                let h = qc.height;
                                                BLOCK_HEIGHT.set(h as i64);
                                                MEMPOOL_TXS.set(val_engine_mempool_size(&mempool) as i64);

                                                let _ = tx_bc.send(
                                                    serde_json::json!({
                                                        "type": "newHead",
                                                        "height": h,
                                                        "hash": qc.block_hash.to_hex(),
                                                    })
                                                    .to_string(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }

            info!("Aura Node running! Press Ctrl+C to terminate.");
            tokio::signal::ctrl_c().await?;
            info!("Shutting down Aura Node cleanly...");
        }
        Commands::Tx { action } => match action {
            TxCommands::Balance { address, rpc_url } => {
                println!("Querying balance for {} from {}...", address, rpc_url);
                let payload = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getAccount",
                    "params": { "address": address }
                });
                let resp = rpc_post(&rpc_url, &payload.to_string()).await?;
                println!("Account Info: {}", resp);
            }
            TxCommands::Send {
                from_secret,
                to,
                amount,
                fee,
                rpc_url,
            } => {
                let sec_bytes = hex::decode(from_secret)?;
                let arr: [u8; 32] = sec_bytes
                    .try_into()
                    .map_err(|_| "Secret key must be exactly 32 bytes (64 hex characters)")?;
                let sender_kp = KeyPair::from_secret_bytes(&arr);
                let sender_addr = Address::from_pubkey(&sender_kp.public_key());
                let recipient_addr = Address::from_str(&to)?;

                // Query account nonce
                let acc_query = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getAccount",
                    "params": { "address": sender_addr.to_string() }
                });
                let acc_res = rpc_post(&rpc_url, &acc_query.to_string()).await?;
                let acc_val: serde_json::Value = serde_json::from_str(&acc_res)?;
                let nonce = acc_val
                    .get("result")
                    .and_then(|r| r.get("nonce"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0)
                    + 1;

                let tx = Transaction::new_unsigned(
                    sender_addr,
                    recipient_addr,
                    amount,
                    fee,
                    nonce,
                    TransactionType::Transfer,
                    vec![],
                    sender_kp.public_key(),
                )
                .sign(&sender_kp)?;

                let send_query = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "sendTransaction",
                    "params": tx
                });

                let send_res = rpc_post(&rpc_url, &send_query.to_string()).await?;
                println!("============================================================");
                println!("   Transaction Broadcasted Successfully!");
                println!("============================================================");
                println!("Sender:    {}", sender_addr);
                println!("Recipient: {}", recipient_addr);
                println!("Amount:    {} micro-AUR", amount);
                println!("Fee:       {} micro-AUR", fee);
                println!("Nonce:     #{}", nonce);
                println!("Result:    {}", send_res);
                println!("============================================================");
            }
        },
    }

    Ok(())
}

async fn rpc_post(
    url_str: &str,
    json_body: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let clean_url = url_str.strip_prefix("http://").unwrap_or(url_str);
    let mut parts = clean_url.split('/');
    let host_port = parts.next().unwrap_or("127.0.0.1:8545");
    let path = format!("/{}", parts.collect::<Vec<_>>().join("/"));
    let actual_path = if path == "/" { "/rpc" } else { &path };

    let mut stream = TcpStream::connect(host_port).await?;

    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        actual_path,
        host_port,
        json_body.len(),
        json_body
    );

    stream.write_all(req.as_bytes()).await?;

    let mut resp_bytes = Vec::new();
    stream.read_to_end(&mut resp_bytes).await?;

    let resp_str = String::from_utf8_lossy(&resp_bytes);
    if let Some(pos) = resp_str.find("\r\n\r\n") {
        Ok(resp_str[pos + 4..].to_string())
    } else {
        Ok(resp_str.to_string())
    }
}

fn val_engine_mempool_size(mempool: &Mempool) -> usize {
    mempool.len()
}
