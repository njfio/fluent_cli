use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Modular pipeline execution architecture with improved testability and maintainability
/// 
/// Key improvements:
/// - Separation of concerns with dedicated step executors
/// - Dependency injection for better testability
/// - Event-driven architecture for monitoring
/// - Plugin-based step execution
/// - Comprehensive error handling and recovery
/// - Performance monitoring and metrics

/// Pipeline execution context with rich metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub run_id: String,
    pub pipeline_name: String,
    pub current_step: usize,
    pub variables: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
    pub start_time: SystemTime,
    pub step_history: Vec<StepExecution>,
}

/// Individual step execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecution {
    pub step_name: String,
    pub step_type: String,
    pub start_time: SystemTime,
    pub end_time: Option<SystemTime>,
    pub status: ExecutionStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub retry_count: u32,
}

/// Execution status for steps and pipelines
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

/// Pipeline step definition with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub name: String,
    pub step_type: String,
    pub config: HashMap<String, serde_json::Value>,
    pub timeout: Option<Duration>,
    pub retry_config: Option<RetryConfig>,
    pub depends_on: Vec<String>,
    pub condition: Option<String>,
    pub parallel_group: Option<String>,
}

/// Retry configuration for step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub retry_on: Vec<String>, // Error patterns to retry on
}

/// Pipeline definition with metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub steps: Vec<PipelineStep>,
    pub global_config: HashMap<String, serde_json::Value>,
    pub timeout: Option<Duration>,
    pub max_parallel: Option<usize>,
}

/// Event emitted during pipeline execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub event_type: EventType,
    pub timestamp: SystemTime,
    pub run_id: String,
    pub step_name: Option<String>,
    pub data: HashMap<String, serde_json::Value>,
}

/// Types of execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    PipelineStarted,
    PipelineCompleted,
    PipelineFailed,
    StepStarted,
    StepCompleted,
    StepFailed,
    StepRetrying,
    VariableSet,
    ConditionEvaluated,
}

/// Trait for step executors - allows pluggable step implementations
#[async_trait]
pub trait StepExecutor: Send + Sync {
    /// Execute a step with the given context
    async fn execute(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<StepResult>;
    
    /// Get the step types this executor can handle
    fn supported_types(&self) -> Vec<String>;
    
    /// Validate step configuration
    fn validate_config(&self, step: &PipelineStep) -> Result<()>;
}

/// Result of step execution
#[derive(Debug, Clone)]
pub struct StepResult {
    pub output: Option<String>,
    pub variables: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

/// Trait for event listeners
#[async_trait]
pub trait EventListener: Send + Sync {
    async fn on_event(&self, event: &ExecutionEvent) -> Result<()>;
}

/// Trait for state persistence
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save_context(&self, context: &ExecutionContext) -> Result<()>;
    async fn load_context(&self, run_id: &str) -> Result<Option<ExecutionContext>>;
    async fn delete_context(&self, run_id: &str) -> Result<()>;
}

/// Trait for variable expansion and templating
#[async_trait]
pub trait VariableExpander: Send + Sync {
    async fn expand(&self, template: &str, variables: &HashMap<String, String>) -> Result<String>;
    async fn evaluate_condition(&self, condition: &str, variables: &HashMap<String, String>) -> Result<bool>;
}

/// Modular pipeline executor with dependency injection
pub struct ModularPipelineExecutor {
    step_executors: HashMap<String, Arc<dyn StepExecutor>>,
    event_listeners: Vec<Arc<dyn EventListener>>,
    state_store: Arc<dyn StateStore>,
    variable_expander: Arc<dyn VariableExpander>,
    metrics: Arc<RwLock<ExecutionMetrics>>,
}

/// Execution metrics for monitoring
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub total_pipelines: u64,
    pub successful_pipelines: u64,
    pub failed_pipelines: u64,
    pub total_steps: u64,
    pub successful_steps: u64,
    pub failed_steps: u64,
    pub average_execution_time_ms: f64,
    pub step_type_metrics: HashMap<String, StepTypeMetrics>,
}

/// Metrics for specific step types
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StepTypeMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub retry_count: u64,
}

impl ModularPipelineExecutor {
    /// Create a new modular pipeline executor
    pub fn new(
        state_store: Arc<dyn StateStore>,
        variable_expander: Arc<dyn VariableExpander>,
    ) -> Self {
        Self {
            step_executors: HashMap::new(),
            event_listeners: Vec::new(),
            state_store,
            variable_expander,
            metrics: Arc::new(RwLock::new(ExecutionMetrics::default())),
        }
    }

