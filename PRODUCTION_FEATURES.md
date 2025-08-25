# tops-worker Production Features

This document describes the production-ready features implemented in tops-worker to make it robust, monitorable, and maintainable in production environments.

## 🏗️ **Architecture Overview**

The tops-worker now includes a comprehensive production stack:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Configuration │    │   Error Handling│    │   Health Server │
│   Management    │    │   & Recovery    │    │   (Port 8082)   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Metrics       │    │   Rate Limiting │    │   Main Worker   │
│   Collection    │    │   & Throttling  │    │   Loop          │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 📋 **1. Configuration Management**

### **Environment Variables**

The worker now supports comprehensive configuration through environment variables:

#### **Required Configuration**
- `WORKER_SK_HEX` - 64-character hex private key for signing receipts

#### **Worker Configuration**
- `DEVICE_DID` - Device identifier (default: `did:peaq:DEVICE123`)
- `AGGREGATOR_URL` - URL for submitting receipts (default: `http://localhost:8081/verify`)

#### **Performance Tuning**
- `AUTOTUNE_TARGET_MS` - Target execution time in milliseconds (default: 300)
- `AUTOTUNE_PRESETS` - Matrix size presets in format `"m1,n1,k1;m2,n2,k2"` (default: `"512,512,512;1024,1024,1024"`)
- `AUTOTUNE_DISABLE` - Set to `1` to disable autotuning (default: disabled)

#### **OpenCL Kernel Tuning**
- `WG_M` - Work group size for M dimension
- `WG_N` - Work group size for N dimension  
- `TK` - Tile size for K dimension

#### **Monitoring & Logging**
- `WORKER_DEBUG_RECEIPT` - Set to `1` to print full receipts (default: disabled)
- `LOG_LEVEL` - Logging level (default: `info`)
- `METRICS_ENABLED` - Enable metrics collection and health server (default: enabled)

#### **Error Handling & Recovery**
- `MAX_RETRIES` - Maximum retry attempts for failed operations (default: 3)
- `RETRY_DELAY_MS` - Delay between retries in milliseconds (default: 1000)
- `HEALTH_CHECK_INTERVAL_MS` - Health check interval (default: 30000)

#### **Security & Rate Limiting**
- `RATE_LIMIT_PER_SECOND` - Maximum requests per second (default: 10)
- `MAX_CONCURRENT_REQUESTS` - Maximum concurrent operations (default: 5)

### **Configuration Validation**

The configuration system includes comprehensive validation:

```rust
// Example validation errors
ConfigError::MissingEnvVar("WORKER_SK_HEX".to_string())
ConfigError::InvalidEnvVar("AUTOTUNE_TARGET_MS".to_string(), "invalid".to_string())
ConfigError::ValidationError("WORKER_SK_HEX must be 64 characters".to_string())
```

## 📊 **2. Metrics Collection**

### **Performance Metrics**
- `total_attempts` - Total number of attempts made
- `successful_attempts` - Number of successful attempts
- `failed_attempts` - Number of failed attempts
- `average_time_ms` - Average execution time per attempt
- `min_time_ms` - Minimum execution time
- `max_time_ms` - Maximum execution time
- `attempts_per_second` - Throughput rate
- `receipts_per_second` - Successful receipts per second

### **Error Metrics**
- `gpu_errors` - GPU-related errors
- `network_errors` - Network communication errors
- `signature_errors` - Cryptographic signing errors
- `validation_errors` - Data validation errors

### **Health Metrics**
- `uptime_seconds` - Worker uptime
- `last_successful_attempt` - Timestamp of last success
- `consecutive_failures` - Number of consecutive failures

### **Usage Example**

```rust
let metrics = Arc::new(MetricsCollector::new());

// Record an attempt
metrics.record_attempt(150, true);  // 150ms, successful

// Record an error
metrics.record_error(ErrorType::Network);

// Get current metrics
let current_metrics = metrics.get_metrics();
println!("Success rate: {:.2}%", 
    (current_metrics.successful_attempts as f64 / current_metrics.total_attempts as f64) * 100.0);
```

## 🛡️ **3. Error Handling & Recovery**

### **Circuit Breaker Pattern**

The worker implements a circuit breaker to prevent cascading failures:

```rust
let circuit_breaker = CircuitBreaker::new(5, Duration::from_secs(60));

// Check if operation can proceed
if circuit_breaker.can_execute() {
    // Perform operation
    circuit_breaker.record_success();
} else {
    // Circuit is open, skip operation
}
```

### **Retry Logic with Exponential Backoff**

```rust
let retry_config = RetryConfig {
    max_retries: 3,
    retry_delay: Duration::from_millis(1000),
    backoff_multiplier: 2.0,
    max_retry_delay: Duration::from_secs(30),
};
```

### **Error Classification**

Errors are classified into categories for better monitoring:

- **GPU Errors** - OpenCL/CUDA execution failures
- **Network Errors** - HTTP request failures
- **Signature Errors** - Cryptographic operation failures
- **Validation Errors** - Data validation failures

## 🏥 **4. Health Monitoring**

### **Health Server**

The worker includes a built-in HTTP server (port 8082) with health endpoints:

#### **Endpoints**

- `GET /health` - Basic health status
- `GET /metrics` - Detailed metrics
- `GET /status` - Comprehensive status including configuration
- `GET /` - HTML dashboard with links to all endpoints

#### **Health Status Levels**

- **Healthy** - Worker is functioning normally
- **Degraded** - Some issues detected but still operational
- **Unhealthy** - Significant problems affecting performance
- **Critical** - Worker is failing and needs immediate attention

