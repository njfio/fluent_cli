use crate::modular_pipeline_executor::{
    EventListener, ExecutionContext, ExecutionEvent, StateStore, VariableExpander,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Infrastructure implementations for the modular pipeline executor

/// Simple variable expander with template support
pub struct SimpleVariableExpander;

#[async_trait]
impl VariableExpander for SimpleVariableExpander {
    async fn expand(&self, template: &str, variables: &HashMap<String, String>) -> Result<String> {
        let mut result = template.to_string();

        // Replace ${variable} patterns
        for (key, value) in variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Replace $variable patterns (without braces)
        for (key, value) in variables {
            let placeholder = format!("${}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    async fn evaluate_condition(
        &self,
        condition: &str,
        variables: &HashMap<String, String>,
    ) -> Result<bool> {
        // Expand variables first
        let expanded = self.expand(condition, variables).await?;

        // Simple condition evaluation
        match expanded.trim() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => {
                // Try to evaluate simple comparisons
                if expanded.contains("==") {
                    let parts: Vec<&str> = expanded.split("==").collect();
                    if parts.len() == 2 {
                        return Ok(parts[0].trim() == parts[1].trim());
                    }
                }
                if expanded.contains("!=") {
                    let parts: Vec<&str> = expanded.split("!=").collect();
                    if parts.len() == 2 {
                        return Ok(parts[0].trim() != parts[1].trim());
                    }
                }
                if expanded.contains(">") {
                    let parts: Vec<&str> = expanded.split(">").collect();
                    if parts.len() == 2 {
                        if let (Ok(left), Ok(right)) = (
                            parts[0].trim().parse::<f64>(),
                            parts[1].trim().parse::<f64>(),
                        ) {
                            return Ok(left > right);
                        }
                    }
                }
                if expanded.contains("<") {
                    let parts: Vec<&str> = expanded.split("<").collect();
                    if parts.len() == 2 {
                        if let (Ok(left), Ok(right)) = (
                            parts[0].trim().parse::<f64>(),
                            parts[1].trim().parse::<f64>(),
                        ) {
                            return Ok(left < right);
                        }
                    }
                }

                // Default to false for unknown conditions
                Ok(false)
            }
        }
    }
}

/// File-based state store for pipeline execution context
pub struct FileStateStore {
    directory: PathBuf,
}

impl FileStateStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
}

#[async_trait]
impl StateStore for FileStateStore {
    async fn save_context(&self, context: &ExecutionContext) -> Result<()> {
        tokio::fs::create_dir_all(&self.directory).await?;

        let file_path = self.directory.join(format!("{}.json", context.run_id));
        let json = serde_json::to_string_pretty(context)?;
        tokio::fs::write(&file_path, json).await?;

        Ok(())
    }

    async fn load_context(&self, run_id: &str) -> Result<Option<ExecutionContext>> {
        let file_path = self.directory.join(format!("{}.json", run_id));

        if !file_path.exists() {
            return Ok(None);
        }

        let json = tokio::fs::read_to_string(&file_path).await?;
        let context: ExecutionContext = serde_json::from_str(&json)?;

        Ok(Some(context))
    }

    async fn delete_context(&self, run_id: &str) -> Result<()> {
        let file_path = self.directory.join(format!("{}.json", run_id));

        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await?;
        }

        Ok(())
    }
}

/// In-memory state store for testing and temporary storage
#[derive(Clone)]
pub struct MemoryStateStore {
    contexts: Arc<RwLock<HashMap<String, ExecutionContext>>>,
    pipeline_states: Arc<RwLock<HashMap<String, crate::pipeline_executor::PipelineState>>>,
}

impl MemoryStateStore {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            pipeline_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StateStore for MemoryStateStore {
    async fn save_context(&self, context: &ExecutionContext) -> Result<()> {
        let mut contexts = self.contexts.write().await;
        contexts.insert(context.run_id.clone(), context.clone());
        Ok(())
    }

