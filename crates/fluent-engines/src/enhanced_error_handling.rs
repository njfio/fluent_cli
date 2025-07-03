use anyhow::Result;
use fluent_core::error::FluentError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Enhanced error handling system with recovery strategies and detailed context
///
/// This system provides:
/// - Structured error context and metadata
/// - Error recovery strategies and retry mechanisms
/// - Error aggregation and correlation
/// - Performance impact tracking
/// - User-friendly error messages
/// - Debugging and troubleshooting information

/// Error context with detailed metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub error_id: String,
    pub timestamp: String,
    pub component: String,
    pub operation: String,
    pub user_message: String,
    pub technical_details: String,
    pub recovery_suggestions: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub correlation_id: Option<String>,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,      // Minor issues, system continues normally
    Medium,   // Noticeable issues, some functionality affected
    High,     // Significant issues, major functionality affected
    Critical, // System-threatening issues, immediate attention required
}

/// Error categories for better organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    UserError,        // User input or configuration issues
    SystemError,      // Internal system failures
    ExternalError,    // Third-party service failures
    SecurityError,    // Security-related issues
    PerformanceError, // Performance degradation
}

/// Recovery strategy for handling errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Retry with exponential backoff
    Retry {
        max_attempts: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        backoff_multiplier: f64,
    },
    /// Fallback to alternative implementation
    Fallback {
        fallback_component: String,
        fallback_config: HashMap<String, String>,
    },
    /// Circuit breaker pattern
    CircuitBreaker {
        failure_threshold: u32,
        timeout_ms: u64,
        recovery_timeout_ms: u64,
    },
    /// Graceful degradation
    Degrade {
        reduced_functionality: Vec<String>,
        degradation_level: u32,
    },
    /// Manual intervention required
    Manual {
        escalation_contact: String,
        urgency_level: ErrorSeverity,
    },
}

/// Enhanced error with context and recovery information
#[derive(Debug)]
pub struct EnhancedError {
    pub base_error: FluentError,
    pub context: ErrorContext,
    pub severity: ErrorSeverity,
    pub category: ErrorCategory,
    pub recovery_strategy: Option<RecoveryStrategy>,
    pub occurred_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
    pub resolution_notes: Option<String>,
}

/// Serializable version of EnhancedError for export
#[derive(Debug, Serialize)]
pub struct SerializableError {
    pub error_id: String,
    pub timestamp: String,
    pub component: String,
    pub operation: String,
    pub severity: ErrorSeverity,
    pub category: ErrorCategory,
    pub user_message: String,
    pub technical_details: String,
    pub recovery_suggestions: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub resolved: bool,
    pub resolution_notes: Option<String>,
}

/// Error aggregator for collecting and analyzing errors
pub struct ErrorAggregator {
    errors: Arc<RwLock<Vec<EnhancedError>>>,
    error_counts: Arc<RwLock<HashMap<String, u32>>>,
    recovery_stats: Arc<RwLock<HashMap<String, RecoveryStats>>>,
}

/// Recovery statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStats {
    pub total_attempts: u32,
    pub successful_recoveries: u32,
    pub failed_recoveries: u32,
    pub average_recovery_time_ms: f64,
    pub last_recovery_attempt: Option<String>,
}

/// Error handler with recovery capabilities
pub struct ErrorHandler {
    pub aggregator: ErrorAggregator,
    recovery_enabled: bool,
    max_recovery_attempts: u32,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            error_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            component: component.to_string(),
            operation: operation.to_string(),
            user_message: String::new(),
            technical_details: String::new(),
            recovery_suggestions: Vec::new(),
            metadata: HashMap::new(),
            correlation_id: None,
            request_id: None,
            user_id: None,
            session_id: None,
        }
    }

    /// Add user-friendly message
    pub fn with_user_message(mut self, message: &str) -> Self {
        self.user_message = message.to_string();
        self
    }

    /// Add technical details
    pub fn with_technical_details(mut self, details: &str) -> Self {
        self.technical_details = details.to_string();
        self
    }

    /// Add recovery suggestion
    pub fn with_recovery_suggestion(mut self, suggestion: &str) -> Self {
        self.recovery_suggestions.push(suggestion.to_string());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Add correlation ID for request tracing
    pub fn with_correlation_id(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }
}

