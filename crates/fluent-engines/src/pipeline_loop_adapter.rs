//! ExecutionLoop adapter for PipelineExecutor
//!
//! Provides a unified interface for running pipelines through the ExecutionLoop trait,
//! enabling consistent execution patterns across different execution models.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::pipeline_executor::{Pipeline, PipelineState, PipelineStep, StateStore};

/// State for the pipeline loop adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineLoopState {
    /// Pipeline state data
    pub pipeline_state: PipelineState,
    /// Current step index
    pub current_step_index: usize,
    /// Total number of steps
    pub total_steps: usize,
    /// Recent observations/logs
    pub observations: Vec<String>,
    /// Error count
    pub error_count: u32,
    /// Retry count
    pub retry_count: u32,
    /// Whether initialization completed
    pub initialized: bool,
    /// Last error encountered (if retryable)
    pub last_error: Option<String>,
    /// Step results by name
    pub step_results: HashMap<String, String>,
}

impl Default for PipelineLoopState {
    fn default() -> Self {
        Self {
            pipeline_state: PipelineState {
                current_step: 0,
                data: HashMap::new(),
                run_id: String::new(),
                start_time: 0,
            },
            current_step_index: 0,
            total_steps: 0,
            observations: Vec::new(),
            error_count: 0,
            retry_count: 0,
            initialized: false,
            last_error: None,
            step_results: HashMap::new(),
        }
    }
}

/// Adapter that wraps Pipeline execution in the ExecutionLoop trait
pub struct PipelineLoopAdapter<S: StateStore + Clone + Send + Sync> {
    /// The pipeline to execute
    pipeline: Pipeline,
    /// State store for persistence
    state_store: S,
    /// Current execution state
    state: PipelineLoopState,
    /// Initial input for the pipeline
    initial_input: String,
    /// Maximum iterations (steps) allowed
    max_iterations: Option<u32>,
    /// Time when execution started
    start_time: Option<Instant>,
    /// Run ID for this execution
    run_id: String,
    /// Force fresh execution (ignore saved state)
    force_fresh: bool,
    /// JSON output mode
    json_output: bool,
}

impl<S: StateStore + Clone + Send + Sync> PipelineLoopAdapter<S> {
    /// Create a new pipeline loop adapter
    pub fn new(pipeline: Pipeline, state_store: S, initial_input: String) -> Self {
        let total_steps = pipeline.steps.len();
        Self {
            pipeline,
            state_store,
            state: PipelineLoopState {
                total_steps,
                ..Default::default()
            },
            initial_input,
            max_iterations: None,
            start_time: None,
            run_id: uuid::Uuid::new_v4().to_string(),
            force_fresh: false,
            json_output: false,
        }
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = Some(max);
        self
    }

    /// Set run ID
    pub fn with_run_id(mut self, run_id: String) -> Self {
        self.run_id = run_id;
        self
    }

    /// Force fresh execution
    pub fn force_fresh(mut self) -> Self {
        self.force_fresh = true;
        self
    }

    /// Enable JSON output mode
    pub fn with_json_output(mut self, json: bool) -> Self {
        self.json_output = json;
        self
    }

    /// Get pipeline name
    pub fn pipeline_name(&self) -> &str {
        &self.pipeline.name
    }

    /// Get current step name
    fn current_step_name(&self) -> String {
        if self.state.current_step_index < self.pipeline.steps.len() {
            self.pipeline.steps[self.state.current_step_index]
                .name()
                .to_string()
        } else {
            "complete".to_string()
        }
    }

    /// Add an observation to the log
    fn add_observation(&mut self, observation: String) {
        const MAX_OBSERVATIONS: usize = 100;
        self.state.observations.push(observation);
        while self.state.observations.len() > MAX_OBSERVATIONS {
            self.state.observations.remove(0);
        }
    }
}

/// Step result from ExecutionLoop
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
    pub metadata: HashMap<String, serde_json::Value>,
}

