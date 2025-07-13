// Comprehensive metrics collection for production MCP implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use super::error::McpError;

/// Comprehensive metrics collection system
pub struct MetricsCollector {
    client_metrics: Arc<RwLock<ClientMetrics>>,
    server_metrics: Arc<RwLock<ServerMetrics>>,
    transport_metrics: Arc<RwLock<TransportMetrics>>,
    tool_metrics: Arc<RwLock<ToolMetrics>>,
    resource_metrics: Arc<RwLock<ResourceMetrics>>,
    system_metrics: Arc<RwLock<SystemMetrics>>,
    start_time: Instant,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            client_metrics: Arc::new(RwLock::new(ClientMetrics::new())),
            server_metrics: Arc::new(RwLock::new(ServerMetrics::new())),
            transport_metrics: Arc::new(RwLock::new(TransportMetrics::new())),
            tool_metrics: Arc::new(RwLock::new(ToolMetrics::new())),
            resource_metrics: Arc::new(RwLock::new(ResourceMetrics::new())),
            system_metrics: Arc::new(RwLock::new(SystemMetrics::new())),
            start_time: Instant::now(),
        }
    }

    /// Start metrics collection
    pub async fn start(&self) -> Result<(), McpError> {
        // Start background metrics collection tasks
        self.start_system_metrics_collection().await;
        Ok(())
    }

    /// Stop metrics collection
    pub async fn stop(&self) -> Result<(), McpError> {
        // Stop background tasks
        Ok(())
    }

    /// Get current comprehensive metrics
    pub async fn get_current_metrics(&self) -> McpMetrics {
        McpMetrics {
            client_metrics: self.client_metrics.read().await.clone(),
            server_metrics: self.server_metrics.read().await.clone(),
            transport_metrics: self.transport_metrics.read().await.clone(),
            tool_metrics: self.tool_metrics.read().await.clone(),
            resource_metrics: self.resource_metrics.read().await.clone(),
            system_metrics: self.system_metrics.read().await.clone(),
            uptime: self.start_time.elapsed(),
        }
    }

    /// Get client metrics
    pub async fn get_client_metrics(&self) -> ClientMetrics {
        self.client_metrics.read().await.clone()
    }

    /// Record tool execution success
    pub async fn record_tool_execution_success(&self, tool_name: &str, server_name: &str) {
        let mut metrics = self.tool_metrics.write().await;
        metrics.record_execution_success(tool_name, server_name);
    }

    /// Record tool execution failure
    pub async fn record_tool_execution_failure(
        &self,
        tool_name: &str,
        server_name: &str,
        error: &McpError,
    ) {
        let mut metrics = self.tool_metrics.write().await;
        metrics.record_execution_failure(tool_name, server_name, error);
    }

    /// Record client connection
    pub async fn record_client_connection(&self, server_name: &str) {
        let mut metrics = self.client_metrics.write().await;
        metrics.record_connection(server_name);
    }

    /// Record client disconnection
    pub async fn record_client_disconnection(&self, server_name: &str) {
        let mut metrics = self.client_metrics.write().await;
        metrics.record_disconnection(server_name);
    }

    /// Record request latency
    pub async fn record_request_latency(&self, operation: &str, latency: Duration) {
        let mut metrics = self.transport_metrics.write().await;
        metrics.record_latency(operation, latency);
    }

    /// Start system metrics collection background task
    async fn start_system_metrics_collection(&self) {
        let system_metrics = self.system_metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let mut metrics = system_metrics.write().await;
                metrics.update_system_metrics().await;
            }
        });
    }
}

/// Comprehensive MCP metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMetrics {
    pub client_metrics: ClientMetrics,
    pub server_metrics: ServerMetrics,
    pub transport_metrics: TransportMetrics,
    pub tool_metrics: ToolMetrics,
    pub resource_metrics: ResourceMetrics,
    pub system_metrics: SystemMetrics,
    pub uptime: Duration,
}

/// Client-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMetrics {
    pub connections_active: u64,
    pub connections_total: u64,
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub response_time_avg: Duration,
    pub response_time_p95: Duration,
    pub response_time_p99: Duration,
    pub tools_executed: u64,
    pub resources_accessed: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_errors: u64,
    pub timeout_errors: u64,
    pub server_connections: HashMap<String, ServerConnectionMetrics>,
}

