// Configuration management for production MCP implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use anyhow::Result;
use tokio::sync::RwLock;
use std::sync::Arc;

/// Comprehensive MCP configuration for production use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionMcpConfig {
    pub client: ClientConfig,
    pub server: ServerConfig,
    pub transport: TransportConfig,
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
}

impl Default for ProductionMcpConfig {
    fn default() -> Self {
        Self {
            client: ClientConfig::default(),
            server: ServerConfig::default(),
            transport: TransportConfig::default(),
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub default_timeout: Duration,
    pub max_concurrent_connections: usize,
    pub retry_policy: RetryPolicy,
    pub health_check_interval: Duration,
    pub connection_pool_size: usize,
    pub tool_execution_timeout: Duration,
    pub resource_cache_ttl: Duration,
    pub enable_failover: bool,
    pub preferred_servers: Vec<String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            max_concurrent_connections: 10,
            retry_policy: RetryPolicy::default(),
            health_check_interval: Duration::from_secs(30),
            connection_pool_size: 5,
            tool_execution_timeout: Duration::from_secs(60),
            resource_cache_ttl: Duration::from_secs(300),
            enable_failover: true,
            preferred_servers: Vec::new(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub max_connections: usize,
    pub tool_execution_timeout: Duration,
    pub resource_cache_size: usize,
    pub enable_metrics: bool,
    pub enable_health_checks: bool,
    pub request_rate_limit: Option<RateLimit>,
    pub tool_sandboxing: SandboxConfig,
    pub memory_limits: MemoryLimits,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:0".to_string(),
            max_connections: 100,
            tool_execution_timeout: Duration::from_secs(300),
            resource_cache_size: 1000,
            enable_metrics: true,
            enable_health_checks: true,
            request_rate_limit: Some(RateLimit::default()),
            tool_sandboxing: SandboxConfig::default(),
            memory_limits: MemoryLimits::default(),
        }
    }
}

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub stdio: StdioConfig,
    pub http: HttpConfig,
    pub websocket: WebSocketConfig,
    pub default_transport: TransportType,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            stdio: StdioConfig::default(),
            http: HttpConfig::default(),
            websocket: WebSocketConfig::default(),
            default_transport: TransportType::Stdio,
        }
    }
}

/// Transport type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportType {
    Stdio,
    Http,
    WebSocket,
}

/// STDIO transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioConfig {
    pub buffer_size: usize,
    pub process_timeout: Duration,
    pub restart_on_failure: bool,
    pub max_restart_attempts: u32,
    pub restart_delay: Duration,
}

impl Default for StdioConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            process_timeout: Duration::from_secs(300),
            restart_on_failure: true,
            max_restart_attempts: 3,
            restart_delay: Duration::from_secs(5),
        }
    }
}

/// HTTP transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub base_url: Option<String>,
    pub headers: HashMap<String, String>,
    pub auth: Option<AuthConfig>,
    pub connection_pool_size: usize,
    pub keep_alive: bool,
    pub compression: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            base_url: None,
            headers: HashMap::new(),
            auth: None,
            connection_pool_size: 10,
            keep_alive: true,
            compression: true,
        }
    }
}

/// WebSocket transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: Option<String>,
    pub headers: HashMap<String, String>,
    pub auth: Option<AuthConfig>,
    pub ping_interval: Duration,
    pub reconnect_attempts: u32,
    pub reconnect_delay: Duration,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: None,
            headers: HashMap::new(),
            auth: None,
            ping_interval: Duration::from_secs(30),
            reconnect_attempts: 5,
            reconnect_delay: Duration::from_secs(5),
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub credentials: HashMap<String, String>,
    pub token_refresh_threshold: Option<Duration>,
}

/// Authentication type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    None,
    Bearer,
    ApiKey,
    Basic,
    OAuth2,
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub window_size: Duration,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_second: 100,
            burst_size: 10,
            window_size: Duration::from_secs(1),
        }
    }
}

/// Sandbox configuration for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub blocked_paths: Vec<String>,
    pub network_access: bool,
    pub max_execution_time: Duration,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_commands: vec![
                "ls".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "find".to_string(),
            ],
            blocked_commands: vec![
                "rm".to_string(),
                "sudo".to_string(),
                "chmod".to_string(),
            ],
            allowed_paths: vec!["/tmp".to_string(), "/var/tmp".to_string()],
            blocked_paths: vec!["/etc".to_string(), "/root".to_string()],
            network_access: false,
            max_execution_time: Duration::from_secs(60),
        }
    }
}

