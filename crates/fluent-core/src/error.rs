use std::fmt;

/// Core error types for the fluent system
#[derive(Debug)]
pub enum FluentError {
    /// Configuration-related errors
    Config(ConfigError),
    
    /// Authentication and authorization errors
    Auth(AuthError),
    
    /// Network and HTTP-related errors
    Network(NetworkError),
    
    /// Engine-specific errors
    Engine(EngineError),
    
    /// Pipeline execution errors
    Pipeline(PipelineError),
    
    /// File I/O and validation errors
    File(FileError),
    
    /// Input validation errors
    Validation(ValidationError),
    
    /// Cost calculation errors
    Cost(CostError),
    
    /// Database and storage errors
    Storage(StorageError),
    
    /// Cache-related errors
    Cache(CacheError),
    
    /// Internal system errors
    Internal(String),
}

/// Configuration-related errors
#[derive(Debug)]
pub enum ConfigError {
    /// Missing required configuration parameter
    MissingParameter(String),
    
    /// Invalid configuration value
    InvalidValue { parameter: String, value: String, expected: String },
    
    /// Configuration file not found
    FileNotFound(String),
    
    /// Invalid configuration format
    InvalidFormat(String),
    
    /// Environment variable resolution failed
    EnvironmentResolution(String),
    
    /// Credential resolution failed
    CredentialResolution(String),
}

/// Authentication and authorization errors
#[derive(Debug)]
pub enum AuthError {
    /// Missing authentication token
    MissingToken,
    
    /// Invalid token format
    InvalidToken(String),
    
    /// Token validation failed
    TokenValidation(String),
    
    /// Authentication failed
    AuthenticationFailed(String),
    
    /// Authorization denied
    AuthorizationDenied(String),
    
    /// Expired credentials
    ExpiredCredentials,
}

/// Network and HTTP-related errors
#[derive(Debug)]
pub enum NetworkError {
    /// HTTP request failed
    RequestFailed { url: String, status: Option<u16>, message: String },
    
    /// Connection timeout
    Timeout(String),
    
    /// DNS resolution failed
    DnsResolution(String),
    
    /// SSL/TLS error
    TlsError(String),
    
    /// Invalid URL
    InvalidUrl(String),
    
    /// Network unreachable
    NetworkUnreachable,
    
    /// Rate limit exceeded
    RateLimitExceeded { retry_after: Option<u64> },
}

/// Engine-specific errors
#[derive(Debug)]
pub enum EngineError {
    /// Engine not found
    NotFound(String),
    
    /// Engine initialization failed
    InitializationFailed { engine: String, reason: String },
    
    /// API error from external service
    ApiError { engine: String, code: Option<String>, message: String },
    
    /// Unsupported operation
    UnsupportedOperation { engine: String, operation: String },
    
    /// Response parsing failed
    ResponseParsing { engine: String, reason: String },
    
    /// Model not available
    ModelNotAvailable { engine: String, model: String },
    
    /// Quota exceeded
    QuotaExceeded { engine: String },
}

/// Pipeline execution errors
#[derive(Debug)]
pub enum PipelineError {
    /// Invalid pipeline definition
    InvalidDefinition(String),
    
    /// Step execution failed
    StepFailed { step: String, reason: String },
    
    /// Variable expansion failed
    VariableExpansion(String),
    
    /// Condition evaluation failed
    ConditionEvaluation(String),
    
    /// Loop execution failed
    LoopExecution(String),
    
    /// Parallel execution failed
    ParallelExecution(String),
    
    /// Timeout exceeded
    TimeoutExceeded { step: String, timeout: u64 },
    
    /// State management error
    StateManagement(String),
}

/// File I/O and validation errors
#[derive(Debug)]
pub enum FileError {
    /// File not found
    NotFound(String),
    
    /// Permission denied
    PermissionDenied(String),
    
    /// Invalid file format
    InvalidFormat { file: String, expected: String },
    
    /// File too large
    TooLarge { file: String, size: u64, max_size: u64 },
    
    /// Path traversal attempt
    PathTraversal(String),
    
