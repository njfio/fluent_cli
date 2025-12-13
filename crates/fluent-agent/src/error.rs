//! Unified Error Types for the Agent Framework
//!
//! This module provides a comprehensive error hierarchy for the agent system,
//! consolidating errors from various subsystems into a unified interface.
//!
//! # Error Categories
//!
//! - **Configuration**: Invalid or missing configuration values
//! - **Tool Execution**: Tool operations that fail
//! - **Reasoning**: LLM reasoning failures
//! - **Memory**: Memory system operations
//! - **MCP**: Model Context Protocol errors (re-exported from production_mcp)
//! - **Security**: Security violations and access control
//! - **Orchestration**: Agent orchestration and workflow errors
//! - **Timeout**: Operation timeouts
//!
//! # Error Codes
//!
//! Each error type has a unique error code prefix for programmatic handling:
//! - `E1xxx`: Configuration errors
//! - `E2xxx`: Tool execution errors
//! - `E3xxx`: Reasoning errors
//! - `E4xxx`: Memory errors
//! - `E5xxx`: MCP errors
//! - `E6xxx`: Security errors
//! - `E7xxx`: Orchestration errors
//! - `E8xxx`: Timeout errors
//! - `E9xxx`: Internal/unknown errors

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

// Re-export existing comprehensive error types
pub use crate::production_mcp::error::{ErrorContext, ErrorSeverity, McpError, RecoveryAction};
pub use crate::security::SecurityError;

// ============================================================================
// Error Code Constants
// ============================================================================

/// Error code constants for programmatic error handling
pub mod codes {
    // Configuration errors (E1xxx)
    pub const CONFIG_MISSING_FIELD: &str = "E1001";
    pub const CONFIG_INVALID_VALUE: &str = "E1002";
    pub const CONFIG_PARSE_ERROR: &str = "E1003";
    pub const CONFIG_FILE_NOT_FOUND: &str = "E1004";
    pub const CONFIG_VALIDATION_FAILED: &str = "E1005";

    // Tool execution errors (E2xxx)
    pub const TOOL_NOT_FOUND: &str = "E2001";
    pub const TOOL_EXECUTION_FAILED: &str = "E2002";
    pub const TOOL_INVALID_PARAMS: &str = "E2003";
    pub const TOOL_PERMISSION_DENIED: &str = "E2004";
    pub const TOOL_TIMEOUT: &str = "E2005";
    pub const TOOL_OUTPUT_TRUNCATED: &str = "E2006";

    // Reasoning errors (E3xxx)
    pub const REASONING_FAILED: &str = "E3001";
    pub const REASONING_MAX_ATTEMPTS: &str = "E3002";
    pub const REASONING_INVALID_RESPONSE: &str = "E3003";
    pub const REASONING_CONTEXT_TOO_LARGE: &str = "E3004";
    pub const REASONING_MODEL_ERROR: &str = "E3005";

    // Memory errors (E4xxx)
    pub const MEMORY_STORAGE_FAILED: &str = "E4001";
    pub const MEMORY_RETRIEVAL_FAILED: &str = "E4002";
    pub const MEMORY_CAPACITY_EXCEEDED: &str = "E4003";
    pub const MEMORY_CORRUPTION: &str = "E4004";
    pub const MEMORY_PERSISTENCE_FAILED: &str = "E4005";

    // MCP errors (E5xxx)
    pub const MCP_PROTOCOL: &str = "E5001";
    pub const MCP_TRANSPORT: &str = "E5002";
    pub const MCP_CONNECTION: &str = "E5003";
    pub const MCP_TIMEOUT: &str = "E5004";
    pub const MCP_RATE_LIMIT: &str = "E5005";

    // Security errors (E6xxx)
    pub const SECURITY_VIOLATION: &str = "E6001";
    pub const SECURITY_ACCESS_DENIED: &str = "E6002";
    pub const SECURITY_CAPABILITY_NOT_GRANTED: &str = "E6003";
    pub const SECURITY_INVALID_SESSION: &str = "E6004";
    pub const SECURITY_COMMAND_BLOCKED: &str = "E6005";

