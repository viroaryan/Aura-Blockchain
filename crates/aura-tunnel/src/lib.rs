pub mod billing;
pub mod cipher;
pub mod client;
pub mod handshake;
pub mod host;
pub mod protocol;

pub use billing::BandwidthVoucher;
pub use cipher::{CipherError, TunnelCipher};
pub use client::Socks5TunnelClient;
pub use handshake::{HandshakeInit, HandshakeResponse, TunnelHandshake};
pub use host::HostExitNode;
pub use protocol::TunnelFrame;

#[cfg(test)]
mod tests {
    use super::*;
    use aura_crypto::KeyPair;

    #[test]
    fn test_tunnel_cipher_encrypt_decrypt_and_tamper_detection() {
        let session_key = [0x42u8; 32];
        let mut client_cipher = TunnelCipher::new(session_key);
        let mut host_cipher = TunnelCipher::new(session_key);

        let secret_payload = b"GET /watch?v=dQw4w9WgXcQ HTTP/1.1\r\nHost: youtube.com\r\n\r\n";
        let encrypted_frame = client_cipher.encrypt(secret_payload);

        // Verify frame is authenticated and decrypted
        let decrypted = host_cipher.decrypt(&encrypted_frame).unwrap();
        assert_eq!(decrypted, secret_payload);

        // Tamper test: Modify 1 bit in ciphertext -> auth tag mismatch!
        let mut tampered_frame = encrypted_frame.clone();
        tampered_frame[42] ^= 0x01;
        assert_eq!(
            host_cipher.decrypt(&tampered_frame),
            Err(CipherError::TagMismatch)
        );
    }

    #[test]
    fn test_ephemeral_handshake_session_key_derivation() {
        let client_kp = KeyPair::generate();
        let host_kp = KeyPair::generate();

        let session_key_1 = TunnelHandshake::derive_session_key(
            &client_kp.public_key(),
            &host_kp.public_key(),
        );

        let session_key_2 = TunnelHandshake::derive_session_key(
            &client_kp.public_key(),
            &host_kp.public_key(),
        );

        assert_eq!(session_key_1, session_key_2);
        assert_ne!(session_key_1, [0u8; 32]);
    }

    #[test]
    fn test_bandwidth_voucher_signing_and_verification() {
        let client_kp = KeyPair::generate();
        let voucher = BandwidthVoucher::new_signed(1001, 52_428_800, 50_000, &client_kp);

        assert!(voucher.verify());

        let mut tampered_voucher = voucher.clone();
        tampered_voucher.aur_amount = 1; // Attempt to cheat amount
        assert!(!tampered_voucher.verify());
    }
}