    /// Unsupported file type
    UnsupportedType(String),
    
    /// File corruption detected
    Corrupted(String),
}

/// Input validation errors
#[derive(Debug)]
pub enum ValidationError {
    /// Invalid input format
    InvalidFormat { input: String, expected: String },
    
    /// Input too long
    TooLong { input: String, length: usize, max_length: usize },
    
    /// Input too short
    TooShort { input: String, length: usize, min_length: usize },
    
    /// Dangerous pattern detected
    DangerousPattern(String),
    
    /// Required field missing
    MissingField(String),
    
    /// Invalid character detected
    InvalidCharacter { input: String, character: char },
    
    /// JSON validation failed
    JsonValidation(String),
}

/// Cost calculation errors
#[derive(Debug)]
pub enum CostError {
    /// Pricing model not found
    PricingModelNotFound { engine: String, model: String },
    
    /// Invalid cost calculation
    InvalidCalculation(String),
    
    /// Cost limit exceeded
    LimitExceeded { cost: f64, limit: f64 },
    
    /// Daily limit exceeded
    DailyLimitExceeded { cost: f64, limit: f64 },
    
    /// Negative cost detected
    NegativeCost(f64),
}

/// Database and storage errors
#[derive(Debug)]
pub enum StorageError {
    /// Connection failed
    ConnectionFailed(String),
    
    /// Query execution failed
    QueryFailed(String),
    
    /// Transaction failed
    TransactionFailed(String),
    
    /// Data not found
    NotFound(String),
    
    /// Constraint violation
    ConstraintViolation(String),
    
    /// Serialization failed
    SerializationFailed(String),
}

/// Cache-related errors
#[derive(Debug)]
pub enum CacheError {
    /// Cache miss
    Miss(String),
    
    /// Cache write failed
    WriteFailed(String),
    
    /// Cache corruption detected
    Corrupted(String),
    
    /// Cache eviction failed
    EvictionFailed(String),
}

