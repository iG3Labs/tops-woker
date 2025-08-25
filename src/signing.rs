use blake3::Hasher;
use hex::ToHex;
use k256::ecdsa::{signature::Signer, SigningKey, Signature};
use crate::types::{WorkReceipt, Sizes};

pub struct Secp { sk: SigningKey }

impl Secp {
    pub fn from_hex(sk_hex: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(sk_hex)?;
        Ok(Self { sk: SigningKey::from_bytes(bytes.as_slice().into())? })
    }
    pub fn sign_receipt(&self, r: &WorkReceipt) -> anyhow::Result<String> {
        // Hash a stable serialization (here: JSON, then blake3)
        let json = serde_json::to_vec(r)?;
        let mut h = Hasher::new(); h.update(&json);
        let msg = h.finalize();
        let sig: Signature = self.sk.sign(msg.as_bytes());
        Ok(sig.to_vec().encode_hex::<String>())
    }
}
