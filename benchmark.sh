#!/bin/bash

# tops-worker OpenCL Kernel Benchmarking Script
# Tests different WG_M, WG_N, and TK configurations to find optimal performance

set -e

echo "🚀 tops-worker OpenCL Kernel Benchmarking"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
RUNS_PER_CONFIG=3
LINES_PER_RUN=10
TIMEOUT_SECONDS=30

# Function to extract timing from worker output
extract_timing() {
    local output="$1"
    # Look for "ms=" pattern and extract the number
    echo "$output" | grep -o "ms=[0-9]*" | head -1 | cut -d'=' -f2
}

# Function to run a single benchmark
run_benchmark() {
    local config_name="$1"
    local env_vars="$2"
    
    echo -e "${BLUE}Testing: ${config_name}${NC}"
    
    local total_time=0
    local valid_runs=0
    
    for ((i=1; i<=$RUNS_PER_CONFIG; i++)); do
        echo -n "  Run $i/$RUNS_PER_CONFIG: "
        
        # Run and capture output (without timeout on macOS)
        local output
        if output=$(bash -c "$env_vars cargo run --release 2>&1 | head -$LINES_PER_RUN"); then
            local timing=$(extract_timing "$output")
            if [[ -n "$timing" && "$timing" -gt 0 ]]; then
                echo -e "${GREEN}${timing}ms${NC}"
                total_time=$((total_time + timing))
                valid_runs=$((valid_runs + 1))
            else
                echo -e "${RED}No valid timing found${NC}"
            fi
        else
            echo -e "${RED}Error running benchmark${NC}"
        fi
        
        # Small delay between runs
        sleep 1
    done
    
    if [[ $valid_runs -gt 0 ]]; then
        local avg_time=$((total_time / valid_runs))
        echo -e "  ${YELLOW}Average: ${avg_time}ms (${valid_runs}/${RUNS_PER_CONFIG} valid runs)${NC}"
        echo "$config_name,$avg_time,$valid_runs" >> benchmark_results.csv
    else
        echo -e "  ${RED}No valid runs completed${NC}"
        echo "$config_name,ERROR,0" >> benchmark_results.csv
    fi
    echo ""
}

# Initialize results file
echo "Configuration,Average_Time_ms,Valid_Runs" > benchmark_results.csv

echo "📊 Starting systematic benchmark..."
echo ""

# Baseline (no tuning)
run_benchmark "Baseline (no tuning)" ""

# Test different local work-group sizes
echo "🔧 Testing Local Work-Group Sizes (WG_M, WG_N)"
echo "----------------------------------------------"

# Common work-group sizes
for wg_m in 8 16 32; do
    for wg_n in 8 16 32; do
        # Skip some combinations that are unlikely to be optimal
        if [[ $wg_m -le 32 && $wg_n -le 32 ]]; then
            run_benchmark "WG_M=${wg_m} WG_N=${wg_n}" "WG_M=$wg_m WG_N=$wg_n"
        fi
    done
done

# Test strip-mining factors (TK)
echo "🔧 Testing Strip-Mining Factors (TK)"
echo "------------------------------------"

for tk in 4 8 16 32 64; do
    run_benchmark "TK=${tk}" "TK=$tk"
done

# Test combinations of work-group and strip-mining
echo "🔧 Testing Combined Configurations"
echo "----------------------------------"

# Some promising combinations
run_benchmark "WG_M=16 WG_N=16 TK=16" "WG_M=16 WG_N=16 TK=16"
run_benchmark "WG_M=16 WG_N=32 TK=16" "WG_M=16 WG_N=32 TK=16"
run_benchmark "WG_M=32 WG_N=16 TK=16" "WG_M=32 WG_N=16 TK=16"
run_benchmark "WG_M=16 WG_N=16 TK=32" "WG_M=16 WG_N=16 TK=32"
run_benchmark "WG_M=16 WG_N=32 TK=32" "WG_M=16 WG_N=32 TK=32"

# Test some power-of-2 combinations that often work well
run_benchmark "WG_M=8 WG_N=8 TK=8" "WG_M=8 WG_N=8 TK=8"
run_benchmark "WG_M=16 WG_N=16 TK=8" "WG_M=16 WG_N=16 TK=8"
run_benchmark "WG_M=32 WG_N=32 TK=16" "WG_M=32 WG_N=32 TK=16"

echo "📈 Benchmark Results Summary"
echo "============================"

# Sort results by performance (best first)
if command -v sort >/dev/null 2>&1; then
    echo "Configuration,Average_Time_ms,Valid_Runs" > sorted_results.csv
    tail -n +2 benchmark_results.csv | sort -t',' -k2,2n >> sorted_results.csv
    
    echo "Top 10 Fastest Configurations:"
    echo "-----------------------------"
    head -11 sorted_results.csv | tail -10 | while IFS=',' read -r config time runs; do
        if [[ "$time" != "ERROR" ]]; then
            echo -e "${GREEN}${time}ms${NC} - $config"
        else
            echo -e "${RED}ERROR${NC} - $config"
        fi
    done
    
    echo ""
    echo "Slowest Configurations:"
    echo "----------------------"
    tail -10 sorted_results.csv | while IFS=',' read -r config time runs; do
        if [[ "$time" != "ERROR" ]]; then
            echo -e "${RED}${time}ms${NC} - $config"
        else
            echo -e "${RED}ERROR${NC} - $config"
        fi
    done
else
    echo "Results saved to benchmark_results.csv"
    echo "Install 'sort' command for automatic ranking"
fi

echo ""
echo "📁 Results saved to:"
echo "  - benchmark_results.csv (raw data)"
echo "  - sorted_results.csv (ranked by performance)"
echo ""
echo "🎯 Recommended next steps:"
echo "  1. Test the top configurations with more runs"
echo "  2. Try intermediate values around the best performers"
echo "  3. Test with different matrix sizes (AUTOTUNE_PRESETS)"
echo "  4. Consider GPU-specific optimizations"
echo ""
echo "✅ Benchmarking complete!"