    async fn load_context(&self, run_id: &str) -> Result<Option<ExecutionContext>> {
        let contexts = self.contexts.read().await;
        Ok(contexts.get(run_id).cloned())
    }

    async fn delete_context(&self, run_id: &str) -> Result<()> {
        let mut contexts = self.contexts.write().await;
        contexts.remove(run_id);
        Ok(())
    }
}

// Implement the pipeline_executor::StateStore trait as well
#[async_trait]
impl crate::pipeline_executor::StateStore for MemoryStateStore {
    async fn save_state(&self, pipeline_name: &str, state: &crate::pipeline_executor::PipelineState) -> anyhow::Result<()> {
        let mut states = self.pipeline_states.write().await;
        states.insert(pipeline_name.to_string(), state.clone());
        Ok(())
    }

    async fn load_state(&self, pipeline_name: &str) -> anyhow::Result<Option<crate::pipeline_executor::PipelineState>> {
        let states = self.pipeline_states.read().await;
        Ok(states.get(pipeline_name).cloned())
    }
}

/// Console event listener for debugging and monitoring
pub struct ConsoleEventListener;

#[async_trait]
impl EventListener for ConsoleEventListener {
    async fn on_event(&self, event: &ExecutionEvent) -> Result<()> {
        let timestamp = event
            .timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match &event.event_type {
            crate::modular_pipeline_executor::EventType::PipelineStarted => {
                println!("[{}] 🚀 Pipeline started: {}", timestamp, event.run_id);
            }
            crate::modular_pipeline_executor::EventType::PipelineCompleted => {
                println!("[{}] ✅ Pipeline completed: {}", timestamp, event.run_id);
            }
            crate::modular_pipeline_executor::EventType::PipelineFailed => {
                println!("[{}] ❌ Pipeline failed: {}", timestamp, event.run_id);
            }
            crate::modular_pipeline_executor::EventType::StepStarted => {
                if let Some(step_name) = &event.step_name {
                    println!(
                        "[{}] 🔄 Step started: {} ({})",
                        timestamp, step_name, event.run_id
                    );
                }
            }
            crate::modular_pipeline_executor::EventType::StepCompleted => {
                if let Some(step_name) = &event.step_name {
                    println!(
                        "[{}] ✅ Step completed: {} ({})",
                        timestamp, step_name, event.run_id
                    );
                }
            }
            crate::modular_pipeline_executor::EventType::StepFailed => {
                if let Some(step_name) = &event.step_name {
                    println!(
                        "[{}] ❌ Step failed: {} ({})",
                        timestamp, step_name, event.run_id
                    );
                }
            }
            crate::modular_pipeline_executor::EventType::StepRetrying => {
                if let Some(step_name) = &event.step_name {
                    println!(
                        "[{}] 🔄 Step retrying: {} ({})",
                        timestamp, step_name, event.run_id
                    );
                }
            }
            crate::modular_pipeline_executor::EventType::VariableSet => {
                println!("[{}] 📝 Variable set ({})", timestamp, event.run_id);
            }
            crate::modular_pipeline_executor::EventType::ConditionEvaluated => {
                println!("[{}] 🤔 Condition evaluated ({})", timestamp, event.run_id);
            }
        }

        Ok(())
    }
}

/// File-based event listener for persistent logging
pub struct FileEventListener {
    log_file: PathBuf,
}

impl FileEventListener {
    pub fn new(log_file: PathBuf) -> Self {
        Self { log_file }
    }
}

#[async_trait]
impl EventListener for FileEventListener {
    async fn on_event(&self, event: &ExecutionEvent) -> Result<()> {
        let log_entry = serde_json::to_string(event)?;
        let log_line = format!("{}\n", log_entry);

        // Ensure parent directory exists
        if let Some(parent) = self.log_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Append to log file
        tokio::fs::write(&self.log_file, log_line).await?;

        Ok(())
    }
}

/// Metrics event listener for collecting execution statistics
pub struct MetricsEventListener {
    metrics: Arc<RwLock<PipelineMetrics>>,
}

#[derive(Debug, Default, Clone)]
pub struct PipelineMetrics {
    pub total_pipelines: u64,
    pub successful_pipelines: u64,
    pub failed_pipelines: u64,
    pub total_steps: u64,
    pub successful_steps: u64,
    pub failed_steps: u64,
    pub step_durations: HashMap<String, Vec<u64>>, // step_name -> durations in ms
}

impl MetricsEventListener {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PipelineMetrics::default())),
        }
    }

    pub async fn get_metrics(&self) -> PipelineMetrics {
        self.metrics.read().await.clone()
    }
}

