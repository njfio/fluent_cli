use std::fmt;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

    /// Lock timeout errors
    LockTimeout(String),

    /// Internal system errors
    Internal(String),
}

/// Configuration-related errors
#[derive(Debug)]
pub enum ConfigError {
    /// Missing required configuration parameter
    MissingParameter(String),

    /// Invalid configuration value
    InvalidValue {
        parameter: String,
        value: String,
        expected: String,
    },

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
    RequestFailed {
        url: String,
        status: Option<u16>,
        message: String,
    },

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
    ApiError {
        engine: String,
        code: Option<String>,
        message: String,
    },

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
    TooLarge {
        file: String,
        size: u64,
        max_size: u64,
    },

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
    TooLong {
        input: String,
        length: usize,
        max_length: usize,
    },

    /// Input too short
    TooShort {
        input: String,
        length: usize,
        min_length: usize,
    },

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
            FluentError::LockTimeout(msg) => write!(f, "Lock timeout error: {}", msg),
            FluentError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingParameter(param) => {
                write!(f, "Missing required parameter: {}", param)
            }
            ConfigError::InvalidValue {
                parameter,
                value,
                expected,
            } => {
                write!(
                    f,
                    "Invalid value '{}' for parameter '{}', expected: {}",
                    value, parameter, expected
                )
            }
            ConfigError::FileNotFound(file) => write!(f, "Configuration file not found: {}", file),
            ConfigError::InvalidFormat(msg) => write!(f, "Invalid configuration format: {}", msg),
            ConfigError::EnvironmentResolution(var) => {
                write!(f, "Failed to resolve environment variable: {}", var)
            }
            ConfigError::CredentialResolution(cred) => {
                write!(f, "Failed to resolve credential: {}", cred)
            }
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Authentication token is missing"),
            AuthError::InvalidToken(reason) => {
                write!(f, "Invalid authentication token: {}", reason)
            }
            AuthError::TokenValidation(reason) => write!(f, "Token validation failed: {}", reason),
            AuthError::AuthenticationFailed(reason) => {
                write!(f, "Authentication failed: {}", reason)
            }
            AuthError::AuthorizationDenied(reason) => write!(f, "Authorization denied: {}", reason),
            AuthError::ExpiredCredentials => write!(f, "Credentials have expired"),
        }
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::RequestFailed {
                url,
                status,
                message,
            } => match status {
                Some(code) => write!(
                    f,
                    "HTTP request to {} failed with status {}: {}",
                    url, code, message
                ),
                None => write!(f, "HTTP request to {} failed: {}", url, message),
            },
            NetworkError::Timeout(url) => write!(f, "Request timeout for: {}", url),
            NetworkError::DnsResolution(host) => write!(f, "DNS resolution failed for: {}", host),
            NetworkError::TlsError(reason) => write!(f, "TLS error: {}", reason),
            NetworkError::InvalidUrl(url) => write!(f, "Invalid URL: {}", url),
            NetworkError::NetworkUnreachable => write!(f, "Network unreachable"),
            NetworkError::RateLimitExceeded { retry_after } => match retry_after {
                Some(seconds) => write!(f, "Rate limit exceeded, retry after {} seconds", seconds),
                None => write!(f, "Rate limit exceeded"),
            },
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
            EngineError::ApiError {
                engine,
                code,
                message,
            } => match code {
                Some(c) => write!(f, "API error from {}: {} - {}", engine, c, message),
                None => write!(f, "API error from {}: {}", engine, message),
            },
            EngineError::UnsupportedOperation { engine, operation } => {
                write!(
                    f,
                    "Engine '{}' does not support operation: {}",
                    engine, operation
                )
            }
            EngineError::ResponseParsing { engine, reason } => {
                write!(f, "Failed to parse response from {}: {}", engine, reason)
            }
            EngineError::ModelNotAvailable { engine, model } => {
                write!(f, "Model '{}' not available for engine '{}'", model, engine)
            }
            EngineError::QuotaExceeded { engine } => {
                write!(f, "Quota exceeded for engine: {}", engine)
            }
        }
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::InvalidDefinition(msg) => {
                write!(f, "Invalid pipeline definition: {}", msg)
            }
            PipelineError::StepFailed { step, reason } => {
                write!(f, "Step '{}' failed: {}", step, reason)
            }
            PipelineError::VariableExpansion(msg) => {
                write!(f, "Variable expansion failed: {}", msg)
            }
            PipelineError::ConditionEvaluation(msg) => {
                write!(f, "Condition evaluation failed: {}", msg)
            }
            PipelineError::LoopExecution(msg) => write!(f, "Loop execution failed: {}", msg),
            PipelineError::ParallelExecution(msg) => {
                write!(f, "Parallel execution failed: {}", msg)
            }
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
                write!(
                    f,
                    "Invalid format for file '{}', expected: {}",
                    file, expected
                )
            }
            FileError::TooLarge {
                file,
                size,
                max_size,
            } => {
                write!(
                    f,
                    "File '{}' too large: {} bytes (max: {} bytes)",
                    file, size, max_size
                )
            }
            FileError::PathTraversal(path) => {
                write!(f, "Path traversal attempt detected: {}", path)
            }
            FileError::UnsupportedType(file_type) => {
                write!(f, "Unsupported file type: {}", file_type)
            }
            FileError::Corrupted(file) => write!(f, "File corrupted: {}", file),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidFormat { input, expected } => {
                write!(
                    f,
                    "Invalid format for input '{}', expected: {}",
                    input, expected
                )
            }
            ValidationError::TooLong {
                input,
                length,
                max_length,
            } => {
                write!(
                    f,
                    "Input '{}' too long: {} characters (max: {})",
                    input, length, max_length
                )
            }
            ValidationError::TooShort {
                input,
                length,
                min_length,
            } => {
                write!(
                    f,
                    "Input '{}' too short: {} characters (min: {})",
                    input, length, min_length
                )
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
                write!(
                    f,
                    "Pricing model not found for engine '{}', model '{}'",
                    engine, model
                )
            }
            CostError::InvalidCalculation(msg) => write!(f, "Invalid cost calculation: {}", msg),
            CostError::LimitExceeded { cost, limit } => {
                write!(
                    f,
                    "Cost limit exceeded: ${:.6} (limit: ${:.2})",
                    cost, limit
                )
            }
            CostError::DailyLimitExceeded { cost, limit } => {
                write!(
                    f,
                    "Daily cost limit exceeded: ${:.6} (limit: ${:.2})",
                    cost, limit
                )
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

// Thread-safety implementations for all error types
unsafe impl Send for FluentError {}
unsafe impl Sync for FluentError {}
unsafe impl Send for ConfigError {}
unsafe impl Sync for ConfigError {}
unsafe impl Send for AuthError {}
unsafe impl Sync for AuthError {}
unsafe impl Send for NetworkError {}
unsafe impl Sync for NetworkError {}
unsafe impl Send for EngineError {}
unsafe impl Sync for EngineError {}
unsafe impl Send for PipelineError {}
unsafe impl Sync for PipelineError {}
unsafe impl Send for FileError {}
unsafe impl Sync for FileError {}
unsafe impl Send for ValidationError {}
unsafe impl Sync for ValidationError {}
unsafe impl Send for CostError {}
unsafe impl Sync for CostError {}
unsafe impl Send for StorageError {}
unsafe impl Sync for StorageError {}
unsafe impl Send for CacheError {}
unsafe impl Sync for CacheError {}

/// Result type alias for fluent operations
pub type FluentResult<T> = Result<T, FluentError>;

/// Thread-safe error context for tracking errors across threads
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Thread ID where the error occurred
    pub thread_id: thread::ThreadId,
    /// Thread name if available
    pub thread_name: Option<String>,
    /// Stack of operation contexts
    pub operation_stack: Arc<Vec<String>>,
    /// Additional metadata
    pub metadata: Arc<std::collections::HashMap<String, String>>,
}

