use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "aura-node")]
#[command(about = "High-Performance Modular Proof-of-Stake BFT Blockchain Daemon", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a fresh Ed25519 keypair, Bech32 address, and BIP-39 mnemonic
    Keygen {
        #[arg(long, default_value_t = 12)]
        words: usize,
    },
    /// Initialize data directory and genesis state
    Init {
        #[arg(long, default_value = "./.aura-data")]
        data_dir: PathBuf,
        #[arg(long, default_value = "aura-testnet-1")]
        chain_id: String,
    },
    /// Run the Aura full node or validator daemon
    Run {
        #[arg(long, default_value = "./.aura-data")]
        data_dir: PathBuf,
        #[arg(long, default_value = "127.0.0.1:8545")]
        rpc_addr: SocketAddr,
        #[arg(long)]
        validator_key: Option<String>,
        #[arg(long, default_value = "aura-testnet-1")]
        chain_id: String,
    },
    /// Send transactions or check balance via RPC
    Tx {
        #[command(subcommand)]
        action: TxCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum TxCommands {
    /// Send AUR from one address to another
    Send {
        #[arg(long)]
        from_secret: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        amount: u64,
        #[arg(long, default_value_t = 1000)]
        fee: u64,
        #[arg(long, default_value = "http://127.0.0.1:8545")]
        rpc_url: String,
    },
    /// Query account balance and nonce
    Balance {
        #[arg(long)]
        address: String,
        #[arg(long, default_value = "http://127.0.0.1:8545")]
        rpc_url: String,
    },
}
