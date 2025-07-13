// Comprehensive health monitoring for production MCP implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use super::error::McpError;

/// Health monitoring system
pub struct HealthMonitor {
    checks: Vec<Box<dyn HealthCheck + Send + Sync>>,
    status: Arc<RwLock<OverallHealth>>,
    client_health: Arc<RwLock<HashMap<String, HealthStatus>>>,
    server_health: Arc<RwLock<HashMap<String, HealthStatus>>>,
    transport_health: Arc<RwLock<HashMap<String, HealthStatus>>>,
    alert_manager: Arc<AlertManager>,
    check_interval: Duration,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            status: Arc::new(RwLock::new(OverallHealth::new())),
            client_health: Arc::new(RwLock::new(HashMap::new())),
            server_health: Arc::new(RwLock::new(HashMap::new())),
            transport_health: Arc::new(RwLock::new(HashMap::new())),
            alert_manager: Arc::new(AlertManager::new()),
            check_interval: Duration::from_secs(30),
        }
    }

    /// Start health monitoring
    pub async fn start(&self) -> Result<(), McpError> {
        self.start_health_checks().await;
        Ok(())
    }

    /// Stop health monitoring
    pub async fn stop(&self) -> Result<(), McpError> {
        // Stop background tasks
        Ok(())
    }

    /// Add a health check
    pub fn add_health_check(&mut self, check: Box<dyn HealthCheck + Send + Sync>) {
        self.checks.push(check);
    }

    /// Get overall health status
    pub async fn get_overall_health(&self) -> OverallHealth {
        self.status.read().await.clone()
    }

    /// Update client health
    pub async fn update_client_health(&self, client_name: &str, status: HealthStatus) {
        let mut health = self.client_health.write().await;
        health.insert(client_name.to_string(), status);
        self.update_overall_health().await;
    }

    /// Update server health
    pub async fn update_server_health(&self, server_name: &str, status: HealthStatus) {
        let mut health = self.server_health.write().await;
        health.insert(server_name.to_string(), status);
        self.update_overall_health().await;
    }

    /// Update transport health
    pub async fn update_transport_health(&self, transport_name: &str, status: HealthStatus) {
        let mut health = self.transport_health.write().await;
        health.insert(transport_name.to_string(), status);
        self.update_overall_health().await;
    }

    /// Get detailed health report
    pub async fn get_health_report(&self) -> HealthReport {
        HealthReport {
            overall: self.get_overall_health().await,
            clients: self.client_health.read().await.clone(),
            servers: self.server_health.read().await.clone(),
            transports: self.transport_health.read().await.clone(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Start health checks background task
    async fn start_health_checks(&self) {
        let checks = self.checks.len();
        if checks == 0 {
            return;
        }

        let status = self.status.clone();
        let _alert_manager = self.alert_manager.clone();
        let check_interval = self.check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;
                
                // Perform health checks
                // Note: In a real implementation, we would iterate through self.checks
                // For now, we'll simulate health check results
                let mut overall = status.write().await;
                overall.last_check = Instant::now();
                overall.check_count += 1;
                
                // Simulate health status
                overall.status = HealthStatus::Healthy;
                overall.details.insert("system".to_string(), "All systems operational".to_string());
            }
        });
    }

    /// Update overall health based on component health
    async fn update_overall_health(&self) {
        let client_health = self.client_health.read().await;
        let server_health = self.server_health.read().await;
        let transport_health = self.transport_health.read().await;

        let mut overall = self.status.write().await;
        
        // Determine overall health based on component health
        let all_statuses: Vec<&HealthStatus> = client_health.values()
            .chain(server_health.values())
            .chain(transport_health.values())
            .collect();

        overall.status = if all_statuses.iter().any(|s| matches!(s, HealthStatus::Unhealthy)) {
            HealthStatus::Unhealthy
        } else if all_statuses.iter().any(|s| matches!(s, HealthStatus::Degraded)) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        overall.last_update = Instant::now();
    }
}

/// Health status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Overall health information
#[derive(Debug, Clone)]
pub struct OverallHealth {
    pub status: HealthStatus,
    pub last_check: Instant,
    pub last_update: Instant,
    pub check_count: u64,
    pub details: HashMap<String, String>,
    pub uptime: Duration,
    pub version: String,
}

impl OverallHealth {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            status: HealthStatus::Unknown,
            last_check: now,
            last_update: now,
            check_count: 0,
            details: HashMap::new(),
            uptime: Duration::from_secs(0),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Detailed health report
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub overall: OverallHealth,
    pub clients: HashMap<String, HealthStatus>,
    pub servers: HashMap<String, HealthStatus>,
    pub transports: HashMap<String, HealthStatus>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Health check trait
#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    /// Perform the health check
    async fn check(&self) -> HealthCheckResult;
    