impl ErrorContext {
    /// Create a new error context for the current thread
    pub fn new() -> Self {
        let current_thread = thread::current();
        Self {
            thread_id: current_thread.id(),
            thread_name: current_thread.name().map(|s| s.to_string()),
            operation_stack: Arc::new(Vec::new()),
            metadata: Arc::new(std::collections::HashMap::new()),
        }
    }

    /// Add an operation to the context stack
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        let mut stack = (*self.operation_stack).clone();
        stack.push(operation.into());
        self.operation_stack = Arc::new(stack);
        self
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut metadata = (*self.metadata).clone();
        metadata.insert(key.into(), value.into());
        self.metadata = Arc::new(metadata);
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe error handling utilities
pub struct ThreadSafeErrorHandler;

/// Mutex poison recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoisonRecoveryStrategy {
    /// Fail immediately when mutex is poisoned
    FailFast,
    /// Attempt to recover by accessing the poisoned data
    RecoverData,
    /// Log the poison error and continue with default value
    UseDefault,
    /// Retry the operation after a brief delay
    RetryWithDelay,
}

/// Configuration for mutex poison handling
#[derive(Debug, Clone)]
pub struct PoisonHandlingConfig {
    /// Recovery strategy to use
    pub strategy: PoisonRecoveryStrategy,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Delay between retry attempts in milliseconds
    pub retry_delay_ms: u64,
    /// Whether to log poison events
    pub log_poison_events: bool,
}

