#![cfg(feature = "cuda")]
use anyhow::{anyhow, Result};
use cudarc::cublaslt::{CublasLt, Gemm, MatLayout, Scale, TypeI8};
use cudarc::driver::{CudaDevice, DeviceRepr, LaunchAsync};

pub struct CudaExec {
    dev: CudaDevice,
    lt: CublasLt,
}

impl CudaExec {
    pub fn new() -> Result<Self> {
        let dev = CudaDevice::new(0)?;
        let lt = CublasLt::new()?;
        Ok(Self { dev, lt })
    }

    // Interface mirrors GpuExec::gemm_int8_relu_q
    pub fn gemm_int8_relu_q(
        &self,
        a: &[i8], b: &[i8], m: usize, n: usize, k: usize,
        scale_num: i32, scale_den: i32,
    ) -> Result<Vec<i8>> {
        // Allocate device buffers
        let d_a = self.dev.htod_copy(a)?;
        let d_b = self.dev.htod_copy(b)?;
        let mut d_y = self.dev.alloc_zeros::<i8>(m * n)?;

        // Set layouts (row-major int8)
        let a_layout = MatLayout::row_major::<TypeI8>(m as i32, k as i32, k as i32);
        let b_layout = MatLayout::row_major::<TypeI8>(k as i32, n as i32, n as i32);
        let y_layout = MatLayout::row_major::<TypeI8>(m as i32, n as i32, n as i32);

        // Scale factor as rational -> convert to f32 alpha/beta
        let alpha = (scale_num as f32) / (scale_den as f32);
        let beta = 0.0f32;

        // Run int8 GEMM with ReLU epilogue using cuBLASLt (if available in crate)
        // Fallback: plain GEMM + clamp on host
        let gemm = Gemm::new_i8_i8_i32(a_layout, b_layout, y_layout)
            .with_alpha(Scale::from_f32(alpha))
            .with_beta(Scale::from_f32(beta))
            .with_relu(true);

        unsafe { self.lt.run(&self.dev, &gemm, &d_a, &d_b, &mut d_y)?; }
        self.dev.synchronize()?;

        let mut y = vec![0i8; m * n];
        self.dev.dtoh_sync_copy_into(&d_y, &mut y)?;
        Ok(y)
    }
}


