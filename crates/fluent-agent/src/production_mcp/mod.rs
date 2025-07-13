// Production-ready MCP implementation for Fluent CLI
// This module provides a comprehensive, production-ready implementation of the Model Context Protocol

pub mod error;
pub mod client;
pub mod server;
pub mod transport;
pub mod config;
pub mod metrics;
pub mod health;
pub mod registry;

pub use error::*;
pub use client::*;
pub use server::*;
pub use transport::*;
pub use config::*;
pub use metrics::*;
pub use health::*;
pub use registry::*;

use anyhow::Result;
use std::sync::Arc;


/// Production MCP manager that coordinates all MCP functionality
pub struct ProductionMcpManager {
    client_manager: Arc<ProductionMcpClientManager>,
    server_manager: Arc<ProductionMcpServerManager>,
    config_manager: Arc<ConfigManager>,
    metrics_collector: Arc<MetricsCollector>,
    health_monitor: Arc<HealthMonitor>,
}

impl ProductionMcpManager {
    /// Create a new production MCP manager
    pub async fn new(config: ProductionMcpConfig) -> Result<Self> {
        let config_manager = Arc::new(ConfigManager::new(config.clone()));
        let metrics_collector = Arc::new(MetricsCollector::new());
        let health_monitor = Arc::new(HealthMonitor::new());

        let client_manager = Arc::new(
            ProductionMcpClientManager::new(
                config.client.clone(),
                metrics_collector.clone(),
                health_monitor.clone(),
            )
            .await?,
        );

        let server_manager = Arc::new(
            ProductionMcpServerManager::new(
                config.server.clone(),
                metrics_collector.clone(),
                health_monitor.clone(),
            )
            .await?,
        );

        Ok(Self {
            client_manager,
            server_manager,
            config_manager,
            metrics_collector,
            health_monitor,
        })
    }

    /// Start the MCP manager
    pub async fn start(&self) -> Result<()> {
        // Start health monitoring
        self.health_monitor.start().await?;

        // Start metrics collection
        self.metrics_collector.start().await?;

        // Start server manager
        self.server_manager.start().await?;

        // Initialize client connections
        self.client_manager.initialize().await?;

        Ok(())
    }

    /// Stop the MCP manager gracefully
    pub async fn stop(&self) -> Result<()> {
        // Stop client connections
        self.client_manager.shutdown().await?;

        // Stop server manager
        self.server_manager.stop().await?;

        // Stop metrics collection
        self.metrics_collector.stop().await?;

        // Stop health monitoring
        self.health_monitor.stop().await?;

        Ok(())
    }

    /// Get client manager
    pub fn client_manager(&self) -> Arc<ProductionMcpClientManager> {
        self.client_manager.clone()
    }

    /// Get server manager
    pub fn server_manager(&self) -> Arc<ProductionMcpServerManager> {
        self.server_manager.clone()
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> McpMetrics {
        self.metrics_collector.get_current_metrics().await
    }

    /// Get health status
    pub async fn get_health_status(&self) -> OverallHealth {
        self.health_monitor.get_overall_health().await
    }

    /// Reload configuration
    pub async fn reload_config(&self, new_config: ProductionMcpConfig) -> Result<()> {
        self.config_manager.update_config(new_config).await?;

        // Notify components of config change
        self.client_manager.on_config_change().await?;
        self.server_manager.on_config_change().await?;

        Ok(())
    }
}

/// Initialize production MCP with default configuration
pub async fn initialize_production_mcp() -> Result<Arc<ProductionMcpManager>> {
    let config = ProductionMcpConfig::default();
    let manager = ProductionMcpManager::new(config).await?;
    manager.start().await?;
    Ok(Arc::new(manager))
}

/// Initialize production MCP with custom configuration
pub async fn initialize_production_mcp_with_config(
    config: ProductionMcpConfig,
) -> Result<Arc<ProductionMcpManager>> {
    let manager = ProductionMcpManager::new(config).await?;
    manager.start().await?;
    Ok(Arc::new(manager))
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_production_mcp_manager_lifecycle() {
        let config = ProductionMcpConfig::default();
        let manager = ProductionMcpManager::new(config).await.unwrap();

        // Test start
        manager.start().await.unwrap();

        // Test health status
        let health = manager.get_health_status().await;
        assert!(matches!(health.status, HealthStatus::Healthy));

        // Test metrics
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.client_metrics.connections_active, 0);

        // Test stop
        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_config_reload() {
        let config = ProductionMcpConfig::default();
        let manager = ProductionMcpManager::new(config.clone()).await.unwrap();
        manager.start().await.unwrap();

        // Test config reload
        let mut new_config = config;
        new_config.client.max_concurrent_connections = 20;
        manager.reload_config(new_config).await.unwrap();

        manager.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_initialize_production_mcp() {
        let manager = initialize_production_mcp().await.unwrap();

        // Verify manager is running
        let health = manager.get_health_status().await;
        assert!(matches!(health.status, HealthStatus::Healthy));

        manager.stop().await.unwrap();
    }
}
