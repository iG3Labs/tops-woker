#!/bin/bash

# Quick test to verify the worker is working before running full benchmarks

echo "🧪 Quick Worker Test"
echo "==================="
echo ""

# Test if worker runs and produces timing output
echo "Testing baseline worker..."
output=$(bash -c 'export WORKER_SK_HEX=7b706b652278aba9b01dd473e026fd0baf215fd5afbf92d860b03fa661e07dc2 && cargo run --release 2>&1' | head -10 || echo "ERROR")

if echo "$output" | grep -q "ms="; then
    timing=$(echo "$output" | grep -o "ms=[0-9]*" | head -1 | cut -d'=' -f2)
    echo "✅ Worker is working! Baseline timing: ${timing}ms"
    echo ""
    echo "Ready to run full benchmark with:"
    echo "  ./benchmark.sh"
    echo ""
    echo "Or run a quick subset with:"
    echo "  ./benchmark.sh | head -50"
else
    echo "❌ Worker test failed. Check if:"
    echo "  - OpenCL drivers are installed"
    echo "  - GPU is available"
    echo "  - WORKER_SK_HEX is set"
    echo ""
    echo "Full error output:"
    echo "$output"
    exit 1
fi
