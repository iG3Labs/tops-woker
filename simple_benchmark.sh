#!/bin/bash

# Simple OpenCL Kernel Benchmarking Script
# Tests different WG_M, WG_N, and TK configurations

set -e

echo "🚀 Simple OpenCL Kernel Benchmarking"
echo "===================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Environment setup
export WORKER_SK_HEX=7b706b652278aba9b01dd473e026fd0baf215fd5afbf92d860b03fa661e07dc2
export AGGREGATOR_URL=https://httpbin.org/post

# Function to run a single benchmark
run_benchmark() {
    local config_name="$1"
    local env_vars="$2"
    
    echo -e "${BLUE}Testing: ${config_name}${NC}"
    
    # Run the worker and capture timing
    local output
    output=$(bash -c "$env_vars cargo run --release 2>&1" | grep "ms=" | head -1)
    
    if [[ -n "$output" ]]; then
        local timing=$(echo "$output" | grep -o "ms=[0-9]*" | cut -d'=' -f2)
        echo -e "  ${GREEN}Timing: ${timing}ms${NC}"
        echo "$config_name,$timing" >> benchmark_results.csv
    else
        echo -e "  ${YELLOW}No timing found${NC}"
        echo "$config_name,ERROR" >> benchmark_results.csv
    fi
    echo ""
}

# Initialize results file
echo "Configuration,Time_ms" > benchmark_results.csv

echo "📊 Starting benchmarks..."
echo ""

# Baseline (no tuning)
run_benchmark "Baseline" ""

# Test different local work-group sizes
echo "🔧 Testing Local Work-Group Sizes"
echo "--------------------------------"

run_benchmark "WG_M=8 WG_N=8" "WG_M=8 WG_N=8"
run_benchmark "WG_M=16 WG_N=16" "WG_M=16 WG_N=16"
run_benchmark "WG_M=32 WG_N=16" "WG_M=32 WG_N=16"
run_benchmark "WG_M=16 WG_N=32" "WG_M=16 WG_N=32"
run_benchmark "WG_M=32 WG_N=32" "WG_M=32 WG_N=32"

# Test strip-mining factors (TK)
echo "🔧 Testing Strip-Mining Factors"
echo "-------------------------------"

run_benchmark "TK=8" "TK=8"
run_benchmark "TK=16" "TK=16"
run_benchmark "TK=32" "TK=32"

# Test combinations
echo "🔧 Testing Combined Configurations"
echo "----------------------------------"

run_benchmark "WG_M=16 WG_N=16 TK=16" "WG_M=16 WG_N=16 TK=16"
run_benchmark "WG_M=16 WG_N=32 TK=16" "WG_M=16 WG_N=32 TK=16"
run_benchmark "WG_M=32 WG_N=16 TK=16" "WG_M=32 WG_N=16 TK=16"

echo "📈 Results Summary"
echo "=================="

# Show results
echo "Configuration,Time_ms" > sorted_results.csv
tail -n +2 benchmark_results.csv | sort -t',' -k2,2n >> sorted_results.csv

echo "Top 5 Fastest Configurations:"
echo "----------------------------"
head -6 sorted_results.csv | tail -5 | while IFS=',' read -r config time; do
    if [[ "$time" != "ERROR" ]]; then
        echo -e "${GREEN}${time}ms${NC} - $config"
    else
        echo -e "${YELLOW}ERROR${NC} - $config"
    fi
done

echo ""
echo "📁 Results saved to:"
echo "  - benchmark_results.csv (raw data)"
echo "  - sorted_results.csv (ranked by performance)"
echo ""
echo "✅ Benchmarking complete!"