impl fmt::Display for FluentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FluentError::Config(e) => write!(f, "Configuration error: {}", e),
            FluentError::Auth(e) => write!(f, "Authentication error: {}", e),
            FluentError::Network(e) => write!(f, "Network error: {}", e),
            FluentError::Engine(e) => write!(f, "Engine error: {}", e),
            FluentError::Pipeline(e) => write!(f, "Pipeline error: {}", e),
            FluentError::File(e) => write!(f, "File error: {}", e),
            FluentError::Validation(e) => write!(f, "Validation error: {}", e),
            FluentError::Cost(e) => write!(f, "Cost error: {}", e),
            FluentError::Storage(e) => write!(f, "Storage error: {}", e),
            FluentError::Cache(e) => write!(f, "Cache error: {}", e),
            FluentError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingParameter(param) => write!(f, "Missing required parameter: {}", param),
            ConfigError::InvalidValue { parameter, value, expected } => {
                write!(f, "Invalid value '{}' for parameter '{}', expected: {}", value, parameter, expected)
            }
            ConfigError::FileNotFound(file) => write!(f, "Configuration file not found: {}", file),
            ConfigError::InvalidFormat(msg) => write!(f, "Invalid configuration format: {}", msg),
            ConfigError::EnvironmentResolution(var) => write!(f, "Failed to resolve environment variable: {}", var),
            ConfigError::CredentialResolution(cred) => write!(f, "Failed to resolve credential: {}", cred),
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Authentication token is missing"),
            AuthError::InvalidToken(reason) => write!(f, "Invalid authentication token: {}", reason),
            AuthError::TokenValidation(reason) => write!(f, "Token validation failed: {}", reason),
            AuthError::AuthenticationFailed(reason) => write!(f, "Authentication failed: {}", reason),
            AuthError::AuthorizationDenied(reason) => write!(f, "Authorization denied: {}", reason),
            AuthError::ExpiredCredentials => write!(f, "Credentials have expired"),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::RequestFailed { url, status, message } => {
                match status {
                    Some(code) => write!(f, "HTTP request to {} failed with status {}: {}", url, code, message),
                    None => write!(f, "HTTP request to {} failed: {}", url, message),
                }
            }
            NetworkError::Timeout(url) => write!(f, "Request timeout for: {}", url),
            NetworkError::DnsResolution(host) => write!(f, "DNS resolution failed for: {}", host),
            NetworkError::TlsError(reason) => write!(f, "TLS error: {}", reason),
            NetworkError::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
            NetworkError::NetworkUnreachable => write!(f, "Network unreachable"),
            NetworkError::RateLimitExceeded { retry_after } => {
                match retry_after {
                    Some(seconds) => write!(f, "Rate limit exceeded, retry after {} seconds", seconds),
                    None => write!(f, "Rate limit exceeded"),
                }
            }
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::NotFound(engine) => write!(f, "Engine not found: {}", engine),
            EngineError::InitializationFailed { engine, reason } => {
                write!(f, "Engine '{}' initialization failed: {}", engine, reason)
            }
            EngineError::ApiError { engine, code, message } => {
                match code {
                    Some(c) => write!(f, "API error from {}: {} - {}", engine, c, message),
                    None => write!(f, "API error from {}: {}", engine, message),
                }
            }
            EngineError::UnsupportedOperation { engine, operation } => {
                write!(f, "Engine '{}' does not support operation: {}", engine, operation)
            }
            EngineError::ResponseParsing { engine, reason } => {
                write!(f, "Failed to parse response from {}: {}", engine, reason)
            }
            EngineError::ModelNotAvailable { engine, model } => {
                write!(f, "Model '{}' not available for engine '{}'", model, engine)
            }
            EngineError::QuotaExceeded { engine } => write!(f, "Quota exceeded for engine: {}", engine),
        }
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::InvalidDefinition(msg) => write!(f, "Invalid pipeline definition: {}", msg),
            PipelineError::StepFailed { step, reason } => write!(f, "Step '{}' failed: {}", step, reason),
            PipelineError::VariableExpansion(msg) => write!(f, "Variable expansion failed: {}", msg),
            PipelineError::ConditionEvaluation(msg) => write!(f, "Condition evaluation failed: {}", msg),
            PipelineError::LoopExecution(msg) => write!(f, "Loop execution failed: {}", msg),
            PipelineError::ParallelExecution(msg) => write!(f, "Parallel execution failed: {}", msg),
            PipelineError::TimeoutExceeded { step, timeout } => {
                write!(f, "Step '{}' exceeded timeout of {} seconds", step, timeout)
            }
            PipelineError::StateManagement(msg) => write!(f, "State management error: {}", msg),
        }
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::NotFound(file) => write!(f, "File not found: {}", file),
            FileError::PermissionDenied(file) => write!(f, "Permission denied: {}", file),
            FileError::InvalidFormat { file, expected } => {
                write!(f, "Invalid format for file '{}', expected: {}", file, expected)
            }
            FileError::TooLarge { file, size, max_size } => {
                write!(f, "File '{}' too large: {} bytes (max: {} bytes)", file, size, max_size)
            }
            FileError::PathTraversal(path) => write!(f, "Path traversal attempt detected: {}", path),
            FileError::UnsupportedType(file_type) => write!(f, "Unsupported file type: {}", file_type),
            FileError::Corrupted(file) => write!(f, "File corrupted: {}", file),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidFormat { input, expected } => {
                write!(f, "Invalid format for input '{}', expected: {}", input, expected)
            }
            ValidationError::TooLong { input, length, max_length } => {
                write!(f, "Input '{}' too long: {} characters (max: {})", input, length, max_length)
            }
            ValidationError::TooShort { input, length, min_length } => {
                write!(f, "Input '{}' too short: {} characters (min: {})", input, length, min_length)
            }
            ValidationError::DangerousPattern(pattern) => {
                write!(f, "Dangerous pattern detected: {}", pattern)
            }
            ValidationError::MissingField(field) => write!(f, "Required field missing: {}", field),
            ValidationError::InvalidCharacter { input, character } => {
                write!(f, "Invalid character '{}' in input: {}", character, input)
            }
            ValidationError::JsonValidation(msg) => write!(f, "JSON validation failed: {}", msg),
        }
    }
}

