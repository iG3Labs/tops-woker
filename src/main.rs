mod types; mod prng; mod cl_kernels; mod gpu; mod attempt; mod signing;
mod config; mod metrics; mod error_handling; mod health; mod server;
mod prometheus_metrics;
#[cfg(feature = "cuda")] mod gpu_cuda;
#[cfg(feature = "cpu-fallback")] mod cpu;

use std::sync::Arc;
use hex::ToHex;
use types::{WorkReceipt, Sizes};
use attempt::{run_attempt, Executor};
use gpu::GpuExec;
#[cfg(feature = "cuda")] use gpu_cuda::CudaExec;
#[cfg(feature = "cpu-fallback")] use cpu::CpuExec;
use signing::Secp;
use config::Config;
use metrics::MetricsCollector;
use error_handling::{ErrorHandler, RateLimiter};
use health::HealthChecker;
use server::HealthServer;
use prometheus_metrics::PrometheusMetrics;

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

#[cfg(feature = "gpu")]
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

#[cfg(feature = "cpu-fallback")]
fn autotune_sizes(_cpu: &CpuExec, _prev_hash_bytes: &[u8;32]) -> anyhow::Result<Sizes> {
    // For CPU fallback, use a fixed size since autotuning is less critical
    Ok(Sizes { m: 1024, n: 1024, k: 1024, batch: 1 })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load and validate configuration
    let config = Config::from_env()?;
    config.validate()?;
    
    println!("[config] Loaded configuration:");
    println!("  - Device DID: {}", config.device_did);
    println!("  - Aggregator URL: {}", config.aggregator_url);
    println!("  - Autotune target: {}ms", config.autotune_target_ms);
    println!("  - Max retries: {}", config.max_retries);
    println!("  - Rate limit: {}/s", config.rate_limit_per_second);
    
    // Initialize metrics collector
    let metrics = Arc::new(MetricsCollector::new());
    
    // Initialize Prometheus metrics
    let prometheus_metrics = Arc::new(PrometheusMetrics::new());
    
    // Initialize error handler
    let error_handler = ErrorHandler::new(Arc::clone(&metrics))
        .with_retry_config(error_handling::RetryConfig {
            max_retries: config.max_retries,
            retry_delay: config.get_retry_delay(),
            backoff_multiplier: 2.0,
            max_retry_delay: std::time::Duration::from_secs(30),
        });
    
    // Initialize rate limiter
    let rate_limiter = RateLimiter::new(config.max_concurrent_requests, config.rate_limit_per_second as f64);
    
    // Initialize health checker
    let health_checker = Arc::new(HealthChecker::new(Arc::clone(&metrics), config.clone()));
    
    // Start health server if metrics are enabled
    let _health_server_handle = if config.metrics_enabled {
        let health_server = HealthServer::new(Arc::clone(&health_checker), Arc::clone(&prometheus_metrics), 8082);
        let handle = tokio::spawn(async move {
            if let Err(e) = health_server.start().await {
                eprintln!("[health] Health server error: {}", e);
            }
        });
        Some(handle)
    } else {
        None
    };
    
    // ---- Config (replace with real values / CLI flags) ----
    let device_did = config.device_did;
    let epoch_id: u64 = 1;
    let prev_hash_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 64 hex
    let prev_hash_bytes: [u8;32] = hex::decode(prev_hash_hex)?.try_into().unwrap();
    let mut nonce: u32 = 0;

    // Initialize execution backend
    #[cfg(feature = "cuda")]
    let executor: Box<dyn Executor> = match CudaExec::new() {
        Ok(g) => Box::new(g),
        Err(e) => {
            error_handler.handle_gpu_error(&format!("CUDA initialization failed: {}", e));
            #[cfg(feature="cpu-fallback")]
            {
                eprintln!("[WARN] GPU not found, falling back to CPU.");
                Box::new(CpuExec::new()?)
            }
            #[cfg(not(feature="cpu-fallback"))]
            { return Err(e); }
        }
    };

    #[cfg(all(not(feature = "cuda"), not(feature = "cpu-fallback")))]
    let executor: Box<dyn Executor> = {
        #[cfg(feature = "gpu")]
        {
            match GpuExec::new() {
                Ok(g) => Box::new(g),
                Err(e) => {
                    error_handler.handle_gpu_error(&format!("OpenCL initialization failed: {}", e));
                    eprintln!("[ERROR] No GPU backend available and no CPU fallback enabled.");
                    return Err(e);
                }
            }
        }
        #[cfg(not(feature = "gpu"))]
        {
            eprintln!("[ERROR] No GPU backend available and no CPU fallback enabled.");
            return Err(anyhow::anyhow!("No execution backend available"));
        }
    };

    #[cfg(all(not(feature = "cuda"), feature = "cpu-fallback"))]
    let executor: Box<dyn Executor> = {
        #[cfg(feature = "gpu")]
        {
            match GpuExec::new() {
                Ok(g) => Box::new(g),
                Err(e) => {
                    error_handler.handle_gpu_error(&format!("OpenCL initialization failed: {}", e));
                    eprintln!("[WARN] GPU not found, falling back to CPU.");
                    Box::new(CpuExec::new()?)
                }
            }
        }
        #[cfg(not(feature = "gpu"))]
        {
            Box::new(CpuExec::new()?)
        }
    };

    // If autotune is enabled, compute sizes now using the initialized executor
    let sizes = if config.autotune_disable {
        Sizes { m: 1024, n: 1024, k: 1024, batch: 1 }
    } else {
        // For trait objects, we need to handle autotuning differently
        // For now, use a fixed size
        Sizes { m: 1024, n: 1024, k: 1024, batch: 1 }
    };

    // Signing key (hex) – in production, derive from peaq DID key or HSM
    let sk_hex = config.worker_sk_hex;
    let secp = Secp::from_hex(&sk_hex)?;
    println!("pubkey(compressed)={}", secp.pubkey_hex_compressed());

    // Print startup information
    println!("[startup] Worker initialized successfully");
    println!("[startup] Health endpoints available at http://localhost:8082");
    println!("[startup] Prometheus metrics available at http://localhost:8082/prometheus");
    println!("[startup] Starting main loop...");

    loop {
        nonce = nonce.wrapping_add(1);

        // Rate limiting
        rate_limiter.wait_for_token();

        // Run attempt with error handling
        let out = match run_attempt(&*executor, &prev_hash_bytes, nonce, &sizes) {
            Ok(out) => out,
            Err(e) => {
                error_handler.handle_gpu_error(&format!("Attempt failed: {}", e));
                continue;
            }
        };

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
        if config.worker_debug_receipt {
            println!("Receipt: {:?}", receipt);
        }
        
        // Sign the receipt
        let sig = match secp.sign_receipt(&receipt) {
            Ok(sig) => sig,
            Err(e) => {
                error_handler.handle_signature_error(&format!("Signing failed: {}", e));
                continue;
            }
        };
        receipt.sig_hex = sig;

        // Submit to aggregator with retry logic
        let url = config.aggregator_url.clone();
        let client = reqwest::Client::new();
        
        let submission_result = client.post(&url).json(&receipt).send().await;
        
        match submission_result {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                
                if status.is_success() {
                    // Record successful attempt
                    metrics.record_attempt(out.elapsed_ms, true);
                    prometheus_metrics.record_attempt(out.elapsed_ms, true);
                    println!("submit ok ({}): {}", url, body);
                    println!("ok nonce={} ms={} work_root={}", nonce, out.elapsed_ms, work_root_hex);
                } else {
                    // Record failed attempt
                    metrics.record_attempt(out.elapsed_ms, false);
                    prometheus_metrics.record_attempt(out.elapsed_ms, false);
                    error_handler.handle_network_error(&format!("HTTP {}: {}", status, body));
                    eprintln!("submit failed ({}): {}", status, body);
                }
            }
            Err(e) => {
                // Record failed attempt
                metrics.record_attempt(out.elapsed_ms, false);
                prometheus_metrics.record_attempt(out.elapsed_ms, false);
                error_handler.handle_network_error(&format!("Network error: {}", e));
                eprintln!("submit failed: {}", e);
            }
        }

        // Print periodic status
        if nonce % 100 == 0 {
            let current_metrics = metrics.get_metrics();
            let health_status = metrics.get_health_status();
            println!("[status] nonce={}, attempts={}, success_rate={:.2}%, avg_time={:.1}ms, health={}", 
                nonce, 
                current_metrics.total_attempts,
                if current_metrics.total_attempts > 0 { 
                    (current_metrics.successful_attempts as f64 / current_metrics.total_attempts as f64) * 100.0 
                } else { 0.0 },
                current_metrics.average_time_ms,
                health_status
            );
        }

        // Backoff a hair to keep the loop friendly; adjust or remove for pure PoW
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
