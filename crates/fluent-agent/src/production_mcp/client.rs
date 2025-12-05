// MCP client implementation (Development Stage)
//
// ⚠️  DEVELOPMENT STATUS: This client implementation provides core MCP functionality
// but should be thoroughly tested in your environment before production use.

use super::config::ClientConfig;
use super::error::McpError;
use super::health::{HealthMonitor, HealthStatus};
use super::metrics::{ClientMetrics, MetricsCollector};
use anyhow::Result;
use rmcp::{model::*, service::RunningService, RoleClient, ServiceExt};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, instrument, warn as tracing_warn};
use uuid::Uuid;

use crate::tools::validation;

/// Health check timeout for production MCP client
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// MCP client manager (Development Stage)
///
/// ⚠️  DEVELOPMENT STATUS: This client manager provides core functionality
/// but requires thorough testing before production deployment.
pub struct ProductionMcpClientManager {
    clients: Arc<RwLock<HashMap<String, Arc<ProductionMcpClient>>>>,
    config: ClientConfig,
    metrics_collector: Arc<MetricsCollector>,
    health_monitor: Arc<HealthMonitor>,
    #[allow(dead_code)]
    connection_pool: Arc<ConnectionPool>,
    #[allow(dead_code)]
    error_recovery: Arc<ErrorRecoveryManager>,
}

