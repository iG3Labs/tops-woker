use std::sync::Arc;
use crate::metrics::{MetricsCollector, HealthStatus};
use crate::config::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub version: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub metrics: crate::metrics::Metrics,
    pub health_status: String,
    pub circuit_breaker_status: Option<String>,
}

pub struct HealthChecker {
    metrics: Arc<MetricsCollector>,
    config: Config,
    start_time: std::time::Instant,
}

impl HealthChecker {
    pub fn new(metrics: Arc<MetricsCollector>, config: Config) -> Self {
        Self {
            metrics,
            config,
            start_time: std::time::Instant::now(),
        }
    }
    
    pub fn get_health(&self) -> HealthResponse {
        let health_status = self.metrics.get_health_status();
        let uptime_seconds = self.start_time.elapsed().as_secs();
        
        HealthResponse {
            status: health_status.to_string(),
            uptime_seconds,
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn get_metrics(&self) -> MetricsResponse {
        let metrics = self.metrics.get_metrics();
        let health_status = self.metrics.get_health_status();
        
        MetricsResponse {
            metrics,
            health_status: health_status.to_string(),
            circuit_breaker_status: None, // Will be set by main if available
        }
    }
    
    pub fn is_healthy(&self) -> bool {
        matches!(self.metrics.get_health_status(), HealthStatus::Healthy)
    }
    
    pub fn get_detailed_status(&self) -> DetailedStatus {
        let metrics = self.metrics.get_metrics();
        let health_status = self.metrics.get_health_status();
        
        DetailedStatus {
            health: health_status.to_string(),
            uptime_seconds: metrics.uptime_seconds,
            total_attempts: metrics.total_attempts,
            successful_attempts: metrics.successful_attempts,
            failed_attempts: metrics.failed_attempts,
            success_rate: if metrics.total_attempts > 0 {
                metrics.successful_attempts as f64 / metrics.total_attempts as f64
            } else {
                0.0
            },
            average_time_ms: metrics.average_time_ms,
            attempts_per_second: metrics.attempts_per_second,
            receipts_per_second: metrics.receipts_per_second,
            consecutive_failures: metrics.consecutive_failures,
            error_counts: ErrorCounts {
                gpu_errors: metrics.gpu_errors,
                network_errors: metrics.network_errors,
                signature_errors: metrics.signature_errors,
                validation_errors: metrics.validation_errors,
            },
            config_summary: ConfigSummary {
                autotune_target_ms: self.config.autotune_target_ms,
                aggregator_url: self.config.aggregator_url.clone(),
                device_did: self.config.device_did.clone(),
                max_retries: self.config.max_retries,
                rate_limit_per_second: self.config.rate_limit_per_second,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DetailedStatus {
    pub health: String,
    pub uptime_seconds: u64,
    pub total_attempts: u64,
    pub successful_attempts: u64,
    pub failed_attempts: u64,
    pub success_rate: f64,
    pub average_time_ms: f64,
    pub attempts_per_second: f64,
    pub receipts_per_second: f64,
    pub consecutive_failures: u32,
    pub error_counts: ErrorCounts,
    pub config_summary: ConfigSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorCounts {
    pub gpu_errors: u64,
    pub network_errors: u64,
    pub signature_errors: u64,
    pub validation_errors: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSummary {
    pub autotune_target_ms: u64,
    pub aggregator_url: String,
    pub device_did: String,
    pub max_retries: u32,
    pub rate_limit_per_second: u32,
}
