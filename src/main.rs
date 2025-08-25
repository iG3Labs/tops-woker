mod types; mod prng; mod cl_kernels; mod gpu; mod attempt; mod signing;

use hex::ToHex;
use types::{WorkReceipt, Sizes};
use attempt::run_attempt;
use gpu::GpuExec;
use signing::Secp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ---- Config (replace with real values / CLI flags) ----
    let device_did = std::env::var("DEVICE_DID").unwrap_or_else(|_| "did:peaq:DEVICE123".into());
    let epoch_id: u64 = 1;
    let prev_hash_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 64 hex
    let prev_hash_bytes: [u8;32] = hex::decode(prev_hash_hex)?.try_into().unwrap();
    let mut nonce: u32 = 0;

    // Autotune (simple: choose size by VRAM class; improve later)
    let sizes = Sizes { m: 1024, n: 1024, k: 1024, batch: 1 };

    let gpu = match GpuExec::new() {
        Ok(g) => g,
        Err(e) => {
            #[cfg(feature="cpu-fallback")]
            {
                eprintln!("[WARN] GPU not found, build with --features cpu-fallback and use CPU path.");
                std::process::exit(1);
            }
            #[cfg(not(feature="cpu-fallback"))]
            { return Err(e); }
        }
    };

    // Signing key (hex) – in production, derive from peaq DID key or HSM
    let sk_hex = std::env::var("WORKER_SK_HEX").expect("export WORKER_SK_HEX=<hex-privkey>");
    let secp = Secp::from_hex(&sk_hex)?;

    loop {
        nonce = nonce.wrapping_add(1);

        let out = run_attempt(&gpu, &prev_hash_bytes, nonce, &sizes)?;
        let work_root_hex = out.work_root.encode_hex::<String>();

        let mut receipt = WorkReceipt {
            device_did: device_did.clone(),
            epoch_id,
            prev_hash_hex: prev_hash_hex.to_string(),
            nonce,
            work_root_hex: work_root_hex.clone(),
            sizes: sizes.clone(),
            time_ms: out.elapsed_ms,
            kernel_ver: "gemm_int8_relu_q_v1".into(),
            driver_hint: "OpenCL".into(),
            sig_hex: String::new(),
        };
        // Sign the receipt
        let sig = secp.sign_receipt(&receipt)?;
        receipt.sig_hex = sig;

        // Submit to iG3 (replace URL)
        let url = std::env::var("AGGREGATOR_URL").unwrap_or_else(|_| "http://localhost:8080/receipts".into());
        let resp = reqwest::Client::new().post(url).json(&receipt).send().await?;
        if !resp.status().is_success() {
            eprintln!("submit failed: {:?}", resp.text().await?);
        } else {
            println!("ok nonce={} ms={} work_root={}", nonce, out.elapsed_ms, work_root_hex);
        }

        // PoW mode (optional): if you want to stop only on success, compute header hash < target here.

        // Backoff a hair to keep the loop friendly; adjust or remove for pure PoW
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
