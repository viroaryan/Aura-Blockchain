use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

use crate::cipher::TunnelCipher;
use crate::protocol::TunnelFrame;

static STREAM_COUNTER: AtomicU32 = AtomicU32::new(1);

pub struct Socks5TunnelClient {
    socks_addr: SocketAddr,
    host_tunnel_addr: SocketAddr,
    session_key: [u8; 32],
}

impl Socks5TunnelClient {
    pub fn new(
        socks_addr: SocketAddr,
        host_tunnel_addr: SocketAddr,
        session_key: [u8; 32],
    ) -> Self {
        Self {
            socks_addr,
            host_tunnel_addr,
            session_key,
        }
    }

    /// Start SOCKS5 proxy listener on 127.0.0.1:1080.
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.socks_addr).await?;
        info!("Aura SOCKS5 Client Proxy active on {}", self.socks_addr);
        info!("Routing all browser / OS traffic through remote host at {}", self.host_tunnel_addr);

        let host_addr = self.host_tunnel_addr;
        let session_key = self.session_key;

        loop {
            let (socks_socket, _) = listener.accept().await?;
            tokio::spawn(async move {
                if let Err(e) = handle_socks5_connection(socks_socket, host_addr, session_key).await {
                    debug!("SOCKS5 connection ended: {}", e);
                }
            });
        }
    }
}

async fn handle_socks5_connection(
    mut client_socket: TcpStream,
    host_addr: SocketAddr,
    session_key: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. SOCKS5 Method Negotiation (RFC 1928)
    let mut header = [0u8; 2];
    client_socket.read_exact(&mut header).await?;
    if header[0] != 0x05 {
        return Err("Unsupported SOCKS version".into());
    }

    let num_methods = header[1] as usize;
    let mut methods = vec![0u8; num_methods];
    client_socket.read_exact(&mut methods).await?;

    // Respond with 0x05 (version 5), 0x00 (NO AUTH REQUIRED)
    client_socket.write_all(&[0x05, 0x00]).await?;

    // 2. SOCKS5 Request Details
    let mut req_header = [0u8; 4];
    client_socket.read_exact(&mut req_header).await?;

    let cmd = req_header[1];
    if cmd != 0x01 {
        // Only CMD CONNECT supported
        client_socket
            .write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await?;
        return Err("Only CONNECT command supported".into());
    }

    let atyp = req_header[3];
    let target_host = match atyp {
        0x01 => {
            // IPv4
            let mut ip = [0u8; 4];
            client_socket.read_exact(&mut ip).await?;
            format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
        }
        0x03 => {
            // Domain Name
            let len = client_socket.read_u8().await? as usize;
            let mut domain = vec![0u8; len];
            client_socket.read_exact(&mut domain).await?;
            String::from_utf8(domain)?
        }
        _ => {
            return Err("Unsupported address type".into());
        }
    };

    let target_port = client_socket.read_u16().await?;

    // 3. Connect to Remote Aura Tunnel Host
    let mut tunnel_socket = TcpStream::connect(host_addr).await?;
    let mut cipher = TunnelCipher::new(session_key);

    let stream_id = STREAM_COUNTER.fetch_add(1, Ordering::Relaxed);

    // Send ConnectTcp Frame
    let frame = TunnelFrame::ConnectTcp {
        stream_id,
        target_host: target_host.clone(),
        target_port,
    };
    let plain = frame.to_bytes();
    let enc = cipher.encrypt(&plain);
    let len = (enc.len() as u32).to_be_bytes();

    tunnel_socket.write_all(&len).await?;
    tunnel_socket.write_all(&enc).await?;

    // Respond SOCKS5 SUCCESS (0x00)
    client_socket
        .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
        .await?;

    info!(target = %target_host, port = target_port, "Established encrypted tunnel to remote host");

    // 4. Bi-directional encrypted forwarding
    let (mut client_read, mut client_write) = client_socket.into_split();
    let (mut tunnel_read, mut tunnel_write) = tunnel_socket.into_split();

    // Client -> Tunnel
    let f1 = async {
        let mut buf = vec![0u8; 16384];
        let mut local_cipher = TunnelCipher::new(session_key);
        loop {
            let n = client_read.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            let frame = TunnelFrame::Data {
                stream_id,
                data: buf[..n].to_vec(),
            };
            let plain = frame.to_bytes();
            let enc = local_cipher.encrypt(&plain);
            tunnel_write.write_all(&(enc.len() as u32).to_be_bytes()).await?;
            tunnel_write.write_all(&enc).await?;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    };

    let _ = f1.await;

    Ok(())
}
