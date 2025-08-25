use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sizes { pub m: usize, pub n: usize, pub k: usize, pub batch: usize }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkReceipt {
    pub device_did: String,
    pub epoch_id: u64,
    pub prev_hash_hex: String,
    pub nonce: u32,
    pub work_root_hex: String,
    pub sizes: Sizes,
    pub time_ms: u64,
    pub kernel_ver: String,
    pub driver_hint: String,
    pub sig_hex: String, // secp256k1 signature (DER or compact)
}