    // Orchestration errors (E7xxx)
    pub const ORCHESTRATION_TASK_FAILED: &str = "E7001";
    pub const ORCHESTRATION_WORKFLOW_FAILED: &str = "E7002";
    pub const ORCHESTRATION_CYCLE_DETECTED: &str = "E7003";
    pub const ORCHESTRATION_MAX_ITERATIONS: &str = "E7004";
    pub const ORCHESTRATION_CHECKPOINT_FAILED: &str = "E7005";

    // Timeout errors (E8xxx)
    pub const TIMEOUT_OPERATION: &str = "E8001";
    pub const TIMEOUT_REQUEST: &str = "E8002";
    pub const TIMEOUT_TASK: &str = "E8003";

    // Internal errors (E9xxx)
    pub const INTERNAL_ERROR: &str = "E9001";
    pub const INTERNAL_UNEXPECTED: &str = "E9002";
    pub const INTERNAL_NOT_IMPLEMENTED: &str = "E9003";
}

// ============================================================================
// Configuration Errors
// ============================================================================

/// Errors related to configuration
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ConfigError {
    #[error("[{code}] Missing required field: {field}")]
    MissingField { code: String, field: String },

    #[error("[{code}] Invalid value for {field}: {message}")]
    InvalidValue {
        code: String,
        field: String,
        message: String,
    },

    #[error("[{code}] Configuration parse error: {message}")]
    ParseError { code: String, message: String },

    #[error("[{code}] Configuration file not found: {path}")]
    FileNotFound { code: String, path: String },

    #[error("[{code}] Configuration validation failed: {message}")]
    ValidationFailed { code: String, message: String },
}

impl ConfigError {
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            code: codes::CONFIG_MISSING_FIELD.to_string(),
            field: field.into(),
        }
    }

    pub fn invalid_value(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidValue {
            code: codes::CONFIG_INVALID_VALUE.to_string(),
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            code: codes::CONFIG_PARSE_ERROR.to_string(),
            message: message.into(),
        }
    }

    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self::FileNotFound {
            code: codes::CONFIG_FILE_NOT_FOUND.to_string(),
            path: path.into(),
        }
    }

    pub fn validation_failed(message: impl Into<String>) -> Self {
        Self::ValidationFailed {
            code: codes::CONFIG_VALIDATION_FAILED.to_string(),
            message: message.into(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::MissingField { code, .. } => code,
            Self::InvalidValue { code, .. } => code,
            Self::ParseError { code, .. } => code,
            Self::FileNotFound { code, .. } => code,
            Self::ValidationFailed { code, .. } => code,
        }
    }
}

// ============================================================================
// Tool Execution Errors
// ============================================================================

/// Errors related to tool execution
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ToolError {
    #[error("[{code}] Tool not found: {tool_name}")]
    NotFound { code: String, tool_name: String },

    #[error("[{code}] Tool execution failed: {tool_name} - {message}")]
    ExecutionFailed {
        code: String,
        tool_name: String,
        message: String,
        exit_code: Option<i32>,
    },

    #[error("[{code}] Invalid parameters for tool {tool_name}: {message}")]
    InvalidParams {
        code: String,
        tool_name: String,
        message: String,
    },

    #[error("[{code}] Permission denied for tool {tool_name}: {message}")]
    PermissionDenied {
        code: String,
        tool_name: String,
        message: String,
    },

    #[error("[{code}] Tool {tool_name} timed out after {timeout:?}")]
    Timeout {
        code: String,
        tool_name: String,
        timeout: Duration,
    },

    #[error("[{code}] Tool {tool_name} output truncated at {max_bytes} bytes")]
    OutputTruncated {
        code: String,
        tool_name: String,
        max_bytes: usize,
    },
}

impl ToolError {
    pub fn not_found(tool_name: impl Into<String>) -> Self {
        Self::NotFound {
            code: codes::TOOL_NOT_FOUND.to_string(),
            tool_name: tool_name.into(),
        }
    }

    pub fn execution_failed(
        tool_name: impl Into<String>,
        message: impl Into<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self::ExecutionFailed {
            code: codes::TOOL_EXECUTION_FAILED.to_string(),
            tool_name: tool_name.into(),
            message: message.into(),
            exit_code,
        }
    }