/// Configuration for lock timeout handling
#[derive(Debug, Clone)]
pub struct LockTimeoutConfig {
    /// Timeout duration for lock acquisition
    pub timeout: Duration,
    /// Whether to log timeout events
    pub log_timeout_events: bool,
    /// Whether to enable lock contention monitoring
    pub monitor_contention: bool,
    /// Maximum number of concurrent waiters before warning
    pub max_waiters_warning_threshold: u32,
}

impl Default for PoisonHandlingConfig {
    fn default() -> Self {
        Self {
            strategy: PoisonRecoveryStrategy::FailFast,
            max_retries: 3,
            retry_delay_ms: 100,
            log_poison_events: true,
        }
    }
}

impl Default for LockTimeoutConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            log_timeout_events: true,
            monitor_contention: true,
            max_waiters_warning_threshold: 10,
        }
    }
}

impl LockTimeoutConfig {
    /// Create a config with a specific timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            timeout,
            ..Default::default()
        }
    }

    /// Create a config for short operations (5 seconds)
    pub fn short_timeout() -> Self {
        Self::with_timeout(Duration::from_secs(5))
    }

    /// Create a config for medium operations (30 seconds)
    pub fn medium_timeout() -> Self {
        Self::with_timeout(Duration::from_secs(30))
    }

    /// Create a config for long operations (2 minutes)
    pub fn long_timeout() -> Self {
        Self::with_timeout(Duration::from_secs(120))
    }

    /// Create a config with no timeout (for critical operations)
    pub fn no_timeout() -> Self {
        Self {
            timeout: Duration::from_secs(u64::MAX),
            log_timeout_events: false,
            monitor_contention: false,
            max_waiters_warning_threshold: u32::MAX,
        }
    }
}

impl PoisonHandlingConfig {
    /// Create a config for fail-fast strategy
    pub fn fail_fast() -> Self {
        Self {
            strategy: PoisonRecoveryStrategy::FailFast,
            ..Default::default()
        }
    }

    /// Create a config for data recovery strategy
    pub fn recover_data() -> Self {
        Self {
            strategy: PoisonRecoveryStrategy::RecoverData,
            log_poison_events: true,
            ..Default::default()
        }
    }

    /// Create a config for retry strategy
    pub fn retry_with_delay(max_retries: u32, delay_ms: u64) -> Self {
        Self {
            strategy: PoisonRecoveryStrategy::RetryWithDelay,
            max_retries,
            retry_delay_ms: delay_ms,
            log_poison_events: true,
        }
    }

    /// Create a config for default value strategy
    pub fn use_default() -> Self {
        Self {
            strategy: PoisonRecoveryStrategy::UseDefault,
            log_poison_events: true,
            ..Default::default()
        }
    }
}

