use anyhow::Result;
use blake3::Hasher;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use crate::{prng::DPrng, gpu::GpuExec};
use crate::types::{Sizes};

fn gen_int8_matrix(pr: &mut DPrng, len: usize) -> Vec<i8> {
    let mut v = Vec::with_capacity(len);
    for _ in 0..len { v.push(pr.next_i8()); }
    v
}

// Optional lightweight permutation/sign-flip (omitted here for brevity). Add later if needed.

pub struct AttemptOutput {
    pub work_root: [u8;32],
    pub y1: Vec<i8>,
    pub y2_samples: Vec<i8>,
    pub elapsed_ms: u64,
}

pub fn run_attempt(
    gpu: &GpuExec,
    prev_hash_32: &[u8;32],
    nonce: u32,
    sizes: &Sizes,
) -> Result<AttemptOutput> {
    let seed = crate::prng::derive_seed(prev_hash_32, nonce);
    let mut pr = DPrng::from_seed(seed);

    let m = sizes.m; let n = sizes.n; let k = sizes.k;

    // Fixed reference weights should be baked-in; for MVP we pseudo-randomize with a fixed global seed.
    // In production: store int8 W1, W2 in files or embed as consts.
    let mut w_seed = [0u8;16]; w_seed.copy_from_slice(&blake3::hash(b"FIXED_WEIGHTS_V1").as_bytes()[..16]);
    let mut prw = DPrng::from_seed(w_seed);

    let a = gen_int8_matrix(&mut pr, m*k);
    let w1 = gen_int8_matrix(&mut prw, k*n);
    let w2 = gen_int8_matrix(&mut prw, n*n);

    // Scale for requantization; tune later
    let scale_num = 1i32;
    let scale_den = 256i32;

    let t0 = std::time::Instant::now();
    let y1 = gpu.gemm_int8_relu_q(&a, &w1, m, n, k, scale_num, scale_den)?;
    let y2 = gpu.gemm_int8_relu_q(&y1, &w2, m, n, n, scale_num, scale_den)?;
    let elapsed_ms = t0.elapsed().as_millis() as u64;

    // Deterministic sampling: pick S positions derived from seed
    let sample_count = 256usize;
    let mut indices: Vec<usize> = (0..(m*n)).collect();
    // Shuffle with a RNG seeded by a 32-byte Blake3 digest of the 16-byte seed
    let seed32: [u8; 32] = blake3::hash(&seed).into();
    indices.shuffle(&mut rand::rngs::StdRng::from_seed(seed32));
    let take = &indices[..sample_count];

    let mut samp = Vec::with_capacity(sample_count);
    for &idx in take { samp.push(y2[idx]); }

    // work_root = Blake3(samples || sizes)
    let mut hasher = Hasher::new();
    let samp_bytes: Vec<u8> = samp.iter().map(|&v| v as u8).collect();
    hasher.update(&samp_bytes);
    hasher.update(&m.to_le_bytes());
    hasher.update(&n.to_le_bytes());
    hasher.update(&k.to_le_bytes());
    let work_root = hasher.finalize().into();

    Ok(AttemptOutput { work_root, y1, y2_samples: samp, elapsed_ms })
}