### **Health Response Example**

```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "version": "0.1.0",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### **Detailed Status Example**

```json
{
  "health": "healthy",
  "uptime_seconds": 3600,
  "total_attempts": 1000,
  "successful_attempts": 995,
  "failed_attempts": 5,
  "success_rate": 0.995,
  "average_time_ms": 174.5,
  "attempts_per_second": 0.28,
  "receipts_per_second": 0.28,
  "consecutive_failures": 0,
  "error_counts": {
    "gpu_errors": 0,
    "network_errors": 3,
    "signature_errors": 0,
    "validation_errors": 2
  },
  "config_summary": {
    "autotune_target_ms": 300,
    "aggregator_url": "http://localhost:8081/verify",
    "device_did": "did:peaq:DEVICE123",
    "max_retries": 3,
    "rate_limit_per_second": 10
  }
}
```

## ⚡ **5. Rate Limiting**

### **Token Bucket Rate Limiter**

The worker implements a token bucket rate limiter to prevent overwhelming external services:

```rust
let rate_limiter = RateLimiter::new(10, 5.0);  // 10 tokens, 5 tokens/sec refill

// Wait for available token
rate_limiter.wait_for_token();

// Or check if token is available
if rate_limiter.try_acquire() {
    // Proceed with operation
}
```

## 🔧 **6. Usage Examples**

### **Basic Production Setup**

```bash
# Required configuration
export WORKER_SK_HEX=7b706b652278aba9b01dd473e026fd0baf215fd5afbf92d860b03fa661e07dc2
export AGGREGATOR_URL=https://my-aggregator.com/verify

# Performance tuning
export AUTOTUNE_TARGET_MS=300
export AUTOTUNE_PRESETS="512,512,512;1024,1024,1024;1536,1536,1536"

# Monitoring
export METRICS_ENABLED=1
export WORKER_DEBUG_RECEIPT=0

# Error handling
export MAX_RETRIES=5
export RETRY_DELAY_MS=2000

# Rate limiting
export RATE_LIMIT_PER_SECOND=20
export MAX_CONCURRENT_REQUESTS=10

# Start the worker
cargo run --release
```

### **Monitoring with curl**

```bash
# Check basic health
curl http://localhost:8082/health

# Get detailed metrics
curl http://localhost:8082/metrics | jq .

# Get comprehensive status
curl http://localhost:8082/status | jq .
```

### **Testing Production Features**

```bash
# Run the test script
./test_production_features.sh
```

## 🚀 **7. Deployment Considerations**

### **Docker Deployment**

```dockerfile
FROM rust:1.88 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/tops-worker /usr/local/bin/
EXPOSE 8082
CMD ["tops-worker"]
```

### **Kubernetes Deployment**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: tops-worker
spec:
  replicas: 3
  selector:
    matchLabels:
      app: tops-worker
  template:
    metadata:
      labels:
        app: tops-worker
    spec:
      containers:
      - name: tops-worker
        image: tops-worker:latest
        env:
        - name: WORKER_SK_HEX
          valueFrom:
            secretKeyRef:
              name: worker-secrets
              key: worker-sk-hex
        - name: AGGREGATOR_URL
          value: "https://aggregator.example.com/verify"
        - name: METRICS_ENABLED
          value: "1"
        ports:
        - containerPort: 8082
          name: health
        livenessProbe:
          httpGet:
            path: /health
            port: 8082
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8082
          initialDelaySeconds: 5
          periodSeconds: 5
```

### **Monitoring Integration**

The health endpoints can be easily integrated with monitoring systems:

- **Prometheus** - Scrape metrics from `/metrics`
- **Grafana** - Create dashboards using health data
- **AlertManager** - Set up alerts based on health status
- **ELK Stack** - Parse structured logs for analysis

## 📈 **8. Performance Monitoring**

### **Key Performance Indicators (KPIs)**

1. **Throughput** - Receipts per second
2. **Latency** - Average execution time
3. **Success Rate** - Percentage of successful attempts
4. **Error Rate** - Error frequency by type
5. **Uptime** - Worker availability

### **Alerting Thresholds**

- **Critical**: Success rate < 80% OR consecutive failures > 10
- **Warning**: Success rate < 95% OR average latency > 500ms
- **Info**: Uptime milestones or configuration changes

## 🔒 **9. Security Features**

### **Input Validation**
- Environment variable validation
- Configuration parameter bounds checking
- URL format validation

### **Rate Limiting**
- Prevents DoS attacks
- Protects external services
- Configurable limits

### **Error Handling**
- No sensitive data in error messages
- Graceful degradation
- Circuit breaker protection

## 🎯 **10. Future Enhancements**

### **Planned Features**
- [ ] Structured logging with JSON format
- [ ] Prometheus metrics export
- [ ] Configuration hot-reloading
- [ ] Distributed tracing support
- [ ] Advanced alerting rules
- [ ] Performance profiling endpoints
- [ ] Configuration validation webhook
- [ ] Health check dependencies

### **Integration Opportunities**
- [ ] Kubernetes operator
- [ ] Helm charts
- [ ] Terraform modules
- [ ] CI/CD pipelines
- [ ] Monitoring dashboards
- [ ] Log aggregation

---

## 📚 **Additional Resources**

- [README.md](./README.md) - Main project documentation
- [test_production_features.sh](./test_production_features.sh) - Production features test script
- [BENCHMARKING_RESULTS.md](./BENCHMARKING_RESULTS.md) - Performance benchmarking results

---

*This document covers the production features implemented in tops-worker v0.1.0. For questions or contributions, please refer to the main project documentation.*