impl fmt::Display for CostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CostError::PricingModelNotFound { engine, model } => {
                write!(f, "Pricing model not found for engine '{}', model '{}'", engine, model)
            }
            CostError::InvalidCalculation(msg) => write!(f, "Invalid cost calculation: {}", msg),
            CostError::LimitExceeded { cost, limit } => {
                write!(f, "Cost limit exceeded: ${:.6} (limit: ${:.2})", cost, limit)
            }
            CostError::DailyLimitExceeded { cost, limit } => {
                write!(f, "Daily cost limit exceeded: ${:.6} (limit: ${:.2})", cost, limit)
            }
            CostError::NegativeCost(cost) => write!(f, "Negative cost detected: ${:.6}", cost),
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed(msg) => write!(f, "Storage connection failed: {}", msg),
            StorageError::QueryFailed(msg) => write!(f, "Query execution failed: {}", msg),
            StorageError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            StorageError::NotFound(item) => write!(f, "Item not found in storage: {}", item),
            StorageError::ConstraintViolation(msg) => write!(f, "Constraint violation: {}", msg),
            StorageError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
        }
    }
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::Miss(key) => write!(f, "Cache miss for key: {}", key),
            CacheError::WriteFailed(msg) => write!(f, "Cache write failed: {}", msg),
            CacheError::Corrupted(msg) => write!(f, "Cache corruption detected: {}", msg),
            CacheError::EvictionFailed(msg) => write!(f, "Cache eviction failed: {}", msg),
        }
    }
}

impl std::error::Error for FluentError {}
impl std::error::Error for ConfigError {}
impl std::error::Error for AuthError {}
impl std::error::Error for NetworkError {}
impl std::error::Error for EngineError {}
impl std::error::Error for PipelineError {}
impl std::error::Error for FileError {}
impl std::error::Error for ValidationError {}
impl std::error::Error for CostError {}
impl std::error::Error for StorageError {}
impl std::error::Error for CacheError {}

/// Result type alias for fluent operations
pub type FluentResult<T> = Result<T, FluentError>;

/// Conversion from anyhow::Error to FluentError
impl From<anyhow::Error> for FluentError {
    fn from(err: anyhow::Error) -> Self {
        FluentError::Internal(err.to_string())
    }
}

/// Conversion from reqwest::Error to FluentError
impl From<reqwest::Error> for FluentError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            FluentError::Network(NetworkError::Timeout(
                err.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".to_string())
            ))
        } else if err.is_connect() {
            FluentError::Network(NetworkError::NetworkUnreachable)
        } else if let Some(status) = err.status() {
            FluentError::Network(NetworkError::RequestFailed {
                url: err.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".to_string()),
                status: Some(status.as_u16()),
                message: err.to_string(),
            })
        } else {
            FluentError::Network(NetworkError::RequestFailed {
                url: err.url().map(|u| u.to_string()).unwrap_or_else(|| "unknown".to_string()),
                status: None,
                message: err.to_string(),
            })
        }
    }
}

/// Conversion from std::io::Error to FluentError
impl From<std::io::Error> for FluentError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => FluentError::File(FileError::NotFound(err.to_string())),
            std::io::ErrorKind::PermissionDenied => FluentError::File(FileError::PermissionDenied(err.to_string())),
            _ => FluentError::File(FileError::Corrupted(err.to_string())),
        }
    }
}

/// Conversion from serde_json::Error to FluentError
impl From<serde_json::Error> for FluentError {
    fn from(err: serde_json::Error) -> Self {
        FluentError::Validation(ValidationError::JsonValidation(err.to_string()))
    }
}

/// Conversion from url::ParseError to FluentError
impl From<url::ParseError> for FluentError {
    fn from(err: url::ParseError) -> Self {
        FluentError::Network(NetworkError::InvalidUrl(err.to_string()))
    }
}
