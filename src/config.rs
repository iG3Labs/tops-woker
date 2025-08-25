use std::env;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid environment variable {0}: {1}")]
    InvalidEnvVar(String, String),
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Worker configuration
    pub worker_sk_hex: String,
    pub device_did: String,
    pub aggregator_url: String,
    
    // Performance tuning
    pub autotune_target_ms: u64,
    pub autotune_presets: Vec<String>,
    pub autotune_disable: bool,
    
    // OpenCL tuning
    pub wg_m: Option<u32>,
    pub wg_n: Option<u32>,
    pub tk: Option<u32>,
    
    // Monitoring and logging
    pub worker_debug_receipt: bool,
    pub log_level: String,
    pub metrics_enabled: bool,
    
    // Error handling and recovery
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub health_check_interval_ms: u64,
    
    // Security
    pub rate_limit_per_second: u32,
    pub max_concurrent_requests: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            worker_sk_hex: String::new(),
            device_did: "did:peaq:DEVICE123".to_string(),
            aggregator_url: "http://localhost:8081/verify".to_string(),
            
            autotune_target_ms: 300,
            autotune_presets: vec![
                "512,512,512".to_string(),
                "1024,1024,1024".to_string(),
            ],
            autotune_disable: false,
            
            wg_m: None,
            wg_n: None,
            tk: None,
            
            worker_debug_receipt: false,
            log_level: "info".to_string(),
            metrics_enabled: true,
            
            max_retries: 3,
            retry_delay_ms: 1000,
            health_check_interval_ms: 30000,
            
            rate_limit_per_second: 10,
            max_concurrent_requests: 5,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut config = Config::default();
        
        // Required configuration
        config.worker_sk_hex = env::var("WORKER_SK_HEX")
            .map_err(|_| ConfigError::MissingEnvVar("WORKER_SK_HEX".to_string()))?;
        
        // Optional configuration with defaults
        if let Ok(val) = env::var("DEVICE_DID") {
            config.device_did = val;
        }
        
        if let Ok(val) = env::var("AGGREGATOR_URL") {
            config.aggregator_url = val;
        }
        
        if let Ok(val) = env::var("AUTOTUNE_TARGET_MS") {
            config.autotune_target_ms = val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("AUTOTUNE_TARGET_MS".to_string(), val))?;
        }
        
        if let Ok(val) = env::var("AUTOTUNE_PRESETS") {
            config.autotune_presets = val.split(';').map(|s| s.to_string()).collect();
        }
        
        if let Ok(val) = env::var("AUTOTUNE_DISABLE") {
            config.autotune_disable = val == "1";
        }
        
        // OpenCL tuning parameters
        if let Ok(val) = env::var("WG_M") {
            config.wg_m = Some(val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("WG_M".to_string(), val))?);
        }
        
        if let Ok(val) = env::var("WG_N") {
            config.wg_n = Some(val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("WG_N".to_string(), val))?);
        }
        
        if let Ok(val) = env::var("TK") {
            config.tk = Some(val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("TK".to_string(), val))?);
        }
        
        // Debug and logging
        if let Ok(val) = env::var("WORKER_DEBUG_RECEIPT") {
            config.worker_debug_receipt = val == "1";
        }
        
        if let Ok(val) = env::var("LOG_LEVEL") {
            config.log_level = val;
        }
        
        if let Ok(val) = env::var("METRICS_ENABLED") {
            config.metrics_enabled = val == "1";
        }
        
        // Error handling
        if let Ok(val) = env::var("MAX_RETRIES") {
            config.max_retries = val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("MAX_RETRIES".to_string(), val))?;
        }
        
        if let Ok(val) = env::var("RETRY_DELAY_MS") {
            config.retry_delay_ms = val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("RETRY_DELAY_MS".to_string(), val))?;
        }
        
        if let Ok(val) = env::var("HEALTH_CHECK_INTERVAL_MS") {
            config.health_check_interval_ms = val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("HEALTH_CHECK_INTERVAL_MS".to_string(), val))?;
        }
        
        // Security
        if let Ok(val) = env::var("RATE_LIMIT_PER_SECOND") {
            config.rate_limit_per_second = val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("RATE_LIMIT_PER_SECOND".to_string(), val))?;
        }
        
        if let Ok(val) = env::var("MAX_CONCURRENT_REQUESTS") {
            config.max_concurrent_requests = val.parse()
                .map_err(|_| ConfigError::InvalidEnvVar("MAX_CONCURRENT_REQUESTS".to_string(), val))?;
        }
        
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.worker_sk_hex.is_empty() {
            return Err(ConfigError::ValidationError("WORKER_SK_HEX is required".to_string()));
        }
        
        if self.worker_sk_hex.len() != 64 {
            return Err(ConfigError::ValidationError("WORKER_SK_HEX must be 64 characters".to_string()));
        }
        
        if !self.aggregator_url.starts_with("http") {
            return Err(ConfigError::ValidationError("AGGREGATOR_URL must be a valid HTTP URL".to_string()));
        }
        
        if self.autotune_target_ms == 0 {
            return Err(ConfigError::ValidationError("AUTOTUNE_TARGET_MS must be greater than 0".to_string()));
        }
        
        Ok(())
    }
    
    pub fn get_retry_delay(&self) -> Duration {
        Duration::from_millis(self.retry_delay_ms)
    }
    
    pub fn get_health_check_interval(&self) -> Duration {
        Duration::from_millis(self.health_check_interval_ms)
    }
}
