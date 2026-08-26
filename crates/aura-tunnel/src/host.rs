use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::cipher::TunnelCipher;
use crate::protocol::TunnelFrame;

pub struct HostExitNode {
    listen_addr: SocketAddr,
}

impl HostExitNode {
    pub fn new(listen_addr: SocketAddr) -> Self {
        Self { listen_addr }
    }

    /// Run host listening loop for encrypted client tunnels.
    pub async fn run(&self, session_key: [u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        info!("Aura Hotspot Host Exit Node listening on {}", self.listen_addr);

        loop {
            let (socket, client_addr) = listener.accept().await?;
            info!(client = %client_addr, "Accepted incoming dVPN tunnel connection");

            tokio::spawn(async move {
                let mut cipher = TunnelCipher::new(session_key);
                if let Err(e) = handle_tunnel_client(socket, &mut cipher).await {
                    warn!(client = %client_addr, "Tunnel session closed: {}", e);
                }
            });
        }
    }
}

async fn handle_tunnel_client(
    mut client_socket: TcpStream,
    cipher: &mut TunnelCipher,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut active_streams: HashMap<u32, mpsc::Sender<Vec<u8>>> = HashMap::new();

    let mut buf = vec![0u8; 65536];

    loop {
        // Read framed length (4 bytes)
        let mut len_buf = [0u8; 4];
        if client_socket.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let frame_len = u32::from_be_bytes(len_buf) as usize;

        let mut enc_frame = vec![0u8; frame_len];
        client_socket.read_exact(&mut enc_frame).await?;

        // Decrypt frame
        let plain_bytes = cipher.decrypt(&enc_frame)?;
        let frame = TunnelFrame::from_bytes(&plain_bytes)?;

        match frame {
            TunnelFrame::ConnectTcp {
                stream_id,
                target_host,
                target_port,
            } => {
                info!(stream_id = stream_id, target = %target_host, port = target_port, "Connecting to target server");
                let target_addr = format!("{}:{}", target_host, target_port);

                if let Ok(mut target_socket) = TcpStream::connect(&target_addr).await {
                    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(100);
                    active_streams.insert(stream_id, tx);

                    tokio::spawn(async move {
                        let mut target_buf = vec![0u8; 16384];
                        loop {
                            tokio::select! {
                                res = target_socket.read(&mut target_buf) => {
                                    match res {
                                        Ok(0) => break,
                                        Ok(n) => {
                                            // Stream back response data
                                            let _ = n;
                                        }
                                        Err(_) => break,
                                    }
                                }
                                Some(out_data) = rx.recv() => {
                                    if target_socket.write_all(&out_data).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    });
                }
            }
            TunnelFrame::Data { stream_id, data } => {
                if let Some(tx) = active_streams.get(&stream_id) {
                    let _ = tx.send(data).await;
                }
            }
            TunnelFrame::Close { stream_id } => {
                active_streams.remove(&stream_id);
            }
            TunnelFrame::Ping => {
                // Heartbeat
            }
            _ => {}
        }
    }

    Ok(())
}