    pub fn invalid_params(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidParams {
            code: codes::TOOL_INVALID_PARAMS.to_string(),
            tool_name: tool_name.into(),
            message: message.into(),
        }
    }

    pub fn permission_denied(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::PermissionDenied {
            code: codes::TOOL_PERMISSION_DENIED.to_string(),
            tool_name: tool_name.into(),
            message: message.into(),
        }
    }

    pub fn timeout(tool_name: impl Into<String>, timeout: Duration) -> Self {
        Self::Timeout {
            code: codes::TOOL_TIMEOUT.to_string(),
            tool_name: tool_name.into(),
            timeout,
        }
    }

    pub fn output_truncated(tool_name: impl Into<String>, max_bytes: usize) -> Self {
        Self::OutputTruncated {
            code: codes::TOOL_OUTPUT_TRUNCATED.to_string(),
            tool_name: tool_name.into(),
            max_bytes,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::NotFound { code, .. } => code,
            Self::ExecutionFailed { code, .. } => code,
            Self::InvalidParams { code, .. } => code,
            Self::PermissionDenied { code, .. } => code,
            Self::Timeout { code, .. } => code,
            Self::OutputTruncated { code, .. } => code,
        }
    }
}

// ============================================================================
// Reasoning Errors
// ============================================================================

/// Errors related to LLM reasoning
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ReasoningError {
    #[error("[{code}] Reasoning failed: {message}")]
    Failed { code: String, message: String },

    #[error("[{code}] Reasoning failed after {attempts} attempts: {message}")]
    MaxAttemptsExceeded {
        code: String,
        attempts: u32,
        message: String,
    },

    #[error("[{code}] Invalid response from reasoning engine: {message}")]
    InvalidResponse { code: String, message: String },

    #[error("[{code}] Context too large: {size} tokens exceeds limit of {limit}")]
    ContextTooLarge {
        code: String,
        size: usize,
        limit: usize,
    },

    #[error("[{code}] Model error: {model} - {message}")]
    ModelError {
        code: String,
        model: String,
        message: String,
    },
}

impl ReasoningError {
    pub fn failed(message: impl Into<String>) -> Self {
        Self::Failed {
            code: codes::REASONING_FAILED.to_string(),
            message: message.into(),
        }
    }

    pub fn max_attempts_exceeded(attempts: u32, message: impl Into<String>) -> Self {
        Self::MaxAttemptsExceeded {
            code: codes::REASONING_MAX_ATTEMPTS.to_string(),
            attempts,
            message: message.into(),
        }
    }

    pub fn invalid_response(message: impl Into<String>) -> Self {
        Self::InvalidResponse {
            code: codes::REASONING_INVALID_RESPONSE.to_string(),
            message: message.into(),
        }
    }

    pub fn context_too_large(size: usize, limit: usize) -> Self {
        Self::ContextTooLarge {
            code: codes::REASONING_CONTEXT_TOO_LARGE.to_string(),
            size,
            limit,
        }
    }

    pub fn model_error(model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ModelError {
            code: codes::REASONING_MODEL_ERROR.to_string(),
            model: model.into(),
            message: message.into(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Failed { code, .. } => code,
            Self::MaxAttemptsExceeded { code, .. } => code,
            Self::InvalidResponse { code, .. } => code,
            Self::ContextTooLarge { code, .. } => code,
            Self::ModelError { code, .. } => code,
        }
    }
}

// ============================================================================
// Memory Errors
// ============================================================================

/// Errors related to memory operations
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum MemoryError {
    #[error("[{code}] Memory storage failed: {message}")]
    StorageFailed { code: String, message: String },

    #[error("[{code}] Memory retrieval failed: {message}")]
    RetrievalFailed { code: String, message: String },

    #[error("[{code}] Memory capacity exceeded: {current} items, limit is {limit}")]
    CapacityExceeded {
        code: String,
        current: usize,
        limit: usize,
    },

    #[error("[{code}] Memory corruption detected: {message}")]
    Corruption { code: String, message: String },

    #[error("[{code}] Memory persistence failed: {message}")]
    PersistenceFailed { code: String, message: String },
}

impl MemoryError {
    pub fn storage_failed(message: impl Into<String>) -> Self {
        Self::StorageFailed {
            code: codes::MEMORY_STORAGE_FAILED.to_string(),
            message: message.into(),
        }
    }

