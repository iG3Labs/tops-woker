#!/bin/bash

# Quick OpenCL Benchmark Script
# Provides easy commands to test different configurations

echo "🚀 Quick OpenCL Benchmark Commands"
echo "=================================="
echo ""

# Set up environment
export WORKER_SK_HEX=7b706b652278aba9b01dd473e026fd0baf215fd5afbf92d860b03fa661e07dc2
export AGGREGATOR_URL=https://httpbin.org/post

echo "Environment set up!"
echo ""

echo "📊 Run these commands one by one to test configurations:"
echo ""

echo "# 1. Baseline test:"
echo "cargo run --release | grep 'ms=' | head -1"
echo ""

echo "# 2. Test work-group sizes:"
echo "WG_M=16 WG_N=16 cargo run --release | grep 'ms=' | head -1"
echo "WG_M=32 WG_N=16 cargo run --release | grep 'ms=' | head -1"
echo "WG_M=16 WG_N=32 cargo run --release | grep 'ms=' | head -1"
echo ""

echo "# 3. Test strip-mining:"
echo "TK=16 cargo run --release | grep 'ms=' | head -1"
echo "TK=32 cargo run --release | grep 'ms=' | head -1"
echo ""

echo "# 4. Test combinations:"
echo "WG_M=16 WG_N=16 TK=16 cargo run --release | grep 'ms=' | head -1"
echo "WG_M=16 WG_N=32 TK=16 cargo run --release | grep 'ms=' | head -1"
echo ""

echo "# 5. Quick comparison (copy and paste this block):"
echo "echo '=== Performance Comparison ==='"
echo "echo 'Baseline:' && cargo run --release | grep 'ms=' | head -1"
echo "echo 'WG_M=16 WG_N=16:' && WG_M=16 WG_N=16 cargo run --release | grep 'ms=' | head -1"
echo "echo 'TK=16:' && TK=16 cargo run --release | grep 'ms=' | head -1"
echo "echo 'WG_M=16 WG_N=16 TK=16:' && WG_M=16 WG_N=16 TK=16 cargo run --release | grep 'ms=' | head -1"
echo ""

echo "💡 Tips:"
echo "- Run each command and note the 'ms=' value"
echo "- Lower numbers = faster performance"
echo "- Stop each test with Ctrl+C after seeing the timing"
echo "- Compare results to find your optimal configuration"
echo ""

echo "✅ Ready to benchmark!"
