#!/bin/bash

# Docker run script for tops-worker
set -e

# Default values
PROFILE=""
ENV_FILE=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --monitoring)
            PROFILE="--profile monitoring"
            shift
            ;;
        --env-file)
            ENV_FILE="--env-file $2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --monitoring    Start with monitoring stack (Prometheus + Grafana)"
            echo "  --env-file FILE Use environment file"
            echo "  --help          Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                                    # Basic setup"
            echo "  $0 --monitoring                       # With monitoring"
            echo "  $0 --env-file .env.production         # With custom env file"
            echo "  $0 --monitoring --env-file .env.prod  # Full production setup"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "🚀 Starting tops-worker with Docker Compose..."

# Check if images exist, build if not
if ! docker image inspect tops-worker:latest >/dev/null 2>&1; then
    echo "📦 Building images first..."
    ./docker-build.sh
fi

# Start services
echo "🔧 Starting services..."
docker compose $PROFILE up -d

echo "✅ Services started successfully!"
echo ""
echo "🌐 Available endpoints:"
echo "  - Worker health:     http://localhost:8082/health"
echo "  - Worker metrics:    http://localhost:8082/prometheus"
echo "  - Verifier:          http://localhost:8081/verify"
if [[ $PROFILE == *"monitoring"* ]]; then
    echo "  - Prometheus:       http://localhost:9090"
    echo "  - Grafana:          http://localhost:3000 (admin/admin)"
fi
echo ""
echo "📊 To view logs:"
echo "  docker compose logs -f tops-worker"
echo ""
echo "🛑 To stop:"
echo "  docker compose down"
