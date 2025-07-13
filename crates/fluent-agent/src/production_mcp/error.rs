// Comprehensive error handling for production MCP implementation

use thiserror::Error;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Comprehensive MCP error types for production use
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum McpError {
    #[error("Protocol error: {message} (code: {code})")]
    Protocol { code: i32, message: String },

    #[error("Transport error: {transport_type} - {message}")]
    Transport {
        transport_type: String,
        message: String,
        recoverable: bool,
    },

    #[error("Tool execution error: {tool_name} - {message}")]
    ToolExecution {
        tool_name: String,
        message: String,
        exit_code: Option<i32>,
    },

    #[error("Resource error: {resource_uri} - {message}")]
    Resource {
        resource_uri: String,
        message: String,
        resource_type: String,
    },

    #[error("Configuration error: {field} - {message}")]
    Configuration { field: String, message: String },

    #[error("Authentication error: {auth_type} - {message}")]
    Authentication { auth_type: String, message: String },

    #[error("Authorization error: {operation} - {message}")]
    Authorization { operation: String, message: String },

    #[error("Rate limit exceeded: {limit} requests per {window:?}, retry after {retry_after:?}")]
    RateLimit {
        limit: u32,
        window: Duration,
        retry_after: Option<Duration>,
    },

    #[error("Server unavailable: {server_name} - {reason}")]
    ServerUnavailable { server_name: String, reason: String },

    #[error("Connection error: {endpoint} - {message}")]
    Connection {
        endpoint: String,
        message: String,
        retry_count: u32,
    },

    #[error("Timeout error: {operation} - exceeded {timeout:?}")]
    Timeout { operation: String, timeout: Duration },

    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    #[error("Serialization error: {context} - {message}")]
    Serialization { context: String, message: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    #[error("Dependency error: {dependency} - {message}")]
    Dependency { dependency: String, message: String },

    #[error("Capacity error: {resource} - {message}")]
    Capacity { resource: String, message: String },

    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
}

impl McpError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            McpError::Transport { recoverable, .. } => *recoverable,
            McpError::Connection { .. } => true,
            McpError::Timeout { .. } => true,
            McpError::ServerUnavailable { .. } => true,
            McpError::RateLimit { .. } => true,
            McpError::Capacity { .. } => true,
            _ => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            McpError::Internal { .. } => ErrorSeverity::Critical,
            McpError::Configuration { .. } => ErrorSeverity::Critical,
            McpError::Authentication { .. } => ErrorSeverity::High,
            McpError::Authorization { .. } => ErrorSeverity::High,
            McpError::Protocol { .. } => ErrorSeverity::High,
            McpError::VersionMismatch { .. } => ErrorSeverity::High,
            McpError::Transport { .. } => ErrorSeverity::Medium,
            McpError::Connection { .. } => ErrorSeverity::Medium,
            McpError::ServerUnavailable { .. } => ErrorSeverity::Medium,
            McpError::ToolExecution { .. } => ErrorSeverity::Medium,
            McpError::Resource { .. } => ErrorSeverity::Medium,
            McpError::Timeout { .. } => ErrorSeverity::Low,
            McpError::RateLimit { .. } => ErrorSeverity::Low,
            McpError::Validation { .. } => ErrorSeverity::Low,
            McpError::Serialization { .. } => ErrorSeverity::Low,
            McpError::Dependency { .. } => ErrorSeverity::Medium,
            McpError::Capacity { .. } => ErrorSeverity::Medium,
        }
    }

    /// Get suggested retry delay
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            McpError::RateLimit { retry_after, .. } => *retry_after,
            McpError::Connection { retry_count, .. } => {
                Some(Duration::from_millis(1000 * (2_u64.pow(*retry_count))))
            }
            McpError::Timeout { .. } => Some(Duration::from_secs(5)),
            McpError::ServerUnavailable { .. } => Some(Duration::from_secs(30)),
            McpError::Transport { recoverable: true, .. } => Some(Duration::from_secs(10)),
            _ => None,
        }
    }

    /// Create a protocol error
    pub fn protocol(code: i32, message: impl Into<String>) -> Self {
        Self::Protocol {
            code,
            message: message.into(),
        }
    }

    /// Create a transport error
    pub fn transport(
        transport_type: impl Into<String>,
        message: impl Into<String>,
        recoverable: bool,
    ) -> Self {
        Self::Transport {
            transport_type: transport_type.into(),
            message: message.into(),
            recoverable,
        }
    }

    /// Create a tool execution error
    pub fn tool_execution(
        tool_name: impl Into<String>,
        message: impl Into<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self::ToolExecution {
            tool_name: tool_name.into(),
            message: message.into(),
            exit_code,
        }
    }

    /// Create a configuration error
    pub fn configuration(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Configuration {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a connection error
    pub fn connection(endpoint: impl Into<String>, message: impl Into<String>, retry_count: u32) -> Self {
        Self::Connection {
            endpoint: endpoint.into(),
            message: message.into(),
            retry_count,
        }
    }

    /// Create a serialization error
    pub fn serialization(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Serialization {
            context: context.into(),
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, timeout: Duration) -> Self {
        Self::Timeout {
            operation: operation.into(),
            timeout,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Error context for recovery decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation: String,
    pub server_name: Option<String>,
    pub tool_name: Option<String>,
    pub retry_count: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            server_name: None,
            tool_name: None,
            retry_count: 0,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_server(mut self, server_name: impl Into<String>) -> Self {
        self.server_name = Some(server_name.into());
        self
    }

    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.tool_name = Some(tool_name.into());
        self
    }

    pub fn with_retry_count(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Recovery actions that can be taken for errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Retry the operation immediately
    RetryImmediate,
    /// Retry the operation after a delay
    RetryAfterDelay(Duration),
    /// Failover to a different server
    Failover(String),
    /// Use a fallback strategy
    Fallback(String),
    /// Abort the operation
    Abort,
    /// Escalate to manual intervention
    Escalate,
}

/// Convert from anyhow::Error to McpError
impl From<anyhow::Error> for McpError {
    fn from(error: anyhow::Error) -> Self {
        McpError::internal(error.to_string())
    }
}

/// Convert from serde_json::Error to McpError
impl From<serde_json::Error> for McpError {
    fn from(error: serde_json::Error) -> Self {
        McpError::serialization("JSON", error.to_string())
    }
}

/// Convert from tokio::time::error::Elapsed to McpError
impl From<tokio::time::error::Elapsed> for McpError {
    fn from(_error: tokio::time::error::Elapsed) -> Self {
        McpError::timeout("operation", Duration::from_secs(30))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recoverability() {
        let recoverable_error = McpError::transport("stdio", "connection lost", true);
        assert!(recoverable_error.is_recoverable());

        let non_recoverable_error = McpError::configuration("invalid_field", "missing value");
        assert!(!non_recoverable_error.is_recoverable());
    }

    #[test]
    fn test_error_severity() {
        let critical_error = McpError::internal("system failure");
        assert_eq!(critical_error.severity(), ErrorSeverity::Critical);

        let low_error = McpError::validation("field", "invalid format");
        assert_eq!(low_error.severity(), ErrorSeverity::Low);
    }

    #[test]
    fn test_retry_delay() {
        let rate_limit_error = McpError::RateLimit {
            limit: 100,
            window: Duration::from_secs(60),
            retry_after: Some(Duration::from_secs(30)),
        };
        assert_eq!(rate_limit_error.retry_delay(), Some(Duration::from_secs(30)));

        let config_error = McpError::configuration("field", "invalid");
        assert_eq!(config_error.retry_delay(), None);
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("test_operation")
            .with_server("test_server")
            .with_tool("test_tool")
            .with_retry_count(3)
            .with_metadata("key", "value");

        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.server_name, Some("test_server".to_string()));
        assert_eq!(context.tool_name, Some("test_tool".to_string()));
        assert_eq!(context.retry_count, 3);
        assert_eq!(context.metadata.get("key"), Some(&"value".to_string()));
    }
}
