//! Unified Execution Loop Abstraction
//!
//! This module provides a common trait for different execution loop patterns:
//! - ReAct loops (Reasoning-Acting-Observing cycles)
//! - Task-based loops (with todo/goal tracking)
//! - DAG-based execution (dependency resolution)
//! - Linear pipelines (sequential steps)
//!
//! # Design Principles
//! 1. **Separation of Concerns**: Loop control separate from step execution
//! 2. **State Abstraction**: Associated type for flexible state representation
//! 3. **Completion Detection**: Domain-specific completion criteria
//! 4. **Error Resilience**: Built-in error handling and recovery patterns
//! 5. **Observability**: Queryable state and metrics

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Unified trait for different execution loop patterns
///
/// This trait abstracts over different execution models used throughout the codebase,
/// providing a consistent interface for loop control, state management, and completion detection.
#[async_trait]
pub trait ExecutionLoop: Send + Sync {
    /// Opaque state type that the executor maintains
    /// This can be AgentState, WorkflowContext, PipelineState, etc.
    type State: Send + Sync;

    // ====== Initialization ======

    /// Initialize the execution loop with inputs
    ///
    /// This must be called before any step execution.
    /// Implementations should set up initial state, validate inputs, etc.
    async fn initialize(&mut self) -> Result<()>;

    // ====== Single Step Execution ======

    /// Execute a single step and return the result
    ///
    /// This is the core primitive for extensibility.
    /// Should be idempotent where possible (allows retries).
    ///
    /// # Behavior
    /// - Updates internal state with step result
    /// - Records metrics/observations
    /// - Does NOT check completion (that's is_complete's job)
    async fn execute_step(&mut self) -> Result<StepResult>;

    /// Determine if this step (or iteration) is retryable
    ///
    /// Used by callers to decide whether to call execute_step again.
    fn is_step_retryable(&self) -> bool {
        true // Default: steps are retryable
    }

    /// Get the current step identifier (for logging/debugging)
    fn current_step_id(&self) -> String;

    // ====== Iteration Control ======

    /// Check if the main loop should continue
    ///
    /// Returns `true` if there are more steps to execute.
    /// Used by callers to control the main `while` or `for` loop.
    ///
    /// # Examples of False cases
    /// - All items in ready queue have been processed (DAG)
    /// - Reached max iterations (bounded loop)
    /// - All todos completed successfully (task-based)
    /// - Iterator is exhausted (sequential)
    fn should_continue(&self) -> bool;

    /// Check if the loop is in a retryable error state
    ///
    /// Returns `true` if the last operation failed but can be retried.
    /// This guides exponential backoff and retry policies.
    fn is_retryable_error(&self) -> bool {
        false // Default: errors are not retryable
    }

    // ====== Completion Criteria ======

    /// Check if the overall goal/workflow is complete
    ///
    /// This is the key completion signal that indicates success.
    /// Implementations may check:
    /// - Multi-signal weighted scoring
    /// - Explicit success criteria
    /// - File creation/output verification
    /// - Goal achievement confidence thresholds
    ///
    /// # Returns
    /// - `Ok(true)`: Goal is achieved, loop can exit
    /// - `Ok(false)`: Goal not yet achieved, continue looping
    /// - `Err`: Unrecoverable error occurred
    fn is_complete(&self) -> Result<bool>;

    /// Check if execution should be terminated immediately
    ///
    /// Reasons for early termination:
    /// - Timeout exceeded
    /// - Resource exhaustion
    /// - Convergence detected (stuck in loop)
    /// - User cancellation
    fn should_terminate(&self) -> Result<bool> {
        Ok(false) // Default: don't terminate
    }

    // ====== State Management ======

    /// Get a reference to the current execution state
    ///
    /// Used for observability, checkpointing, and decision-making.
    fn get_state(&self) -> &Self::State;

    /// Get mutable access to state
    ///
    /// Called by step executors to update context/observations.
    fn get_state_mut(&mut self) -> &mut Self::State;

    /// Save the current execution state (for resumption)
    ///
    /// Optional: Only needed if the executor supports checkpointing.
    async fn save_checkpoint(&self) -> Result<String> {
        Ok(String::new()) // Default: no-op
    }

    /// Load a previously saved execution state
    ///
    /// Optional: Only needed if the executor supports resumption.
    async fn restore_checkpoint(&mut self, _id: &str) -> Result<()> {
        Ok(()) // Default: no-op
    }

    // ====== Iteration Information ======

    /// Get the current iteration number (1-indexed)
    fn iteration(&self) -> u32;

