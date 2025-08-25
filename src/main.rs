mod types; mod prng; mod cl_kernels; mod gpu; mod attempt; mod signing;

use hex::ToHex;
use types::{WorkReceipt, Sizes};
use attempt::run_attempt;
use gpu::GpuExec;
use signing::Secp;

fn parse_target_ms() -> u64 {
    std::env::var("AUTOTUNE_TARGET_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300)
}

fn candidate_sizes() -> Vec<Sizes> {
    if let Ok(preset) = std::env::var("AUTOTUNE_PRESETS") {
        // Format: "m1,n1,k1;m2,n2,k2;..."
        let mut v = Vec::new();
        for triplet in preset.split(';') {
            let parts: Vec<_> = triplet.split(',').collect();
            if parts.len() == 3 {
                if let (Ok(m), Ok(n), Ok(k)) = (parts[0].parse(), parts[1].parse(), parts[2].parse()) {
                    v.push(Sizes { m, n, k, batch: 1 });
                }
            }
        }
        if !v.is_empty() { return v; }
    }
    vec![
        Sizes { m: 512, n: 512, k: 512, batch: 1 },
        Sizes { m: 768, n: 768, k: 768, batch: 1 },
        Sizes { m: 1024, n: 1024, k: 1024, batch: 1 },
        Sizes { m: 1280, n: 1280, k: 1280, batch: 1 },
        Sizes { m: 1536, n: 1536, k: 1536, batch: 1 },
    ]
}

fn autotune_sizes(gpu: &GpuExec, prev_hash_bytes: &[u8;32]) -> anyhow::Result<Sizes> {
    let target_ms = parse_target_ms();
    let mut best_sizes: Option<Sizes> = None;
    let mut best_score: u64 = u64::MAX;
    let mut nonce: u32 = 0;
    for s in candidate_sizes() {
        // Run one attempt to gauge time
        let out = crate::attempt::run_attempt(gpu, prev_hash_bytes, nonce, &s)?;
        let dt = out.elapsed_ms;
        let score = dt.abs_diff(target_ms);
        println!("[autotune] m,n,k=({},{},{}) -> {} ms (|diff|={})", s.m, s.n, s.k, dt, score);
        if score < best_score { best_score = score; best_sizes = Some(s); }
        // Increase nonce so each run is unique yet deterministic
        nonce = nonce.wrapping_add(1);
    }
    best_sizes.ok_or_else(|| anyhow::anyhow!("autotune produced no candidates"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ---- Config (replace with real values / CLI flags) ----
    let device_did = std::env::var("DEVICE_DID").unwrap_or_else(|_| "did:peaq:DEVICE123".into());
    let epoch_id: u64 = 1;
    let prev_hash_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 64 hex
    let prev_hash_bytes: [u8;32] = hex::decode(prev_hash_hex)?.try_into().unwrap();
    let mut nonce: u32 = 0;

    // Autotuner: sizes are determined after GPU initialization below.

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

    // If autotune is enabled, compute sizes now using the initialized GPU
    let sizes = if std::env::var("AUTOTUNE_DISABLE").ok().as_deref() == Some("1") {
        Sizes { m: 1024, n: 1024, k: 1024, batch: 1 }
    } else {
        match autotune_sizes(&gpu, &prev_hash_bytes) {
            Ok(s) => { println!("[autotune] chosen m,n,k=({},{},{})", s.m, s.n, s.k); s }
            Err(err) => { eprintln!("[autotune] failed ({}), falling back to 1024^3", err); Sizes { m: 1024, n: 1024, k: 1024, batch: 1 } }
        }
    };

    // Signing key (hex) – in production, derive from peaq DID key or HSM
    let sk_hex = std::env::var("WORKER_SK_HEX").expect("export WORKER_SK_HEX=<hex-privkey>");
    let secp = Secp::from_hex(&sk_hex)?;
    println!("pubkey(compressed)={}", secp.pubkey_hex_compressed());

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
        // debug: print full receipt if needed
        if std::env::var("WORKER_DEBUG_RECEIPT").ok().as_deref() == Some("1") {
            println!("Receipt: {:?}", receipt);
        }
        // Sign the receipt
        let sig = secp.sign_receipt(&receipt)?;
        receipt.sig_hex = sig;

        // Submit to iG3 (replace URL)
        let url = std::env::var("AGGREGATOR_URL").unwrap_or_else(|_| "http://localhost:8080/receipts".into());
        let resp = reqwest::Client::new().post(&url).json(&receipt).send().await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            eprintln!("submit failed ({}): {}", status, body);
        } else {
            println!("submit ok ({}): {}", url, body);
            println!("ok nonce={} ms={} work_root={}", nonce, out.elapsed_ms, work_root_hex);
        }

        // PoW mode (optional): if you want to stop only on success, compute header hash < target here.

        // Backoff a hair to keep the loop friendly; adjust or remove for pure PoW
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
