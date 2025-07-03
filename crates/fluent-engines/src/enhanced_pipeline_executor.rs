use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::pipeline_executor::{Pipeline, PipelineState, PipelineStep, StateStore};

/// Enhanced pipeline executor with optimized parallel execution
pub struct EnhancedPipelineExecutor<S: StateStore> {
    state_store: S,
    config: ExecutorConfig,
    metrics: Arc<RwLock<ExecutorMetrics>>,
    resource_monitor: Arc<ResourceMonitor>,
}

/// Configuration for the enhanced executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Maximum number of concurrent steps
    pub max_concurrency: usize,
    /// Enable adaptive concurrency based on system resources
    pub adaptive_concurrency: bool,
    /// Maximum memory usage in MB before throttling
    pub max_memory_mb: usize,
    /// CPU usage threshold for throttling (0.0-1.0)
    pub cpu_threshold: f64,
    /// Timeout for individual steps
    pub step_timeout: Duration,
    /// Enable step dependency analysis
    pub dependency_analysis: bool,
    /// Batch size for parallel execution
    pub batch_size: usize,
    /// Enable result caching
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: num_cpus::get() * 2,
            adaptive_concurrency: true,
            max_memory_mb: 1024,
            cpu_threshold: 0.8,
            step_timeout: Duration::from_secs(300),
            dependency_analysis: true,
            batch_size: 10,
            enable_caching: true,
            cache_ttl: 300,
        }
    }
}

/// Execution metrics for monitoring and optimization
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ExecutorMetrics {
    pub total_pipelines: u64,
    pub successful_pipelines: u64,
    pub failed_pipelines: u64,
    pub total_steps: u64,
    pub parallel_steps: u64,
    pub sequential_steps: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub average_execution_time_ms: f64,
    pub peak_concurrency: usize,
    pub resource_throttling_events: u64,
    pub step_type_performance: HashMap<String, StepPerformance>,
}

/// Performance metrics for specific step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepPerformance {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub average_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
    pub error_rate: f64,
}

impl Default for StepPerformance {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            average_duration_ms: 0.0,
            min_duration_ms: f64::MAX,
            max_duration_ms: 0.0,
            error_rate: 0.0,
        }
    }
}

/// Resource monitoring for adaptive concurrency
pub struct ResourceMonitor {
    cpu_usage: Arc<RwLock<f64>>,
    memory_usage_mb: Arc<RwLock<usize>>,
    active_tasks: Arc<RwLock<usize>>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            cpu_usage: Arc::new(RwLock::new(0.0)),
            memory_usage_mb: Arc::new(RwLock::new(0)),
            active_tasks: Arc::new(RwLock::new(0)),
        }
    }

    /// Start background resource monitoring
    pub async fn start_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let cpu_usage = self.cpu_usage.clone();
        let memory_usage = self.memory_usage_mb.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                // Update CPU usage (simplified - in production use sysinfo crate)
                let cpu = Self::get_cpu_usage().await;
                *cpu_usage.write().await = cpu;

                // Update memory usage
                let memory = Self::get_memory_usage_mb().await;
                *memory_usage.write().await = memory;
            }
        })
    }

    async fn get_cpu_usage() -> f64 {
        // Simplified CPU usage - in production use sysinfo crate
        0.5 // Placeholder
    }

    async fn get_memory_usage_mb() -> usize {
        // Simplified memory usage - in production use sysinfo crate
        512 // Placeholder
    }

    /// Increment active task count
    pub async fn increment_active_tasks(&self) {
        let mut count = self.active_tasks.write().await;
        *count += 1;
    }

    /// Decrement active task count
    pub async fn decrement_active_tasks(&self) {
        let mut count = self.active_tasks.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Get current active task count
    pub async fn get_active_task_count(&self) -> usize {
        *self.active_tasks.read().await
    }

    pub async fn should_throttle(&self, config: &ExecutorConfig) -> bool {
        let cpu = *self.cpu_usage.read().await;
        let memory = *self.memory_usage_mb.read().await;

        cpu > config.cpu_threshold || memory > config.max_memory_mb
    }

    pub async fn get_optimal_concurrency(&self, config: &ExecutorConfig) -> usize {
        if !config.adaptive_concurrency {
            return config.max_concurrency;
        }

        let cpu = *self.cpu_usage.read().await;
        let memory = *self.memory_usage_mb.read().await;

        // Adaptive concurrency based on resource usage
        let cpu_factor = (1.0 - cpu).max(0.1);
        let memory_factor = (1.0 - (memory as f64 / config.max_memory_mb as f64)).max(0.1);

        let optimal = (config.max_concurrency as f64 * cpu_factor * memory_factor) as usize;
        optimal.max(1).min(config.max_concurrency)
    }
}