impl StepResult {
    /// Create a successful step result
    pub fn success(
        step_id: impl Into<String>,
        output: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            success: true,
            output: output.into(),
            duration,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed step result
    pub fn failure(
        step_id: impl Into<String>,
        error: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            success: false,
            output: String::new(),
            duration,
            error: Some(error.into()),
            metadata: HashMap::new(),
        }
    }
}

/// ExecutionLoop trait implementation for pipelines
///
/// This is a simplified version that doesn't require the full fluent-agent dependency.
/// It mirrors the trait structure for compatibility.
#[async_trait]
pub trait PipelineExecutionLoop: Send + Sync {
    /// State type
    type State: Send + Sync;

    /// Initialize the execution loop
    async fn initialize(&mut self) -> Result<()>;

    /// Execute a single step
    async fn execute_step(&mut self) -> Result<StepResult>;

    /// Check if step is retryable
    fn is_step_retryable(&self) -> bool {
        true
    }

    /// Get current step identifier
    fn current_step_id(&self) -> String;

    /// Check if loop should continue
    fn should_continue(&self) -> bool;

    /// Check if in retryable error state
    fn is_retryable_error(&self) -> bool {
        false
    }

    /// Check if execution is complete
    fn is_complete(&self) -> Result<bool>;

    /// Check if should terminate early
    fn should_terminate(&self) -> Result<bool> {
        Ok(false)
    }

    /// Get state reference
    fn get_state(&self) -> &Self::State;

    /// Get mutable state reference
    fn get_state_mut(&mut self) -> &mut Self::State;

    /// Save checkpoint
    async fn save_checkpoint(&self) -> Result<String> {
        Ok(String::new())
    }

    /// Restore checkpoint
    async fn restore_checkpoint(&mut self, _id: &str) -> Result<()> {
        Ok(())
    }

    /// Get current iteration
    fn iteration(&self) -> u32;

    /// Get maximum iterations
    fn max_iterations(&self) -> Option<u32>;

    /// Get elapsed time
    fn elapsed_time(&self) -> Duration;

    /// Handle an error
    async fn handle_error(&mut self, error: anyhow::Error) -> Result<()>;

    /// Reset error state
    fn reset_error_state(&mut self);

    /// Get execution metrics
    fn get_metrics(&self) -> serde_json::Value;

    /// Get recent observations
    fn get_recent_observations(&self, n: usize) -> Vec<String>;
}

