//! Circuit Breaker Pattern Implementation
//!
//! Provides a production-ready circuit breaker for protecting against cascading failures.
//! The circuit breaker monitors call failures and "trips" (opens) when failures exceed
//! a threshold, preventing further calls until a timeout period passes.
//!
//! ## States
//!
//! - **Closed**: Normal operation, calls pass through
//! - **Open**: Circuit is tripped, calls fail immediately
//! - **HalfOpen**: Testing if service recovered, limited calls allowed
//!
//! ## Usage
//!
//! ```rust,ignore
//! use fluent_agent::monitoring::CircuitBreaker;
//!
//! let breaker = CircuitBreaker::new("api_service", CircuitBreakerConfig::default());
//!
//! // Use the circuit breaker
//! if breaker.can_execute() {
//!     match make_api_call().await {
//!         Ok(result) => breaker.record_success(),
//!         Err(e) => breaker.record_failure(),
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation - calls pass through
    Closed,
    /// Circuit tripped - calls fail immediately
    Open,
    /// Testing recovery - limited calls allowed
    HalfOpen,
}

impl Default for CircuitState {
    fn default() -> Self {
        Self::Closed
    }
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of successes in half-open state before closing
    pub success_threshold: u32,
    /// Duration to wait before transitioning from open to half-open
    pub timeout: Duration,
    /// Maximum number of calls allowed in half-open state
    pub half_open_max_calls: u32,
    /// Window duration for counting failures (rolling window)
    pub failure_window: Duration,
    /// Name/identifier for this circuit breaker
    pub name: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
            half_open_max_calls: 3,
            failure_window: Duration::from_secs(60),
            name: "default".to_string(),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new config with a specific name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Builder pattern for failure threshold
    pub fn failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Builder pattern for success threshold
    pub fn success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Builder pattern for timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builder pattern for half-open max calls
    pub fn half_open_max_calls(mut self, max_calls: u32) -> Self {
        self.half_open_max_calls = max_calls;
        self
    }
}

/// Statistics about circuit breaker operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CircuitBreakerStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub rejected_calls: u64,
    pub state_transitions: u32,
    pub last_failure_time: Option<SystemTime>,
    pub last_success_time: Option<SystemTime>,
    pub last_state_change: Option<SystemTime>,
    pub current_state: CircuitState,
}

/// Thread-safe circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    half_open_calls: AtomicU32,
    last_failure_time: RwLock<Option<Instant>>,
    opened_at: RwLock<Option<Instant>>,
    // Statistics
    total_calls: AtomicU64,
    successful_calls: AtomicU64,
    failed_calls: AtomicU64,
    rejected_calls: AtomicU64,
    state_transitions: AtomicU32,
    last_state_change: RwLock<Option<SystemTime>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            half_open_calls: AtomicU32::new(0),
            last_failure_time: RwLock::new(None),
            opened_at: RwLock::new(None),
            total_calls: AtomicU64::new(0),
            successful_calls: AtomicU64::new(0),
            failed_calls: AtomicU64::new(0),
            rejected_calls: AtomicU64::new(0),
            state_transitions: AtomicU32::new(0),
            last_state_change: RwLock::new(None),
        }
    }

    /// Create a circuit breaker with default config and a name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self::new(CircuitBreakerConfig::with_name(name))
    }

    /// Get the current state of the circuit breaker
    pub fn state(&self) -> CircuitState {
        self.maybe_transition_state();
        *self.state.read().unwrap()
    }

    /// Check if a call can be executed
    ///
    /// Returns true if the circuit is closed or half-open with capacity
    pub fn can_execute(&self) -> bool {
        self.maybe_transition_state();
        self.total_calls.fetch_add(1, Ordering::SeqCst);

        let state = *self.state.read().unwrap();
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                self.rejected_calls.fetch_add(1, Ordering::SeqCst);
                false
            }
            CircuitState::HalfOpen => {
                let current = self.half_open_calls.fetch_add(1, Ordering::SeqCst);
                if current < self.config.half_open_max_calls {
                    true
                } else {
                    self.rejected_calls.fetch_add(1, Ordering::SeqCst);
                    false
                }
            }
        }
    }

    /// Record a successful call
    pub fn record_success(&self) {
        self.successful_calls.fetch_add(1, Ordering::SeqCst);

        let state = *self.state.read().unwrap();
        match state {
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if successes >= self.config.success_threshold {
                    self.transition_to(CircuitState::Closed);
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but record anyway
            }
        }
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        self.failed_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_failure_time.write().unwrap() = Some(Instant::now());

        let state = *self.state.read().unwrap();
        match state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                if failures >= self.config.failure_threshold {
                    self.transition_to(CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open returns to open
                self.transition_to(CircuitState::Open);
            }
            CircuitState::Open => {
                // Already open, just update opened_at
                *self.opened_at.write().unwrap() = Some(Instant::now());
            }
        }
    }

    /// Force the circuit to open
    pub fn trip(&self) {
        self.transition_to(CircuitState::Open);
    }

    /// Force the circuit to close (reset)
    pub fn reset(&self) {
        self.transition_to(CircuitState::Closed);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        self.half_open_calls.store(0, Ordering::SeqCst);
    }

    /// Get statistics about this circuit breaker
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            total_calls: self.total_calls.load(Ordering::SeqCst),
            successful_calls: self.successful_calls.load(Ordering::SeqCst),
            failed_calls: self.failed_calls.load(Ordering::SeqCst),
            rejected_calls: self.rejected_calls.load(Ordering::SeqCst),
            state_transitions: self.state_transitions.load(Ordering::SeqCst),
            last_failure_time: self
                .last_failure_time
                .read()
                .unwrap()
                .map(|_| SystemTime::now()),
            last_success_time: None, // Would need to track this separately
            last_state_change: *self.last_state_change.read().unwrap(),
            current_state: self.state(),
        }
    }

    /// Get the circuit breaker name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Check and perform automatic state transitions
    fn maybe_transition_state(&self) {
        let state = *self.state.read().unwrap();

        if state == CircuitState::Open {
            // Check if timeout has elapsed
            if let Some(opened_at) = *self.opened_at.read().unwrap() {
                if opened_at.elapsed() >= self.config.timeout {
                    self.transition_to(CircuitState::HalfOpen);
                }
            }
        }

        // Check if we should reset failure count due to window expiration
        if state == CircuitState::Closed {
            if let Some(last_failure) = *self.last_failure_time.read().unwrap() {
                if last_failure.elapsed() > self.config.failure_window {
                    self.failure_count.store(0, Ordering::SeqCst);
                }
            }
        }
    }

    /// Transition to a new state
    fn transition_to(&self, new_state: CircuitState) {
        let mut state = self.state.write().unwrap();
        if *state != new_state {
            *state = new_state;
            self.state_transitions.fetch_add(1, Ordering::SeqCst);
            *self.last_state_change.write().unwrap() = Some(SystemTime::now());

            // Reset counters on state transition
            match new_state {
                CircuitState::Closed => {
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                }
                CircuitState::Open => {
                    *self.opened_at.write().unwrap() = Some(Instant::now());
                }
                CircuitState::HalfOpen => {
                    self.success_count.store(0, Ordering::SeqCst);
                    self.half_open_calls.store(0, Ordering::SeqCst);
                }
            }

            tracing::info!(
                "circuit_breaker.state_change name={} new_state={:?}",
                self.config.name,
                new_state
            );
        }
    }
}