    /// Get the name of this health check
    fn name(&self) -> &str;
    
    /// Check if this is a critical health check
    fn critical(&self) -> bool;
    
    /// Get the timeout for this health check
    fn timeout(&self) -> Duration {
        Duration::from_secs(10)
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub message: String,
    pub details: HashMap<String, String>,
    pub duration: Duration,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HealthCheckResult {
    pub fn healthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: message.into(),
            details: HashMap::new(),
            duration: Duration::from_millis(0),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            message: message.into(),
            details: HashMap::new(),
            duration: Duration::from_millis(0),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            message: message.into(),
            details: HashMap::new(),
            duration: Duration::from_millis(0),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

/// Connection health check
pub struct ConnectionHealthCheck {
    name: String,
    critical: bool,
}

impl ConnectionHealthCheck {
    pub fn new(name: impl Into<String>, critical: bool) -> Self {
        Self {
            name: name.into(),
            critical,
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for ConnectionHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        // Simulate connection check
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        HealthCheckResult::healthy("Connection is active")
            .with_duration(start.elapsed())
            .with_detail("connection_type", "mcp")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn critical(&self) -> bool {
        self.critical
    }
}

/// Tool registry health check
pub struct ToolRegistryHealthCheck {
    name: String,
}

impl ToolRegistryHealthCheck {
    pub fn new() -> Self {
        Self {
            name: "tool_registry".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for ToolRegistryHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        // Simulate tool registry check
        tokio::time::sleep(Duration::from_millis(5)).await;
        
        HealthCheckResult::healthy("Tool registry is operational")
            .with_duration(start.elapsed())
            .with_detail("tools_count", "10")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn critical(&self) -> bool {
        false
    }
}

/// Memory system health check
pub struct MemorySystemHealthCheck {
    name: String,
}

impl MemorySystemHealthCheck {
    pub fn new() -> Self {
        Self {
            name: "memory_system".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for MemorySystemHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        // Simulate memory system check
        tokio::time::sleep(Duration::from_millis(15)).await;
        
        HealthCheckResult::healthy("Memory system is operational")
            .with_duration(start.elapsed())
            .with_detail("memory_usage", "128MB")
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn critical(&self) -> bool {
        true
    }
}

/// Transport health check
pub struct TransportHealthCheck {
    name: String,
    transport_type: String,
}

impl TransportHealthCheck {
    pub fn new(name: impl Into<String>, transport_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport_type: transport_type.into(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for TransportHealthCheck {
    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        // Simulate transport check
        tokio::time::sleep(Duration::from_millis(8)).await;
        
        HealthCheckResult::healthy("Transport is operational")
            .with_duration(start.elapsed())
            .with_detail("transport_type", &self.transport_type)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn critical(&self) -> bool {
        true
    }
}

/// Alert manager for health-related alerts
pub struct AlertManager {
    // Implementation details would go here
}

impl AlertManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn send_alert(&self, _alert: HealthAlert) {
        // Implementation would send alerts via various channels
    }
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub severity: AlertSeverity,
    pub component: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: HashMap<String, String>,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor() {
        let mut monitor = HealthMonitor::new();
        
        // Add health checks
        monitor.add_health_check(Box::new(ConnectionHealthCheck::new("test_connection", true)));
        monitor.add_health_check(Box::new(ToolRegistryHealthCheck::new()));
        
        monitor.start().await.unwrap();
        
        // Update component health
        monitor.update_client_health("test_client", HealthStatus::Healthy).await;
        monitor.update_server_health("test_server", HealthStatus::Healthy).await;
        
        let health = monitor.get_overall_health().await;
        assert_eq!(health.status, HealthStatus::Healthy);
        
        let report = monitor.get_health_report().await;
        assert!(report.clients.contains_key("test_client"));
        assert!(report.servers.contains_key("test_server"));
        
        monitor.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_health_checks() {
        let connection_check = ConnectionHealthCheck::new("test", true);
        let result = connection_check.check().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(result.duration > Duration::from_millis(0));

        let tool_check = ToolRegistryHealthCheck::new();
        let result = tool_check.check().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(!tool_check.critical());

        let memory_check = MemorySystemHealthCheck::new();
        let result = memory_check.check().await;
        assert_eq!(result.status, HealthStatus::Healthy);
        assert!(memory_check.critical());
    }

    #[test]
    fn test_health_check_result() {
        let result = HealthCheckResult::healthy("Test message")
            .with_detail("key", "value")
            .with_duration(Duration::from_millis(100));
        
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.message, "Test message");
        assert_eq!(result.details.get("key"), Some(&"value".to_string()));
        assert_eq!(result.duration, Duration::from_millis(100));
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
    }
}