    /// Get the maximum iteration count (if bounded)
    ///
    /// Returns None for unbounded loops.
    fn max_iterations(&self) -> Option<u32>;

    /// Get elapsed time since loop start
    fn elapsed_time(&self) -> Duration;

    // ====== Error Handling ======

    /// Handle an error from the last step execution
    ///
    /// Implementations should decide on:
    /// - Whether the error is retryable
    /// - Whether to collect it for reporting
    /// - Whether to apply backoff before retry
    async fn handle_error(&mut self, error: anyhow::Error) -> Result<()>;

    /// Reset error state (for retry attempts)
    fn reset_error_state(&mut self);

    // ====== Metrics & Observability ======

    /// Get execution metrics (for monitoring/debugging)
    ///
    /// Returns a JSON value with loop-specific metrics:
    /// - iterations_completed
    /// - steps_executed
    /// - total_duration
    /// - error_count
    /// - retry_count
    /// - success_rate (for task-based loops)
    fn get_metrics(&self) -> serde_json::Value;

    /// Get recent observations/logs (last N items)
    fn get_recent_observations(&self, n: usize) -> Vec<String>;
}

/// Result of executing a single step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Unique identifier for this step execution
    pub step_id: String,
    /// Whether the step succeeded
    pub success: bool,
    /// Output/observation from the step
    pub output: String,
    /// Duration of step execution
    pub duration: Duration,
    /// Optional error message if step failed
    pub error: Option<String>,
    /// Metadata about the step
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl StepResult {
    /// Create a successful step result
    pub fn success(step_id: impl Into<String>, output: impl Into<String>, duration: Duration) -> Self {
        Self {
            step_id: step_id.into(),
            success: true,
            output: output.into(),
            duration,
            error: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a failed step result
    pub fn failure(step_id: impl Into<String>, error: impl Into<String>, duration: Duration) -> Self {
        Self {
            step_id: step_id.into(),
            success: false,
            output: String::new(),
            duration,
            error: Some(error.into()),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata to the step result
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Unified execution state that can represent any executor's state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    /// Current iteration number
    pub iteration: u32,
    /// Maximum iterations (if bounded)
    pub max_iterations: Option<u32>,
    /// When execution started
    pub started_at: std::time::SystemTime,
    /// Current step identifier
    pub current_step: String,
    /// Status of the execution
    pub status: ExecutionStatus,
    /// Recent observations (sliding window)
    pub recent_observations: Vec<String>,
    /// Error count
    pub error_count: u32,
    /// Retry count
    pub retry_count: u32,
    /// Custom state data (domain-specific)
    pub custom_data: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self {
            iteration: 0,
            max_iterations: None,
            started_at: std::time::SystemTime::now(),
            current_step: String::new(),
            status: ExecutionStatus::Pending,
            recent_observations: Vec::new(),
            error_count: 0,
            retry_count: 0,
            custom_data: std::collections::HashMap::new(),
        }
    }
}

impl ExecutionState {
    /// Create a new execution state with max iterations
    pub fn new(max_iterations: Option<u32>) -> Self {
        Self {
            max_iterations,
            ..Default::default()
        }
    }

    /// Add an observation to the sliding window
    pub fn add_observation(&mut self, observation: String, max_observations: usize) {
        self.recent_observations.push(observation);
        while self.recent_observations.len() > max_observations {
            self.recent_observations.remove(0);
        }
    }

    /// Increment iteration counter
    pub fn next_iteration(&mut self) {
        self.iteration += 1;
    }

    /// Check if max iterations exceeded
    pub fn is_max_iterations_exceeded(&self) -> bool {
        if let Some(max) = self.max_iterations {
            self.iteration >= max
        } else {
            false
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.started_at
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
    }
}

/// Status of an execution loop
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Not yet started
    Pending,
    /// Currently running
    Running,
    /// Paused (can be resumed)
    Paused,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Terminated early (timeout, cancellation, etc.)
    Terminated,
}

/// Configuration for the universal executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Maximum retries per step
    pub max_retries_per_step: u32,
    /// Base backoff delay in milliseconds
    pub backoff_base_ms: u64,
    /// Maximum backoff delay in milliseconds
    pub backoff_max_ms: u64,
    /// Backoff multiplier (for exponential backoff)
    pub backoff_multiplier: f64,
    /// Whether to use jitter in backoff
    pub use_jitter: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_retries_per_step: 3,
            backoff_base_ms: 1000,
            backoff_max_ms: 30000,
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

/// A universal executor that can run any ExecutionLoop implementation
pub struct UniversalExecutor {
    config: ExecutorConfig,
    start_time: Option<Instant>,
}

impl Default for UniversalExecutor {
    fn default() -> Self {
        Self::new(ExecutorConfig::default())
    }
}

impl UniversalExecutor {
    /// Create a new universal executor with config
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            config,
            start_time: None,
        }
    }

    /// Execute any ExecutionLoop until completion
    ///
    /// # Main Loop Algorithm
    /// ```text
    /// Initialize
    /// while should_continue() and not should_terminate():
    ///     try:
    ///         result = execute_step()
    ///         reset_error_state()
    ///     catch error:
    ///         handle_error()
    ///         if is_retryable_error():
    ///             continue (retry with backoff)
    ///         else:
    ///             return Err
    ///
    ///     if is_complete():
    ///         return Ok
    ///
    /// if not is_complete():
    ///     return Err("Max iterations reached")
    /// ```
    pub async fn execute<T: ExecutionLoop>(&mut self, executor: &mut T) -> Result<ExecutionSummary> {
        self.start_time = Some(Instant::now());
        let mut summary = ExecutionSummary::default();

        // Initialize
        executor.initialize().await?;
        summary.status = ExecutionStatus::Running;

        loop {
            // Check termination conditions
            if !executor.should_continue() {
                tracing::debug!("execution.loop.no_continue iter={}", executor.iteration());
                break;
            }

            if executor.should_terminate()? {
                summary.status = ExecutionStatus::Terminated;
                summary.termination_reason = Some("Execution terminated by should_terminate()".to_string());
                return Ok(summary);
            }

            // Attempt step execution with retries
            let mut retries = 0;
            let step_result = loop {
                match executor.execute_step().await {
                    Ok(result) => {
                        executor.reset_error_state();
                        summary.steps_executed += 1;
                        if result.success {
                            summary.successful_steps += 1;
                        } else {
                            summary.failed_steps += 1;
                        }
                        break result;
                    }
                    Err(e) => {
                        summary.error_count += 1;
                        executor.handle_error(e).await.ok();

                        if executor.is_retryable_error()
                            && executor.is_step_retryable()
                            && retries < self.config.max_retries_per_step
                        {
                            retries += 1;
                            summary.retry_count += 1;
                            let delay = self.calculate_backoff(retries);
                            tracing::debug!(
                                "execution.step.retry iter={} step={} retry={} delay_ms={}",
                                executor.iteration(),
                                executor.current_step_id(),
                                retries,
                                delay.as_millis()
                            );
                            tokio::time::sleep(delay).await;
                            continue;
                        }

                        summary.status = ExecutionStatus::Failed;
                        summary.termination_reason = Some("Step execution failed after retries".to_string());
                        return Ok(summary);
                    }
                }
            };

            tracing::debug!(
                "execution.step.complete iter={} step={} success={}",
                executor.iteration(),
                step_result.step_id,
                step_result.success
            );

            // Check completion
            match executor.is_complete() {
                Ok(true) => {
                    tracing::info!("execution.loop.complete iter={}", executor.iteration());
                    summary.status = ExecutionStatus::Completed;
                    summary.total_duration = self.start_time.map(|t| t.elapsed()).unwrap_or_default();
                    summary.final_iteration = executor.iteration();
                    return Ok(summary);
                }
                Ok(false) => {
                    // Continue looping
                }
                Err(e) => {
                    summary.status = ExecutionStatus::Failed;
                    summary.termination_reason = Some(format!("Completion check failed: {}", e));
                    return Ok(summary);
                }
            }
        }

        // Fell through without explicit completion
        summary.total_duration = self.start_time.map(|t| t.elapsed()).unwrap_or_default();
        summary.final_iteration = executor.iteration();

        if executor.is_complete()? {
            summary.status = ExecutionStatus::Completed;
        } else {
            summary.status = ExecutionStatus::Terminated;
            summary.termination_reason = Some("Loop ended without completion".to_string());
        }

        Ok(summary)
    }

    /// Calculate backoff delay with optional jitter
    fn calculate_backoff(&self, retry_count: u32) -> Duration {
        let base = self.config.backoff_base_ms as f64;
        let multiplier = self.config.backoff_multiplier;
        let max = self.config.backoff_max_ms as f64;

        let delay = (base * multiplier.powi(retry_count as i32 - 1)).min(max);

        let delay_with_jitter = if self.config.use_jitter {
            // Simple jitter using system time nanos as pseudo-random source
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0);
            let jitter_factor = (nanos % 1000) as f64 / 1000.0 * 0.3; // 0-30% jitter
            delay * (1.0 + jitter_factor)
        } else {
            delay
        };

        Duration::from_millis(delay_with_jitter as u64)
    }
}