    pub fn retrieval_failed(message: impl Into<String>) -> Self {
        Self::RetrievalFailed {
            code: codes::MEMORY_RETRIEVAL_FAILED.to_string(),
            message: message.into(),
        }
    }

    pub fn capacity_exceeded(current: usize, limit: usize) -> Self {
        Self::CapacityExceeded {
            code: codes::MEMORY_CAPACITY_EXCEEDED.to_string(),
            current,
            limit,
        }
    }

    pub fn corruption(message: impl Into<String>) -> Self {
        Self::Corruption {
            code: codes::MEMORY_CORRUPTION.to_string(),
            message: message.into(),
        }
    }

    pub fn persistence_failed(message: impl Into<String>) -> Self {
        Self::PersistenceFailed {
            code: codes::MEMORY_PERSISTENCE_FAILED.to_string(),
            message: message.into(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::StorageFailed { code, .. } => code,
            Self::RetrievalFailed { code, .. } => code,
            Self::CapacityExceeded { code, .. } => code,
            Self::Corruption { code, .. } => code,
            Self::PersistenceFailed { code, .. } => code,
        }
    }
}

// ============================================================================
// Orchestration Errors
// ============================================================================

/// Errors related to agent orchestration
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum OrchestrationError {
    #[error("[{code}] Task failed: {task_name} - {message}")]
    TaskFailed {
        code: String,
        task_name: String,
        message: String,
    },

    #[error("[{code}] Workflow failed: {workflow_name} - {message}")]
    WorkflowFailed {
        code: String,
        workflow_name: String,
        message: String,
    },

    #[error("[{code}] Dependency cycle detected: {path}")]
    CycleDetected { code: String, path: String },

    #[error("[{code}] Maximum iterations exceeded: {iterations}")]
    MaxIterationsExceeded { code: String, iterations: u32 },

    #[error("[{code}] Checkpoint operation failed: {operation} - {message}")]
    CheckpointFailed {
        code: String,
        operation: String,
        message: String,
    },
}

impl OrchestrationError {
    pub fn task_failed(task_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::TaskFailed {
            code: codes::ORCHESTRATION_TASK_FAILED.to_string(),
            task_name: task_name.into(),
            message: message.into(),
        }
    }

    pub fn workflow_failed(workflow_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::WorkflowFailed {
            code: codes::ORCHESTRATION_WORKFLOW_FAILED.to_string(),
            workflow_name: workflow_name.into(),
            message: message.into(),
        }
    }

    pub fn cycle_detected(path: impl Into<String>) -> Self {
        Self::CycleDetected {
            code: codes::ORCHESTRATION_CYCLE_DETECTED.to_string(),
            path: path.into(),
        }
    }

    pub fn max_iterations_exceeded(iterations: u32) -> Self {
        Self::MaxIterationsExceeded {
            code: codes::ORCHESTRATION_MAX_ITERATIONS.to_string(),
            iterations,
        }
    }

