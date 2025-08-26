# CI/CD Pipeline Guide

## Overview

The tops-worker project includes a comprehensive CI/CD pipeline built with GitHub Actions that handles testing, building, security scanning, Docker image creation, and deployment to multiple environments.

## Pipeline Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Code Push     │───▶│   CI Pipeline   │───▶│   CD Pipeline   │
│   PR Created    │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │                        │
                              ▼                        ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │   Artifacts     │    │   Deployment    │
                       │   Docker Images │    │   Staging/Prod  │
                       └─────────────────┘    └─────────────────┘
```

## Workflows

### 1. Main CI/CD Pipeline (`.github/workflows/ci-cd.yml`)

**Triggers:**

- Push to `main`/`master` branches
- Pull requests to `main`/`master` branches
- Tags starting with `v*` (releases)

**Jobs:**

#### Test Job

- **Platforms:** Ubuntu, macOS
- **Rust Versions:** Stable, 1.88.0
- **Features:** CPU fallback, GPU (Ubuntu only)
- **Checks:** Unit tests, clippy, format check

#### Security Audit

- **Dependencies:** `cargo audit`
- **Vulnerabilities:** Deny warnings

#### Build Job

- **Targets:** Linux x64, macOS x64
- **Artifacts:** CPU and GPU binaries
- **Retention:** 30 days

#### Docker Build

- **Images:** CPU, GPU, Verifier
- **Platforms:** linux/amd64, linux/arm64
- **Registry:** GitHub Container Registry (ghcr.io)
- **Caching:** GitHub Actions cache

#### Release Job

- **Trigger:** Tags (v\*)
- **Artifacts:** Binary releases
- **Notes:** Auto-generated

#### Performance Testing

- **Trigger:** Main branch pushes
- **Metrics:** Performance benchmarks
- **Artifacts:** Results storage

### 2. Dependency Scanning (`.github/workflows/dependency-scan.yml`)

**Triggers:**

- Weekly schedule (Sundays 2 AM UTC)
- Push to main branches
- Pull requests

**Jobs:**

- **Dependency Audit:** `cargo audit`, `cargo outdated`
- **Docker Security:** Trivy vulnerability scanning
- **Reporting:** SARIF format for GitHub Security tab

### 3. Deployment (`.github/workflows/deploy.yml`)

**Triggers:**

- Successful CI/CD pipeline completion
- Main branch only

**Jobs:**

- **Staging Deployment:** Automatic deployment to staging
- **Production Deployment:** Manual approval required
- **Health Checks:** Post-deployment verification

## Kubernetes Deployment

### Base Configuration (`k8s/base/`)

**Components:**

- **Namespace:** `tops-worker`
- **ConfigMap:** Application configuration
- **Secret:** Sensitive data (private keys)
- **Deployments:** Worker and verifier services
- **Services:** Internal networking
- **Ingress:** External access
- **HPA:** Horizontal Pod Autoscaler

### Environment Overlays

#### Staging (`k8s/overlays/staging/`)

- **Replicas:** 1 each
- **Features:** CPU fallback, debug mode
- **Resources:** Lower limits
- **Images:** Latest tags

#### Production (`k8s/overlays/production/`)

- **Replicas:** 3 each
- **Features:** GPU acceleration, production mode
- **Resources:** Higher limits
- **Images:** Versioned tags

## Usage

### Local Development

```bash
# Run tests locally
cargo test --features gpu

# Build Docker images
./docker-build.sh

# Run with Docker Compose
./docker-run.sh
```

### CI/CD Pipeline

#### Automatic Triggers

1. **Push to main:** Triggers full pipeline
2. **Pull Request:** Runs tests and security checks
3. **Tag creation:** Creates GitHub release

#### Manual Actions

```bash
# Create a release
git tag v1.0.0
git push origin v1.0.0

