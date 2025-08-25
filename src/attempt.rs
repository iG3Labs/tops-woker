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

// Deterministic permutation + sign-mask mixing
fn sign_flip_i8(value: i8, sign: i8) -> i8 {
    if sign >= 0 { return value; }
    if value == i8::MIN { 127 } else { -value }
}

fn build_permutation_and_signs(k: usize, seed16: [u8;16]) -> (Vec<usize>, Vec<i8>) {
    // Permute 0..k using StdRng seeded from a 32-byte digest of seed16
    let mut perm: Vec<usize> = (0..k).collect();
    let seed32: [u8; 32] = blake3::hash(&seed16).into();
    perm.shuffle(&mut rand::rngs::StdRng::from_seed(seed32));
    // Signs in {+1, -1} from DPrng seeded by seed16
    let mut pr = DPrng::from_seed(seed16);
    let mut signs = Vec::with_capacity(k);
    for _ in 0..k {
        let bit = (pr.next_u32() & 1) as u8;
        signs.push(if bit == 0 { 1 } else { -1 });
    }
    (perm, signs)
}

fn apply_mix_a_columns(a: &[i8], m: usize, k: usize, perm: &[usize], signs: &[i8]) -> Vec<i8> {
    let mut out = vec![0i8; m * k];
    for new_col in 0..k {
        let src_col = perm[new_col];
        let s = signs[src_col];
        for row in 0..m {
            let src = a[row * k + src_col];
            out[row * k + new_col] = sign_flip_i8(src, s);
        }
    }
    out
}

fn apply_mix_w1_rows(w1: &[i8], k: usize, n: usize, perm: &[usize], signs: &[i8]) -> Vec<i8> {
    let mut out = vec![0i8; k * n];
    for new_row in 0..k {
        let src_row = perm[new_row];
        let s = signs[src_row];
        let dst_row_off = new_row * n;
        let src_row_off = src_row * n;
        for col in 0..n {
            let v = w1[src_row_off + col];
            out[dst_row_off + col] = sign_flip_i8(v, s);
        }
    }
    out
}

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

    // Apply deterministic permutation + sign-mask: A's columns and W1's rows
    let (perm, signs) = build_permutation_and_signs(k, seed);
    let a = apply_mix_a_columns(&a, m, k, &perm, &signs);
    let w1 = apply_mix_w1_rows(&w1, k, n, &perm, &signs);

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