impl EnhancedError {
    /// Create a new enhanced error
    pub fn new(
        base_error: FluentError,
        context: ErrorContext,
        severity: ErrorSeverity,
        category: ErrorCategory,
    ) -> Self {
        Self {
            base_error,
            context,
            severity,
            category,
            recovery_strategy: None,
            occurred_at: SystemTime::now(),
            resolved_at: None,
            resolution_notes: None,
        }
    }

    /// Add recovery strategy
    pub fn with_recovery_strategy(mut self, strategy: RecoveryStrategy) -> Self {
        self.recovery_strategy = Some(strategy);
        self
    }

    /// Mark error as resolved
    pub fn mark_resolved(mut self, notes: Option<String>) -> Self {
        self.resolved_at = Some(SystemTime::now());
        self.resolution_notes = notes;
        self
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        if !self.context.user_message.is_empty() {
            self.context.user_message.clone()
        } else {
            match &self.base_error {
                FluentError::Network(_) => "Network connection issue. Please check your internet connection and try again.".to_string(),
                FluentError::Auth(_) => "Authentication failed. Please check your credentials and try again.".to_string(),
                FluentError::Engine(_) => "Service temporarily unavailable. Please try again in a few moments.".to_string(),
                FluentError::Validation(_) => "Invalid input provided. Please check your input and try again.".to_string(),
                FluentError::File(_) => "File operation failed. Please check file permissions and try again.".to_string(),
                _ => "An unexpected error occurred. Please try again or contact support if the issue persists.".to_string(),
            }
        }
    }