/// Memory limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    pub max_memory_mb: usize,
    pub max_cache_size_mb: usize,
    pub gc_threshold_mb: usize,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,
            max_cache_size_mb: 128,
            gc_threshold_mb: 256,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub metrics_port: u16,
    pub health_check_port: u16,
    pub prometheus_enabled: bool,
    pub log_level: String,
    pub trace_sampling_rate: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_port: 9090,
            health_check_port: 8080,
            prometheus_enabled: true,
            log_level: "info".to_string(),
            trace_sampling_rate: 0.1,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub tls_enabled: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
    pub ca_path: Option<String>,
    pub verify_certificates: bool,
    pub allowed_origins: Vec<String>,
    pub api_key_header: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            tls_enabled: false,
            cert_path: None,
            key_path: None,
            ca_path: None,
            verify_certificates: true,
            allowed_origins: vec!["*".to_string()],
            api_key_header: "X-API-Key".to_string(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub output: LogOutput,
    pub file_path: Option<String>,
    pub max_file_size_mb: usize,
    pub max_files: usize,
    pub structured_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            output: LogOutput::Stdout,
            file_path: None,
            max_file_size_mb: 100,
            max_files: 10,
            structured_logging: true,
        }
    }
}

/// Log format enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Compact,
}

/// Log output enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    Stdout,
    Stderr,
    File,
    Both,
}

/// Configuration manager for handling configuration loading, validation, and updates
pub struct ConfigManager {
    config: Arc<RwLock<ProductionMcpConfig>>,
    validators: Vec<Box<dyn ConfigValidator + Send + Sync>>,
    watchers: Vec<Box<dyn ConfigWatcher + Send + Sync>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new(config: ProductionMcpConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            validators: Vec::new(),
            watchers: Vec::new(),
        }
    }

    /// Load configuration from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<ProductionMcpConfig> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: ProductionMcpConfig = if content.trim_start().starts_with('{') {
            serde_json::from_str(&content)?
        } else {
            toml::from_str(&content)?
        };
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file<P: AsRef<Path>>(
        config: &ProductionMcpConfig,
        path: P,
    ) -> Result<()> {
        let content = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::to_string_pretty(config)?
        } else {
            toml::to_string_pretty(config)?
        };
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> ProductionMcpConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, new_config: ProductionMcpConfig) -> Result<()> {
        // Validate configuration
        self.validate_config(&new_config).await?;

        // Update configuration
        *self.config.write().await = new_config.clone();

        // Notify watchers
        for watcher in &self.watchers {
            watcher.on_config_change(&new_config).await;
        }

        Ok(())
    }

    /// Validate configuration
    pub async fn validate_config(&self, config: &ProductionMcpConfig) -> Result<()> {
        for validator in &self.validators {
            validator.validate(config).await?;
        }
        Ok(())
    }

    /// Add configuration validator
    pub fn add_validator(&mut self, validator: Box<dyn ConfigValidator + Send + Sync>) {
        self.validators.push(validator);
    }

    /// Add configuration watcher
    pub fn add_watcher(&mut self, watcher: Box<dyn ConfigWatcher + Send + Sync>) {
        self.watchers.push(watcher);
    }
}

/// Trait for configuration validation
#[async_trait::async_trait]
pub trait ConfigValidator {
    async fn validate(&self, config: &ProductionMcpConfig) -> Result<()>;
}

/// Trait for configuration change notifications
#[async_trait::async_trait]
pub trait ConfigWatcher {
    async fn on_config_change(&self, config: &ProductionMcpConfig);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProductionMcpConfig::default();
        assert_eq!(config.client.max_concurrent_connections, 10);
        assert_eq!(config.server.max_connections, 100);
        assert!(config.monitoring.enabled);
    }

    #[tokio::test]
    async fn test_config_serialization() {
        let config = ProductionMcpConfig::default();
        
        // Test JSON serialization
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProductionMcpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.client.max_concurrent_connections, deserialized.client.max_concurrent_connections);

        // Test TOML serialization
        let toml = toml::to_string(&config).unwrap();
        let deserialized: ProductionMcpConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config.server.max_connections, deserialized.server.max_connections);
    }

    #[tokio::test]
    async fn test_config_manager() {
        let config = ProductionMcpConfig::default();
        let manager = ConfigManager::new(config.clone());
        
        let retrieved_config = manager.get_config().await;
        assert_eq!(config.client.max_concurrent_connections, retrieved_config.client.max_concurrent_connections);
    }
}
