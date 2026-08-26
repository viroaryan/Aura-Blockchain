use aura_crypto::{hash_with_domain, Address, Hash, KeyPair, DOMAIN_SMT_LEAF};
use aura_storage::{Account, KeyValueStore, SparseMerkleTree, StateDB, WriteAheadLog};

#[test]
fn test_sparse_merkle_tree_root_determinism() {
    let mut tree1 = SparseMerkleTree::new();
    let mut tree2 = SparseMerkleTree::new();

    let key_a = Hash::new([0x11; 32]);
    let key_b = Hash::new([0x22; 32]);

    tree1.update(key_a, b"val_1");
    tree1.update(key_b, b"val_2");

    tree2.update(key_b, b"val_2");
    tree2.update(key_a, b"val_1");

    // Independent of insertion order, root must be identical
    assert_eq!(tree1.root(), tree2.root());
}

#[test]
fn test_wal_crash_resilience_and_recovery() {
    let temp_dir = tempfile::tempdir().unwrap();
    let wal_path = temp_dir.path().join("aura_wal.log");

    {
        let wal = WriteAheadLog::open(&wal_path).unwrap();
        wal.append(b"entry_1").unwrap();
        wal.append(b"entry_2").unwrap();
    }

    // Recover from WAL
    let wal = WriteAheadLog::open(&wal_path).unwrap();
    let recovered = wal.recover_and_truncate().unwrap();
    assert_eq!(recovered.len(), 2);
    assert_eq!(recovered[0], b"entry_1");
    assert_eq!(recovered[1], b"entry_2");
}
