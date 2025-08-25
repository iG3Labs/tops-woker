## tops-worker

High-throughput worker that computes a deterministic int8 GEMM pipeline on GPU via OpenCL, produces a succinct work root, signs a receipt, and submits it to an aggregator endpoint.

### What the worker does (high-level)

- Generates an input activation matrix `A` deterministically from `(prev_hash, nonce)`.
- Multiplies `A` by a fixed int8 weight matrix `W1` on the GPU, applies ReLU and requantization → `y1`.
- Multiplies `y1` by a second int8 weight matrix `W2`, applies ReLU and requantization → `y2`.
- Deterministically samples `S=256` positions from `y2`, then hashes those bytes with BLAKE3 → `work_root`.
- Produces a signed `WorkReceipt` and sends it to the aggregator URL.

This yields a small commitment (`work_root`) to a large amount of compute, making verification efficient.

### Determinism model

All randomness is derived deterministically:

- Seed derivation: the 16-byte seed is `BLAKE3(prev_hash || nonce)[..16]` implemented in `derive_seed` in `src/prng.rs`.
- Input activations `A` are generated using `DPrng` (Xoshiro128++ seeded by the 16-byte seed).
- Weights `W1`, `W2` are pseudo-fixed: derived from `BLAKE3("FIXED_WEIGHTS_V1")` inside `src/attempt.rs`. For a real deployment, you would ship audited constant weights.
- Sampling uses a reproducible shuffle: we form a 32-byte seed by hashing the 16-byte seed with BLAKE3 and initialize `StdRng::from_seed(seed32)`, then `shuffle` the index list.

Given the same `(prev_hash, nonce)` and sizes, `work_root` is deterministic across machines/GPUs.

### Math and quantization

Let sizes be \(m, n, k\); by default `m=n=k=1024`.

1. First layer:

   - Compute \(Y_1 = \text{ReLU}(A \cdot W_1)\) with int8 inputs and int8 outputs.
   - After the int32 accumulation, we requantize with a fixed rational scale `scale_num/scale_den` back to int8 with clamping to [-128, 127].

2. Second layer:

   - Compute \(Y_2 = \text{ReLU}(Y_1 \cdot W_2)\) with the same quantization scheme.

3. Sampling and work root:
   - Deterministically choose `S = 256` positions of `Y_2` and collect those int8 values as bytes.
   - `work_root = BLAKE3(sample_bytes || m || n || k)`.

This is implemented by the OpenCL kernel `gemm_int8_relu_q` in `src/cl_kernels.rs` and orchestrated from `src/gpu.rs`.

### File map

- `src/main.rs`: process loop; environment config; device init; runs attempts; signs and submits receipts.
- `src/gpu.rs`: OpenCL context/program/queue setup; enqueues `gemm_int8_relu_q` kernels.
- `src/cl_kernels.rs`: OpenCL C kernel for int8 GEMM with ReLU and requantization.
- `src/attempt.rs`: deterministic data generation, two-layer pipeline, sampling, BLAKE3 `work_root`.
- `src/prng.rs`: `DPrng` (Xoshiro128++), `derive_seed`.
- `src/signing.rs`: secp256k1 signing of a stable JSON serialization hashed with BLAKE3.
- `src/types.rs`: `Sizes`, `WorkReceipt` structs.

### OpenCL and device selection

The worker uses the `ocl` crate. On startup we:

- pick the default platform (`Platform::default()`),
- enumerate GPU devices only via `Device::list(platform, Some(DEVICE_TYPE_GPU))`,
- build a `Context`, `Queue`, and `Program` from inlined kernel source.

If no GPU is found, run with `--features cpu-fallback` to use the CPU path (placeholder/stub).

### Running the worker

Prerequisites:

- Rust toolchain (Rust 1.76+ recommended).
- OpenCL runtime/driver installed (NVIDIA, AMD, Intel or POCL).

Environment:

```bash
export WORKER_SK_HEX=<64-hex seckey>             # required: secp256k1 private key
export DEVICE_DID='did:peaq:DEVICE123'          # optional
export AGGREGATOR_URL='http://localhost:8081/verify'    # point to the verifier by default
export AUTOTUNE_TARGET_MS=300                            # optional
export AUTOTUNE_PRESETS="512,512,512;1024,1024,1024"   # optional
export AUTOTUNE_DISABLE=0                                # set 1 to skip tuning (use 1024^3)
export WORKER_DEBUG_RECEIPT=0                            # set 1 to print full receipt
```

Quick test without a local verifier (uses httpbin echo):

```bash
export WORKER_SK_HEX=...
export AGGREGATOR_URL='https://httpbin.org/post'
cargo run --release
```

