use std::time::Instant;
use crate::types::Sizes;
use crate::prng::DPrng;

pub struct AttemptOutput {
    pub work_root: [u8;32],
    pub y1: Vec<i8>,
    pub y2_samples: Vec<i8>,
    pub elapsed_ms: u64,
}

// Trait for execution backends
pub trait Executor {
    fn run_gemm(&self, a: &[i8], b: &[i8], sizes: &Sizes) -> anyhow::Result<Vec<i8>>;
}

// Implement for GPU (only when gpu feature is enabled)
#[cfg(feature = "gpu")]
impl Executor for crate::gpu::GpuExec {
    fn run_gemm(&self, a: &[i8], b: &[i8], sizes: &Sizes) -> anyhow::Result<Vec<i8>> {
        self.run_gemm(a, b, sizes)
    }
}

// Implement for CPU
#[cfg(feature = "cpu-fallback")]
impl Executor for crate::cpu::CpuExec {
    fn run_gemm(&self, a: &[i8], b: &[i8], sizes: &Sizes) -> anyhow::Result<Vec<i8>> {
        self.run_gemm(a, b, sizes)
    }
}

// Implement for CUDA
#[cfg(feature = "cuda")]
impl Executor for crate::gpu_cuda::CudaExec {
    fn run_gemm(&self, a: &[i8], b: &[i8], sizes: &Sizes) -> anyhow::Result<Vec<i8>> {
        self.run_gemm(a, b, sizes)
    }
}

pub fn run_attempt<E: Executor + ?Sized>(executor: &E, prev_hash_bytes: &[u8;32], nonce: u32, sizes: &Sizes) -> anyhow::Result<AttemptOutput> {
    let start = Instant::now();
    
    // Deterministic PRNG seeded by prev_hash + nonce
    let seed = crate::prng::derive_seed(prev_hash_bytes, nonce);
    let mut prng = DPrng::from_seed(seed);
    
    // Generate input matrices deterministically
    let a: Vec<i8> = (0..sizes.m * sizes.k).map(|_| prng.next_i8()).collect();
    let b: Vec<i8> = (0..sizes.k * sizes.n).map(|_| prng.next_i8()).collect();
    
    // Run GEMM
    let y1 = executor.run_gemm(&a, &b, sizes)?;
    
    // Sample some outputs for work root
    let num_samples = 1024.min(y1.len());
    let y2_samples: Vec<i8> = y1.iter().take(num_samples).cloned().collect();
    
    // Convert i8 samples to u8 for hashing
    let samples_u8: Vec<u8> = y2_samples.iter().map(|&x| x as u8).collect();
    
    // Compute work root (hash of samples)
    let work_root = blake3::hash(&samples_u8).into();
    
    let elapsed_ms = start.elapsed().as_millis() as u64;
    
    Ok(AttemptOutput {
        work_root,
        y1,
        y2_samples,
        elapsed_ms,
    })
}