    /// Register a step executor for specific step types
    pub fn register_step_executor(&mut self, executor: Arc<dyn StepExecutor>) {
        for step_type in executor.supported_types() {
            self.step_executors.insert(step_type, executor.clone());
        }
    }

    /// Add an event listener
    pub fn add_event_listener(&mut self, listener: Arc<dyn EventListener>) {
        self.event_listeners.push(listener);
    }

    /// Execute a pipeline with comprehensive monitoring and error handling
    pub async fn execute_pipeline(
        &self,
        pipeline: &Pipeline,
        initial_variables: HashMap<String, String>,
        resume_run_id: Option<String>,
    ) -> Result<ExecutionContext> {
        let run_id = resume_run_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        
        // Load or create execution context
        let mut context = match self.state_store.load_context(&run_id).await? {
            Some(ctx) => ctx,
            None => ExecutionContext {
                run_id: run_id.clone(),
                pipeline_name: pipeline.name.clone(),
                current_step: 0,
                variables: initial_variables,
                metadata: HashMap::new(),
                start_time: SystemTime::now(),
                step_history: Vec::new(),
            },
        };

        // Emit pipeline started event
        self.emit_event(ExecutionEvent {
            event_type: EventType::PipelineStarted,
            timestamp: SystemTime::now(),
            run_id: run_id.clone(),
            step_name: None,
            data: HashMap::new(),
        }).await?;

        // Validate pipeline
        self.validate_pipeline(pipeline).await?;

        // Execute steps
        let result = self.execute_steps(pipeline, &mut context).await;

        // Update metrics
        self.update_pipeline_metrics(&result).await;

        // Emit completion event
        let event_type = match &result {
            Ok(_) => EventType::PipelineCompleted,
            Err(_) => EventType::PipelineFailed,
        };

        self.emit_event(ExecutionEvent {
            event_type,
            timestamp: SystemTime::now(),
            run_id: run_id.clone(),
            step_name: None,
            data: HashMap::new(),
        }).await?;

        // Save final context
        self.state_store.save_context(&context).await?;

        result
    }

    /// Execute pipeline steps with dependency resolution and parallel execution
    async fn execute_steps(&self, pipeline: &Pipeline, context: &mut ExecutionContext) -> Result<ExecutionContext> {
        let steps_to_execute = self.resolve_step_dependencies(&pipeline.steps)?;
        
        for step_group in steps_to_execute {
            if step_group.len() == 1 {
                // Sequential execution
                self.execute_single_step(&step_group[0], context).await?;
            } else {
                // Parallel execution
                self.execute_parallel_steps(&step_group, context).await?;
            }
            
            // Save context after each step group
            self.state_store.save_context(context).await?;
        }

        Ok(context.clone())
    }

