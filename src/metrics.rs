use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    // Performance metrics
    pub total_attempts: u64,
    pub successful_attempts: u64,
    pub failed_attempts: u64,
    pub average_time_ms: f64,
    pub min_time_ms: u64,
    pub max_time_ms: u64,
    
    // Error metrics
    pub gpu_errors: u64,
    pub network_errors: u64,
    pub signature_errors: u64,
    pub validation_errors: u64,
    
    // Health metrics
    pub uptime_seconds: u64,
    pub last_successful_attempt: Option<u64>,
    pub consecutive_failures: u32,
    
    // Throughput metrics
    pub attempts_per_second: f64,
    pub receipts_per_second: f64,
}

#[derive(Debug)]
pub struct MetricsCollector {
    // Atomic counters for thread-safe updates
    total_attempts: AtomicU64,
    successful_attempts: AtomicU64,
    failed_attempts: AtomicU64,
    gpu_errors: AtomicU64,
    network_errors: AtomicU64,
    signature_errors: AtomicU64,
    validation_errors: AtomicU64,
    consecutive_failures: AtomicU32,
    
    // Timing data
    start_time: Instant,
    last_success_time: Arc<std::sync::Mutex<Option<Instant>>>,
    
    // Performance tracking
    total_time_ms: AtomicU64,
    min_time_ms: AtomicU64,
    max_time_ms: AtomicU64,
    attempt_count: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_attempts: AtomicU64::new(0),
            successful_attempts: AtomicU64::new(0),
            failed_attempts: AtomicU64::new(0),
            gpu_errors: AtomicU64::new(0),
            network_errors: AtomicU64::new(0),
            signature_errors: AtomicU64::new(0),
            validation_errors: AtomicU64::new(0),
            consecutive_failures: AtomicU32::new(0),
            start_time: Instant::now(),
            last_success_time: Arc::new(std::sync::Mutex::new(None)),
            total_time_ms: AtomicU64::new(0),
            min_time_ms: AtomicU64::new(u64::MAX),
            max_time_ms: AtomicU64::new(0),
            attempt_count: AtomicU64::new(0),
        }
    }
    
    pub fn record_attempt(&self, time_ms: u64, success: bool) {
        self.total_attempts.fetch_add(1, Ordering::Relaxed);
        
        if success {
            self.successful_attempts.fetch_add(1, Ordering::Relaxed);
            self.consecutive_failures.store(0, Ordering::Relaxed);
            
            // Update last success time
            if let Ok(mut last_success) = self.last_success_time.lock() {
                *last_success = Some(Instant::now());
            }
        } else {
            self.failed_attempts.fetch_add(1, Ordering::Relaxed);
            self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        }
        
        // Update timing statistics
        self.total_time_ms.fetch_add(time_ms, Ordering::Relaxed);
        self.attempt_count.fetch_add(1, Ordering::Relaxed);
        
        // Update min/max times
        let mut current_min = self.min_time_ms.load(Ordering::Relaxed);
        while time_ms < current_min {
            match self.min_time_ms.compare_exchange_weak(
                current_min, time_ms, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(new_min) => current_min = new_min,
            }
        }
        
        let mut current_max = self.max_time_ms.load(Ordering::Relaxed);
        while time_ms > current_max {
            match self.max_time_ms.compare_exchange_weak(
                current_max, time_ms, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(new_max) => current_max = new_max,
            }
        }
    }
    
    pub fn record_error(&self, error_type: ErrorType) {
        match error_type {
            ErrorType::Gpu => self.gpu_errors.fetch_add(1, Ordering::Relaxed),
            ErrorType::Network => self.network_errors.fetch_add(1, Ordering::Relaxed),
            ErrorType::Signature => self.signature_errors.fetch_add(1, Ordering::Relaxed),
            ErrorType::Validation => self.validation_errors.fetch_add(1, Ordering::Relaxed),
        };
    }
    
    pub fn get_metrics(&self) -> Metrics {
        let total_attempts = self.total_attempts.load(Ordering::Relaxed);
        let successful_attempts = self.successful_attempts.load(Ordering::Relaxed);
        let failed_attempts = self.failed_attempts.load(Ordering::Relaxed);
        let total_time_ms = self.total_time_ms.load(Ordering::Relaxed);
        let attempt_count = self.attempt_count.load(Ordering::Relaxed);
        let min_time_ms = self.min_time_ms.load(Ordering::Relaxed);
        let max_time_ms = self.max_time_ms.load(Ordering::Relaxed);
        let consecutive_failures = self.consecutive_failures.load(Ordering::Relaxed);
        
        let average_time_ms = if attempt_count > 0 {
            total_time_ms as f64 / attempt_count as f64
        } else {
            0.0
        };
        
        let uptime_seconds = self.start_time.elapsed().as_secs();
        
        let last_successful_attempt = if let Ok(last_success) = self.last_success_time.lock() {
            last_success.map(|time| time.duration_since(self.start_time).as_secs())
        } else {
            None
        };
        
        let attempts_per_second = if uptime_seconds > 0 {
            total_attempts as f64 / uptime_seconds as f64
        } else {
            0.0
        };
        
        let receipts_per_second = if uptime_seconds > 0 {
            successful_attempts as f64 / uptime_seconds as f64
        } else {
            0.0
        };
        
        Metrics {
            total_attempts,
            successful_attempts,
            failed_attempts,
            average_time_ms,
            min_time_ms: if min_time_ms == u64::MAX { 0 } else { min_time_ms },
            max_time_ms,
            gpu_errors: self.gpu_errors.load(Ordering::Relaxed),
            network_errors: self.network_errors.load(Ordering::Relaxed),
            signature_errors: self.signature_errors.load(Ordering::Relaxed),
            validation_errors: self.validation_errors.load(Ordering::Relaxed),
            uptime_seconds,
            last_successful_attempt,
            consecutive_failures,
            attempts_per_second,
            receipts_per_second,
        }
    }
    
    pub fn get_health_status(&self) -> HealthStatus {
        let consecutive_failures = self.consecutive_failures.load(Ordering::Relaxed);
        let total_attempts = self.total_attempts.load(Ordering::Relaxed);
        let failed_attempts = self.failed_attempts.load(Ordering::Relaxed);
        
        let failure_rate = if total_attempts > 0 {
            failed_attempts as f64 / total_attempts as f64
        } else {
            0.0
        };
        
        if consecutive_failures >= 10 {
            HealthStatus::Critical
        } else if consecutive_failures >= 5 || failure_rate > 0.5 {
            HealthStatus::Unhealthy
        } else if consecutive_failures >= 2 || failure_rate > 0.2 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    Gpu,
    Network,
    Signature,
    Validation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Critical => write!(f, "critical"),
        }
    }
}