/// Step execution context with dependency tracking
#[derive(Debug, Clone)]
pub struct StepExecutionContext {
    pub step_id: String,
    pub dependencies: HashSet<String>,
    pub dependents: HashSet<String>,
    pub priority: i32,
    pub estimated_duration: Duration,
    pub resource_requirements: ResourceRequirements,
}

/// Resource requirements for a step
#[derive(Debug, Clone, Default)]
pub struct ResourceRequirements {
    pub cpu_intensive: bool,
    pub memory_intensive: bool,
    pub io_intensive: bool,
    pub network_intensive: bool,
}

/// Execution batch for parallel processing
#[derive(Debug)]
pub struct ExecutionBatch {
    pub steps: Vec<(PipelineStep, StepExecutionContext)>,
    pub batch_id: String,
    pub estimated_duration: Duration,
}

impl<S: StateStore + Clone + Send + Sync + 'static> EnhancedPipelineExecutor<S> {
    /// Create a new enhanced pipeline executor
    pub fn new(state_store: S, config: ExecutorConfig) -> Self {
        let resource_monitor = Arc::new(ResourceMonitor::new());

        Self {
            state_store,
            config,
            metrics: Arc::new(RwLock::new(ExecutorMetrics::default())),
            resource_monitor,
        }
    }

    /// Execute a pipeline with enhanced parallel optimization
    pub async fn execute_pipeline(
        &self,
        pipeline: &Pipeline,
        initial_input: &str,
        force_fresh: bool,
        provided_run_id: Option<String>,
    ) -> Result<String> {
        let start_time = Instant::now();
        let run_id = provided_run_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        info!(
            "Starting enhanced pipeline execution: {} (run_id: {})",
            pipeline.name, run_id
        );

        // Start resource monitoring
        let _monitor_handle = self.resource_monitor.start_monitoring().await;

        // Load or create pipeline state
        let state_key = format!("{}-{}", pipeline.name, run_id);
        let mut state = if force_fresh {
            PipelineState {
                current_step: 0,
                data: {
                    let mut data = HashMap::new();
                    data.insert("input".to_string(), initial_input.to_string());
                    data
                },
                run_id: run_id.clone(),
                start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            }
        } else {
            self.state_store
                .load_state(&state_key)
                .await?
                .unwrap_or_else(|| PipelineState {
                    current_step: 0,
                    data: {
                        let mut data = HashMap::new();
                        data.insert("input".to_string(), initial_input.to_string());
                        data
                    },
                    run_id: run_id.clone(),
                    start_time: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                })
        };

        state.data.insert("run_id".to_string(), run_id.clone());

        // Analyze step dependencies and create execution plan
        let execution_plan = self.create_execution_plan(&pipeline.steps).await?;

        // Execute batches in dependency order
        for batch in execution_plan {
            self.execute_batch(&batch, &mut state).await?;

            // Save state after each batch
            self.state_store.save_state(&state_key, &state).await?;
        }

        // Update metrics
        let execution_time = start_time.elapsed();
        self.update_pipeline_metrics(true, execution_time).await;

        info!("Pipeline execution completed in {:?}", execution_time);

        Ok(state
            .data
            .get("output")
            .unwrap_or(&"Pipeline completed successfully".to_string())
            .clone())
    }

    /// Create optimized execution plan with dependency analysis
    async fn create_execution_plan(&self, steps: &[PipelineStep]) -> Result<Vec<ExecutionBatch>> {
        if !self.config.dependency_analysis {
            // Simple batching without dependency analysis
            return self.create_simple_batches(steps).await;
        }

        // Analyze dependencies and create optimized batches
        let step_contexts = self.analyze_step_dependencies(steps).await?;
        let batches = self
            .create_dependency_aware_batches(steps, step_contexts)
            .await?;

        Ok(batches)
    }

    /// Analyze step dependencies for optimal scheduling
    async fn analyze_step_dependencies(
        &self,
        steps: &[PipelineStep],
    ) -> Result<Vec<StepExecutionContext>> {
        let mut contexts = Vec::new();

        for (index, step) in steps.iter().enumerate() {
            let step_id = format!("step_{}", index);
            let mut dependencies = HashSet::new();
            let dependents = HashSet::new();

            // Analyze variable dependencies (simplified)
            if let Some(step_vars) = self.extract_step_variables(step).await {
                for var in step_vars {
                    // Check if this variable is produced by previous steps
                    for (prev_index, _) in steps.iter().enumerate().take(index) {
                        if self.step_produces_variable(&steps[prev_index], &var).await {
                            dependencies.insert(format!("step_{}", prev_index));
                        }
                    }
                }
            }

            // Estimate execution duration based on step type
            let estimated_duration = self.estimate_step_duration(step).await;

            // Determine resource requirements
            let resource_requirements = self.analyze_resource_requirements(step).await;

            contexts.push(StepExecutionContext {
                step_id,
                dependencies,
                dependents,
                priority: self.calculate_step_priority(step).await,
                estimated_duration,
                resource_requirements,
            });
        }

        Ok(contexts)
    }

    /// Create dependency-aware execution batches
    async fn create_dependency_aware_batches(
        &self,
        steps: &[PipelineStep],
        contexts: Vec<StepExecutionContext>,
    ) -> Result<Vec<ExecutionBatch>> {
        let mut batches = Vec::new();
        let mut remaining_steps: VecDeque<_> = steps.iter().zip(contexts.iter()).collect();
        let mut completed_steps = HashSet::new();

        while !remaining_steps.is_empty() {
            let mut current_batch = Vec::new();
            let mut batch_duration = Duration::ZERO;

            // Find steps that can be executed in parallel (no unresolved dependencies)
            let mut i = 0;
            while i < remaining_steps.len() && current_batch.len() < self.config.batch_size {
                let (_step, context) = &remaining_steps[i];

                // Check if all dependencies are satisfied
                let dependencies_satisfied = context
                    .dependencies
                    .iter()
                    .all(|dep| completed_steps.contains(dep));

                if dependencies_satisfied {
                    let (step, context) = remaining_steps.remove(i).unwrap();
                    current_batch.push((step.clone(), context.clone()));
                    batch_duration = batch_duration.max(context.estimated_duration);
                    completed_steps.insert(context.step_id.clone());
                } else {
                    i += 1;
                }
            }

            if current_batch.is_empty() {
                return Err(anyhow!("Circular dependency detected in pipeline steps"));
            }

            batches.push(ExecutionBatch {
                steps: current_batch,
                batch_id: Uuid::new_v4().to_string(),
                estimated_duration: batch_duration,
            });
        }

        Ok(batches)
    }

    /// Execute a batch of steps in parallel
    async fn execute_batch(&self, batch: &ExecutionBatch, state: &mut PipelineState) -> Result<()> {
        info!(
            "Executing batch {} with {} steps",
            batch.batch_id,
            batch.steps.len()
        );

        if batch.steps.len() == 1 {
            // Single step - execute directly
            let (step, _) = &batch.steps[0];
            let result = self.execute_single_step(step, state).await?;
            state.data.extend(result);
            return Ok(());
        }

        // Parallel execution with adaptive concurrency
        let optimal_concurrency = self
            .resource_monitor
            .get_optimal_concurrency(&self.config)
            .await;
        let semaphore = Arc::new(Semaphore::new(optimal_concurrency));
        let state_arc = Arc::new(Mutex::new(state.clone()));
        let mut join_set = JoinSet::new();

        for (step, context) in &batch.steps {
            let permit = semaphore.clone().acquire_owned().await?;
            let step = step.clone();
            let context = context.clone();
            let state_clone = state_arc.clone();
            let metrics = self.metrics.clone();
            let resource_monitor = self.resource_monitor.clone();
            let config = self.config.clone();

            join_set.spawn(async move {
                let _permit = permit; // Hold permit for duration
                let start_time = Instant::now();

                // Check for resource throttling
                if resource_monitor.should_throttle(&config).await {
                    warn!("Resource throttling detected, delaying step execution");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }

                // Execute step with timeout
                let result = tokio::time::timeout(
                    config.step_timeout,
                    Self::execute_step_with_context(&step, &context, state_clone),
                )
                .await;

                let _execution_time = start_time.elapsed();

                // Update metrics
                {
                    let mut m = metrics.write().await;
                    m.total_steps += 1;
                    m.parallel_steps += 1;

                    match &result {
                        Ok(Ok(_)) => m.successful_pipelines += 1,
                        _ => {}
                    }
                }

                match result {
                    Ok(step_result) => step_result,
                    Err(_) => Err(anyhow!(
                        "Step {} timed out after {:?}",
                        context.step_id,
                        config.step_timeout
                    )),
                }
            });
        }

        // Collect results
        let mut combined_results = HashMap::new();
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(step_result)) => {
                    combined_results.extend(step_result);
                }
                Ok(Err(e)) => {
                    error!("Step execution failed: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    error!("Join error: {}", e);
                    return Err(anyhow!("Task join error: {}", e));
                }
            }
        }

        // Merge results back to main state
        let final_state = state_arc.lock().await;
        state.data.extend(final_state.data.clone());
        state.data.extend(combined_results);

        Ok(())
    }

    /// Execute a single step with context
    async fn execute_step_with_context(
        step: &PipelineStep,
        _context: &StepExecutionContext,
        state_arc: Arc<Mutex<PipelineState>>,
    ) -> Result<HashMap<String, String>> {
        let _state_guard = state_arc.lock().await;
        let step_name = Self::get_step_name(step);

        // Simplified step execution - integrate with existing step execution logic
        match step {
            PipelineStep::Command {
                name,
                command,
                save_output: _,
                retry: _,
            } => {
                debug!("Executing command step: {}", name);
                // Placeholder - integrate with actual command execution
                Ok(HashMap::from([(
                    name.clone(),
                    format!("Executed: {}", command),
                )]))
            }
            _ => {
                debug!("Executing step: {}", step_name);
                Ok(HashMap::from([(
                    step_name.to_string(),
                    "Step completed".to_string(),
                )]))
            }
        }
    }

    /// Create simple batches without dependency analysis
    async fn create_simple_batches(&self, steps: &[PipelineStep]) -> Result<Vec<ExecutionBatch>> {
        let mut batches = Vec::new();

        for chunk in steps.chunks(self.config.batch_size) {
            let batch_steps = chunk
                .iter()
                .enumerate()
                .map(|(i, step)| {
                    let context = StepExecutionContext {
                        step_id: format!("step_{}", i),
                        dependencies: HashSet::new(),
                        dependents: HashSet::new(),
                        priority: 0,
                        estimated_duration: Duration::from_secs(30),
                        resource_requirements: ResourceRequirements::default(),
                    };
                    (step.clone(), context)
                })
                .collect();

            batches.push(ExecutionBatch {
                steps: batch_steps,
                batch_id: Uuid::new_v4().to_string(),
                estimated_duration: Duration::from_secs(30),
            });
        }

        Ok(batches)
    }

    /// Get step name helper function
    fn get_step_name(step: &PipelineStep) -> &str {
        match step {
            PipelineStep::Command { name, .. } => name,
            PipelineStep::ShellCommand { name, .. } => name,
            PipelineStep::Condition { name, .. } => name,
            PipelineStep::Loop { name, .. } => name,
            PipelineStep::Map { name, .. } => name,
            PipelineStep::SubPipeline { name, .. } => name,
            PipelineStep::HumanInTheLoop { name, .. } => name,
            PipelineStep::RepeatUntil { name, .. } => name,
            PipelineStep::PrintOutput { name, .. } => name,
            PipelineStep::ForEach { name, .. } => name,
            PipelineStep::TryCatch { name, .. } => name,
            PipelineStep::Parallel { name, .. } => name,
            PipelineStep::Timeout { name, .. } => name,
        }
    }

    /// Execute a single step (fallback to existing implementation)
    async fn execute_single_step(
        &self,
        step: &PipelineStep,
        _state: &mut PipelineState,
    ) -> Result<HashMap<String, String>> {
        // Integrate with existing step execution logic from pipeline_executor.rs
        let step_name = Self::get_step_name(step);
        match step {
            PipelineStep::Command {
                name,
                command,
                save_output: _,
                retry: _,
            } => {
                debug!("Executing command step: {}", name);
                Ok(HashMap::from([(
                    name.clone(),
                    format!("Executed: {}", command),
                )]))
            }
            _ => {
                debug!("Executing step: {}", step_name);
                Ok(HashMap::from([(
                    step_name.to_string(),
                    "Step completed".to_string(),
                )]))
            }
        }
    }

    /// Helper methods for dependency analysis
    async fn extract_step_variables(&self, _step: &PipelineStep) -> Option<Vec<String>> {
        // Placeholder - implement variable extraction logic
        None
    }

    async fn step_produces_variable(&self, _step: &PipelineStep, _var: &str) -> bool {
        // Placeholder - implement variable production analysis
        false
    }

    async fn estimate_step_duration(&self, step: &PipelineStep) -> Duration {
        // Estimate based on step type and historical data
        match step {
            PipelineStep::Command { .. } => Duration::from_secs(10),
            PipelineStep::ShellCommand { .. } => Duration::from_secs(15),
            _ => Duration::from_secs(5),
        }
    }

    async fn analyze_resource_requirements(&self, step: &PipelineStep) -> ResourceRequirements {
        // Analyze step for resource requirements
        match step {
            PipelineStep::Command { command, .. } => ResourceRequirements {
                cpu_intensive: command.contains("compile") || command.contains("build"),
                memory_intensive: command.contains("large") || command.contains("memory"),
                io_intensive: command.contains("file") || command.contains("disk"),
                network_intensive: command.contains("http") || command.contains("download"),
            },
            _ => ResourceRequirements::default(),
        }
    }

    async fn calculate_step_priority(&self, _step: &PipelineStep) -> i32 {
        // Calculate priority based on step characteristics
        0 // Placeholder
    }

    /// Update pipeline execution metrics
    async fn update_pipeline_metrics(&self, success: bool, execution_time: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.total_pipelines += 1;

        if success {
            metrics.successful_pipelines += 1;
        } else {
            metrics.failed_pipelines += 1;
        }

        // Update average execution time
        let total_time = metrics.average_execution_time_ms * (metrics.total_pipelines - 1) as f64;
        metrics.average_execution_time_ms =
            (total_time + execution_time.as_millis() as f64) / metrics.total_pipelines as f64;
    }

    /// Get current execution metrics
    pub async fn get_metrics(&self) -> ExecutorMetrics {
        self.metrics.read().await.clone()
    }

    /// Get current configuration
    pub fn get_config(&self) -> &ExecutorConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_infrastructure::MemoryStateStore;

    #[tokio::test]
    async fn test_enhanced_executor_creation() {
        let state_store = MemoryStateStore::new();
        let config = ExecutorConfig::default();
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        assert_eq!(executor.get_config().max_concurrency, num_cpus::get() * 2);
    }

    #[tokio::test]
    async fn test_resource_monitor() {
        let monitor = ResourceMonitor::new();
        let config = ExecutorConfig::default();

        let optimal_concurrency = monitor.get_optimal_concurrency(&config).await;
        assert!(optimal_concurrency > 0);
        assert!(optimal_concurrency <= config.max_concurrency);
    }

    #[tokio::test]
    async fn test_execution_batch_creation() {
        let state_store = MemoryStateStore::new();
        let config = ExecutorConfig::default();
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        let steps = vec![
            PipelineStep::Command {
                name: "test1".to_string(),
                command: "echo test1".to_string(),
                save_output: true,
                retry: None,
            },
            PipelineStep::Command {
                name: "test2".to_string(),
                command: "echo test2".to_string(),
                save_output: true,
                retry: None,
            },
        ];

        let batches = executor.create_simple_batches(&steps).await.unwrap();
        assert!(!batches.is_empty());
        assert_eq!(batches[0].steps.len(), 2);
    }
}

// Include comprehensive test suite
#[path = "enhanced_pipeline_executor_tests.rs"]
mod enhanced_pipeline_executor_tests;
