# Docker Containerization Guide

## Overview

The tops-worker is now fully containerized with Docker, providing consistent deployment across different environments. The setup includes the worker, verifier, and optional monitoring stack.

## Quick Start

### 1. Basic Setup

```bash
# Build and run basic services
./docker-run.sh

# Or manually:
docker-compose up -d
```

### 2. With Monitoring Stack

```bash
# Build and run with Prometheus + Grafana
./docker-run.sh --monitoring

# Or manually:
docker-compose --profile monitoring up -d
```

### 3. Custom Environment

```bash
# Copy example environment file
cp env.example .env

# Edit configuration
nano .env

# Run with custom config
./docker-run.sh --env-file .env
```

## Architecture

### Services

1. **tops-worker**: Main Rust worker with OpenCL GPU acceleration
2. **tops-verifier**: Node.js verification service
3. **prometheus**: Metrics collection (optional)
4. **grafana**: Visualization dashboard (optional)

### Network

- **tops-network**: Internal bridge network for service communication
- **Port 8081**: Verifier service
- **Port 8082**: Worker health and metrics
- **Port 9090**: Prometheus (monitoring profile)
- **Port 3000**: Grafana (monitoring profile)

## Docker Images

### tops-worker

**Base**: `rust:1.88-slim` → `debian:bookworm-slim`

**Features**:

- Multi-stage build for optimized image size
- OpenCL runtime dependencies
- Non-root user for security
- Health checks
- Volume mounts for logs

**Build**:

```bash
docker build -t tops-worker:latest .
```

### tops-verifier

**Base**: `node:18-alpine`

**Features**:

- Lightweight Alpine Linux
- Non-root user
- Health checks
- Optimized for production

**Build**:

```bash
docker build -t tops-verifier:latest ./verifier
```

## Configuration

### Environment Variables

| Variable                | Default                       | Description               |
| ----------------------- | ----------------------------- | ------------------------- |
| `WORKER_SK_HEX`         | Required                      | Worker private key (hex)  |
| `AGGREGATOR_URL`        | `http://verifier:8081/verify` | Verifier endpoint         |
| `METRICS_ENABLED`       | `1`                           | Enable metrics collection |
| `AUTOTUNE_TARGET_MS`    | `300`                         | Target execution time     |
| `MAX_RETRIES`           | `3`                           | Maximum retry attempts    |
| `RATE_LIMIT_PER_SECOND` | `10`                          | Rate limiting             |

### OpenCL Tuning

| Variable | Default | Description  |
| -------- | ------- | ------------ |
| `TM`     | `8`     | Tile size M  |
| `TN`     | `8`     | Tile size N  |
| `TK`     | `8`     | Tile size K  |
| `WG_M`   | `16`    | Work group M |
| `WG_N`   | `16`    | Work group N |

## Monitoring Stack

### Prometheus Configuration

**File**: `monitoring/prometheus.yml`

**Features**:

- Scrapes worker metrics every 15s
- Alerting rules for critical conditions
- 200-hour data retention

**Alerts**:

- High error rate (>5%)
- Worker down
- High response time (>1000ms)
- Consecutive failures (>10)
- Low success rate (<90%)

### Grafana Dashboard

**File**: `monitoring/grafana/dashboards/tops-worker-dashboard.json`

**Panels**:

- Success rate gauge
- Throughput graphs
- Response time percentiles
- Error counts
- Uptime statistics

## Deployment Scenarios

### Development

```bash
# Basic development setup
docker-compose up -d

# View logs
docker-compose logs -f tops-worker

# Access services
curl http://localhost:8082/health
curl http://localhost:8081/verify
```

### Production

```bash
# Production with monitoring
docker-compose --profile monitoring up -d

# Scale worker instances
docker-compose up -d --scale tops-worker=3

# Use external volumes
docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
```

### Kubernetes

```bash
# Deploy to Kubernetes
kubectl apply -f k8s/

# Check status
kubectl get pods -l app=tops-worker
kubectl logs -f deployment/tops-worker
```