    /// Execute a single step with retry logic and monitoring
    async fn execute_single_step(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<()> {
        // Check condition if specified
        if let Some(condition) = &step.condition {
            let should_execute = self.variable_expander
                .evaluate_condition(condition, &context.variables).await?;
            if !should_execute {
                self.record_step_skipped(step, context).await?;
                return Ok(());
            }
        }

        // Find appropriate executor
        let executor = self.step_executors.get(&step.step_type)
            .ok_or_else(|| anyhow!("No executor found for step type: {}", step.step_type))?;

        // Execute with retry logic
        let retry_config = step.retry_config.clone().unwrap_or_default();
        let mut attempt = 0;
        let mut last_error = None;

        loop {
            attempt += 1;
            
            // Record step start
            self.record_step_started(step, context, attempt).await?;
            
            // Execute step
            match executor.execute(step, context).await {
                Ok(result) => {
                    // Update context with results
                    if let Some(output) = result.output {
                        context.variables.insert(format!("{}_output", step.name), output);
                    }
                    context.variables.extend(result.variables);
                    context.metadata.extend(result.metadata);
                    
                    // Record success
                    self.record_step_completed(step, context).await?;
                    self.update_step_metrics(&step.step_type, true, attempt).await;
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("{}", e));
                    
                    // Check if we should retry
                    if attempt < retry_config.max_attempts && self.should_retry(&e, &retry_config) {
                        // Record retry
                        self.record_step_retrying(step, context, &e).await?;
                        
                        // Calculate delay
                        let delay = self.calculate_retry_delay(attempt, &retry_config);
                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        // Record failure
                        self.record_step_failed(step, context, &e).await?;
                        self.update_step_metrics(&step.step_type, false, attempt).await;
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Execute multiple steps in parallel
    async fn execute_parallel_steps(&self, steps: &[PipelineStep], context: &mut ExecutionContext) -> Result<()> {
        let context_arc = Arc::new(Mutex::new(context.clone()));
        let mut handles = Vec::new();

        for step in steps {
            let step = step.clone();
            let context_clone = Arc::clone(&context_arc);
            let executor = self.step_executors.get(&step.step_type)
                .ok_or_else(|| anyhow!("No executor found for step type: {}", step.step_type))?
                .clone();

            let handle = tokio::spawn(async move {
                let mut ctx = context_clone.lock().await;
                executor.execute(&step, &mut ctx).await
            });
            
            handles.push(handle);
        }

        // Wait for all steps to complete
        let results = futures::future::try_join_all(handles).await?;
        
        // Collect results
        for result in results {
            result?; // Propagate any step errors
        }

        // Update context with final state
        let final_context = context_arc.lock().await;
        *context = final_context.clone();

        Ok(())
    }

    /// Resolve step dependencies and create execution groups
    fn resolve_step_dependencies(&self, steps: &[PipelineStep]) -> Result<Vec<Vec<PipelineStep>>> {
        // Simple implementation - can be enhanced with topological sorting
        let mut groups = Vec::new();
        let mut remaining_steps = steps.to_vec();

        while !remaining_steps.is_empty() {
            let mut current_group = Vec::new();
            let mut indices_to_remove = Vec::new();

            for (i, step) in remaining_steps.iter().enumerate() {
                // Check if all dependencies are satisfied
                let dependencies_satisfied = step.depends_on.iter().all(|dep| {
                    groups.iter().flatten().any(|executed_step: &PipelineStep| executed_step.name == *dep)
                });

                if dependencies_satisfied {
                    current_group.push(step.clone());
                    indices_to_remove.push(i);
                }
            }

            if current_group.is_empty() {
                return Err(anyhow!("Circular dependency detected in pipeline steps"));
            }

            // Remove processed steps
            for &i in indices_to_remove.iter().rev() {
                remaining_steps.remove(i);
            }

            groups.push(current_group);
        }

        Ok(groups)
    }

    /// Validate pipeline configuration
    async fn validate_pipeline(&self, pipeline: &Pipeline) -> Result<()> {
        // Validate step configurations
        for step in &pipeline.steps {
            if let Some(executor) = self.step_executors.get(&step.step_type) {
                executor.validate_config(step)?;
            } else {
                return Err(anyhow!("Unknown step type: {}", step.step_type));
            }
        }

        // Validate dependencies
        for step in &pipeline.steps {
            for dep in &step.depends_on {
                if !pipeline.steps.iter().any(|s| s.name == *dep) {
                    return Err(anyhow!("Step '{}' depends on non-existent step '{}'", step.name, dep));
                }
            }
        }

        Ok(())
    }

    // Helper methods for event emission and metrics
    async fn emit_event(&self, event: ExecutionEvent) -> Result<()> {
        for listener in &self.event_listeners {
            listener.on_event(&event).await?;
        }
        Ok(())
    }

    async fn record_step_started(&self, step: &PipelineStep, context: &mut ExecutionContext, attempt: u32) -> Result<()> {
        let execution = StepExecution {
            step_name: step.name.clone(),
            step_type: step.step_type.clone(),
            start_time: SystemTime::now(),
            end_time: None,
            status: ExecutionStatus::Running,
            output: None,
            error: None,
            retry_count: attempt - 1,
        };
        
        context.step_history.push(execution);
        
        self.emit_event(ExecutionEvent {
            event_type: EventType::StepStarted,
            timestamp: SystemTime::now(),
            run_id: context.run_id.clone(),
            step_name: Some(step.name.clone()),
            data: HashMap::new(),
        }).await
    }

    async fn record_step_completed(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<()> {
        if let Some(execution) = context.step_history.last_mut() {
            execution.end_time = Some(SystemTime::now());
            execution.status = ExecutionStatus::Completed;
        }
        
        self.emit_event(ExecutionEvent {
            event_type: EventType::StepCompleted,
            timestamp: SystemTime::now(),
            run_id: context.run_id.clone(),
            step_name: Some(step.name.clone()),
            data: HashMap::new(),
        }).await
    }

    async fn record_step_failed(&self, step: &PipelineStep, context: &mut ExecutionContext, error: &anyhow::Error) -> Result<()> {
        if let Some(execution) = context.step_history.last_mut() {
            execution.end_time = Some(SystemTime::now());
            execution.status = ExecutionStatus::Failed;
            execution.error = Some(error.to_string());
        }
        
        self.emit_event(ExecutionEvent {
            event_type: EventType::StepFailed,
            timestamp: SystemTime::now(),
            run_id: context.run_id.clone(),
            step_name: Some(step.name.clone()),
            data: HashMap::new(),
        }).await
    }

    async fn record_step_skipped(&self, step: &PipelineStep, context: &mut ExecutionContext) -> Result<()> {
        let execution = StepExecution {
            step_name: step.name.clone(),
            step_type: step.step_type.clone(),
            start_time: SystemTime::now(),
            end_time: Some(SystemTime::now()),
            status: ExecutionStatus::Skipped,
            output: None,
            error: None,
            retry_count: 0,
        };
        
        context.step_history.push(execution);
        Ok(())
    }

    async fn record_step_retrying(&self, step: &PipelineStep, _context: &mut ExecutionContext, error: &anyhow::Error) -> Result<()> {
        self.emit_event(ExecutionEvent {
            event_type: EventType::StepRetrying,
            timestamp: SystemTime::now(),
            run_id: _context.run_id.clone(),
            step_name: Some(step.name.clone()),
            data: [("error".to_string(), serde_json::json!(error.to_string()))].into_iter().collect(),
        }).await
    }

    fn should_retry(&self, error: &anyhow::Error, retry_config: &RetryConfig) -> bool {
        if retry_config.retry_on.is_empty() {
            return true; // Retry on all errors if no specific patterns
        }
        
        let error_str = error.to_string();
        retry_config.retry_on.iter().any(|pattern| error_str.contains(pattern))
    }

    fn calculate_retry_delay(&self, attempt: u32, retry_config: &RetryConfig) -> Duration {
        let delay_ms = (retry_config.base_delay_ms as f64 * retry_config.backoff_multiplier.powi(attempt as i32 - 1)) as u64;
        Duration::from_millis(delay_ms.min(retry_config.max_delay_ms))
    }

    async fn update_pipeline_metrics(&self, result: &Result<ExecutionContext>) -> () {
        let mut metrics = self.metrics.write().await;
        metrics.total_pipelines += 1;
        
        match result {
            Ok(_) => metrics.successful_pipelines += 1,
            Err(_) => metrics.failed_pipelines += 1,
        }
    }

    async fn update_step_metrics(&self, step_type: &str, success: bool, attempts: u32) -> () {
        let mut metrics = self.metrics.write().await;
        metrics.total_steps += 1;
        
        if success {
            metrics.successful_steps += 1;
        } else {
            metrics.failed_steps += 1;
        }
        
        let step_metrics = metrics.step_type_metrics.entry(step_type.to_string()).or_default();
        step_metrics.total_executions += 1;
        step_metrics.retry_count += attempts as u64 - 1;
        
        if success {
            step_metrics.successful_executions += 1;
        } else {
            step_metrics.failed_executions += 1;
        }
    }

    /// Get execution metrics
    pub async fn get_metrics(&self) -> ExecutionMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            retry_on: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_creation() {
        let context = ExecutionContext {
            run_id: "test-run".to_string(),
            pipeline_name: "test-pipeline".to_string(),
            current_step: 0,
            variables: HashMap::new(),
            metadata: HashMap::new(),
            start_time: SystemTime::now(),
            step_history: Vec::new(),
        };
        
        assert_eq!(context.run_id, "test-run");
        assert_eq!(context.pipeline_name, "test-pipeline");
        assert_eq!(context.current_step, 0);
    }

    #[test]
    fn test_pipeline_step_creation() {
        let step = PipelineStep {
            name: "test-step".to_string(),
            step_type: "command".to_string(),
            config: HashMap::new(),
            timeout: Some(Duration::from_secs(30)),
            retry_config: Some(RetryConfig::default()),
            depends_on: vec!["previous-step".to_string()],
            condition: Some("${success} == true".to_string()),
            parallel_group: None,
        };
        
        assert_eq!(step.name, "test-step");
        assert_eq!(step.step_type, "command");
        assert!(step.timeout.is_some());
        assert!(step.retry_config.is_some());
        assert_eq!(step.depends_on.len(), 1);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 1);
        assert_eq!(config.base_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.retry_on.is_empty());
    }
}