#[async_trait]
impl EventListener for MetricsEventListener {
    async fn on_event(&self, event: &ExecutionEvent) -> Result<()> {
        let mut metrics = self.metrics.write().await;

        match &event.event_type {
            crate::modular_pipeline_executor::EventType::PipelineStarted => {
                metrics.total_pipelines += 1;
            }
            crate::modular_pipeline_executor::EventType::PipelineCompleted => {
                metrics.successful_pipelines += 1;
            }
            crate::modular_pipeline_executor::EventType::PipelineFailed => {
                metrics.failed_pipelines += 1;
            }
            crate::modular_pipeline_executor::EventType::StepStarted => {
                metrics.total_steps += 1;
            }
            crate::modular_pipeline_executor::EventType::StepCompleted => {
                metrics.successful_steps += 1;

                // Record step duration if available
                if let Some(step_name) = &event.step_name {
                    if let Some(duration) = event.data.get("duration_ms").and_then(|v| v.as_u64()) {
                        metrics
                            .step_durations
                            .entry(step_name.clone())
                            .or_insert_with(Vec::new)
                            .push(duration);
                    }
                }
            }
            crate::modular_pipeline_executor::EventType::StepFailed => {
                metrics.failed_steps += 1;
            }
            _ => {} // Ignore other event types for metrics
        }

        Ok(())
    }
}

/// Builder for creating configured pipeline executors
pub struct PipelineExecutorBuilder {
    state_store: Option<Arc<dyn StateStore>>,
    variable_expander: Option<Arc<dyn VariableExpander>>,
    event_listeners: Vec<Arc<dyn EventListener>>,
}

impl PipelineExecutorBuilder {
    pub fn new() -> Self {
        Self {
            state_store: None,
            variable_expander: None,
            event_listeners: Vec::new(),
        }
    }

    pub fn with_file_state_store(mut self, directory: PathBuf) -> Self {
        self.state_store = Some(Arc::new(FileStateStore::new(directory)));
        self
    }

    pub fn with_memory_state_store(mut self) -> Self {
        self.state_store = Some(Arc::new(MemoryStateStore::new()));
        self
    }

    pub fn with_simple_variable_expander(mut self) -> Self {
        self.variable_expander = Some(Arc::new(SimpleVariableExpander));
        self
    }

    pub fn with_console_logging(mut self) -> Self {
        self.event_listeners.push(Arc::new(ConsoleEventListener));
        self
    }

    pub fn with_file_logging(mut self, log_file: PathBuf) -> Self {
        self.event_listeners
            .push(Arc::new(FileEventListener::new(log_file)));
        self
    }

    pub fn with_metrics(mut self) -> (Self, Arc<MetricsEventListener>) {
        let metrics_listener = Arc::new(MetricsEventListener::new());
        let metrics_clone = metrics_listener.clone();
        self.event_listeners.push(metrics_listener);
        (self, metrics_clone)
    }