## Health Checks

### Worker Health Check

```bash
# Manual health check
curl -f http://localhost:8082/health

# Expected response:
{
  "status": "healthy",
  "uptime_seconds": 1234,
  "version": "0.1.0",
  "timestamp": "2025-08-26T..."
}
```

### Container Health

```bash
# Check container health
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

# View health check logs
docker inspect tops-worker | jq '.[0].State.Health'
```

## Logging

### Log Configuration

```yaml
# docker-compose.yml
services:
  tops-worker:
    volumes:
      - ./logs:/app/logs
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
```

### Log Access

```bash
# View worker logs
docker-compose logs -f tops-worker

# View specific service logs
docker logs -f tops-worker

# Export logs
docker-compose logs tops-worker > worker.log
```

## Security

### Security Features

1. **Non-root users**: Both containers run as non-root
2. **Minimal base images**: Alpine and slim Debian
3. **No secrets in images**: Environment variables only
4. **Network isolation**: Internal bridge network
5. **Health checks**: Automatic failure detection

### Security Best Practices

```bash
# Use secrets management
docker secret create worker_key worker.key

# Run with read-only root
docker run --read-only tops-worker:latest

# Limit container resources
docker run --memory=1g --cpus=2 tops-worker:latest
```

## Troubleshooting

### Common Issues

1. **OpenCL not found**

   ```bash
   # Check GPU drivers
   docker run --rm --gpus all nvidia/cuda:11.0-base nvidia-smi

   # Install OpenCL drivers
   docker run --rm -v /usr/lib/x86_64-linux-gnu:/host-lib tops-worker:latest
   ```

2. **Port conflicts**

   ```bash
   # Check port usage
   netstat -tulpn | grep :8082

   # Use different ports
   docker-compose -f docker-compose.yml -f docker-compose.override.yml up -d
   ```

3. **Memory issues**

   ```bash
   # Monitor memory usage
   docker stats tops-worker

   # Increase memory limit
   docker-compose up -d --scale tops-worker=1
   ```

### Debug Commands

```bash
# Enter container
docker exec -it tops-worker bash

# Check OpenCL devices
docker exec tops-worker clinfo

# Test GPU access
docker exec tops-worker cargo test

# View environment
docker exec tops-worker env | grep WORKER
```

## Performance Optimization

### Build Optimization

```dockerfile
# Use build cache
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Multi-stage build
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/tops-worker /app/
```

### Runtime Optimization

```yaml
# docker-compose.yml
services:
  tops-worker:
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: "2.0"
        reservations:
          memory: 1G
          cpus: "1.0"
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/docker.yml
name: Docker Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build Docker image
        run: |
          docker build -t tops-worker:${{ github.sha }} .
          docker build -t tops-verifier:${{ github.sha }} ./verifier
```

### Docker Hub

```bash
# Push to registry
docker tag tops-worker:latest your-registry/tops-worker:latest
docker push your-registry/tops-worker:latest

# Pull from registry
docker pull your-registry/tops-worker:latest
```

## Monitoring Integration

### Prometheus Queries

```promql
# Success rate
tops_worker_success_rate / 100

# Throughput
rate(tops_worker_successful_attempts_total[1m])

# Error rate
rate(tops_worker_failed_attempts_total[5m]) / rate(tops_worker_total_attempts_total[5m]) * 100
```

### Grafana Alerts

```yaml
# monitoring/alerts.yml
- alert: HighErrorRate
  expr: (rate(tops_worker_failed_attempts_total[5m]) / rate(tops_worker_total_attempts_total[5m])) * 100 > 5
  for: 2m
  labels:
    severity: warning
```

## Next Steps

1. **Kubernetes manifests**: Create k8s deployment files
2. **Helm chart**: Package for Kubernetes deployment
3. **Service mesh**: Integrate with Istio/Linkerd
4. **Auto-scaling**: Implement HPA based on metrics
5. **Backup strategy**: Volume backup and restore
6. **Disaster recovery**: Multi-region deployment
