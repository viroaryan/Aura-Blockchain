use aura_crypto::{Address, Signature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelFrame {
    ConnectTcp {
        stream_id: u32,
        target_host: String,
        target_port: u16,
    },
    ConnectOk {
        stream_id: u32,
    },
    ConnectFail {
        stream_id: u32,
        reason: String,
    },
    Data {
        stream_id: u32,
        data: Vec<u8>,
    },
    Close {
        stream_id: u32,
    },
    BandwidthVoucher {
        session_id: u64,
        bytes_served: u64,
        aur_amount: u64,
        client_address: Address,
        signature: Signature,
    },
    Ping,
    Pong,
}

impl TunnelFrame {
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