impl ThreadSafeErrorHandler {
    /// Handle a mutex lock result safely with default fail-fast strategy
    pub fn handle_mutex_lock<'a, T>(
        result: Result<std::sync::MutexGuard<'a, T>, std::sync::PoisonError<std::sync::MutexGuard<'a, T>>>,
        context: &str,
    ) -> Result<std::sync::MutexGuard<'a, T>, FluentError> {
        Self::handle_mutex_lock_with_config(result, context, &PoisonHandlingConfig::fail_fast())
    }

    /// Handle a mutex lock result with configurable poison recovery strategy
    pub fn handle_mutex_lock_with_config<'a, T>(
        result: Result<std::sync::MutexGuard<'a, T>, std::sync::PoisonError<std::sync::MutexGuard<'a, T>>>,
        context: &str,
        config: &PoisonHandlingConfig,
    ) -> Result<std::sync::MutexGuard<'a, T>, FluentError> {
        match result {
            Ok(guard) => Ok(guard),
            Err(poison_error) => {
                if config.log_poison_events {
                    eprintln!(
                        "⚠️  Mutex poisoned in {}: {}. Thread: {:?}, Strategy: {:?}",
                        context,
                        poison_error,
                        thread::current().id(),
                        config.strategy
                    );
                }

                match config.strategy {
                    PoisonRecoveryStrategy::FailFast => {
                        Err(FluentError::Internal(format!(
                            "Mutex poisoned in {}: {}. Thread: {:?}",
                            context,
                            poison_error,
                            thread::current().id()
                        )))
                    }
                    PoisonRecoveryStrategy::RecoverData => {
                        // Recover the data from the poisoned mutex
                        Ok(poison_error.into_inner())
                    }
                    PoisonRecoveryStrategy::UseDefault => {
                        // This strategy requires a default value, so we still fail
                        // but with a more informative message
                        Err(FluentError::Internal(format!(
                            "Mutex poisoned in {} and no default value available: {}. Thread: {:?}",
                            context,
                            poison_error,
                            thread::current().id()
                        )))
                    }
                    PoisonRecoveryStrategy::RetryWithDelay => {
                        // For retry strategy, we still fail but indicate retry is possible
                        Err(FluentError::Internal(format!(
                            "Mutex poisoned in {} (retry possible): {}. Thread: {:?}",
                            context,
                            poison_error,
                            thread::current().id()
                        )))
                    }
                }
            }
        }
    }

    /// Handle a RwLock read result safely
    pub fn handle_rwlock_read<'a, T>(
        result: Result<std::sync::RwLockReadGuard<'a, T>, std::sync::PoisonError<std::sync::RwLockReadGuard<'a, T>>>,
        context: &str,
    ) -> Result<std::sync::RwLockReadGuard<'a, T>, FluentError> {
        result.map_err(|e| {
            FluentError::Internal(format!(
                "RwLock read poisoned in {}: {}. Thread: {:?}",
                context,
                e,
                thread::current().id()
            ))
        })
    }

    /// Handle a RwLock write result safely
    pub fn handle_rwlock_write<'a, T>(
        result: Result<std::sync::RwLockWriteGuard<'a, T>, std::sync::PoisonError<std::sync::RwLockWriteGuard<'a, T>>>,
        context: &str,
    ) -> Result<std::sync::RwLockWriteGuard<'a, T>, FluentError> {
        result.map_err(|e| {
            FluentError::Internal(format!(
                "RwLock write poisoned in {}: {}. Thread: {:?}",
                context,
                e,
                thread::current().id()
            ))
        })
    }

    /// Handle mutex lock with retry logic
    pub fn handle_mutex_lock_with_retry<T, F>(
        mutex: &std::sync::Mutex<T>,
        context: &str,
        config: &PoisonHandlingConfig,
        operation: F,
    ) -> Result<(), FluentError>
    where
        F: Fn(&mut T) -> Result<(), FluentError>,
    {
        let mut attempts = 0;
        loop {
            match mutex.lock() {
                Ok(mut guard) => {
                    return operation(&mut *guard);
                }
                Err(poison_error) => {
                    attempts += 1;

                    if config.log_poison_events {
                        eprintln!(
                            "⚠️  Mutex poisoned in {} (attempt {}/{}): {}. Thread: {:?}",
                            context,
                            attempts,
                            config.max_retries + 1,
                            poison_error,
                            thread::current().id()
                        );
                    }

                    match config.strategy {
                        PoisonRecoveryStrategy::RecoverData => {
                            // Try to recover and continue
                            let mut guard = poison_error.into_inner();
                            return operation(&mut *guard);
                        }
                        PoisonRecoveryStrategy::RetryWithDelay if attempts <= config.max_retries => {
                            // Wait and retry
                            std::thread::sleep(std::time::Duration::from_millis(config.retry_delay_ms));
                            continue;
                        }
                        _ => {
                            return Err(FluentError::Internal(format!(
                                "Mutex poisoned in {} after {} attempts: {}. Thread: {:?}",
                                context,
                                attempts,
                                poison_error,
                                thread::current().id()
                            )));
                        }
                    }
                }
            }
        }
    }

    /// Handle mutex lock with default value fallback
    pub fn handle_mutex_lock_with_default<T: Clone + Default>(
        mutex: &std::sync::Mutex<T>,
        context: &str,
        config: &PoisonHandlingConfig,
    ) -> Result<T, FluentError> {
        match mutex.lock() {
            Ok(guard) => Ok(guard.clone()),
            Err(poison_error) => {
                if config.log_poison_events {
                    eprintln!(
                        "⚠️  Mutex poisoned in {}: {}. Thread: {:?}, using default value",
                        context,
                        poison_error,
                        thread::current().id()
                    );
                }

                match config.strategy {
                    PoisonRecoveryStrategy::RecoverData => {
                        Ok(poison_error.into_inner().clone())
                    }
                    PoisonRecoveryStrategy::UseDefault => {
                        Ok(T::default())
                    }
                    _ => {
                        Err(FluentError::Internal(format!(
                            "Mutex poisoned in {} and recovery not configured: {}. Thread: {:?}",
                            context,
                            poison_error,
                            thread::current().id()
                        )))
                    }
                }
            }
        }
    }

    /// Handle tokio mutex lock with timeout
    pub async fn handle_tokio_mutex_lock_with_timeout<'a, T>(
        mutex: &'a tokio::sync::Mutex<T>,
        context: &str,
        config: &LockTimeoutConfig,
    ) -> Result<tokio::sync::MutexGuard<'a, T>, FluentError> {
        let start_time = std::time::Instant::now();

        if config.log_timeout_events {
            if config.timeout.as_secs() < 60 {
                eprintln!(
                    "🔒 Acquiring mutex lock in {} with timeout: {:?}",
                    context, config.timeout
                );
            }
        }

        match tokio::time::timeout(config.timeout, mutex.lock()).await {
            Ok(guard) => {
                let elapsed = start_time.elapsed();
                if config.log_timeout_events && elapsed > Duration::from_millis(100) {
                    eprintln!(
                        "✅ Acquired mutex lock in {} after {:?}",
                        context, elapsed
                    );
                }
                Ok(guard)
            }
            Err(_) => {
                let elapsed = start_time.elapsed();
                if config.log_timeout_events {
                    eprintln!(
                        "⏰ Mutex lock timeout in {} after {:?} (timeout: {:?}). Thread: {:?}",
                        context,
                        elapsed,
                        config.timeout,
                        thread::current().id()
                    );
                }
                Err(FluentError::LockTimeout(format!(
                    "Mutex lock timeout in {} after {:?} (timeout: {:?}). Thread: {:?}",
                    context,
                    elapsed,
                    config.timeout,
                    thread::current().id()
                )))
            }
        }
    }

    /// Handle tokio RwLock read with timeout
    pub async fn handle_tokio_rwlock_read_with_timeout<'a, T>(
        rwlock: &'a tokio::sync::RwLock<T>,
        context: &str,
        config: &LockTimeoutConfig,
    ) -> Result<tokio::sync::RwLockReadGuard<'a, T>, FluentError> {
        let start_time = std::time::Instant::now();

        if config.log_timeout_events {
            if config.timeout.as_secs() < 60 {
                eprintln!(
                    "🔒 Acquiring RwLock read lock in {} with timeout: {:?}",
                    context, config.timeout
                );
            }
        }

        match tokio::time::timeout(config.timeout, rwlock.read()).await {
            Ok(guard) => {
                let elapsed = start_time.elapsed();
                if config.log_timeout_events && elapsed > Duration::from_millis(100) {
                    eprintln!(
                        "✅ Acquired RwLock read lock in {} after {:?}",
                        context, elapsed
                    );
                }
                Ok(guard)
            }
            Err(_) => {
                let elapsed = start_time.elapsed();
                if config.log_timeout_events {
                    eprintln!(
                        "⏰ RwLock read lock timeout in {} after {:?} (timeout: {:?}). Thread: {:?}",
                        context,
                        elapsed,
                        config.timeout,
                        thread::current().id()
                    );
                }
                Err(FluentError::LockTimeout(format!(
                    "RwLock read lock timeout in {} after {:?} (timeout: {:?}). Thread: {:?}",
                    context,
                    elapsed,
                    config.timeout,
                    thread::current().id()
                )))
            }
        }
    }

    /// Handle tokio RwLock write with timeout
    pub async fn handle_tokio_rwlock_write_with_timeout<'a, T>(
        rwlock: &'a tokio::sync::RwLock<T>,
        context: &str,
        config: &LockTimeoutConfig,
    ) -> Result<tokio::sync::RwLockWriteGuard<'a, T>, FluentError> {
        let start_time = std::time::Instant::now();

        if config.log_timeout_events {
            if config.timeout.as_secs() < 60 {
                eprintln!(
                    "🔒 Acquiring RwLock write lock in {} with timeout: {:?}",
                    context, config.timeout
                );
            }
        }

        match tokio::time::timeout(config.timeout, rwlock.write()).await {
            Ok(guard) => {
                let elapsed = start_time.elapsed();
                if config.log_timeout_events && elapsed > Duration::from_millis(100) {
                    eprintln!(
                        "✅ Acquired RwLock write lock in {} after {:?}",
                        context, elapsed
                    );
                }
                Ok(guard)
            }
            Err(_) => {
                let elapsed = start_time.elapsed();
                if config.log_timeout_events {
                    eprintln!(
                        "⏰ RwLock write lock timeout in {} after {:?} (timeout: {:?}). Thread: {:?}",
                        context,
                        elapsed,
                        config.timeout,
                        thread::current().id()
                    );
                }
                Err(FluentError::LockTimeout(format!(
                    "RwLock write lock timeout in {} after {:?} (timeout: {:?}). Thread: {:?}",
                    context,
                    elapsed,
                    config.timeout,
                    thread::current().id()
                )))
            }
        }
    }

    /// Create a thread-safe error with context
    pub fn create_error_with_context(
        error: FluentError,
        context: ErrorContext,
    ) -> FluentError {
        match error {
            FluentError::Internal(msg) => FluentError::Internal(format!(
                "{} | Context: Thread {:?} ({}), Operations: {:?}",
                msg,
                context.thread_id,
                context.thread_name.unwrap_or_else(|| "unnamed".to_string()),
                context.operation_stack
            )),
            other => other,
        }
    }
}

