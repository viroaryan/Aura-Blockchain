pub mod address;
pub mod hash;
pub mod hd;
pub mod keys;

pub use address::{Address, AddressError, DEFAULT_HRP};
pub use hash::{
    hash_bytes, hash_two, hash_with_domain, Hash, HashError, DOMAIN_BLOCK_HEADER,
    DOMAIN_MERKLE_LEAF, DOMAIN_MERKLE_NODE, DOMAIN_SMT_LEAF, DOMAIN_SMT_NODE, DOMAIN_TX,
    DOMAIN_VOTE,
};
pub use hd::{HdWallet, HdWalletError, AURA_COIN_TYPE};
pub use keys::{CryptoError, KeyPair, PublicKey, Signature};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hashing_and_domain_separation() {
        let msg = b"aura blockchain transaction payload";
        let h1 = hash_bytes(msg);
        let h2 = hash_with_domain(DOMAIN_TX, msg);
        let h3 = hash_with_domain(DOMAIN_BLOCK_HEADER, msg);

        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_eq!(h1, hash_bytes(msg));
    }

    #[test]
    fn test_ed25519_sign_and_verify() {
        let keypair = KeyPair::generate();
        let pubkey = keypair.public_key();

        let message = b"transfer 1000000 AUR to aura1receiver";
        let signature = keypair.sign(message);

        assert!(pubkey.verify(message, &signature).is_ok());

        let corrupted_message = b"transfer 9999999 AUR to aura1receiver";
        assert!(pubkey.verify(corrupted_message, &signature).is_err());
    }

    #[test]
    fn test_bech32_address_derivation_and_roundtrip() {
        let keypair = KeyPair::generate();
        let pubkey = keypair.public_key();

        let addr = Address::from_pubkey(&pubkey);
        let bech32_str = addr.to_bech32(DEFAULT_HRP).unwrap();

        assert!(bech32_str.starts_with("aura1"));

        let decoded_addr = Address::from_bech32(&bech32_str, DEFAULT_HRP).unwrap();
        assert_eq!(addr, decoded_addr);

        // Invalid HRP check
        assert!(Address::from_bech32(&bech32_str, "eth").is_err());
    }

    #[test]
    fn test_hd_wallet_bip39_and_slip0010() {
        let mnemonic = HdWallet::generate_mnemonic(12).unwrap();
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12);

        let seed = HdWallet::seed_from_mnemonic(&mnemonic, "").unwrap();
        let keypair_0 = HdWallet::derive_keypair(&seed, 0, 0, 0).unwrap();
        let keypair_1 = HdWallet::derive_keypair(&seed, 0, 0, 1).unwrap();

        assert_ne!(
            keypair_0.public_key().as_bytes(),
            keypair_1.public_key().as_bytes()
        );

        // Deterministic reproduction
        let keypair_0_again = HdWallet::derive_keypair(&seed, 0, 0, 0).unwrap();
        assert_eq!(
            keypair_0.public_key().as_bytes(),
            keypair_0_again.public_key().as_bytes()
        );
    }
}