impl ClientMetrics {
    pub fn new() -> Self {
        Self {
            connections_active: 0,
            connections_total: 0,
            requests_total: 0,
            requests_successful: 0,
            requests_failed: 0,
            response_time_avg: Duration::from_millis(0),
            response_time_p95: Duration::from_millis(0),
            response_time_p99: Duration::from_millis(0),
            tools_executed: 0,
            resources_accessed: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_errors: 0,
            timeout_errors: 0,
            server_connections: HashMap::new(),
        }
    }

    pub fn record_connection(&mut self, server_name: &str) {
        self.connections_active += 1;
        self.connections_total += 1;
        
        let server_metrics = self.server_connections
            .entry(server_name.to_string())
            .or_insert_with(ServerConnectionMetrics::new);
        server_metrics.connections_active += 1;
        server_metrics.connections_total += 1;
    }

    pub fn record_disconnection(&mut self, server_name: &str) {
        if self.connections_active > 0 {
            self.connections_active -= 1;
        }
        
        if let Some(server_metrics) = self.server_connections.get_mut(server_name) {
            if server_metrics.connections_active > 0 {
                server_metrics.connections_active -= 1;
            }
        }
    }
}

/// Server connection metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnectionMetrics {
    pub connections_active: u64,
    pub connections_total: u64,
    pub last_connected: Option<chrono::DateTime<chrono::Utc>>,
    pub last_disconnected: Option<chrono::DateTime<chrono::Utc>>,
    pub connection_duration_total: Duration,
}

impl ServerConnectionMetrics {
    pub fn new() -> Self {
        Self {
            connections_active: 0,
            connections_total: 0,
            last_connected: None,
            last_disconnected: None,
            connection_duration_total: Duration::from_secs(0),
        }
    }
}

/// Server-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub clients_connected: u64,
    pub clients_total: u64,
    pub tools_registered: u64,
    pub resources_available: u64,
    pub requests_processed: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub errors_total: u64,
    pub processing_time_avg: Duration,
    pub processing_time_p95: Duration,
    pub processing_time_p99: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

impl ServerMetrics {
    pub fn new() -> Self {
        Self {
            clients_connected: 0,
            clients_total: 0,
            tools_registered: 0,
            resources_available: 0,
            requests_processed: 0,
            requests_successful: 0,
            requests_failed: 0,
            errors_total: 0,
            processing_time_avg: Duration::from_millis(0),
            processing_time_p95: Duration::from_millis(0),
            processing_time_p99: Duration::from_millis(0),
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
        }
    }
}

/// Transport-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportMetrics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_attempts: u64,
    pub connection_failures: u64,
    pub reconnections: u64,
    pub latency_avg: Duration,
    pub latency_p95: Duration,
    pub latency_p99: Duration,
    pub transport_errors: HashMap<String, u64>,
    pub operation_latencies: HashMap<String, Vec<Duration>>,
}

impl TransportMetrics {
    pub fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_attempts: 0,
            connection_failures: 0,
            reconnections: 0,
            latency_avg: Duration::from_millis(0),
            latency_p95: Duration::from_millis(0),
            latency_p99: Duration::from_millis(0),
            transport_errors: HashMap::new(),
            operation_latencies: HashMap::new(),
        }
    }

    pub fn record_latency(&mut self, operation: &str, latency: Duration) {
        let latencies = self.operation_latencies
            .entry(operation.to_string())
            .or_insert_with(Vec::new);
        
        latencies.push(latency);
        
        // Keep only recent latencies (last 1000)
        if latencies.len() > 1000 {
            latencies.drain(0..latencies.len() - 1000);
        }
        
        // Update percentiles
        self.update_latency_percentiles();
    }

    fn update_latency_percentiles(&mut self) {
        let mut all_latencies: Vec<Duration> = self.operation_latencies
            .values()
            .flatten()
            .cloned()
            .collect();
        
        if all_latencies.is_empty() {
            return;
        }
        
        all_latencies.sort();
        
        let len = all_latencies.len();
        let total_nanos = all_latencies.iter().map(|d| d.as_nanos()).sum::<u128>();
        let avg_nanos = (total_nanos / len as u128).min(u64::MAX as u128) as u64;
        self.latency_avg = Duration::from_nanos(avg_nanos);
        
        if len > 0 {
            self.latency_p95 = all_latencies[(len * 95 / 100).min(len - 1)];
            self.latency_p99 = all_latencies[(len * 99 / 100).min(len - 1)];
        }
    }
}

/// Tool execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub tools_executed: u64,
    pub tools_successful: u64,
    pub tools_failed: u64,
    pub execution_time_avg: Duration,
    pub execution_time_p95: Duration,
    pub execution_time_p99: Duration,
    pub tool_usage: HashMap<String, ToolUsageMetrics>,
    pub server_tool_usage: HashMap<String, HashMap<String, ToolUsageMetrics>>,
}