/// Macro for safe mutex locking with context
#[macro_export]
macro_rules! safe_lock {
    ($mutex:expr, $context:expr) => {
        $crate::error::ThreadSafeErrorHandler::handle_mutex_lock($mutex.lock(), $context)
    };
}

/// Macro for safe RwLock read locking with context
#[macro_export]
macro_rules! safe_read_lock {
    ($rwlock:expr, $context:expr) => {
        $crate::error::ThreadSafeErrorHandler::handle_rwlock_read($rwlock.read(), $context)
    };
}

/// Macro for safe RwLock write locking with context
#[macro_export]
macro_rules! safe_write_lock {
    ($rwlock:expr, $context:expr) => {
        $crate::error::ThreadSafeErrorHandler::handle_rwlock_write($rwlock.write(), $context)
    };
}

/// Macro for safe mutex locking with configurable poison handling
#[macro_export]
macro_rules! safe_lock_with_config {
    ($mutex:expr, $context:expr, $config:expr) => {
        $crate::error::ThreadSafeErrorHandler::handle_mutex_lock_with_config($mutex.lock(), $context, $config)
    };
}

/// Macro for safe mutex locking with retry
#[macro_export]
macro_rules! safe_lock_with_retry {
    ($mutex:expr, $context:expr, $config:expr, $operation:expr) => {
        $crate::error::ThreadSafeErrorHandler::handle_mutex_lock_with_retry($mutex, $context, $config, $operation)
    };
}