# Deploy to staging (automatic after CI success)
# Deploy to production (requires manual approval)
```

### Kubernetes Deployment

#### Staging

```bash
kubectl apply -k k8s/overlays/staging/
```

#### Production

```bash
kubectl apply -k k8s/overlays/production/
```

## Configuration

### Environment Variables

#### Worker Configuration

- `WORKER_SK_HEX`: Private key for signing
- `AGGREGATOR_URL`: Verifier service URL
- `METRICS_ENABLED`: Enable Prometheus metrics
- `AUTOTUNE_TARGET_MS`: Target execution time
- `RATE_LIMIT_PER_SECOND`: Request rate limiting
- `CPU_FALLBACK`: Enable CPU fallback mode

#### Verifier Configuration

- `VERIFY_PUBKEY`: Public key for verification
- `VERIFY_DISABLE`: Disable signature verification
- `PORT`: Service port

### Secrets Management

**GitHub Secrets:**

- `GITHUB_TOKEN`: Auto-provided
- `DOCKER_USERNAME`: Docker registry username
- `DOCKER_PASSWORD`: Docker registry password

**Kubernetes Secrets:**

- `WORKER_SK_HEX`: Worker private key
- `VERIFY_PUBKEY`: Verifier public key

## Security Features

### Code Security

- **Dependency Scanning:** Weekly automated scans
- **Vulnerability Detection:** Cargo audit integration
- **Code Quality:** Clippy and format checks

### Container Security

- **Base Images:** Minimal, non-root users
- **Vulnerability Scanning:** Trivy integration
- **Image Signing:** Docker content trust (optional)

### Runtime Security

- **Pod Security:** Non-root execution
- **Network Policies:** Restricted communication
- **Resource Limits:** CPU and memory constraints

## Monitoring and Observability

### Metrics

- **Prometheus:** Built-in metrics endpoint
- **Health Checks:** Liveness and readiness probes
- **Custom Metrics:** Work throughput, error rates

### Logging

- **Structured Logs:** JSON format
- **Log Levels:** Configurable verbosity
- **Centralized:** Kubernetes logging

### Alerting

- **Health Checks:** Automatic failure detection
- **Performance:** Resource utilization alerts
- **Security:** Vulnerability notifications

## Troubleshooting

### Common Issues

#### Pipeline Failures

1. **Test Failures:** Check test output for specific errors
2. **Build Failures:** Verify dependencies and features
3. **Docker Build Failures:** Check Dockerfile syntax

#### Deployment Issues

1. **Image Pull Errors:** Verify registry access
2. **Pod Startup Failures:** Check logs and configuration
3. **Service Connectivity:** Verify network policies

#### Performance Issues

1. **High Resource Usage:** Adjust resource limits
2. **Slow Response Times:** Check autoscaling configuration
3. **GPU Issues:** Verify GPU drivers and configuration

### Debug Commands

```bash
# Check pipeline status
gh run list

# View pipeline logs
gh run view <run-id>

# Check Kubernetes resources
kubectl get pods -n tops-worker
kubectl logs -f deployment/tops-worker

# Test service connectivity
kubectl port-forward svc/tops-worker-service 8082:8082
curl http://localhost:8082/health
```

## Best Practices

### Development

1. **Feature Branches:** Use feature branches for development
2. **Pull Requests:** Require reviews for main branch
3. **Testing:** Write comprehensive tests
4. **Documentation:** Keep documentation updated

### Deployment

1. **Staging First:** Always deploy to staging first
2. **Rollback Plan:** Have rollback procedures ready
3. **Monitoring:** Monitor deployments closely
4. **Security:** Regular security updates

### Maintenance

1. **Dependencies:** Keep dependencies updated
2. **Images:** Regular base image updates
3. **Backups:** Regular configuration backups
4. **Documentation:** Keep runbooks updated

## Future Enhancements

### Planned Features

- **Multi-Region Deployment:** Geographic distribution
- **Blue-Green Deployment:** Zero-downtime deployments
- **Canary Releases:** Gradual rollout
- **Advanced Monitoring:** Custom dashboards
- **Security Scanning:** SAST/DAST integration

### Scalability Improvements

- **Horizontal Scaling:** Multi-instance deployment
- **Load Balancing:** Advanced traffic management
- **Caching:** Redis integration
- **Database:** Persistent storage

## Support

For issues with the CI/CD pipeline:

1. Check the GitHub Actions logs
2. Review the troubleshooting section
3. Create an issue with detailed information
4. Contact the development team