impl ToolMetrics {
    pub fn new() -> Self {
        Self {
            tools_executed: 0,
            tools_successful: 0,
            tools_failed: 0,
            execution_time_avg: Duration::from_millis(0),
            execution_time_p95: Duration::from_millis(0),
            execution_time_p99: Duration::from_millis(0),
            tool_usage: HashMap::new(),
            server_tool_usage: HashMap::new(),
        }
    }

    pub fn record_execution_success(&mut self, tool_name: &str, server_name: &str) {
        self.tools_executed += 1;
        self.tools_successful += 1;
        
        let tool_metrics = self.tool_usage
            .entry(tool_name.to_string())
            .or_insert_with(ToolUsageMetrics::new);
        tool_metrics.executions += 1;
        tool_metrics.successes += 1;
        
        let server_tools = self.server_tool_usage
            .entry(server_name.to_string())
            .or_insert_with(HashMap::new);
        let server_tool_metrics = server_tools
            .entry(tool_name.to_string())
            .or_insert_with(ToolUsageMetrics::new);
        server_tool_metrics.executions += 1;
        server_tool_metrics.successes += 1;
    }

    pub fn record_execution_failure(&mut self, tool_name: &str, server_name: &str, _error: &McpError) {
        self.tools_executed += 1;
        self.tools_failed += 1;
        
        let tool_metrics = self.tool_usage
            .entry(tool_name.to_string())
            .or_insert_with(ToolUsageMetrics::new);
        tool_metrics.executions += 1;
        tool_metrics.failures += 1;
        
        let server_tools = self.server_tool_usage
            .entry(server_name.to_string())
            .or_insert_with(HashMap::new);
        let server_tool_metrics = server_tools
            .entry(tool_name.to_string())
            .or_insert_with(ToolUsageMetrics::new);
        server_tool_metrics.executions += 1;
        server_tool_metrics.failures += 1;
    }
}

/// Tool usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageMetrics {
    pub executions: u64,
    pub successes: u64,
    pub failures: u64,
    pub avg_execution_time: Duration,
    pub last_executed: Option<chrono::DateTime<chrono::Utc>>,
}

impl ToolUsageMetrics {
    pub fn new() -> Self {
        Self {
            executions: 0,
            successes: 0,
            failures: 0,
            avg_execution_time: Duration::from_millis(0),
            last_executed: None,
        }
    }
}

/// Resource access metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub resources_accessed: u64,
    pub resources_cached: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub resource_access: HashMap<String, ResourceAccessMetrics>,
}

impl ResourceMetrics {
    pub fn new() -> Self {
        Self {
            resources_accessed: 0,
            resources_cached: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_hit_rate: 0.0,
            resource_access: HashMap::new(),
        }
    }
}

/// Resource access metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAccessMetrics {
    pub access_count: u64,
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    pub avg_access_time: Duration,
}

/// System-level metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub disk_usage_mb: f64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub open_file_descriptors: u64,
    pub thread_count: u64,
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            disk_usage_mb: 0.0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            open_file_descriptors: 0,
            thread_count: 0,
        }
    }

    pub async fn update_system_metrics(&mut self) {
        // Update system metrics
        // This would integrate with system monitoring libraries
        // For now, we'll use placeholder values
        self.memory_usage_mb = 128.0; // Placeholder
        self.cpu_usage_percent = 5.0; // Placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        collector.start().await.unwrap();
        
        // Record some metrics
        collector.record_tool_execution_success("test_tool", "test_server").await;
        collector.record_client_connection("test_server").await;
        
        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.tool_metrics.tools_executed, 1);
        assert_eq!(metrics.client_metrics.connections_active, 1);
        
        collector.stop().await.unwrap();
    }

    #[test]
    fn test_client_metrics() {
        let mut metrics = ClientMetrics::new();
        metrics.record_connection("server1");
        assert_eq!(metrics.connections_active, 1);
        assert_eq!(metrics.connections_total, 1);
        
        metrics.record_disconnection("server1");
        assert_eq!(metrics.connections_active, 0);
        assert_eq!(metrics.connections_total, 1);
    }

    #[test]
    fn test_transport_metrics() {
        let mut metrics = TransportMetrics::new();
        metrics.record_latency("test_op", Duration::from_millis(100));
        metrics.record_latency("test_op", Duration::from_millis(200));
        
        assert!(!metrics.operation_latencies.is_empty());
        assert!(metrics.latency_avg > Duration::from_millis(0));
    }
}