CPU fallback:

```bash
cargo run --release --features cpu-fallback
```

The program runs in a loop and prints lines like:

```
ok nonce=123 ms=456 work_root=abcd...
```

Press Ctrl-C to stop.

### Signing and verification

- The worker computes a stable JSON of the `WorkReceipt` with `sig_hex` blank, hashes with BLAKE3, then SHA-256, and signs the prehash (secp256k1).
- The verifier recomputes the same digest and verifies the signature against a configured public key.
- Signature encodings supported: DER or 64-byte compact.

### Verifier (Node.js)

A tiny verifier HTTP service you can deploy alongside your aggregator for light checks.

Run locally:

```bash
cd verifier
npm install
npm start
# listens on :8081
curl -s http://localhost:8081/healthz
```

Verify a receipt (schema/format + digest of JSON-without-sig):

```bash
curl -s -X POST http://localhost:8081/verify \
  -H 'content-type: application/json' \
  -d '{
    "device_did":"did:peaq:DEVICE123",
    "epoch_id":1,
    "prev_hash_hex":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "nonce":1,
    "work_root_hex":"...",
    "sizes": {"m":1536,"n":1536,"k":1536,"batch":1},
    "time_ms":180,
    "kernel_ver":"gemm_int8_relu_q_v1",
    "driver_hint":"OpenCL",
    "sig_hex":""
  }'
```

Environment:

```bash
export VERIFY_PUBKEY=<hex pubkey>   # 33B compressed or 65B uncompressed
export VERIFY_DISABLE=0             # set 1 to bypass signature checks
export PORT=8081
```

Typical flow:

```bash
# 1) Run worker once to print pubkey
export WORKER_SK_HEX=... && cargo run --release | head -n 20
# Note pubkey(compressed)=...
# 2) Start verifier with VERIFY_PUBKEY
cd verifier && VERIFY_PUBKEY=<hex> npm start
# 3) Point worker to verifier
export AGGREGATOR_URL=http://localhost:8081/verify
cargo run --release
```

### Security and validation notes

- Signing: We sign the BLAKE3 hash of the JSON-serialized `WorkReceipt` with secp256k1. See `src/signing.rs`.
- Determinism: Input generation, weights, and sampling are derived from `(prev_hash, nonce)` and constants. Any node can recompute `work_root`.
- Audit: For production, freeze `W1`, `W2` as public constants and ship precompiled kernels with digests.

### Performance knobs

- Matrix sizes `m, n, k` in `src/main.rs` under `Sizes`.
- `scale_num/scale_den` quantization parameters in `src/attempt.rs`.
- Global work size currently `[m, n]`; tuning local sizes and tiling inside the kernel can yield large speedups.

### CUDA backend (NVIDIA)

An experimental CUDA backend using `cudarc` + cuBLASLt int8 GEMM is available.

Prerequisites:

- NVIDIA driver + CUDA Toolkit installed. Ensure `cuda.h` is present under `$CUDA_ROOT/include/cuda.h`.
- Set `CUDA_ROOT` if autodetection fails, e.g. `export CUDA_ROOT=/usr/local/cuda`.

Build and run with CUDA:

```bash
export CUDA_ROOT=/usr/local/cuda                              # adjust if needed
cargo run --release --features cuda
```

Notes:

- On non-NVIDIA systems, omit `--features cuda` and the OpenCL path will be used.
- The CUDA path uses cuBLASLt int8 GEMM with ReLU epilogue for peak performance where supported.

### Pseudocode

```text
seed16 = BLAKE3(prev_hash || nonce)[0..16]
pr = DPrng(seed16)
A  = pr.gen_i8(m * k)

w_seed16 = BLAKE3("FIXED_WEIGHTS_V1")[0..16]
prw = DPrng(w_seed16)
W1 = prw.gen_i8(k * n)
W2 = prw.gen_i8(n * n)

y1 = GEMM_INT8_RELU_Q(A, W1, m, n, k, scale)
y2 = GEMM_INT8_RELU_Q(y1, W2, m, n, n, scale)

indices = [0..m*n)
rng32 = BLAKE3(seed16) // 32 bytes
shuffle(indices, StdRng(rng32))
samp = take(indices, 256).map(|i| y2[i])

work_root = BLAKE3(samp || m || n || k)
receipt = { device_did, epoch_id, prev_hash_hex, nonce, work_root_hex, sizes, ... }
sig = secp256k1_sign(BLAKE3(JSON(receipt)))
post(aggregator_url, receipt + sig)
```

### License

Apache-2.0 or MIT (choose one and update if needed).