    /// Get technical error details
    pub fn technical_details(&self) -> String {
        if !self.context.technical_details.is_empty() {
            format!(
                "{}\n\nOriginal error: {}",
                self.context.technical_details, self.base_error
            )
        } else {
            self.base_error.to_string()
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.recovery_strategy.is_some()
            && matches!(self.severity, ErrorSeverity::Low | ErrorSeverity::Medium)
    }

    /// Convert to serializable format
    pub fn to_serializable(&self) -> SerializableError {
        SerializableError {
            error_id: self.context.error_id.clone(),
            timestamp: self.context.timestamp.clone(),
            component: self.context.component.clone(),
            operation: self.context.operation.clone(),
            severity: self.severity,
            category: self.category.clone(),
            user_message: self.user_message(),
            technical_details: self.technical_details(),
            recovery_suggestions: self.context.recovery_suggestions.clone(),
            metadata: self.context.metadata.clone(),
            resolved: self.resolved_at.is_some(),
            resolution_notes: self.resolution_notes.clone(),
        }
    }
}

impl ErrorAggregator {
    /// Create a new error aggregator
    pub fn new() -> Self {
        Self {
            errors: Arc::new(RwLock::new(Vec::new())),
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            recovery_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record an error
    pub async fn record_error(&self, error: EnhancedError) {
        let error_type = format!(
            "{}::{}",
            error.category.clone() as u8,
            error.context.component.clone()
        );

        // Update error counts
        {
            let mut counts = self.error_counts.write().await;
            *counts.entry(error_type.clone()).or_insert(0) += 1;
        }

        // Store error
        {
            let mut errors = self.errors.write().await;
            errors.push(error);

            // Keep only last 1000 errors to prevent memory growth
            let len = errors.len();
            if len > 1000 {
                errors.drain(0..len - 1000);
            }
        }
    }

    /// Get error statistics
    pub async fn get_error_stats(&self) -> HashMap<String, u32> {
        self.error_counts.read().await.clone()
    }

    /// Get recent errors
    pub async fn get_recent_errors(&self, limit: usize) -> Vec<EnhancedError> {
        let errors = self.errors.read().await;
        errors.iter().rev().take(limit).cloned().collect()
    }

    /// Update recovery statistics
    pub async fn update_recovery_stats(
        &self,
        strategy_type: &str,
        success: bool,
        duration_ms: u64,
    ) {
        let mut stats = self.recovery_stats.write().await;
        let recovery_stats = stats
            .entry(strategy_type.to_string())
            .or_insert(RecoveryStats {
                total_attempts: 0,
                successful_recoveries: 0,
                failed_recoveries: 0,
                average_recovery_time_ms: 0.0,
                last_recovery_attempt: None,
            });

        recovery_stats.total_attempts += 1;
        if success {
            recovery_stats.successful_recoveries += 1;
        } else {
            recovery_stats.failed_recoveries += 1;
        }

        // Update average recovery time
        let total_time =
            recovery_stats.average_recovery_time_ms * (recovery_stats.total_attempts - 1) as f64;
        recovery_stats.average_recovery_time_ms =
            (total_time + duration_ms as f64) / recovery_stats.total_attempts as f64;
        recovery_stats.last_recovery_attempt = Some(chrono::Utc::now().to_rfc3339());
    }
}

impl ErrorHandler {
    /// Create a new error handler
    pub fn new(recovery_enabled: bool, max_recovery_attempts: u32) -> Self {
        Self {
            aggregator: ErrorAggregator::new(),
            recovery_enabled,
            max_recovery_attempts,
        }
    }

    /// Handle an error with potential recovery
    pub async fn handle_error(
        &self,
        error: EnhancedError,
    ) -> Result<Option<String>, EnhancedError> {
        // Record the error
        self.aggregator.record_error(error.clone()).await;

        // Attempt recovery if enabled and strategy is available
        if self.recovery_enabled && error.is_recoverable() {
            if let Some(strategy) = error.recovery_strategy.clone() {
                return self.attempt_recovery(error, &strategy).await;
            }
        }

        // No recovery possible, return the error
        Err(error)
    }

    /// Attempt error recovery based on strategy
    async fn attempt_recovery(
        &self,
        error: EnhancedError,
        strategy: &RecoveryStrategy,
    ) -> Result<Option<String>, EnhancedError> {
        let start_time = SystemTime::now();

        let result = match strategy {
            RecoveryStrategy::Retry {
                max_attempts,
                base_delay_ms,
                max_delay_ms,
                backoff_multiplier,
            } => {
                self.retry_with_backoff(
                    *max_attempts,
                    *base_delay_ms,
                    *max_delay_ms,
                    *backoff_multiplier,
                )
                .await
            }
            RecoveryStrategy::Fallback {
                fallback_component,
                fallback_config,
            } => {
                self.fallback_recovery(fallback_component, fallback_config)
                    .await
            }
            RecoveryStrategy::CircuitBreaker {
                failure_threshold,
                timeout_ms,
                recovery_timeout_ms,
            } => {
                self.circuit_breaker_recovery(*failure_threshold, *timeout_ms, *recovery_timeout_ms)
                    .await
            }
            RecoveryStrategy::Degrade {
                reduced_functionality,
                degradation_level,
            } => {
                self.graceful_degradation(reduced_functionality, *degradation_level)
                    .await
            }
            RecoveryStrategy::Manual {
                escalation_contact,
                urgency_level,
            } => {
                self.manual_escalation(escalation_contact, *urgency_level)
                    .await
            }
        };

        // Update recovery statistics
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let strategy_type = match strategy {
            RecoveryStrategy::Retry { .. } => "retry",
            RecoveryStrategy::Fallback { .. } => "fallback",
            RecoveryStrategy::CircuitBreaker { .. } => "circuit_breaker",
            RecoveryStrategy::Degrade { .. } => "degrade",
            RecoveryStrategy::Manual { .. } => "manual",
        };

        self.aggregator
            .update_recovery_stats(strategy_type, result.is_ok(), duration)
            .await;

        result.map_err(|_| error)
    }

    async fn retry_with_backoff(
        &self,
        max_attempts: u32,
        base_delay_ms: u64,
        max_delay_ms: u64,
        backoff_multiplier: f64,
    ) -> Result<Option<String>, ()> {
        for attempt in 1..=max_attempts {
            if attempt > 1 {
                let delay = std::cmp::min(
                    (base_delay_ms as f64 * backoff_multiplier.powi(attempt as i32 - 1)) as u64,
                    max_delay_ms,
                );
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            // Simulate retry logic (in real implementation, this would retry the original operation)
            if attempt == max_attempts {
                return Ok(Some(format!("Recovered after {} attempts", attempt)));
            }
        }
        Err(())
    }

    async fn fallback_recovery(
        &self,
        _fallback_component: &str,
        _fallback_config: &HashMap<String, String>,
    ) -> Result<Option<String>, ()> {
        // Simulate fallback logic
        Ok(Some("Fallback recovery successful".to_string()))
    }

    async fn circuit_breaker_recovery(
        &self,
        _failure_threshold: u32,
        _timeout_ms: u64,
        _recovery_timeout_ms: u64,
    ) -> Result<Option<String>, ()> {
        // Simulate circuit breaker logic
        Ok(Some("Circuit breaker recovery successful".to_string()))
    }

    async fn graceful_degradation(
        &self,
        _reduced_functionality: &[String],
        _degradation_level: u32,
    ) -> Result<Option<String>, ()> {
        // Simulate graceful degradation
        Ok(Some("Graceful degradation applied".to_string()))
    }

    async fn manual_escalation(
        &self,
        _escalation_contact: &str,
        _urgency_level: ErrorSeverity,
    ) -> Result<Option<String>, ()> {
        // Simulate manual escalation
        Ok(Some("Manual escalation initiated".to_string()))
    }

    /// Get error handler statistics
    pub async fn get_stats(&self) -> ErrorHandlerStats {
        let error_stats = self.aggregator.get_error_stats().await;
        let recovery_stats = self.aggregator.recovery_stats.read().await.clone();

        ErrorHandlerStats {
            total_errors: error_stats.values().sum(),
            error_breakdown: error_stats,
            recovery_stats,
            recovery_enabled: self.recovery_enabled,
            max_recovery_attempts: self.max_recovery_attempts,
        }
    }
}

/// Error handler statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorHandlerStats {
    pub total_errors: u32,
    pub error_breakdown: HashMap<String, u32>,
    pub recovery_stats: HashMap<String, RecoveryStats>,
    pub recovery_enabled: bool,
    pub max_recovery_attempts: u32,
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.context.error_id,
            self.context.component,
            self.user_message()
        )
    }
}

impl Clone for EnhancedError {
    fn clone(&self) -> Self {
        // Create a new FluentError based on the original
        let base_error = match &self.base_error {
            FluentError::Config(e) => FluentError::Internal(format!("Config error: {}", e)),
            FluentError::Auth(e) => FluentError::Internal(format!("Auth error: {}", e)),
            FluentError::Network(e) => FluentError::Internal(format!("Network error: {}", e)),
            FluentError::Engine(e) => FluentError::Internal(format!("Engine error: {}", e)),
            FluentError::Pipeline(e) => FluentError::Internal(format!("Pipeline error: {}", e)),
            FluentError::File(e) => FluentError::Internal(format!("File error: {}", e)),
            FluentError::Validation(e) => FluentError::Internal(format!("Validation error: {}", e)),
            FluentError::Cost(e) => FluentError::Internal(format!("Cost error: {}", e)),
            FluentError::Storage(e) => FluentError::Internal(format!("Storage error: {}", e)),
            FluentError::Cache(e) => FluentError::Internal(format!("Cache error: {}", e)),
            FluentError::Internal(s) => FluentError::Internal(s.clone()),
        };

        Self {
            base_error,
            context: self.context.clone(),
            severity: self.severity,
            category: self.category.clone(),
            recovery_strategy: self.recovery_strategy.clone(),
            occurred_at: self.occurred_at,
            resolved_at: self.resolved_at,
            resolution_notes: self.resolution_notes.clone(),
        }
    }
}

/// Helper macros for creating enhanced errors
#[macro_export]
macro_rules! enhanced_error {
    ($base_error:expr, $component:expr, $operation:expr, $severity:expr, $category:expr) => {
        EnhancedError::new(
            $base_error,
            ErrorContext::new($component, $operation),
            $severity,
            $category,
        )
    };
}

#[macro_export]
macro_rules! user_error {
    ($message:expr, $component:expr, $operation:expr) => {
        enhanced_error!(
            FluentError::Validation(ValidationError::InvalidFormat {
                input: "user_input".to_string(),
                expected: "valid_input".to_string(),
            }),
            $component,
            $operation,
            ErrorSeverity::Low,
            ErrorCategory::UserError
        )
        .with_user_message($message)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::error::{FluentError, ValidationError};

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("test_component", "test_operation")
            .with_user_message("Test user message")
            .with_technical_details("Test technical details")
            .with_recovery_suggestion("Try again later")
            .with_metadata("key", "value")
            .with_correlation_id("test-correlation-id");

        assert_eq!(context.component, "test_component");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.user_message, "Test user message");
        assert_eq!(context.technical_details, "Test technical details");
        assert_eq!(context.recovery_suggestions, vec!["Try again later"]);
        assert_eq!(context.metadata.get("key"), Some(&"value".to_string()));
        assert_eq!(
            context.correlation_id,
            Some("test-correlation-id".to_string())
        );
    }

