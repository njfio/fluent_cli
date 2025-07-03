use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

pub mod engine;
pub mod template;

/// Workflow definition structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub inputs: Vec<WorkflowInput>,
    pub outputs: Vec<WorkflowOutput>,
    pub steps: Vec<WorkflowStep>,
    pub error_handling: Option<ErrorHandlingConfig>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Workflow input definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub name: String,
    pub input_type: String,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
    pub validation: Option<ValidationConfig>,
}

/// Workflow output definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    pub name: String,
    pub output_type: String,
    pub description: Option<String>,
}

/// Workflow step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: Option<String>,
    pub tool: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub depends_on: Option<Vec<String>>,
    pub outputs: Option<HashMap<String, String>>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<String>,
    pub parallel: Option<bool>,
    pub condition: Option<String>,
    pub on_error: Option<ErrorAction>,
}

/// Validation configuration for inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<serde_json::Value>>,
    pub numeric_range: Option<NumericRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Retry configuration for steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub retry_on: Option<Vec<String>>,
}

/// Backoff strategy for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Fixed {
        delay: String,
    },
    Exponential {
        initial_delay: String,
        max_delay: String,
    },
    Linear {
        increment: String,
    },
}

/// Error handling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingConfig {
    pub on_failure: FailureStrategy,
    pub compensation: Option<CompensationConfig>,
    pub notifications: Option<Vec<NotificationConfig>>,
}

/// Failure strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureStrategy {
    Fail,
    Continue,
    Retry { config: RetryConfig },
    Compensate { steps: Vec<String> },
    Rollback,
}

/// Error action for individual steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorAction {
    Fail,
    Continue,
    Retry,
    Skip,
    Compensate { step: String },
}

/// Compensation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationConfig {
    pub steps: Vec<CompensationStep>,
    pub timeout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationStep {
    pub for_step: String,
    pub tool: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub channel: String,
    pub events: Vec<String>,
    pub template: Option<String>,
}

/// Workflow execution context
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    pub workflow_id: String,
    pub execution_id: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub step_outputs: HashMap<String, HashMap<String, serde_json::Value>>,
    pub step_status: HashMap<String, StepStatus>,
    pub variables: HashMap<String, serde_json::Value>,
    pub start_time: std::time::SystemTime,
    pub metadata: HashMap<String, serde_json::Value>,
    pub step_start_times: HashMap<String, std::time::SystemTime>,
    pub step_end_times: HashMap<String, std::time::SystemTime>,
    pub step_attempts: HashMap<String, u32>,
}

/// Step execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String, attempt: u32 },
    Skipped,
    Cancelled,
    Compensated,
}

/// Workflow execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub workflow_id: String,
    pub execution_id: String,
    pub status: WorkflowStatus,
    pub outputs: HashMap<String, serde_json::Value>,
    pub step_results: HashMap<String, StepResult>,
    pub start_time: std::time::SystemTime,
    pub end_time: std::time::SystemTime,
    pub duration: Duration,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Overall workflow status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Compensated,
}

/// Individual step result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub status: StepStatus,
    pub outputs: HashMap<String, serde_json::Value>,
    pub start_time: std::time::SystemTime,
    pub end_time: Option<std::time::SystemTime>,
    pub duration: Option<Duration>,
    pub error: Option<String>,
    pub attempts: u32,
}

impl WorkflowContext {
    pub fn new(
        workflow_id: String,
        execution_id: String,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            workflow_id,
            execution_id,
            inputs,
            step_outputs: HashMap::new(),
            step_status: HashMap::new(),
            variables: HashMap::new(),
            start_time: std::time::SystemTime::now(),
            metadata: HashMap::new(),
            step_start_times: HashMap::new(),
            step_end_times: HashMap::new(),
            step_attempts: HashMap::new(),
        }
    }

    pub fn set_step_status(&mut self, step_id: &str, status: StepStatus) {
        self.step_status.insert(step_id.to_string(), status);
    }

    pub fn get_step_status(&self, step_id: &str) -> Option<&StepStatus> {
        self.step_status.get(step_id)
    }

    pub fn set_step_output(&mut self, step_id: &str, key: &str, value: serde_json::Value) {
        self.step_outputs
            .entry(step_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value);
    }

    pub fn start_step_timing(&mut self, step_id: &str) {
        self.step_start_times.insert(step_id.to_string(), std::time::SystemTime::now());
        // Initialize attempt counter if not exists
        self.step_attempts.entry(step_id.to_string()).or_insert(0);
    }

    pub fn end_step_timing(&mut self, step_id: &str) {
        self.step_end_times.insert(step_id.to_string(), std::time::SystemTime::now());
    }

    pub fn increment_step_attempts(&mut self, step_id: &str) {
        *self.step_attempts.entry(step_id.to_string()).or_insert(0) += 1;
    }

    pub fn get_step_duration(&self, step_id: &str) -> Option<std::time::Duration> {
        if let (Some(start), Some(end)) = (
            self.step_start_times.get(step_id),
            self.step_end_times.get(step_id),
        ) {
            end.duration_since(*start).ok()
        } else {
            None
        }
    }

    pub fn get_step_attempts(&self, step_id: &str) -> u32 {
        self.step_attempts.get(step_id).copied().unwrap_or(0)
    }

    pub fn get_step_output(&self, step_id: &str, key: &str) -> Option<&serde_json::Value> {
        self.step_outputs
            .get(step_id)
            .and_then(|outputs| outputs.get(key))
    }

    pub fn set_variable(&mut self, key: &str, value: serde_json::Value) {
        self.variables.insert(key.to_string(), value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                initial_delay: "1s".to_string(),
                max_delay: "30s".to_string(),
            },
            retry_on: Some(vec![
                "timeout".to_string(),
                "connection_error".to_string(),
                "server_error".to_string(),
            ]),
        }
    }
}

