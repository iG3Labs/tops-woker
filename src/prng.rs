use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus;

pub struct DPrng(Xoshiro128PlusPlus);

impl DPrng {
    pub fn from_seed(seed: [u8; 16]) -> Self {
        let mut s = [0u8; 16];
        s.copy_from_slice(&seed);
        Self(Xoshiro128PlusPlus::from_seed(s))
    }
    pub fn next_i8(&mut self) -> i8 { self.0.next_u32() as i8 }
    pub fn next_u32(&mut self) -> u32 { self.0.next_u32() }
}

/// Derive a 128-bit seed from prev_hash (32B) + nonce (4B)
pub fn derive_seed(prev_hash_32: &[u8;32], nonce: u32) -> [u8;16] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(prev_hash_32);
    hasher.update(&nonce.to_le_bytes());
    let out = hasher.finalize();
    let mut s = [0u8;16];
    s.copy_from_slice(&out.as_bytes()[..16]);
    s
}