#[async_trait]
impl<S: StateStore + Clone + Send + Sync + 'static> PipelineExecutionLoop
    for PipelineLoopAdapter<S>
{
    type State = PipelineLoopState;

    async fn initialize(&mut self) -> Result<()> {
        self.start_time = Some(Instant::now());

        let state_key = format!("{}-{}", self.pipeline.name, self.run_id);

        // Try to load existing state
        if !self.force_fresh {
            if let Some(saved_state) = self.state_store.load_state(&state_key).await? {
                tracing::debug!(
                    "Resuming pipeline from saved state at step {}",
                    saved_state.current_step
                );
                self.state.pipeline_state = saved_state;
                self.state.current_step_index = self.state.pipeline_state.current_step;
                self.state.initialized = true;
                self.add_observation(format!(
                    "Resumed pipeline at step {}",
                    self.state.current_step_index
                ));
                return Ok(());
            }
        }

        // Fresh start
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.state.pipeline_state = PipelineState {
            current_step: 0,
            data: HashMap::new(),
            run_id: self.run_id.clone(),
            start_time,
        };

        // Set initial input
        self.state
            .pipeline_state
            .data
            .insert("input".to_string(), self.initial_input.clone());
        self.state
            .pipeline_state
            .data
            .insert("run_id".to_string(), self.run_id.clone());

        self.state.current_step_index = 0;
        self.state.initialized = true;
        self.state.total_steps = self.pipeline.steps.len();

        self.add_observation(format!(
            "Initialized pipeline '{}' with {} steps",
            self.pipeline.name, self.state.total_steps
        ));

        Ok(())
    }

    async fn execute_step(&mut self) -> Result<StepResult> {
        if !self.state.initialized {
            return Err(anyhow!(
                "Pipeline not initialized. Call initialize() first."
            ));
        }

        if self.state.current_step_index >= self.pipeline.steps.len() {
            return Err(anyhow!("No more steps to execute"));
        }

        let step_start = Instant::now();
        let step = &self.pipeline.steps[self.state.current_step_index];
        let step_name = step.name().to_string();
        let step_id = format!("{}-step-{}", self.run_id, self.state.current_step_index);

        tracing::debug!(
            "Executing step {} ({}/{}): {}",
            step_name,
            self.state.current_step_index + 1,
            self.state.total_steps,
            step_id
        );

        // Update pipeline state
        self.state.pipeline_state.current_step = self.state.current_step_index;
        self.state
            .pipeline_state
            .data
            .insert("step".to_string(), step_name.clone());

        // Execute the step
        let result = self.execute_pipeline_step(step).await;
        let duration = step_start.elapsed();

        match result {
            Ok(step_data) => {
                // Merge step results into pipeline state
                self.state.pipeline_state.data.extend(step_data.clone());

                // Record step results
                for (key, value) in &step_data {
                    self.state.step_results.insert(key.clone(), value.clone());
                }

                // Save state
                let state_key = format!("{}-{}", self.pipeline.name, self.run_id);
                if let Err(e) = self
                    .state_store
                    .save_state(&state_key, &self.state.pipeline_state)
                    .await
                {
                    tracing::warn!("Failed to save pipeline state: {}", e);
                }

                // Advance to next step
                self.state.current_step_index += 1;
                self.state.last_error = None;

                let output = step_data
                    .values()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "completed".to_string());

                self.add_observation(format!(
                    "Step '{}' completed successfully in {:?}",
                    step_name, duration
                ));

                Ok(StepResult::success(step_id, output, duration))
            }
            Err(e) => {
                self.state.error_count += 1;
                self.state.last_error = Some(e.to_string());

                self.add_observation(format!("Step '{}' failed: {}", step_name, e));

                Ok(StepResult::failure(step_id, e.to_string(), duration))
            }
        }
    }

    fn current_step_id(&self) -> String {
        format!(
            "{}-step-{}-{}",
            self.run_id,
            self.state.current_step_index,
            self.current_step_name()
        )
    }

    fn should_continue(&self) -> bool {
        if !self.state.initialized {
            return false;
        }

        // Check max iterations
        if let Some(max) = self.max_iterations {
            if self.state.current_step_index as u32 >= max {
                return false;
            }
        }

        // More steps remaining
        self.state.current_step_index < self.state.total_steps
    }

    fn is_retryable_error(&self) -> bool {
        self.state.last_error.is_some()
    }

    fn is_complete(&self) -> Result<bool> {
        Ok(self.state.current_step_index >= self.state.total_steps)
    }

    fn should_terminate(&self) -> Result<bool> {
        // Could add timeout logic here
        Ok(false)
    }

    fn get_state(&self) -> &Self::State {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    async fn save_checkpoint(&self) -> Result<String> {
        let state_key = format!("{}-{}", self.pipeline.name, self.run_id);
        self.state_store
            .save_state(&state_key, &self.state.pipeline_state)
            .await?;
        Ok(state_key)
    }

    async fn restore_checkpoint(&mut self, id: &str) -> Result<()> {
        if let Some(saved_state) = self.state_store.load_state(id).await? {
            self.state.pipeline_state = saved_state;
            self.state.current_step_index = self.state.pipeline_state.current_step;
            self.state.initialized = true;
            Ok(())
        } else {
            Err(anyhow!("Checkpoint not found: {}", id))
        }
    }

    fn iteration(&self) -> u32 {
        self.state.current_step_index as u32
    }

    fn max_iterations(&self) -> Option<u32> {
        self.max_iterations
    }

    fn elapsed_time(&self) -> Duration {
        self.start_time
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    async fn handle_error(&mut self, error: anyhow::Error) -> Result<()> {
        self.state.error_count += 1;
        self.state.last_error = Some(error.to_string());
        self.add_observation(format!("Error handled: {}", error));
        Ok(())
    }

    fn reset_error_state(&mut self) {
        self.state.last_error = None;
    }

    fn get_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "pipeline_name": self.pipeline.name,
            "run_id": self.run_id,
            "current_step": self.state.current_step_index,
            "total_steps": self.state.total_steps,
            "error_count": self.state.error_count,
            "retry_count": self.state.retry_count,
            "elapsed_ms": self.elapsed_time().as_millis(),
            "initialized": self.state.initialized,
            "complete": self.state.current_step_index >= self.state.total_steps,
        })
    }

    fn get_recent_observations(&self, n: usize) -> Vec<String> {
        let start = self.state.observations.len().saturating_sub(n);
        self.state.observations[start..].to_vec()
    }
}