/// Utility functions for workflow processing
pub mod utils {
    use super::*;
    use std::time::Duration;

    pub fn parse_duration(duration_str: &str) -> Result<Duration> {
        let duration_str = duration_str.trim();

        if duration_str.ends_with("ms") {
            let ms: u64 = duration_str[..duration_str.len() - 2].parse()?;
            Ok(Duration::from_millis(ms))
        } else if duration_str.ends_with('s') {
            let secs: u64 = duration_str[..duration_str.len() - 1].parse()?;
            Ok(Duration::from_secs(secs))
        } else if duration_str.ends_with('m') {
            let mins: u64 = duration_str[..duration_str.len() - 1].parse()?;
            Ok(Duration::from_secs(mins * 60))
        } else if duration_str.ends_with('h') {
            let hours: u64 = duration_str[..duration_str.len() - 1].parse()?;
            Ok(Duration::from_secs(hours * 3600))
        } else {
            // Default to seconds if no unit specified
            let secs: u64 = duration_str.parse()?;
            Ok(Duration::from_secs(secs))
        }
    }

    pub fn validate_workflow_definition(definition: &WorkflowDefinition) -> Result<()> {
        // Validate step dependencies
        let step_ids: std::collections::HashSet<_> =
            definition.steps.iter().map(|s| &s.id).collect();

        for step in &definition.steps {
            if let Some(ref deps) = step.depends_on {
                for dep in deps {
                    if !step_ids.contains(dep) {
                        return Err(anyhow::anyhow!(
                            "Step '{}' depends on non-existent step '{}'",
                            step.id,
                            dep
                        ));
                    }
                }
            }
        }

        // Check for circular dependencies (simplified check)
        // TODO: Implement proper topological sort validation

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_context_creation() {
        let mut inputs = HashMap::new();
        inputs.insert("test_input".to_string(), serde_json::json!("test_value"));

        let context = WorkflowContext::new(
            "workflow_123".to_string(),
            "execution_456".to_string(),
            inputs,
        );

        assert_eq!(context.workflow_id, "workflow_123");
        assert_eq!(context.execution_id, "execution_456");
        assert_eq!(
            context.inputs.get("test_input"),
            Some(&serde_json::json!("test_value"))
        );
    }

    #[test]
    fn test_duration_parsing() {
        assert_eq!(utils::parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(
            utils::parse_duration("100ms").unwrap(),
            Duration::from_millis(100)
        );
        assert_eq!(
            utils::parse_duration("2m").unwrap(),
            Duration::from_secs(120)
        );
        assert_eq!(
            utils::parse_duration("1h").unwrap(),
            Duration::from_secs(3600)
        );
    }

    #[test]
    fn test_workflow_validation() {
        let definition = WorkflowDefinition {
            name: "test_workflow".to_string(),
            version: "1.0".to_string(),
            description: None,
            inputs: vec![],
            outputs: vec![],
            steps: vec![
                WorkflowStep {
                    id: "step1".to_string(),
                    name: None,
                    tool: "test_tool".to_string(),
                    parameters: HashMap::new(),
                    depends_on: None,
                    outputs: None,
                    retry: None,
                    timeout: None,
                    parallel: None,
                    condition: None,
                    on_error: None,
                },
                WorkflowStep {
                    id: "step2".to_string(),
                    name: None,
                    tool: "test_tool2".to_string(),
                    parameters: HashMap::new(),
                    depends_on: Some(vec!["step1".to_string()]),
                    outputs: None,
                    retry: None,
                    timeout: None,
                    parallel: None,
                    condition: None,
                    on_error: None,
                },
            ],
            error_handling: None,
            metadata: None,
        };

        assert!(utils::validate_workflow_definition(&definition).is_ok());
    }
}