    pub fn build(self) -> Result<crate::modular_pipeline_executor::ModularPipelineExecutor> {
        let state_store = self
            .state_store
            .ok_or_else(|| anyhow!("State store is required"))?;

        let variable_expander = self
            .variable_expander
            .ok_or_else(|| anyhow!("Variable expander is required"))?;

        let mut executor = crate::modular_pipeline_executor::ModularPipelineExecutor::new(
            state_store,
            variable_expander,
        );

        // Add step executors
        executor.register_step_executor(Arc::new(
            crate::pipeline_step_executors::CommandStepExecutor,
        ));
        executor.register_step_executor(Arc::new(
            crate::pipeline_step_executors::HttpStepExecutor::new(),
        ));
        executor.register_step_executor(Arc::new(crate::pipeline_step_executors::FileStepExecutor));
        executor.register_step_executor(Arc::new(
            crate::pipeline_step_executors::ConditionStepExecutor,
        ));

        // Add event listeners
        for listener in self.event_listeners {
            executor.add_event_listener(listener);
        }

        Ok(executor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_simple_variable_expander() {
        let expander = SimpleVariableExpander;
        let variables = [
            ("name".to_string(), "world".to_string()),
            ("count".to_string(), "42".to_string()),
        ]
        .into_iter()
        .collect();

        let result = expander.expand("Hello ${name}!", &variables).await.unwrap();
        assert_eq!(result, "Hello world!");

        let result = expander.expand("Count: $count", &variables).await.unwrap();
        assert_eq!(result, "Count: 42");
    }

    #[tokio::test]
    async fn test_condition_evaluation() {
        let expander = SimpleVariableExpander;
        let variables = [
            ("status".to_string(), "success".to_string()),
            ("count".to_string(), "5".to_string()),
        ]
        .into_iter()
        .collect();

        assert!(expander
            .evaluate_condition("true", &variables)
            .await
            .unwrap());
        assert!(!expander
            .evaluate_condition("false", &variables)
            .await
            .unwrap());
        assert!(expander
            .evaluate_condition("${status} == success", &variables)
            .await
            .unwrap());
        assert!(!expander
            .evaluate_condition("${status} == failure", &variables)
            .await
            .unwrap());
        assert!(expander
            .evaluate_condition("${count} > 3", &variables)
            .await
            .unwrap());
        assert!(!expander
            .evaluate_condition("${count} < 3", &variables)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_memory_state_store() {
        let store = MemoryStateStore::new();
        let context = ExecutionContext {
            run_id: "test-run".to_string(),
            pipeline_name: "test-pipeline".to_string(),
            current_step: 0,
            variables: HashMap::new(),
            metadata: HashMap::new(),
            start_time: std::time::SystemTime::now(),
            step_history: Vec::new(),
        };

        // Save context
        store.save_context(&context).await.unwrap();

        // Load context
        let loaded = store.load_context("test-run").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().run_id, "test-run");

        // Delete context
        store.delete_context("test-run").await.unwrap();
        let deleted = store.load_context("test-run").await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_file_state_store() {
        let temp_dir = tempdir().unwrap();
        let store = FileStateStore::new(temp_dir.path().to_path_buf());

        let context = ExecutionContext {
            run_id: "test-run".to_string(),
            pipeline_name: "test-pipeline".to_string(),
            current_step: 0,
            variables: HashMap::new(),
            metadata: HashMap::new(),
            start_time: std::time::SystemTime::now(),
            step_history: Vec::new(),
        };

        // Save context
        store.save_context(&context).await.unwrap();

        // Load context
        let loaded = store.load_context("test-run").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().run_id, "test-run");
    }

    #[tokio::test]
    async fn test_pipeline_executor_builder() {
        let temp_dir = tempdir().unwrap();

        let executor = PipelineExecutorBuilder::new()
            .with_file_state_store(temp_dir.path().to_path_buf())
            .with_simple_variable_expander()
            .with_console_logging()
            .build()
            .unwrap();

        // Test that executor was created successfully
        let metrics = executor.get_metrics().await;
        assert_eq!(metrics.total_pipelines, 0);
    }
}
