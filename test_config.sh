#!/bin/bash

# Test a single OpenCL configuration
# Usage: ./test_config.sh [CONFIG_NAME] [ENV_VARS]

set -e

# Default environment
export WORKER_SK_HEX=7b706b652278aba9b01dd473e026fd0baf215fd5afbf92d860b03fa661e07dc2
export AGGREGATOR_URL=http://localhost:8081/verify

CONFIG_NAME=${1:-"Baseline"}
ENV_VARS=${2:-""}

echo "🧪 Testing: $CONFIG_NAME"
echo "========================="

# Run the worker and capture output
echo "Running worker (will stop after first timing output)..."
echo ""

# Run in background and capture output
temp_file=$(mktemp)
$ENV_VARS cargo run --release > "$temp_file" 2>&1 &
worker_pid=$!

# Wait for timing output or timeout
timeout=60
elapsed=0
while [ $elapsed -lt $timeout ]; do
    if grep -q "ms=" "$temp_file"; then
        timing=$(grep "ms=" "$temp_file" | head -1 | grep -o "ms=[0-9]*" | cut -d'=' -f2)
        echo "✅ Timing: ${timing}ms"
        break
    fi
    sleep 1
    elapsed=$((elapsed + 1))
    if [ $((elapsed % 5)) -eq 0 ]; then
        echo -n "."
    fi
done

# Kill the worker
kill $worker_pid 2>/dev/null || true
wait $worker_pid 2>/dev/null || true

# Clean up
rm -f "$temp_file"

if [ $elapsed -ge $timeout ]; then
    echo ""
    echo "❌ Timeout - no timing output found"
    exit 1
fi

echo ""
echo "Configuration: $CONFIG_NAME"
echo "Environment: $ENV_VARS"
echo "Result: ${timing}ms"
