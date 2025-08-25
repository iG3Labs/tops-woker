# OpenCL Kernel Benchmarking Guide

This guide helps you find the optimal OpenCL kernel configuration for your GPU by testing different tuning parameters.

## Quick Start

Set up your environment:

```bash
export WORKER_SK_HEX=7b706b652278aba9b01dd473e026fd0baf215fd5afbf92d860b03fa661e07dc2
export AGGREGATOR_URL=https://httpbin.org/post
```

## Manual Benchmarking Process

### 1. Baseline Test

First, test the default configuration:

```bash
cargo run --release | grep "ms=" | head -5
```

Look for lines like `ok nonce=123 ms=456 work_root=abcd...` and note the `ms=` value.

### 2. Test Local Work-Group Sizes (WG_M, WG_N)

Test different work-group sizes to find optimal thread block configuration:

```bash
# Test 8x8 work groups
WG_M=8 WG_N=8 cargo run --release | grep "ms=" | head -5

# Test 16x16 work groups (often optimal)
WG_M=16 WG_N=16 cargo run --release | grep "ms=" | head -5

# Test 32x16 work groups
WG_M=32 WG_N=16 cargo run --release | grep "ms=" | head -5

# Test 16x32 work groups
WG_M=16 WG_N=32 cargo run --release | grep "ms=" | head -5

# Test 32x32 work groups
WG_M=32 WG_N=32 cargo run --release | grep "ms=" | head -5
```

### 3. Test Strip-Mining Factors (TK)

Test different K-dimension strip-mining to optimize memory access:

```bash
# Test TK=8
TK=8 cargo run --release | grep "ms=" | head -5

# Test TK=16
TK=16 cargo run --release | grep "ms=" | head -5

# Test TK=32
TK=32 cargo run --release | grep "ms=" | head -5

# Test TK=64
TK=64 cargo run --release | grep "ms=" | head -5
```

### 4. Test Combined Configurations

Combine the best work-group sizes with strip-mining:

```bash
# Combine 16x16 work groups with TK=16
WG_M=16 WG_N=16 TK=16 cargo run --release | grep "ms=" | head -5

# Combine 16x32 work groups with TK=16
WG_M=16 WG_N=32 TK=16 cargo run --release | grep "ms=" | head -5

# Combine 32x16 work groups with TK=16
WG_M=32 WG_N=16 TK=16 cargo run --release | grep "ms=" | head -5

# Combine 16x16 work groups with TK=32
WG_M=16 WG_N=16 TK=32 cargo run --release | grep "ms=" | head -5
```

## Automated Benchmarking Script

For more systematic testing, use the provided script:

```bash
# Make it executable
chmod +x simple_benchmark.sh

# Run the benchmark
./simple_benchmark.sh
```

**Note**: The automated script may take a while as it compiles for each configuration. For faster testing, use the manual approach above.

## Understanding the Results

### What to Look For

1. **Consistent Timing**: Look for configurations that produce consistent timing across multiple runs
2. **Lower is Better**: Smaller `ms=` values indicate faster performance
3. **GPU-Specific**: Different GPUs may prefer different configurations

### Common Patterns

- **Work-Group Sizes**: Often 16x16 or 32x16 work well on modern GPUs
- **Strip-Mining**: TK=16 or TK=32 often provide good memory locality
- **Power-of-2**: Configurations with power-of-2 values often perform better

### Example Results

```
Baseline: ms=52
WG_M=16 WG_N=16: ms=45  ← Better!
WG_M=32 WG_N=16: ms=48
TK=16: ms=43  ← Even better!
WG_M=16 WG_N=16 TK=16: ms=40  ← Best combination!
```

## GPU-Specific Recommendations

### NVIDIA GPUs

- Try `WG_M=16 WG_N=16` or `WG_M=32 WG_N=16`
- `TK=16` or `TK=32` often work well
- RTX series may prefer larger work groups

### AMD GPUs

- Try `WG_M=16 WG_N=16` or `WG_M=16 WG_N=32`
- `TK=16` is often optimal
- RDNA architecture may prefer different configurations

### Intel GPUs

- Try `WG_M=8 WG_N=8` or `WG_M=16 WG_N=16`
- `TK=8` or `TK=16` often work well
- Integrated graphics may prefer smaller work groups

## Advanced Tuning

### Matrix Size Impact

Test with different matrix sizes to see how tuning affects performance:

```bash
# Test with smaller matrices
export AUTOTUNE_PRESETS="512,512,512;768,768,768"
cargo run --release | grep "ms=" | head -5

# Test with larger matrices
export AUTOTUNE_PRESETS="1536,1536,1536;2048,2048,2048"
cargo run --release | grep "ms=" | head -5
```

### Multiple Runs

For more accurate results, run each configuration multiple times:

```bash
# Run baseline 3 times and average
for i in {1..3}; do
    cargo run --release | grep "ms=" | head -1
done
```

## Troubleshooting

### Common Issues

1. **No timing output**: Check if OpenCL drivers are installed
2. **Very slow performance**: Try smaller work-group sizes
3. **Compilation errors**: Check if TK values are too large for your GPU
4. **Inconsistent results**: Run multiple times and average

### Debug Mode

For debugging, run with more verbose output:

```bash
export WORKER_DEBUG_RECEIPT=1
cargo run --release
```

## Next Steps

After finding optimal configurations:

1. **Document Results**: Save your best configurations for your GPU
2. **Test Edge Cases**: Try configurations around your best performers
3. **Profile Further**: Use GPU profiling tools for deeper analysis
4. **Optimize Code**: Consider kernel code optimizations based on results

## Example Benchmark Session

```bash
# Set up environment
export WORKER_SK_HEX=7b706b652278aba9b01dd473e026fd0baf215fd5afbf92d860b03fa661e07dc2
export AGGREGATOR_URL=https://httpbin.org/post

# Quick baseline
echo "Baseline:" && cargo run --release | grep "ms=" | head -1

# Test work groups
echo "WG_M=16 WG_N=16:" && WG_M=16 WG_N=16 cargo run --release | grep "ms=" | head -1
echo "WG_M=32 WG_N=16:" && WG_M=32 WG_N=16 cargo run --release | grep "ms=" | head -1

# Test strip mining
echo "TK=16:" && TK=16 cargo run --release | grep "ms=" | head -1
echo "TK=32:" && TK=32 cargo run --release | grep "ms=" | head -1

# Test combinations
echo "WG_M=16 WG_N=16 TK=16:" && WG_M=16 WG_N=16 TK=16 cargo run --release | grep "ms=" | head -1
```

This approach gives you quick feedback on which configurations work best for your specific hardware!
