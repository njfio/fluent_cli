use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub mod stdio;
pub mod http;
pub mod websocket;

/// JSON-RPC 2.0 request structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 notification structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 error structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// Transport abstraction for MCP communication
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Send a request and wait for response
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse>;
    
    /// Start listening for notifications
    async fn start_listening(&self) -> Result<mpsc::UnboundedReceiver<JsonRpcNotification>>;
    
    /// Close the transport connection
    async fn close(&self) -> Result<()>;
    
    /// Check if the transport is connected
    async fn is_connected(&self) -> bool;
    
    /// Get transport-specific metadata
    fn get_metadata(&self) -> HashMap<String, String>;
}

/// Transport configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransportConfig {
    pub transport_type: TransportType,
    pub connection_config: ConnectionConfig,
    pub auth_config: Option<AuthConfig>,
    pub timeout_config: TimeoutConfig,
    pub retry_config: RetryConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransportType {
    Stdio,
    Http,
    WebSocket,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConnectionConfig {
    Stdio {
        command: String,
        args: Vec<String>,
    },
    Http {
        base_url: String,
        headers: HashMap<String, String>,
    },
    WebSocket {
        url: String,
        headers: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub credentials: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthType {
    None,
    Bearer,
    ApiKey,
    Basic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeoutConfig {
    pub connect_timeout_ms: u64,
    pub request_timeout_ms: u64,
    pub idle_timeout_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout_ms: 5000,
            request_timeout_ms: 30000,
            idle_timeout_ms: 300000,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_on_errors: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential {
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
            },
            retry_on_errors: vec![
                "connection_error".to_string(),
                "timeout".to_string(),
                "server_error".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BackoffStrategy {
    Fixed { delay_ms: u64 },
    Exponential { initial_delay_ms: u64, max_delay_ms: u64 },
    Linear { increment_ms: u64 },
}

/// Transport factory for creating transport instances
pub struct TransportFactory;

impl TransportFactory {
    pub async fn create_transport(config: TransportConfig) -> Result<Box<dyn McpTransport>> {
        match config.transport_type {
            TransportType::Stdio => {
                if let ConnectionConfig::Stdio { command, args } = config.connection_config {
                    let transport = stdio::StdioTransport::new(command, args, config.timeout_config, config.retry_config).await?;
                    Ok(Box::new(transport))
                } else {
                    Err(anyhow::anyhow!("Invalid connection config for STDIO transport"))
                }
            }
            TransportType::Http => {
                if let ConnectionConfig::Http { base_url, headers } = config.connection_config {
                    let transport = http::HttpTransport::new(
                        base_url,
                        headers,
                        config.auth_config,
                        config.timeout_config,
                        config.retry_config,
                    ).await?;
                    Ok(Box::new(transport))
                } else {
                    Err(anyhow::anyhow!("Invalid connection config for HTTP transport"))
                }
            }
            TransportType::WebSocket => {
                if let ConnectionConfig::WebSocket { url, headers } = config.connection_config {
                    let transport = websocket::WebSocketTransport::new(
                        url,
                        headers,
                        config.auth_config,
                        config.timeout_config,
                        config.retry_config,
                    ).await?;
                    Ok(Box::new(transport))
                } else {
                    Err(anyhow::anyhow!("Invalid connection config for WebSocket transport"))
                }
            }
        }
    }
}

/// Utility functions for transport implementations
pub mod utils {
    use super::*;
    use std::time::Duration;
    
    pub fn create_request_id() -> Value {
        serde_json::Value::String(uuid::Uuid::new_v4().to_string())
    }
    
    pub async fn retry_with_backoff<F, T, E>(
        mut operation: F,
        retry_config: &RetryConfig,
    ) -> Result<T>
    where
        F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
        E: std::fmt::Display,
    {
        let mut attempts = 0;
        let mut delay = match &retry_config.backoff_strategy {
            BackoffStrategy::Fixed { delay_ms } => *delay_ms,
            BackoffStrategy::Exponential { initial_delay_ms, .. } => *initial_delay_ms,
            BackoffStrategy::Linear { increment_ms } => *increment_ms,
        };
        
        loop {
            attempts += 1;
            
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempts >= retry_config.max_attempts {
                        return Err(anyhow::anyhow!("Max retry attempts exceeded: {}", error));
                    }
                    
                    // Calculate next delay
                    match &retry_config.backoff_strategy {
                        BackoffStrategy::Fixed { .. } => {
                            // delay stays the same
                        }
                        BackoffStrategy::Exponential { max_delay_ms, .. } => {
                            delay = (delay * 2).min(*max_delay_ms);
                        }
                        BackoffStrategy::Linear { increment_ms } => {
                            delay += increment_ms;
                        }
                    }
                    
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transport_config_serialization() {
        let config = TransportConfig {
            transport_type: TransportType::Http,
            connection_config: ConnectionConfig::Http {
                base_url: "https://api.example.com".to_string(),
                headers: HashMap::new(),
            },
            auth_config: Some(AuthConfig {
                auth_type: AuthType::Bearer,
                credentials: {
                    let mut creds = HashMap::new();
                    creds.insert("token".to_string(), "test-token".to_string());
                    creds
                },
            }),
            timeout_config: TimeoutConfig::default(),
            retry_config: RetryConfig::default(),
        };
        
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: TransportConfig = serde_json::from_str(&serialized).unwrap();
        
        assert!(matches!(deserialized.transport_type, TransportType::Http));
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff() {
        let retry_config = RetryConfig {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Fixed { delay_ms: 10 },
            retry_on_errors: vec!["test_error".to_string()],
        };
        
        let mut call_count = 0;
        let result = utils::retry_with_backoff(
            || {
                call_count += 1;
                Box::pin(async move {
                    if call_count < 3 {
                        Err("test_error")
                    } else {
                        Ok("success")
                    }
                })
            },
            &retry_config,
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(call_count, 3);
    }
}
