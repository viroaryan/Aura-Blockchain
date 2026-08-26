use aura_crypto::{
    hash_bytes, hash_with_domain, Address, Hash, HdWallet, KeyPair, DEFAULT_HRP, DOMAIN_TX,
};
use aura_primitives::{
    Block, BlockHeader, MerkleTree, Transaction, TransactionType,
};

#[test]
fn test_tamper_detection_breaks_hash_chain() {
    let kp = KeyPair::generate();
    let addr = Address::from_pubkey(&kp.public_key());

    let mut header = BlockHeader::new_unsigned(
        1,
        "aura-mainnet".into(),
        1,
        0,
        Hash::ZERO,
        Hash::ZERO,
        Hash::ZERO,
        Hash::ZERO,
        1700000000,
        addr,
    );
    header.sign(&kp).unwrap();
    let original_hash = header.header_hash();

    // Any 1-bit change in version, height, timestamp, or proposer breaks hash
    let mut tampered_header = header.clone();
    tampered_header.height = 2;
    assert_ne!(original_hash, tampered_header.header_hash());

    tampered_header = header.clone();
    tampered_header.timestamp += 1;
    assert_ne!(original_hash, tampered_header.header_hash());
}

#[test]
fn test_rfc6962_merkle_tree_consistency_and_proofs() {
    let mut payloads = Vec::new();
    for i in 0..16 {
        payloads.push(format!("aura_tx_payload_{i}").into_bytes());
    }

    let tree = MerkleTree::from_leaf_payloads(&payloads);
    let root = tree.root();

    for (idx, payload) in payloads.iter().enumerate() {
        let proof = tree.generate_proof(idx).expect("proof must exist");
        assert!(proof.verify(&root, payload));

        // Tampering payload fails proof verification
        let mut tampered = payload.clone();
        tampered.push(0xFF);
        assert!(!proof.verify(&root, &tampered));
    }
}

#[test]
fn test_bech32_address_prefix_and_checksum() {
    let kp = KeyPair::generate();
    let addr = Address::from_pubkey(&kp.public_key());
    let bech32_str = addr.to_bech32(DEFAULT_HRP).unwrap();

    assert!(bech32_str.starts_with("aura1"));
    let decoded = Address::from_bech32(&bech32_str, DEFAULT_HRP).unwrap();
    assert_eq!(addr, decoded);
}
