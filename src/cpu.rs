use crate::types::Sizes;

pub struct CpuExec;

impl CpuExec {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
    
    pub fn gemm_int8_relu_q(&self, a: &[i8], b: &[i8], m: usize, n: usize, k: usize, num: i32, den: i32) -> Vec<i8> {
        let mut y = vec![0i8; m*n];
        for row in 0..m {
            for col in 0..n {
                let mut acc: i64 = 0;
                for t in 0..k {
                    acc += (a[row*k + t] as i32 as i64) * (b[t*n + col] as i32 as i64);
                }
                let mut q = (acc * num as i64) / den as i64;
                if q < 0 { q = 0; }
                if q > 127 { q = 127; }
                y[row*n + col] = q as i8;
            }
        }
        y
    }
    
    pub fn run_gemm(&self, a: &[i8], b: &[i8], sizes: &Sizes) -> anyhow::Result<Vec<i8>> {
        let result = self.gemm_int8_relu_q(a, b, sizes.m, sizes.n, sizes.k, 1, 1);
        Ok(result)
    }
}