/// Execute a function with circuit breaker protection
pub async fn with_circuit_breaker<F, T, E>(
    breaker: &CircuitBreaker,
    f: F,
) -> Result<T, CircuitBreakerError<E>>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    if !breaker.can_execute() {
        return Err(CircuitBreakerError::CircuitOpen);
    }

    match f.await {
        Ok(result) => {
            breaker.record_success();
            Ok(result)
        }
        Err(e) => {
            breaker.record_failure();
            Err(CircuitBreakerError::OperationFailed(e))
        }
    }
}

/// Error type for circuit breaker operations
#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    /// The circuit is open and rejecting calls
    CircuitOpen,
    /// The underlying operation failed
    OperationFailed(E),
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitOpen => write!(f, "Circuit breaker is open"),
            Self::OperationFailed(e) => write!(f, "Operation failed: {}", e),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CircuitOpen => None,
            Self::OperationFailed(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let breaker = CircuitBreaker::with_name("test");
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.can_execute());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let config = CircuitBreakerConfig::with_name("test").failure_threshold(3);
        let breaker = CircuitBreaker::new(config);

        // First 2 failures should not trip
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        // Third failure should trip
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.can_execute());
    }

    #[test]
    fn test_circuit_breaker_success_resets_failure_count() {
        let config = CircuitBreakerConfig::with_name("test").failure_threshold(3);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure();
        breaker.record_failure();
        breaker.record_success(); // Should reset

        // Now need 3 more failures
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_success() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(1)
            .success_threshold(2)
            .timeout(Duration::from_millis(1));
        let breaker = CircuitBreaker::new(config);

        // Trip the circuit
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(5));

        // Should transition to half-open
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Two successes should close it
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::HalfOpen);
        breaker.record_success();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_half_open_failure() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(1)
            .timeout(Duration::from_millis(1));
        let breaker = CircuitBreaker::new(config);

        // Trip the circuit
        breaker.record_failure();

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(5));

        // Should be half-open
        assert_eq!(breaker.state(), CircuitState::HalfOpen);

        // Failure in half-open should reopen
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_stats() {
        let breaker = CircuitBreaker::with_name("test");

        breaker.can_execute();
        breaker.record_success();
        breaker.can_execute();
        breaker.record_failure();

        let stats = breaker.stats();
        assert_eq!(stats.total_calls, 2);
        assert_eq!(stats.successful_calls, 1);
        assert_eq!(stats.failed_calls, 1);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let config = CircuitBreakerConfig::with_name("test").failure_threshold(1);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.can_execute());
    }

    #[test]
    fn test_circuit_breaker_trip() {
        let breaker = CircuitBreaker::with_name("test");

        assert_eq!(breaker.state(), CircuitState::Closed);
        breaker.trip();
        assert_eq!(breaker.state(), CircuitState::Open);
    }
}
