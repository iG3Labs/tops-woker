use anyhow::{Result, anyhow};
use ocl::{Buffer, Context, Device, Kernel, Platform, Program, Queue};
use crate::cl_kernels::GEMM_INT8;

pub struct GpuExec {
    ctx: Context,
    q: Queue,
    prog: Program,
}

impl GpuExec {
    pub fn new() -> Result<Self> {
        // Choose a GPU device if available, else error (caller may CPU-fallback)
        let platform = Platform::default();
        let devices = Device::list(platform, Some(ocl::flags::DEVICE_TYPE_GPU))?;
        let device = devices.into_iter()
            .next()
            .ok_or_else(|| anyhow!("No GPU device found"))?;
        let ctx = Context::builder().platform(platform).devices(device.clone()).build()?;
        let q = Queue::new(&ctx, device, None)?;
        let prog = Program::builder().src(GEMM_INT8).build(&ctx)?;
        Ok(Self { ctx, q, prog })
    }

    pub fn gemm_int8_relu_q(
        &self,
        a: &[i8], b: &[i8], m: usize, n: usize, k: usize,
        scale_num: i32, scale_den: i32,
    ) -> Result<Vec<i8>> {
        let lda = k; let ldb = n; let ldy = n;
        let len_a = m*k; let len_b = k*n; let len_y = m*n;

        let buf_a: Buffer<i8> = Buffer::builder().queue(self.q.clone()).len(len_a).copy_host_slice(a).build()?;
        let buf_b: Buffer<i8> = Buffer::builder().queue(self.q.clone()).len(len_b).copy_host_slice(b).build()?;
        let buf_y: Buffer<i8> = Buffer::builder().queue(self.q.clone()).len(len_y).build()?;

        let kernel = Kernel::builder()
            .program(&self.prog).name("gemm_int8_relu_q")
            .queue(self.q.clone())
            .global_work_size([m, n])
            .arg(&buf_a).arg(&buf_b).arg(&buf_y)
            .arg(&(m as i32)).arg(&(n as i32)).arg(&(k as i32))
            .arg(&(lda as i32)).arg(&(ldb as i32)).arg(&(ldy as i32))
            .arg(&scale_num).arg(&scale_den)
            .build()?;

        unsafe { kernel.enq()?; }
        self.q.finish()?;

        let mut y = vec![0i8; len_y];
        buf_y.read(&mut y).enq()?;
        Ok(y)
    }
}
