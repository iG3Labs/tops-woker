#!/bin/bash

# Docker build script for tops-worker
set -e

echo "🐳 Building tops-worker Docker images..."

# Build the GPU worker image
echo "📦 Building tops-worker GPU image..."
docker build -t tops-worker:gpu .

# Build the CPU fallback worker image
echo "📦 Building tops-worker CPU image..."
docker build -t tops-worker:cpu -f Dockerfile.cpu .

# Tag CPU as latest for docker-compose
docker tag tops-worker:cpu tops-worker:latest

# Build the verifier image
echo "📦 Building tops-verifier image..."
docker build -t tops-verifier:latest ./verifier

echo "✅ All images built successfully!"
echo ""
echo "Available images:"
echo "  - tops-worker:gpu (GPU version with OpenCL)"
echo "  - tops-worker:cpu (CPU fallback version)"
echo "  - tops-worker:latest (CPU version for docker-compose)"
echo "  - tops-verifier:latest"
echo ""
echo "To run with Docker Compose (CPU mode):"
echo "  docker compose up -d"
echo ""
echo "To run with monitoring stack:"
echo "  docker compose --profile monitoring up -d"
echo ""
echo "To run GPU version manually:"
echo "  docker run --gpus all tops-worker:gpu"
