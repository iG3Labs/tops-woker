use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::health::{HealthChecker, HealthResponse, MetricsResponse};
use crate::config::Config;
use crate::prometheus_metrics::{PrometheusMetrics, get_metric_help_text};
use serde_json;

pub struct HealthServer {
    health_checker: Arc<HealthChecker>,
    prometheus_metrics: Arc<PrometheusMetrics>,
    port: u16,
}

impl HealthServer {
    pub fn new(health_checker: Arc<HealthChecker>, prometheus_metrics: Arc<PrometheusMetrics>, port: u16) -> Self {
        Self {
            health_checker,
            prometheus_metrics,
            port,
        }
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port)).await?;
        println!("Health server listening on port {}", self.port);
        
        loop {
            let (mut socket, _) = listener.accept().await?;
            let health_checker = Arc::clone(&self.health_checker);
            let prometheus_metrics = Arc::clone(&self.prometheus_metrics);
            
            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                let n = match socket.read(&mut buffer).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(_) => return,
                };
                
                let request = String::from_utf8_lossy(&buffer[..n]);
                let response = Self::handle_request(&request, &health_checker, &prometheus_metrics).await;
                
                if let Err(_) = socket.write_all(response.as_bytes()).await {
                    return;
                }
            });
        }
    }
    
    async fn handle_request(request: &str, health_checker: &HealthChecker, prometheus_metrics: &PrometheusMetrics) -> String {
        let lines: Vec<&str> = request.lines().collect();
        if lines.is_empty() {
            return Self::error_response(400, "Bad Request");
        }
        
        let request_line = lines[0];
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        
        if parts.len() < 2 {
            return Self::error_response(400, "Bad Request");
        }
        
        let method = parts[0];
        let path = parts[1];
        
        match (method, path) {
            ("GET", "/health") => {
                let health = health_checker.get_health();
                match serde_json::to_string(&health) {
                    Ok(json) => Self::json_response(200, &json),
                    Err(_) => Self::error_response(500, "Internal Server Error"),
                }
            }
            ("GET", "/metrics") => {
                let metrics = health_checker.get_metrics();
                match serde_json::to_string(&metrics) {
                    Ok(json) => Self::json_response(200, &json),
                    Err(_) => Self::error_response(500, "Internal Server Error"),
                }
            }
            ("GET", "/prometheus") => {
                // Update Prometheus metrics from current metrics
                let current_metrics = health_checker.get_metrics();
                prometheus_metrics.update_from_metrics(&current_metrics.metrics);
                
                match prometheus_metrics.export_metrics() {
                    Ok(metrics_text) => Self::text_response(200, &metrics_text),
                    Err(_) => Self::error_response(500, "Internal Server Error"),
                }
            }
            ("GET", "/status") => {
                let status = health_checker.get_detailed_status();
                match serde_json::to_string(&status) {
                    Ok(json) => Self::json_response(200, &json),
                    Err(_) => Self::error_response(500, "Internal Server Error"),
                }
            }
            ("GET", "/") => {
                let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>tops-worker Health</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .endpoint { margin: 20px 0; padding: 10px; background: #f5f5f5; }
        .endpoint h3 { margin: 0 0 10px 0; }
        .endpoint a { color: #0066cc; text-decoration: none; }
        .endpoint a:hover { text-decoration: underline; }
        .prometheus { background: #e8f4f8; border-left: 4px solid #0066cc; }
    </style>
</head>
<body>
    <h1>tops-worker Health Endpoints</h1>
    <div class="endpoint">
        <h3><a href="/health">/health</a></h3>
        <p>Basic health status and uptime information</p>
    </div>
    <div class="endpoint">
        <h3><a href="/metrics">/metrics</a></h3>
        <p>Detailed performance metrics and statistics (JSON)</p>
    </div>
    <div class="endpoint prometheus">
        <h3><a href="/prometheus">/prometheus</a></h3>
        <p>Prometheus-formatted metrics for monitoring systems</p>
    </div>
    <div class="endpoint">
        <h3><a href="/status">/status</a></h3>
        <p>Comprehensive status including configuration and error counts</p>
    </div>
</body>
</html>
                "#;
                Self::html_response(200, html)
            }
            _ => Self::error_response(404, "Not Found"),
        }
    }
    
    fn json_response(status: u16, body: &str) -> String {
        format!(
            "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            status,
            body.len(),
            body
        )
    }
    
    fn text_response(status: u16, body: &str) -> String {
        format!(
            "HTTP/1.1 {} OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            status,
            body.len(),
            body
        )
    }
    
    fn html_response(status: u16, body: &str) -> String {
        format!(
            "HTTP/1.1 {} OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            status,
            body.len(),
            body
        )
    }
    
    fn error_response(status: u16, message: &str) -> String {
        let body = format!("{{\"error\": \"{}\"}}", message);
        Self::json_response(status, &body)
    }
}