/// Macro for safe mutex locking with default value fallback
#[macro_export]
macro_rules! safe_lock_with_default {
    ($mutex:expr, $context:expr, $config:expr) => {
        $crate::error::ThreadSafeErrorHandler::handle_mutex_lock_with_default($mutex, $context, $config)
    };
}

/// Macro for poison-resistant mutex operation
#[macro_export]
macro_rules! poison_resistant_operation {
    ($mutex:expr, $context:expr, $operation:expr) => {{
        let config = $crate::error::PoisonHandlingConfig::recover_data();
        $crate::error::ThreadSafeErrorHandler::handle_mutex_lock_with_retry($mutex, $context, &config, $operation)
    }};
}

/// Macro for tokio mutex lock with timeout
#[macro_export]
macro_rules! safe_tokio_lock_with_timeout {
    ($mutex:expr, $context:expr, $timeout:expr) => {{
        let config = $crate::error::LockTimeoutConfig::with_timeout($timeout);
        $crate::error::ThreadSafeErrorHandler::handle_tokio_mutex_lock_with_timeout($mutex, $context, &config).await
    }};
}

/// Macro for tokio RwLock read with timeout
#[macro_export]
macro_rules! safe_tokio_read_lock_with_timeout {
    ($rwlock:expr, $context:expr, $timeout:expr) => {{
        let config = $crate::error::LockTimeoutConfig::with_timeout($timeout);
        $crate::error::ThreadSafeErrorHandler::handle_tokio_rwlock_read_with_timeout($rwlock, $context, &config).await
    }};
}

