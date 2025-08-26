
use prometheus_client::{
    encoding::text::encode,
    metrics::{counter::Counter, gauge::Gauge, histogram::Histogram},
    registry::Registry,
};
use crate::metrics::ErrorType;

pub struct PrometheusMetrics {
    registry: Registry,
    
    // Counters
    total_attempts: Counter,
    successful_attempts: Counter,
    failed_attempts: Counter,
    gpu_errors: Counter,
    network_errors: Counter,
    signature_errors: Counter,
    validation_errors: Counter,
    
    // Gauges
    uptime_seconds: Gauge<i64>,
    consecutive_failures: Gauge<i64>,
    success_rate: Gauge<i64>,
    
    // Histograms
    attempt_duration_ms: Histogram,
    network_latency_ms: Histogram,
}

impl PrometheusMetrics {
    pub fn new() -> Self {
        let mut registry = Registry::default();
        
        // Initialize counters
        let total_attempts = Counter::default();
        let successful_attempts = Counter::default();
        let failed_attempts = Counter::default();
        let gpu_errors = Counter::default();
        let network_errors = Counter::default();
        let signature_errors = Counter::default();
        let validation_errors = Counter::default();
        
        // Initialize gauges
        let uptime_seconds = Gauge::default();
        let consecutive_failures = Gauge::default();
        let success_rate = Gauge::default();
        
        // Initialize histograms with custom buckets
        let attempt_duration_ms = Histogram::new(
            [10.0, 25.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0].into_iter()
        );
        let network_latency_ms = Histogram::new(
            [1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0].into_iter()
        );
        
        // Register metrics
        registry.register(
            "tops_worker_total_attempts",
            "Total number of attempts made",
            total_attempts.clone(),
        );
        registry.register(
            "tops_worker_successful_attempts",
            "Total number of successful attempts",
            successful_attempts.clone(),
        );
        registry.register(
            "tops_worker_failed_attempts",
            "Total number of failed attempts",
            failed_attempts.clone(),
        );
        registry.register(
            "tops_worker_gpu_errors",
            "Total number of GPU errors",
            gpu_errors.clone(),
        );
        registry.register(
            "tops_worker_network_errors",
            "Total number of network errors",
            network_errors.clone(),
        );
        registry.register(
            "tops_worker_signature_errors",
            "Total number of signature errors",
            signature_errors.clone(),
        );
        registry.register(
            "tops_worker_validation_errors",
            "Total number of validation errors",
            validation_errors.clone(),
        );
        registry.register(
            "tops_worker_uptime_seconds",
            "Worker uptime in seconds",
            uptime_seconds.clone(),
        );
        registry.register(
            "tops_worker_consecutive_failures",
            "Number of consecutive failures",
            consecutive_failures.clone(),
        );
        registry.register(
            "tops_worker_success_rate",
            "Success rate as a percentage (multiplied by 100)",
            success_rate.clone(),
        );
        registry.register(
            "tops_worker_attempt_duration_ms",
            "Duration of attempts in milliseconds",
            attempt_duration_ms.clone(),
        );
        registry.register(
            "tops_worker_network_latency_ms",
            "Network request latency in milliseconds",
            network_latency_ms.clone(),
        );
        
        Self {
            registry,
            total_attempts,
            successful_attempts,
            failed_attempts,
            gpu_errors,
            network_errors,
            signature_errors,
            validation_errors,
            uptime_seconds,
            consecutive_failures,
            success_rate,
            attempt_duration_ms,
            network_latency_ms,
        }
    }
    
    pub fn update_from_metrics(&self, metrics: &crate::metrics::Metrics) {
        // Update uptime
        self.uptime_seconds.set(metrics.uptime_seconds as i64);
        
        // Update consecutive failures
        self.consecutive_failures.set(metrics.consecutive_failures as i64);
        
        // Update success rate (multiply by 100 to preserve 2 decimal places)
        let rate = if metrics.total_attempts > 0 {
            ((metrics.successful_attempts as f64 / metrics.total_attempts as f64) * 10000.0) as i64
        } else {
            0
        };
        self.success_rate.set(rate);
    }
    
    pub fn record_attempt(&self, duration_ms: u64, success: bool) {
        self.total_attempts.inc();
        
        if success {
            self.successful_attempts.inc();
        } else {
            self.failed_attempts.inc();
        }
        
        self.attempt_duration_ms.observe(duration_ms as f64);
    }
    
    pub fn record_error(&self, error_type: ErrorType) {
        match error_type {
            ErrorType::Gpu => self.gpu_errors.inc(),
            ErrorType::Network => self.network_errors.inc(),
            ErrorType::Signature => self.signature_errors.inc(),
            ErrorType::Validation => self.validation_errors.inc(),
        };
    }
    
    pub fn record_network_latency(&self, latency_ms: f64) {
        self.network_latency_ms.observe(latency_ms);
    }
    
    pub fn export_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut buffer = String::new();
        encode(&mut buffer, &self.registry)?;
        Ok(buffer)
    }
    
    pub fn get_registry(&self) -> &Registry {
        &self.registry
    }
}

// Helper function to create metric descriptions
pub fn get_metric_help_text() -> &'static str {
    r#"# tops-worker Prometheus Metrics

# Counters
tops_worker_total_attempts - Total number of attempts made
tops_worker_successful_attempts - Total number of successful attempts  
tops_worker_failed_attempts - Total number of failed attempts
tops_worker_gpu_errors - Total number of GPU errors
tops_worker_network_errors - Total number of network errors
tops_worker_signature_errors - Total number of signature errors
tops_worker_validation_errors - Total number of validation errors

# Gauges
tops_worker_uptime_seconds - Worker uptime in seconds
tops_worker_consecutive_failures - Number of consecutive failures
tops_worker_success_rate - Success rate as a percentage (multiplied by 100)

# Histograms
tops_worker_attempt_duration_ms - Duration of attempts in milliseconds
tops_worker_network_latency_ms - Network request latency in milliseconds

# Example queries:
# - Success rate: tops_worker_success_rate / 100
# - Average attempt duration: histogram_quantile(0.5, tops_worker_attempt_duration_ms_bucket)
# - Error rate: rate(tops_worker_gpu_errors[5m]) + rate(tops_worker_network_errors[5m])
# - Throughput: rate(tops_worker_successful_attempts[1m])
"#
}