impl<S: StateStore + Clone + Send + Sync + 'static> PipelineLoopAdapter<S> {
    /// Execute a single pipeline step (internal implementation)
    async fn execute_pipeline_step(&self, step: &PipelineStep) -> Result<HashMap<String, String>> {
        use crate::pipeline::{
            CommandExecutor, ConditionExecutor, LoopExecutor, ParallelExecutor, StepExecutor,
            VariableExpander,
        };

        // Clone state data for variable expansion
        let state_data = self.state.pipeline_state.data.clone();

        match step {
            PipelineStep::Command {
                name,
                command,
                save_output,
                retry,
            } => {
                tracing::debug!("Executing Command step: {}", name);
                let expanded_command =
                    VariableExpander::expand_variables(command, &state_data).await?;
                CommandExecutor::execute_command_with_retry(&expanded_command, save_output, retry)
                    .await
            }

            PipelineStep::ShellCommand {
                name,
                command,
                save_output,
                retry,
            } => {
                tracing::debug!("Executing ShellCommand step: {}", name);
                let expanded_command =
                    VariableExpander::expand_variables(command, &state_data).await?;
                CommandExecutor::execute_shell_command_with_retry(
                    &expanded_command,
                    save_output,
                    retry,
                )
                .await
            }

            PipelineStep::Condition {
                name,
                condition,
                if_true,
                if_false,
            } => {
                tracing::debug!("Evaluating Condition step: {}", name);
                ConditionExecutor::execute_condition_with_expansion(
                    name,
                    condition,
                    if_true,
                    if_false,
                    &state_data,
                )
                .await
            }

            PipelineStep::PrintOutput { name, value } => {
                tracing::debug!("Executing PrintOutput step: {}", name);
                let expanded_value = VariableExpander::expand_variables(value, &state_data).await?;
                if !self.json_output {
                    eprintln!("{}", expanded_value);
                }
                Ok(HashMap::new())
            }

            PipelineStep::ForEach { name, items, steps } => {
                tracing::debug!("Executing ForEach step: {}", name);
                let expanded_items = VariableExpander::expand_variables(items, &state_data).await?;
                // Need mutable state for loop executor
                let mut temp_state = self.state.pipeline_state.clone();
                LoopExecutor::execute_for_each(name, &expanded_items, steps, &mut temp_state).await
            }

            PipelineStep::TryCatch {
                try_steps,
                catch_steps,
                finally_steps,
                ..
            } => {
                let mut temp_state = self.state.pipeline_state.clone();
                StepExecutor::execute_try_catch(
                    try_steps,
                    catch_steps,
                    finally_steps,
                    &mut temp_state,
                )
                .await
            }

            PipelineStep::Parallel { name, steps } => {
                tracing::debug!("Executing Parallel step: {}", name);
                let mut temp_state = self.state.pipeline_state.clone();
                ParallelExecutor::execute_parallel_steps(steps, &mut temp_state).await
            }

            PipelineStep::Timeout {
                name,
                duration,
                step,
            } => {
                tracing::debug!("Executing Timeout step: {} ({}s)", name, duration);
                let mut temp_state = self.state.pipeline_state.clone();
                StepExecutor::execute_timeout(*duration, step, &mut temp_state).await
            }

            PipelineStep::RepeatUntil {
                steps, condition, ..
            } => {
                let mut temp_state = self.state.pipeline_state.clone();
                LoopExecutor::execute_repeat_until(steps, condition, &mut temp_state).await
            }

            _ => {
                tracing::warn!("Unsupported step type: {:?}", step);
                Ok(HashMap::new())
            }
        }
    }

    /// Get final output as JSON
    pub fn get_final_output(&self) -> Result<String> {
        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let runtime = end_time - self.state.pipeline_state.start_time;

        let output = serde_json::json!({
            "pipeline_name": self.pipeline.name,
            "run_id": self.run_id,
            "current_step": self.state.current_step_index,
            "total_steps": self.state.total_steps,
            "start_time": self.state.pipeline_state.start_time,
            "end_time": end_time,
            "runtime_seconds": runtime,
            "error_count": self.state.error_count,
            "data": self.state.pipeline_state.data,
        });

        Ok(serde_json::to_string_pretty(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_executor::FileStateStore;
    use tempfile::tempdir;

    fn create_test_adapter() -> PipelineLoopAdapter<FileStateStore> {
        let dir = tempdir().unwrap();
        let store = FileStateStore {
            directory: dir.into_path(),
        };
        let pipeline = Pipeline {
            name: "test_pipeline".to_string(),
            steps: vec![
                PipelineStep::PrintOutput {
                    name: "step1".to_string(),
                    value: "Hello".to_string(),
                },
                PipelineStep::PrintOutput {
                    name: "step2".to_string(),
                    value: "World".to_string(),
                },
            ],
        };
        PipelineLoopAdapter::new(pipeline, store, "test input".to_string())
    }

    #[tokio::test]
    async fn test_adapter_initialization() {
        let mut adapter = create_test_adapter();
        adapter.initialize().await.unwrap();

        assert!(adapter.state.initialized);
        assert_eq!(adapter.state.total_steps, 2);
        assert_eq!(adapter.state.current_step_index, 0);
    }

    #[tokio::test]
    async fn test_adapter_should_continue() {
        let mut adapter = create_test_adapter();

        // Before init, should not continue
        assert!(!adapter.should_continue());

        adapter.initialize().await.unwrap();

        // After init with steps remaining
        assert!(adapter.should_continue());
    }

    #[tokio::test]
    async fn test_adapter_is_complete() {
        let mut adapter = create_test_adapter();
        adapter.initialize().await.unwrap();

        // Not complete initially
        assert!(!adapter.is_complete().unwrap());

        // Execute all steps
        while adapter.should_continue() {
            adapter.execute_step().await.unwrap();
        }

        // Should be complete now
        assert!(adapter.is_complete().unwrap());
    }

    #[tokio::test]
    async fn test_adapter_metrics() {
        let mut adapter = create_test_adapter();
        adapter.initialize().await.unwrap();

        let metrics = adapter.get_metrics();
        assert_eq!(metrics["pipeline_name"], "test_pipeline");
        assert_eq!(metrics["total_steps"], 2);
        assert_eq!(metrics["current_step"], 0);
    }

    #[tokio::test]
    async fn test_adapter_observations() {
        let mut adapter = create_test_adapter();
        adapter.initialize().await.unwrap();

        let obs = adapter.get_recent_observations(5);
        assert!(!obs.is_empty());
        assert!(obs[0].contains("Initialized pipeline"));
    }
}