/// Macro for tokio RwLock write with timeout
#[macro_export]
macro_rules! safe_tokio_write_lock_with_timeout {
    ($rwlock:expr, $context:expr, $timeout:expr) => {{
        let config = $crate::error::LockTimeoutConfig::with_timeout($timeout);
        $crate::error::ThreadSafeErrorHandler::handle_tokio_rwlock_write_with_timeout($rwlock, $context, &config).await
    }};
}

/// Macro for short timeout operations (5 seconds)
#[macro_export]
macro_rules! safe_tokio_lock_short_timeout {
    ($mutex:expr, $context:expr) => {{
        let config = $crate::error::LockTimeoutConfig::short_timeout();
        $crate::error::ThreadSafeErrorHandler::handle_tokio_mutex_lock_with_timeout($mutex, $context, &config).await
    }};
}

/// Macro for medium timeout operations (30 seconds)
#[macro_export]
macro_rules! safe_tokio_lock_medium_timeout {
    ($mutex:expr, $context:expr) => {{
        let config = $crate::error::LockTimeoutConfig::medium_timeout();
        $crate::error::ThreadSafeErrorHandler::handle_tokio_mutex_lock_with_timeout($mutex, $context, &config).await
    }};
}

/// Macro for long timeout operations (2 minutes)
#[macro_export]
macro_rules! safe_tokio_lock_long_timeout {
    ($mutex:expr, $context:expr) => {{
        let config = $crate::error::LockTimeoutConfig::long_timeout();
        $crate::error::ThreadSafeErrorHandler::handle_tokio_mutex_lock_with_timeout($mutex, $context, &config).await
    }};
}

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
                err.url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            ))
        } else if err.is_connect() {
            FluentError::Network(NetworkError::NetworkUnreachable)
        } else if let Some(status) = err.status() {
            FluentError::Network(NetworkError::RequestFailed {
                url: err
                    .url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                status: Some(status.as_u16()),
                message: err.to_string(),
            })
        } else {
            FluentError::Network(NetworkError::RequestFailed {
                url: err
                    .url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
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
            std::io::ErrorKind::PermissionDenied => {
                FluentError::File(FileError::PermissionDenied(err.to_string()))
            }
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
