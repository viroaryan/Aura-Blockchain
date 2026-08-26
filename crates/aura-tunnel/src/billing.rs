use aura_crypto::{hash_with_domain, Address, KeyPair, PublicKey, Signature};
use serde::{Deserialize, Serialize};

const DOMAIN_VOUCHER: &[u8] = b"AURA_BANDWIDTH_VOUCHER_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthVoucher {
    pub session_id: u64,
    pub bytes_served: u64,
    pub aur_amount: u64,
    pub client_address: Address,
    pub client_pubkey: PublicKey,
    pub signature: Signature,
}

impl BandwidthVoucher {
    pub fn signing_bytes(
        session_id: u64,
        bytes_served: u64,
        aur_amount: u64,
        client_address: &Address,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&session_id.to_be_bytes());
        data.extend_from_slice(&bytes_served.to_be_bytes());
        data.extend_from_slice(&aur_amount.to_be_bytes());
        data.extend_from_slice(client_address.as_bytes());
        data
    }

    pub fn new_signed(
        session_id: u64,
        bytes_served: u64,
        aur_amount: u64,
        client_keypair: &KeyPair,
    ) -> Self {
        let client_address = Address::from_pubkey(&client_keypair.public_key());
        let data = Self::signing_bytes(session_id, bytes_served, aur_amount, &client_address);
        let hash = hash_with_domain(DOMAIN_VOUCHER, &data);
        let signature = client_keypair.sign(hash.as_bytes());

        Self {
            session_id,
            bytes_served,
            aur_amount,
            client_address,
            client_pubkey: client_keypair.public_key(),
            signature,
        }
    }

    pub fn verify(&self) -> bool {
        if Address::from_pubkey(&self.client_pubkey) != self.client_address {
            return false;
        }
        let data = Self::signing_bytes(
            self.session_id,
            self.bytes_served,
            self.aur_amount,
            &self.client_address,
        );
        let hash = hash_with_domain(DOMAIN_VOUCHER, &data);
        self.client_pubkey.verify(hash.as_bytes(), &self.signature).is_ok()
    }
}
