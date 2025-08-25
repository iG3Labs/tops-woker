use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use crate::metrics::{ErrorType, MetricsCollector};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub backoff_multiplier: f64,
    pub max_retry_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_retry_delay: Duration::from_secs(30),
        }
    }
}

#[derive(Debug)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout: Duration,
    state: Arc<Mutex<CircuitBreakerState>>,
}

#[derive(Debug, Clone)]
enum CircuitBreakerState {
    Closed { failure_count: u32 },
    Open { opened_at: Instant },
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed { failure_count: 0 })),
        }
    }
    
    pub fn can_execute(&self) -> bool {
        if let Ok(state) = self.state.lock() {
            match &*state {
                CircuitBreakerState::Closed { .. } => true,
                CircuitBreakerState::Open { opened_at } => {
                    opened_at.elapsed() >= self.recovery_timeout
                }
                CircuitBreakerState::HalfOpen => true,
            }
        } else {
            false
        }
    }
    
    pub fn record_success(&self) {
        if let Ok(mut state) = self.state.lock() {
            *state = CircuitBreakerState::Closed { failure_count: 0 };
        }
    }
    
    pub fn record_failure(&self) {
        if let Ok(mut state) = self.state.lock() {
            match &mut *state {
                CircuitBreakerState::Closed { failure_count } => {
                    *failure_count += 1;
                    if *failure_count >= self.failure_threshold {
                        *state = CircuitBreakerState::Open { opened_at: Instant::now() };
                    }
                }
                CircuitBreakerState::Open { opened_at } => {
                    if opened_at.elapsed() >= self.recovery_timeout {
                        *state = CircuitBreakerState::HalfOpen;
                    }
                }
                CircuitBreakerState::HalfOpen => {
                    *state = CircuitBreakerState::Open { opened_at: Instant::now() };
                }
            }
        }
    }
    
    pub fn get_state(&self) -> String {
        if let Ok(state) = self.state.lock() {
            match &*state {
                CircuitBreakerState::Closed { failure_count } => {
                    format!("closed (failures: {})", failure_count)
                }
                CircuitBreakerState::Open { opened_at } => {
                    let elapsed = opened_at.elapsed();
                    format!("open (elapsed: {:?})", elapsed)
                }
                CircuitBreakerState::HalfOpen => "half-open".to_string(),
            }
        } else {
            "unknown".to_string()
        }
    }
}

pub struct ErrorHandler {
    retry_config: RetryConfig,
    circuit_breaker: CircuitBreaker,
    metrics: Arc<MetricsCollector>,
}

impl ErrorHandler {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self {
            retry_config: RetryConfig::default(),
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(60)),
            metrics,
        }
    }
    
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }
    
    pub fn with_circuit_breaker(mut self, failure_threshold: u32, recovery_timeout: Duration) -> Self {
        self.circuit_breaker = CircuitBreaker::new(failure_threshold, recovery_timeout);
        self
    }
    
    pub async fn execute_with_retry<F, T, E>(&self, operation: F) -> Result<T, E>
    where
        F: Fn() -> Result<T, E>,
        E: std::fmt::Debug + std::convert::From<std::string::String>,
    {
        if !self.circuit_breaker.can_execute() {
            return Err(format!("Circuit breaker is open: {}", self.circuit_breaker.get_state()).into());
        }
        
        let mut last_error = None;
        let mut delay = self.retry_config.retry_delay;
        
        for attempt in 0..=self.retry_config.max_retries {
            match operation() {
                Ok(result) => {
                    self.circuit_breaker.record_success();
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error);
                    
                    if attempt < self.retry_config.max_retries {
                        // Record error in metrics
                        self.metrics.record_error(ErrorType::Network);
                        
                        // Wait before retry
                        tokio::time::sleep(delay).await;
                        
                        // Exponential backoff
                        delay = Duration::from_secs_f64(
                            (delay.as_secs_f64() * self.retry_config.backoff_multiplier)
                                .min(self.retry_config.max_retry_delay.as_secs_f64())
                        );
                    }
                }
            }
        }
        
        self.circuit_breaker.record_failure();
        Err(last_error.unwrap())
    }
    
    pub fn handle_gpu_error(&self, error: &str) {
        eprintln!("GPU Error: {}", error);
        self.metrics.record_error(ErrorType::Gpu);
    }
    
    pub fn handle_network_error(&self, error: &str) {
        eprintln!("Network Error: {}", error);
        self.metrics.record_error(ErrorType::Network);
    }
    
    pub fn handle_signature_error(&self, error: &str) {
        eprintln!("Signature Error: {}", error);
        self.metrics.record_error(ErrorType::Signature);
    }
    
    pub fn handle_validation_error(&self, error: &str) {
        eprintln!("Validation Error: {}", error);
        self.metrics.record_error(ErrorType::Validation);
    }
    
    pub fn get_circuit_breaker_status(&self) -> String {
        self.circuit_breaker.get_state()
    }
}

// Rate limiting
pub struct RateLimiter {
    tokens: Arc<Mutex<u32>>,
    max_tokens: u32,
    refill_rate: f64, // tokens per second
    last_refill: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    pub fn new(max_tokens: u32, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(Mutex::new(max_tokens)),
            max_tokens,
            refill_rate,
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }
    
    pub fn try_acquire(&self) -> bool {
        if let (Ok(mut tokens), Ok(mut last_refill)) = (self.tokens.lock(), self.last_refill.lock()) {
            // Refill tokens based on time elapsed
            let now = Instant::now();
            let elapsed = now.duration_since(*last_refill);
            let tokens_to_add = (elapsed.as_secs_f64() * self.refill_rate) as u32;
            
            *tokens = (*tokens + tokens_to_add).min(self.max_tokens);
            *last_refill = now;
            
            if *tokens > 0 {
                *tokens -= 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    
    pub fn wait_for_token(&self) {
        while !self.try_acquire() {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
