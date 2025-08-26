use blake3::Hasher;
use hex::ToHex;
use k256::ecdsa::{SigningKey, Signature};
use k256::ecdsa::signature::hazmat::PrehashSigner;

use sha2::Digest;
use crate::types::WorkReceipt;

pub struct Secp { sk: SigningKey }

impl Secp {
    pub fn from_hex(sk_hex: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(sk_hex)?;
        Ok(Self { sk: SigningKey::from_bytes(bytes.as_slice().into())? })
    }
    pub fn sign_receipt(&self, r: &WorkReceipt) -> anyhow::Result<String> {
        // Hash a stable serialization (here: JSON without sig, then blake3, then sha256)
        let mut copy = r.clone();
        copy.sig_hex = String::new();
        let json = serde_json::to_vec(&copy)?;
        let mut h = Hasher::new(); h.update(&json);
        let b3 = h.finalize();
        let digest = sha2::Sha256::digest(b3.as_bytes());
        let sig: Signature = self.sk.sign_prehash(&digest)?;
        Ok(sig.to_vec().encode_hex::<String>())
    }
    pub fn pubkey_hex_compressed(&self) -> String {
        let vk = self.sk.verifying_key();
        let ep = vk.to_encoded_point(true);
        hex::encode(ep.as_bytes())
    }
}