    pub fn checkpoint_failed(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::CheckpointFailed {
            code: codes::ORCHESTRATION_CHECKPOINT_FAILED.to_string(),
            operation: operation.into(),
            message: message.into(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::TaskFailed { code, .. } => code,
            Self::WorkflowFailed { code, .. } => code,
            Self::CycleDetected { code, .. } => code,
            Self::MaxIterationsExceeded { code, .. } => code,
            Self::CheckpointFailed { code, .. } => code,
        }
    }
}

// ============================================================================
// Unified Agent Error
// ============================================================================

/// The unified error type for the agent framework
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    Config(#[source] ConfigError),

    #[error("Tool execution error: {0}")]
    Tool(#[source] ToolError),

    #[error("Reasoning error: {0}")]
    Reasoning(#[source] ReasoningError),

    #[error("Memory error: {0}")]
    Memory(#[source] MemoryError),

    #[error("MCP error: {0}")]
    Mcp(#[source] McpError),

    #[error("Security error: {0}")]
    Security(#[source] SecurityError),

    #[error("Orchestration error: {0}")]
    Orchestration(#[source] OrchestrationError),

    #[error("[{code}] Timeout after {duration:?}: {operation}")]
    Timeout {
        code: String,
        operation: String,
        duration: Duration,
    },

    #[error("[{code}] Internal error: {message}")]
    Internal { code: String, message: String },

    #[error("External error: {0}")]
    External(#[from] anyhow::Error),
}

impl AgentError {
    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, duration: Duration) -> Self {
        Self::Timeout {
            code: codes::TIMEOUT_OPERATION.to_string(),
            operation: operation.into(),
            duration,
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            code: codes::INTERNAL_ERROR.to_string(),
            message: message.into(),
        }
    }

    /// Get the error code for this error
    pub fn code(&self) -> &str {
        match self {
            Self::Config(e) => e.code(),
            Self::Tool(e) => e.code(),
            Self::Reasoning(e) => e.code(),
            Self::Memory(e) => e.code(),
            Self::Mcp(_) => codes::MCP_PROTOCOL,
            Self::Security(_) => codes::SECURITY_VIOLATION,
            Self::Orchestration(e) => e.code(),
            Self::Timeout { code, .. } => code,
            Self::Internal { code, .. } => code,
            Self::External(_) => codes::INTERNAL_UNEXPECTED,
        }
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Mcp(e) => e.is_recoverable(),
            Self::Timeout { .. } => true,
            Self::Tool(ToolError::Timeout { .. }) => true,
            Self::Tool(ToolError::ExecutionFailed { .. }) => true,
            Self::Reasoning(ReasoningError::Failed { .. }) => true,
            Self::Reasoning(ReasoningError::ModelError { .. }) => true,
            Self::Memory(MemoryError::StorageFailed { .. }) => true,
            Self::Memory(MemoryError::PersistenceFailed { .. }) => true,
            _ => false,
        }
    }

    /// Get the severity of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Config(_) => ErrorSeverity::Critical,
            Self::Security(_) => ErrorSeverity::Critical,
            Self::Internal { .. } => ErrorSeverity::Critical,
            Self::Mcp(e) => e.severity(),
            Self::Orchestration(OrchestrationError::CycleDetected { .. }) => ErrorSeverity::High,
            Self::Orchestration(OrchestrationError::WorkflowFailed { .. }) => ErrorSeverity::High,
            Self::Reasoning(ReasoningError::ModelError { .. }) => ErrorSeverity::High,
            Self::Tool(ToolError::PermissionDenied { .. }) => ErrorSeverity::High,
            Self::Memory(MemoryError::Corruption { .. }) => ErrorSeverity::High,
            Self::Timeout { .. } => ErrorSeverity::Medium,
            Self::Tool(_) => ErrorSeverity::Medium,
            Self::Reasoning(_) => ErrorSeverity::Medium,
            Self::Memory(_) => ErrorSeverity::Medium,
            Self::Orchestration(_) => ErrorSeverity::Medium,
            Self::External(_) => ErrorSeverity::Medium,
        }
    }

    /// Get suggested retry delay if applicable
    pub fn retry_delay(&self) -> Option<Duration> {
        match self {
            Self::Mcp(e) => e.retry_delay(),
            Self::Timeout { duration, .. } => Some(*duration / 2),
            Self::Tool(ToolError::Timeout { timeout, .. }) => Some(*timeout / 2),
            Self::Reasoning(ReasoningError::MaxAttemptsExceeded { .. }) => {
                Some(Duration::from_secs(5))
            }
            _ => None,
        }
    }
}

// ============================================================================
// From implementations for conversion
// ============================================================================

impl From<ConfigError> for AgentError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<ToolError> for AgentError {
    fn from(error: ToolError) -> Self {
        Self::Tool(error)
    }
}

impl From<ReasoningError> for AgentError {
    fn from(error: ReasoningError) -> Self {
        Self::Reasoning(error)
    }
}

impl From<MemoryError> for AgentError {
    fn from(error: MemoryError) -> Self {
        Self::Memory(error)
    }
}

impl From<McpError> for AgentError {
    fn from(error: McpError) -> Self {
        Self::Mcp(error)
    }
}

impl From<SecurityError> for AgentError {
    fn from(error: SecurityError) -> Self {
        Self::Security(error)
    }
}

impl From<OrchestrationError> for AgentError {
    fn from(error: OrchestrationError) -> Self {
        Self::Orchestration(error)
    }
}

impl From<std::io::Error> for AgentError {
    fn from(error: std::io::Error) -> Self {
        Self::Internal {
            code: codes::INTERNAL_ERROR.to_string(),
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(error: serde_json::Error) -> Self {
        Self::Config(ConfigError::parse_error(error.to_string()))
    }
}

impl From<tokio::time::error::Elapsed> for AgentError {
    fn from(_error: tokio::time::error::Elapsed) -> Self {
        Self::Timeout {
            code: codes::TIMEOUT_OPERATION.to_string(),
            operation: "operation".to_string(),
            duration: Duration::from_secs(30),
        }
    }
}

// ============================================================================
// Result type alias
// ============================================================================

/// Convenience Result type for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========== ConfigError Tests ==========

    #[test]
    fn test_config_error_missing_field() {
        let error = ConfigError::missing_field("api_key");
        assert_eq!(error.code(), codes::CONFIG_MISSING_FIELD);
        assert!(error.to_string().contains("api_key"));
    }

    #[test]
    fn test_config_error_invalid_value() {
        let error = ConfigError::invalid_value("timeout", "must be positive");
        assert_eq!(error.code(), codes::CONFIG_INVALID_VALUE);
        assert!(error.to_string().contains("timeout"));
    }

    #[test]
    fn test_config_error_parse_error() {
        let error = ConfigError::parse_error("invalid JSON");
        assert_eq!(error.code(), codes::CONFIG_PARSE_ERROR);
    }

    #[test]
    fn test_config_error_file_not_found() {
        let error = ConfigError::file_not_found("/path/to/config");
        assert_eq!(error.code(), codes::CONFIG_FILE_NOT_FOUND);
    }

    #[test]
    fn test_config_error_validation_failed() {
        let error = ConfigError::validation_failed("schema mismatch");
        assert_eq!(error.code(), codes::CONFIG_VALIDATION_FAILED);
    }

    // ========== ToolError Tests ==========

    #[test]
    fn test_tool_error_not_found() {
        let error = ToolError::not_found("unknown_tool");
        assert_eq!(error.code(), codes::TOOL_NOT_FOUND);
    }

    #[test]
    fn test_tool_error_execution_failed() {
        let error = ToolError::execution_failed("read_file", "file not found", Some(1));
        assert_eq!(error.code(), codes::TOOL_EXECUTION_FAILED);
        assert!(error.to_string().contains("read_file"));
    }

    #[test]
    fn test_tool_error_invalid_params() {
        let error = ToolError::invalid_params("write_file", "path is required");
        assert_eq!(error.code(), codes::TOOL_INVALID_PARAMS);
    }

    #[test]
    fn test_tool_error_permission_denied() {
        let error = ToolError::permission_denied("execute_command", "not in allowlist");
        assert_eq!(error.code(), codes::TOOL_PERMISSION_DENIED);
    }

    #[test]
    fn test_tool_error_timeout() {
        let error = ToolError::timeout("long_running_tool", Duration::from_secs(30));
        assert_eq!(error.code(), codes::TOOL_TIMEOUT);
    }

    #[test]
    fn test_tool_error_output_truncated() {
        let error = ToolError::output_truncated("command", 1024 * 1024);
        assert_eq!(error.code(), codes::TOOL_OUTPUT_TRUNCATED);
    }

    // ========== ReasoningError Tests ==========

    #[test]
    fn test_reasoning_error_failed() {
        let error = ReasoningError::failed("could not process input");
        assert_eq!(error.code(), codes::REASONING_FAILED);
    }

    #[test]
    fn test_reasoning_error_max_attempts() {
        let error = ReasoningError::max_attempts_exceeded(5, "still failing");
        assert_eq!(error.code(), codes::REASONING_MAX_ATTEMPTS);
        assert!(error.to_string().contains("5"));
    }

    #[test]
    fn test_reasoning_error_invalid_response() {
        let error = ReasoningError::invalid_response("malformed JSON");
        assert_eq!(error.code(), codes::REASONING_INVALID_RESPONSE);
    }

    #[test]
    fn test_reasoning_error_context_too_large() {
        let error = ReasoningError::context_too_large(200000, 128000);
        assert_eq!(error.code(), codes::REASONING_CONTEXT_TOO_LARGE);
    }

    #[test]
    fn test_reasoning_error_model_error() {
        let error = ReasoningError::model_error("gpt-4", "rate limited");
        assert_eq!(error.code(), codes::REASONING_MODEL_ERROR);
    }

    // ========== MemoryError Tests ==========

    #[test]
    fn test_memory_error_storage_failed() {
        let error = MemoryError::storage_failed("disk full");
        assert_eq!(error.code(), codes::MEMORY_STORAGE_FAILED);
    }

    #[test]
    fn test_memory_error_retrieval_failed() {
        let error = MemoryError::retrieval_failed("key not found");
        assert_eq!(error.code(), codes::MEMORY_RETRIEVAL_FAILED);
    }

    #[test]
    fn test_memory_error_capacity_exceeded() {
        let error = MemoryError::capacity_exceeded(1001, 1000);
        assert_eq!(error.code(), codes::MEMORY_CAPACITY_EXCEEDED);
    }

    #[test]
    fn test_memory_error_corruption() {
        let error = MemoryError::corruption("checksum mismatch");
        assert_eq!(error.code(), codes::MEMORY_CORRUPTION);
    }

    #[test]
    fn test_memory_error_persistence_failed() {
        let error = MemoryError::persistence_failed("IO error");
        assert_eq!(error.code(), codes::MEMORY_PERSISTENCE_FAILED);
    }

    // ========== OrchestrationError Tests ==========

    #[test]
    fn test_orchestration_error_task_failed() {
        let error = OrchestrationError::task_failed("task1", "dependency missing");
        assert_eq!(error.code(), codes::ORCHESTRATION_TASK_FAILED);
    }

    #[test]
    fn test_orchestration_error_workflow_failed() {
        let error = OrchestrationError::workflow_failed("main_workflow", "step 3 failed");
        assert_eq!(error.code(), codes::ORCHESTRATION_WORKFLOW_FAILED);
    }

    #[test]
    fn test_orchestration_error_cycle_detected() {
        let error = OrchestrationError::cycle_detected("A -> B -> C -> A");
        assert_eq!(error.code(), codes::ORCHESTRATION_CYCLE_DETECTED);
    }

    #[test]
    fn test_orchestration_error_max_iterations() {
        let error = OrchestrationError::max_iterations_exceeded(100);
        assert_eq!(error.code(), codes::ORCHESTRATION_MAX_ITERATIONS);
    }

    #[test]
    fn test_orchestration_error_checkpoint_failed() {
        let error = OrchestrationError::checkpoint_failed("save", "disk full");
        assert_eq!(error.code(), codes::ORCHESTRATION_CHECKPOINT_FAILED);
    }

    // ========== AgentError Tests ==========

    #[test]
    fn test_agent_error_timeout() {
        let error = AgentError::timeout("api_call", Duration::from_secs(30));
        assert_eq!(error.code(), codes::TIMEOUT_OPERATION);
        assert!(error.is_recoverable());
    }

    #[test]
    fn test_agent_error_internal() {
        let error = AgentError::internal("unexpected state");
        assert_eq!(error.code(), codes::INTERNAL_ERROR);
        assert!(!error.is_recoverable());
    }

    #[test]
    fn test_agent_error_from_config() {
        let config_error = ConfigError::missing_field("key");
        let agent_error: AgentError = config_error.into();
        assert_eq!(agent_error.code(), codes::CONFIG_MISSING_FIELD);
        assert_eq!(agent_error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_agent_error_from_tool() {
        let tool_error = ToolError::not_found("tool");
        let agent_error: AgentError = tool_error.into();
        assert_eq!(agent_error.code(), codes::TOOL_NOT_FOUND);
    }

    #[test]
    fn test_agent_error_from_reasoning() {
        let reasoning_error = ReasoningError::failed("test");
        let agent_error: AgentError = reasoning_error.into();
        assert!(agent_error.is_recoverable());
    }

    #[test]
    fn test_agent_error_from_memory() {
        let memory_error = MemoryError::storage_failed("test");
        let agent_error: AgentError = memory_error.into();
        assert!(agent_error.is_recoverable());
    }

    #[test]
    fn test_agent_error_from_orchestration() {
        let orch_error = OrchestrationError::task_failed("task", "error");
        let agent_error: AgentError = orch_error.into();
        assert_eq!(agent_error.code(), codes::ORCHESTRATION_TASK_FAILED);
    }

    // ========== Severity Tests ==========

    #[test]
    fn test_agent_error_severity_critical() {
        let error = AgentError::internal("test");
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_agent_error_severity_from_config() {
        let error: AgentError = ConfigError::missing_field("key").into();
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_agent_error_severity_timeout() {
        let error = AgentError::timeout("op", Duration::from_secs(1));
        assert_eq!(error.severity(), ErrorSeverity::Medium);
    }

    // ========== Retry Delay Tests ==========

    #[test]
    fn test_agent_error_retry_delay_timeout() {
        let error = AgentError::timeout("op", Duration::from_secs(10));
        assert_eq!(error.retry_delay(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_agent_error_retry_delay_none() {
        let error = AgentError::internal("test");
        assert_eq!(error.retry_delay(), None);
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_config_error_serialization() {
        let error = ConfigError::missing_field("key");
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ConfigError = serde_json::from_str(&json).unwrap();
        assert_eq!(error.code(), deserialized.code());
    }

    #[test]
    fn test_tool_error_serialization() {
        let error = ToolError::not_found("tool");
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ToolError = serde_json::from_str(&json).unwrap();
        assert_eq!(error.code(), deserialized.code());
    }

    #[test]
    fn test_reasoning_error_serialization() {
        let error = ReasoningError::failed("test");
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ReasoningError = serde_json::from_str(&json).unwrap();
        assert_eq!(error.code(), deserialized.code());
    }

    #[test]
    fn test_memory_error_serialization() {
        let error = MemoryError::storage_failed("test");
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: MemoryError = serde_json::from_str(&json).unwrap();
        assert_eq!(error.code(), deserialized.code());
    }

    #[test]
    fn test_orchestration_error_serialization() {
        let error = OrchestrationError::task_failed("task", "msg");
        let json = serde_json::to_string(&error).unwrap();
        let deserialized: OrchestrationError = serde_json::from_str(&json).unwrap();
        assert_eq!(error.code(), deserialized.code());
    }

    // ========== Display Tests ==========

    #[test]
    fn test_error_display_includes_code() {
        let error = ConfigError::missing_field("api_key");
        let display = error.to_string();
        assert!(display.contains(codes::CONFIG_MISSING_FIELD));
        assert!(display.contains("api_key"));
    }

    #[test]
    fn test_agent_error_display() {
        let error = AgentError::timeout("test_op", Duration::from_secs(5));
        let display = error.to_string();
        assert!(display.contains(codes::TIMEOUT_OPERATION));
        assert!(display.contains("test_op"));
    }

    // ========== Error Code Module Tests ==========

    #[test]
    fn test_error_codes_unique() {
        // Verify error codes are unique by category
        let config_codes = vec![
            codes::CONFIG_MISSING_FIELD,
            codes::CONFIG_INVALID_VALUE,
            codes::CONFIG_PARSE_ERROR,
            codes::CONFIG_FILE_NOT_FOUND,
            codes::CONFIG_VALIDATION_FAILED,
        ];
        assert_eq!(config_codes.len(), 5);
        for code in &config_codes {
            assert!(code.starts_with("E1"));
        }

        let tool_codes = vec![
            codes::TOOL_NOT_FOUND,
            codes::TOOL_EXECUTION_FAILED,
            codes::TOOL_INVALID_PARAMS,
            codes::TOOL_PERMISSION_DENIED,
            codes::TOOL_TIMEOUT,
            codes::TOOL_OUTPUT_TRUNCATED,
        ];
        for code in &tool_codes {
            assert!(code.starts_with("E2"));
        }
    }
}