impl ProductionMcpClientManager {
    /// Create a new production MCP client manager
    pub async fn new(
        config: ClientConfig,
        metrics_collector: Arc<MetricsCollector>,
        health_monitor: Arc<HealthMonitor>,
    ) -> Result<Self> {
        let connection_pool = Arc::new(ConnectionPool::new(config.connection_pool_size));
        let error_recovery = Arc::new(ErrorRecoveryManager::new());

        Ok(Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics_collector,
            health_monitor,
            connection_pool,
            error_recovery,
        })
    }

    /// Initialize client connections
    pub async fn initialize(&self) -> Result<(), McpError> {
        // Start background tasks
        self.start_health_monitoring().await?;
        self.start_connection_maintenance().await?;
        Ok(())
    }

    /// Connect to an MCP server
    pub async fn connect_server(
        &self,
        name: String,
        command: String,
        args: Vec<String>,
    ) -> Result<(), McpError> {
        let mut clients = self.clients.write().await;

        if clients.contains_key(&name) {
            return Err(McpError::configuration(
                "server_name",
                format!("Server '{}' already connected", name),
            ));
        }

        let client = ProductionMcpClient::new(
            name.clone(),
            command,
            args,
            self.config.clone(),
            self.metrics_collector.clone(),
        )
        .await?;

        client.connect().await?;
        clients.insert(name, Arc::new(client));

        Ok(())
    }

    /// Disconnect from an MCP server
    pub async fn disconnect_server(&self, name: &str) -> Result<(), McpError> {
        let mut clients = self.clients.write().await;

        if let Some(client) = clients.remove(name) {
            client.disconnect().await?;
        }

        Ok(())
    }

    /// Execute a tool with automatic failover
    pub async fn execute_tool_with_failover(
        &self,
        tool_name: &str,
        parameters: Value,
        preferences: ExecutionPreferences,
    ) -> Result<CallToolResult, McpError> {
        let clients = self.clients.read().await;
        let available_servers = self.find_servers_with_tool(&clients, tool_name).await;

        if available_servers.is_empty() {
            return Err(McpError::tool_execution(
                tool_name,
                "No servers available with this tool",
                None,
            ));
        }

        // Try servers in order of preference
        for server_name in available_servers {
            if let Some(client) = clients.get(&server_name) {
                match client.execute_tool(tool_name, parameters.clone()).await {
                    Ok(result) => {
                        self.metrics_collector
                            .record_tool_execution_success(tool_name, &server_name)
                            .await;
                        return Ok(result);
                    }
                    Err(error) => {
                        self.metrics_collector
                            .record_tool_execution_failure(tool_name, &server_name, &error)
                            .await;

                        // Check if we should try the next server
                        if !error.is_recoverable() || preferences.fail_fast {
                            return Err(error);
                        }
                    }
                }
            }
        }

        Err(McpError::tool_execution(
            tool_name,
            "All servers failed to execute tool",
            None,
        ))
    }

    /// Get all available tools across all servers
    pub async fn get_all_tools(&self) -> HashMap<String, Vec<Tool>> {
        let clients = self.clients.read().await;
        let mut all_tools = HashMap::new();

        for (server_name, client) in clients.iter() {
            if let Ok(tools) = client.get_tools().await {
                all_tools.insert(server_name.clone(), tools);
            }
        }

        all_tools
    }

    /// Get client metrics
    pub async fn get_metrics(&self) -> ClientMetrics {
        self.metrics_collector.get_client_metrics().await
    }

    /// Handle configuration changes
    pub async fn on_config_change(&self) -> Result<(), McpError> {
        // Update client configurations
        let clients = self.clients.read().await;
        for client in clients.values() {
            client.update_config(self.config.clone()).await?;
        }
        Ok(())
    }

    /// Shutdown all clients
    pub async fn shutdown(&self) -> Result<(), McpError> {
        let mut clients = self.clients.write().await;

        for (_, client) in clients.drain() {
            if let Err(e) = client.disconnect().await {
                tracing::warn!("Error disconnecting client: {}", e);
            }
        }

        Ok(())
    }

    /// Find servers that have a specific tool
    async fn find_servers_with_tool(
        &self,
        clients: &HashMap<String, Arc<ProductionMcpClient>>,
        tool_name: &str,
    ) -> Vec<String> {
        let mut servers = Vec::new();

        for (server_name, client) in clients {
            if client.has_tool(tool_name).await {
                servers.push(server_name.clone());
            }
        }

        // Sort by server health and performance metrics
        servers.sort_by(|a, b| {
            // Implementation would sort by health score, latency, etc.
            a.cmp(b)
        });

        servers
    }

    /// Start health monitoring background task
    async fn start_health_monitoring(&self) -> Result<(), McpError> {
        let clients = self.clients.clone();
        let health_monitor = self.health_monitor.clone();
        let check_interval = self.config.health_check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;
                let clients_guard = clients.read().await;

                for (name, client) in clients_guard.iter() {
                    let health_status = client.check_health().await;
                    health_monitor
                        .update_client_health(name, health_status)
                        .await;
                }
            }
        });

        Ok(())
    }

    /// Start connection maintenance background task
    async fn start_connection_maintenance(&self) -> Result<(), McpError> {
        let clients = self.clients.clone();
        let maintenance_interval = Duration::from_secs(60);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(maintenance_interval);
            loop {
                interval.tick().await;
                let clients_guard = clients.read().await;

                for client in clients_guard.values() {
                    if let Err(e) = client.maintain_connection().await {
                        tracing::warn!("Connection maintenance failed: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

/// Individual production MCP client
pub struct ProductionMcpClient {
    name: String,
    service: Arc<Mutex<Option<RunningService<RoleClient, ()>>>>,
    command: String,
    args: Vec<String>,
    #[allow(dead_code)]
    config: ClientConfig,
    #[allow(dead_code)]
    metrics: Arc<ClientMetrics>,
    #[allow(dead_code)]
    last_health_check: Arc<RwLock<Instant>>,
    connection_status: Arc<RwLock<ConnectionStatus>>,
    tools_cache: Arc<RwLock<Option<Vec<Tool>>>>,
}

impl ProductionMcpClient {
    /// Create a new production MCP client
    pub async fn new(
        name: String,
        command: String,
        args: Vec<String>,
        config: ClientConfig,
        _metrics_collector: Arc<MetricsCollector>,
    ) -> Result<Self> {
        let metrics = Arc::new(ClientMetrics::new());

        Ok(Self {
            name,
            service: Arc::new(Mutex::new(None)),
            command,
            args,
            config,
            metrics,
            last_health_check: Arc::new(RwLock::new(Instant::now())),
            connection_status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            tools_cache: Arc::new(RwLock::new(None)),
        })
    }

    /// Connect to the MCP server
    #[instrument(skip(self), fields(name = %self.name, command = %self.command))]
    pub async fn connect(&self) -> Result<(), McpError> {
        use rmcp::transport::TokioChildProcess;
        use tokio::process::Command;

        let request_id = Uuid::new_v4();
        info!(request_id = %request_id, name = %self.name, "Starting MCP server connection");

        // Validate command before execution to prevent arbitrary command execution
        let allowed_commands = vec![
            "npx".to_string(),
            "node".to_string(),
            "python".to_string(),
            "python3".to_string(),
            "deno".to_string(),
            "bun".to_string(),
        ];

        validation::validate_command(&self.command, &allowed_commands).map_err(|e| {
            error!(request_id = %request_id, error = %e, "Command validation failed");
            McpError::configuration(
                "command",
                format!("MCP server command validation failed: {}", e),
            )
        })?;

        // Validate arguments for dangerous patterns
        for arg in &self.args {
            // Check for shell injection patterns in arguments
            if arg.contains("$(")
                || arg.contains("`")
                || arg.contains(";")
                || arg.contains("&&")
                || arg.contains("||")
                || arg.contains("|")
                || arg.contains(">")
                || arg.contains("<")
            {
                error!(request_id = %request_id, arg = %arg, "Dangerous shell pattern detected");
                return Err(McpError::configuration(
                    "args",
                    format!(
                        "MCP server argument contains dangerous shell pattern: '{}'",
                        arg
                    ),
                ));
            }

            // Check for null bytes and dangerous control characters
            if arg.contains('\0')
                || arg
                    .chars()
                    .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
            {
                error!(request_id = %request_id, arg = %arg, "Invalid control characters detected");
                return Err(McpError::configuration(
                    "args",
                    format!(
                        "MCP server argument contains invalid control characters: '{}'",
                        arg
                    ),
                ));
            }
        }

        let mut cmd = Command::new(&self.command);
        for arg in &self.args {
            cmd.arg(arg);
        }

        let transport = TokioChildProcess::new(cmd).map_err(|e| {
            error!(request_id = %request_id, error = %e, "Failed to create transport");
            McpError::transport("stdio", e.to_string(), true)
        })?;

        let service = ().serve(transport).await.map_err(|e| {
            error!(request_id = %request_id, error = %e, "Failed to serve transport");
            McpError::connection(&self.name, e.to_string(), 0)
        })?;

        *self.service.lock().await = Some(service);
        *self.connection_status.write().await = ConnectionStatus::Connected;

        info!(request_id = %request_id, name = %self.name, "MCP server connected");

        // Cache tools
        self.refresh_tools_cache().await?;

        // Perform health check
        let health_status = self.perform_health_check().await;
        match health_status {
            HealthStatus::Healthy => {
                info!(request_id = %request_id, name = %self.name, "MCP server health check passed");
                Ok(())
            }
            _ => {
                error!(request_id = %request_id, name = %self.name, status = ?health_status, "MCP server health check failed");
                *self.connection_status.write().await =
                    ConnectionStatus::Error("Health check failed".to_string());
                Err(McpError::connection(
                    &self.name,
                    "Health check failed after connection".to_string(),
                    0,
                ))
            }
        }
    }

    /// Disconnect from the MCP server
    #[instrument(skip(self), fields(name = %self.name))]
    pub async fn disconnect(&self) -> Result<(), McpError> {
        let request_id = Uuid::new_v4();
        info!(request_id = %request_id, name = %self.name, "Disconnecting from MCP server");

        if let Some(_service) = self.service.lock().await.take() {
            // Note: RoleClient doesn't have a cancel method in rmcp 0.2.1
            // The service will be dropped and cleaned up automatically
        }
        *self.connection_status.write().await = ConnectionStatus::Disconnected;

        info!(request_id = %request_id, name = %self.name, "MCP server disconnected successfully");
        Ok(())
    }

    /// Execute a tool
    #[instrument(skip(self, parameters), fields(name = %self.name, tool = %tool_name))]
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: Value,
    ) -> Result<CallToolResult, McpError> {
        let request_id = Uuid::new_v4();
        info!(request_id = %request_id, name = %self.name, tool = %tool_name, "Executing MCP tool");

        let service_guard = self.service.lock().await;
        let service = service_guard.as_ref().ok_or_else(|| {
            error!(request_id = %request_id, name = %self.name, "Not connected to MCP server");
            McpError::connection(&self.name, "Not connected".to_string(), 0)
        })?;

        let request = CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments: parameters.as_object().cloned(),
        };

        let result = service
            .call_tool(request)
            .await
            .map_err(|e| {
                error!(request_id = %request_id, name = %self.name, tool = %tool_name, error = %e, "MCP tool execution failed");
                McpError::tool_execution(tool_name, e.to_string(), None)
            })?;

        info!(request_id = %request_id, name = %self.name, tool = %tool_name, "MCP tool execution succeeded");
        Ok(result)
    }

    /// Get available tools
    pub async fn get_tools(&self) -> Result<Vec<Tool>, McpError> {
        if let Some(tools) = self.tools_cache.read().await.as_ref() {
            return Ok(tools.clone());
        }

        self.refresh_tools_cache().await
    }

    /// Check if client has a specific tool
    pub async fn has_tool(&self, tool_name: &str) -> bool {
        if let Ok(tools) = self.get_tools().await {
            tools.iter().any(|tool| tool.name == tool_name)
        } else {
            false
        }
    }

    /// Check client health
    pub async fn check_health(&self) -> HealthStatus {
        let status = self.connection_status.read().await;
        match *status {
            ConnectionStatus::Connected => {
                // Additional health checks could be performed here
                HealthStatus::Healthy
            }
            ConnectionStatus::Disconnected => HealthStatus::Unhealthy,
            ConnectionStatus::Connecting => HealthStatus::Degraded,
            ConnectionStatus::Error(_) => HealthStatus::Unhealthy,
        }
    }

    /// Maintain connection
    pub async fn maintain_connection(&self) -> Result<(), McpError> {
        // Perform connection maintenance tasks
        // This could include keepalive pings, reconnection logic, etc.
        Ok(())
    }

    /// Update client configuration
    pub async fn update_config(&self, _new_config: ClientConfig) -> Result<(), McpError> {
        // Update configuration and apply changes
        Ok(())
    }

    /// Perform health check on the MCP server
    #[instrument(skip(self), fields(name = %self.name))]
    pub async fn perform_health_check(&self) -> HealthStatus {
        let request_id = Uuid::new_v4();
        info!(request_id = %request_id, name = %self.name, "Performing health check");

        // Check connection status
        let status = self.connection_status.read().await;
        if !matches!(*status, ConnectionStatus::Connected) {
            tracing_warn!(request_id = %request_id, name = %self.name, "Health check failed: not connected");
            return HealthStatus::Unhealthy;
        }
        drop(status);

        // Try to list tools as a health check
        let service_guard = match tokio::time::timeout(HEALTH_CHECK_TIMEOUT, self.service.lock())
            .await
        {
            Ok(guard) => guard,
            Err(_) => {
                error!(request_id = %request_id, name = %self.name, "Health check timed out acquiring lock");
                return HealthStatus::Degraded;
            }
        };

        let service = match service_guard.as_ref() {
            Some(s) => s,
            None => {
                error!(request_id = %request_id, name = %self.name, "Health check failed: no service");
                return HealthStatus::Unhealthy;
            }
        };

        // Perform simple tool list operation as health check
        let health_result =
            tokio::time::timeout(HEALTH_CHECK_TIMEOUT, service.list_tools(Default::default()))
                .await;

        match health_result {
            Ok(Ok(_)) => {
                info!(request_id = %request_id, name = %self.name, "Health check passed");
                HealthStatus::Healthy
            }
            Ok(Err(e)) => {
                error!(request_id = %request_id, name = %self.name, error = %e, "Health check failed with error");
                HealthStatus::Unhealthy
            }
            Err(_) => {
                error!(request_id = %request_id, name = %self.name, timeout = ?HEALTH_CHECK_TIMEOUT, "Health check timed out");
                HealthStatus::Degraded
            }
        }
    }

    /// Refresh tools cache
    async fn refresh_tools_cache(&self) -> Result<Vec<Tool>, McpError> {
        let service_guard = self.service.lock().await;
        let service = service_guard
            .as_ref()
            .ok_or_else(|| McpError::connection(&self.name, "Not connected".to_string(), 0))?;

        let tools_result = service
            .list_tools(Default::default())
            .await
            .map_err(|e| McpError::protocol(-1, e.to_string()))?;

        let tools = tools_result.tools;
        *self.tools_cache.write().await = Some(tools.clone());

        Ok(tools)
    }
}

/// Connection status enumeration
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Tool execution preferences
#[derive(Debug, Clone)]
pub struct ExecutionPreferences {
    pub fail_fast: bool,
    pub preferred_servers: Vec<String>,
    pub timeout: Option<Duration>,
}

impl Default for ExecutionPreferences {
    fn default() -> Self {
        Self {
            fail_fast: false,
            preferred_servers: Vec::new(),
            timeout: Some(Duration::from_secs(30)),
        }
    }
}

/// Connection pool for managing client connections
pub struct ConnectionPool {
    #[allow(dead_code)]
    max_size: usize,
    // Implementation details would go here
}

impl ConnectionPool {
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }
}

/// Error recovery manager
pub struct ErrorRecoveryManager {
    // Implementation details would go here
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_manager_creation() {
        let config = ClientConfig::default();
        let metrics = Arc::new(MetricsCollector::new());
        let health = Arc::new(HealthMonitor::new());

        let manager = ProductionMcpClientManager::new(config, metrics, health).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_execution_preferences() {
        let prefs = ExecutionPreferences::default();
        assert!(!prefs.fail_fast);
        assert!(prefs.preferred_servers.is_empty());
        assert_eq!(prefs.timeout, Some(Duration::from_secs(30)));
    }
}