    #[test]
    fn test_enhanced_error_creation() {
        let base_error = FluentError::Validation(ValidationError::InvalidFormat {
            input: "test".to_string(),
            expected: "valid".to_string(),
        });
        let context = ErrorContext::new("test", "test");
        let error = EnhancedError::new(
            base_error,
            context,
            ErrorSeverity::Medium,
            ErrorCategory::UserError,
        );

        assert_eq!(error.severity, ErrorSeverity::Medium);
        assert_eq!(error.category, ErrorCategory::UserError);
        assert!(error.recovery_strategy.is_none());
    }

    #[tokio::test]
    async fn test_error_aggregator() {
        let aggregator = ErrorAggregator::new();
        let base_error = FluentError::Validation(ValidationError::InvalidFormat {
            input: "test".to_string(),
            expected: "valid".to_string(),
        });
        let context = ErrorContext::new("test", "test");
        let error = EnhancedError::new(
            base_error,
            context,
            ErrorSeverity::Low,
            ErrorCategory::UserError,
        );

        aggregator.record_error(error).await;
        let stats = aggregator.get_error_stats().await;
        assert_eq!(stats.get("0::test"), Some(&1));
    }

    #[tokio::test]
    async fn test_error_handler() {
        let handler = ErrorHandler::new(true, 3);
        let base_error = FluentError::Validation(ValidationError::InvalidFormat {
            input: "test".to_string(),
            expected: "valid".to_string(),
        });
        let context = ErrorContext::new("test", "test");
        let error = EnhancedError::new(
            base_error,
            context,
            ErrorSeverity::Low,
            ErrorCategory::UserError,
        )
        .with_recovery_strategy(RecoveryStrategy::Retry {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 1000,
            backoff_multiplier: 2.0,
        });

        let result = handler.handle_error(error).await;
        assert!(result.is_ok());
    }
}
