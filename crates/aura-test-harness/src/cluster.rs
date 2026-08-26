use std::sync::Arc;
use aura_crypto::{Address, Hash, KeyPair, DEFAULT_HRP};
use aura_mempool::{Mempool, MempoolConfig};
use aura_primitives::{GenesisAccount, GenesisConfig, GenesisValidator};
use aura_storage::StateDB;

pub struct TestNode {
    pub keypair: KeyPair,
    pub address: Address,
    pub state_db: Arc<StateDB>,
    pub mempool: Arc<Mempool>,
}

pub struct TestCluster {
    pub nodes: Vec<TestNode>,
    pub genesis: GenesisConfig,
}

impl TestCluster {
    /// Spawn an in-memory N-validator cluster with uniform or custom stake.
    pub fn spawn_validators(count: usize) -> Self {
        let mut keypairs = Vec::new();
        let mut genesis_accounts = Vec::new();
        let mut genesis_validators = Vec::new();

        for _ in 0..count {
            let kp = KeyPair::generate();
            let addr = Address::from_pubkey(&kp.public_key());

            genesis_accounts.push(GenesisAccount {
                address: addr,
                balance: 100_000_000,
            });

            genesis_validators.push(GenesisValidator {
                address: addr,
                pubkey: kp.public_key(),
                stake: 10_000_000,
            });

            keypairs.push((kp, addr));
        }

        let genesis = GenesisConfig {
            chain_id: "aura-cluster-test".into(),
            timestamp: 1700000000,
            accounts: genesis_accounts,
            validators: genesis_validators,
            initial_state_root: Hash::ZERO,
        };

        let mut nodes = Vec::new();
        for (kp, addr) in keypairs {
            let state_db = Arc::new(StateDB::open_in_memory());
            state_db
                .apply_genesis(&genesis)
                .expect("genesis apply must succeed");

            let mempool = Arc::new(Mempool::new(MempoolConfig::default()));

            nodes.push(TestNode {
                keypair: kp,
                address: addr,
                state_db,
                mempool,
            });
        }

        Self { nodes, genesis }
    }

    /// Produce and commit one BFT block across all nodes synchronously.
    pub fn step_block_round(&self) -> Hash {
        let proposer_node = &self.nodes[0];
        let height = proposer_node.state_db.get_latest_height() + 1;

        let val_engine = aura_consensus::ConsensusEngine::new(
            proposer_node.keypair.clone(),
            proposer_node.state_db.clone(),
            proposer_node.mempool.clone(),
            self.genesis.chain_id.clone(),
        );

        let proposal = val_engine.create_proposal().expect("proposal created");
        let prevote = val_engine.handle_proposal(&proposal).expect("prevote cast");
        let precommit = val_engine.handle_prevote(&prevote).expect("precommit cast");
        let qc = val_engine.handle_precommit(&precommit).expect("block committed");

        // Propagate committed block to all other peer nodes
        let syncer = aura_network::ChainSyncer::new(self.nodes[1].state_db.clone());
        let _ = syncer.apply_block_batch(&[proposal]);

        qc.block_hash
    }
}