/// Summary of an execution run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionSummary {
    /// Final status of the execution
    pub status: ExecutionStatus,
    /// Total duration of execution
    pub total_duration: Duration,
    /// Final iteration number
    pub final_iteration: u32,
    /// Number of steps executed
    pub steps_executed: u32,
    /// Number of successful steps
    pub successful_steps: u32,
    /// Number of failed steps
    pub failed_steps: u32,
    /// Total error count
    pub error_count: u32,
    /// Total retry count
    pub retry_count: u32,
    /// Reason for termination (if terminated early)
    pub termination_reason: Option<String>,
}

impl Default for ExecutionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple test executor for unit tests
    struct TestExecutor {
        state: ExecutionState,
        steps_to_run: u32,
        fail_on_step: Option<u32>,
    }

    impl TestExecutor {
        fn new(steps: u32) -> Self {
            Self {
                state: ExecutionState::new(Some(steps + 5)),
                steps_to_run: steps,
                fail_on_step: None,
            }
        }

        fn with_failure_on(mut self, step: u32) -> Self {
            self.fail_on_step = Some(step);
            self
        }
    }

    #[async_trait]
    impl ExecutionLoop for TestExecutor {
        type State = ExecutionState;

        async fn initialize(&mut self) -> Result<()> {
            self.state.status = ExecutionStatus::Running;
            Ok(())
        }

        async fn execute_step(&mut self) -> Result<StepResult> {
            self.state.next_iteration();
            let step_id = format!("step-{}", self.state.iteration);

            if Some(self.state.iteration) == self.fail_on_step {
                return Err(anyhow::anyhow!("Simulated failure on step {}", self.state.iteration));
            }

            Ok(StepResult::success(step_id, "Test output", Duration::from_millis(10)))
        }

        fn current_step_id(&self) -> String {
            format!("step-{}", self.state.iteration)
        }

        fn should_continue(&self) -> bool {
            self.state.iteration < self.steps_to_run
        }

        fn is_complete(&self) -> Result<bool> {
            Ok(self.state.iteration >= self.steps_to_run)
        }

        fn get_state(&self) -> &Self::State {
            &self.state
        }

        fn get_state_mut(&mut self) -> &mut Self::State {
            &mut self.state
        }

        fn iteration(&self) -> u32 {
            self.state.iteration
        }

        fn max_iterations(&self) -> Option<u32> {
            self.state.max_iterations
        }

        fn elapsed_time(&self) -> Duration {
            self.state.elapsed()
        }

        async fn handle_error(&mut self, _error: anyhow::Error) -> Result<()> {
            self.state.error_count += 1;
            Ok(())
        }

        fn reset_error_state(&mut self) {
            // No-op for test
        }

        fn get_metrics(&self) -> serde_json::Value {
            serde_json::json!({
                "iteration": self.state.iteration,
                "error_count": self.state.error_count,
            })
        }

        fn get_recent_observations(&self, n: usize) -> Vec<String> {
            self.state.recent_observations.iter().take(n).cloned().collect()
        }
    }

    #[tokio::test]
    async fn test_executor_runs_to_completion() {
        let mut executor = TestExecutor::new(5);
        let mut universal = UniversalExecutor::default();

        let summary = universal.execute(&mut executor).await.unwrap();

        assert_eq!(summary.status, ExecutionStatus::Completed);
        assert_eq!(summary.final_iteration, 5);
        assert_eq!(summary.steps_executed, 5);
        assert_eq!(summary.successful_steps, 5);
    }

    #[tokio::test]
    async fn test_step_result_creation() {
        let success = StepResult::success("test-1", "output", Duration::from_secs(1));
        assert!(success.success);
        assert_eq!(success.step_id, "test-1");
        assert!(success.error.is_none());

        let failure = StepResult::failure("test-2", "error msg", Duration::from_secs(1));
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }

    #[tokio::test]
    async fn test_execution_state_observations() {
        let mut state = ExecutionState::default();
        state.add_observation("obs1".to_string(), 3);
        state.add_observation("obs2".to_string(), 3);
        state.add_observation("obs3".to_string(), 3);
        state.add_observation("obs4".to_string(), 3);

        assert_eq!(state.recent_observations.len(), 3);
        assert_eq!(state.recent_observations[0], "obs2");
    }
}
