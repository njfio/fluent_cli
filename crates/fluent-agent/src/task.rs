use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Task that represents a specific unit of work within a goal
///
/// Tasks are smaller, actionable units that contribute to achieving a goal.
/// They have clear inputs, outputs, and success criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: String,
    pub description: String,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub dependencies: Vec<String>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub expected_outputs: Vec<String>,
    pub success_criteria: Vec<String>,
    pub estimated_duration: Option<Duration>,
    pub max_attempts: u32,
    pub current_attempt: u32,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub success: Option<bool>,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of tasks that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    CodeGeneration,
    CodeAnalysis,
    FileOperation,
    ToolExecution,
    Testing,
    Validation,
    Communication,
    Planning,
    Research,
    Documentation,
}

/// Priority levels for tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub outputs: HashMap<String, serde_json::Value>,
    pub execution_time: Duration,
    pub attempt_number: u32,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Status of a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Created,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
    Blocked,
}

/// Builder for creating tasks with fluent API
pub struct TaskBuilder {
    task: Task,
}

impl Task {
    /// Create a new task
    pub fn new(description: String, task_type: TaskType) -> Self {
        Self {
            task_id: uuid::Uuid::new_v4().to_string(),
            description,
            task_type,
            priority: TaskPriority::Medium,
            dependencies: Vec::new(),
            inputs: HashMap::new(),
            expected_outputs: Vec::new(),
            success_criteria: Vec::new(),
            estimated_duration: None,
            max_attempts: 3,
            current_attempt: 0,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            success: None,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a task builder for fluent construction
    pub fn builder(description: String, task_type: TaskType) -> TaskBuilder {
        TaskBuilder {
            task: Self::new(description, task_type),
        }
    }

    /// Get current status of the task
    pub fn get_status(&self) -> TaskStatus {
        if self.completed_at.is_some() {
            if self.success == Some(true) {
                TaskStatus::Completed
            } else {
                TaskStatus::Failed
            }
        } else if self.started_at.is_some() {
            TaskStatus::Running
        } else if !self.dependencies.is_empty() {
            if self.dependencies_satisfied() {
                TaskStatus::Ready
            } else {
                TaskStatus::Blocked
            }
        } else {
            TaskStatus::Created
        }
    }

    /// Check if task dependencies are satisfied
    pub fn dependencies_satisfied(&self) -> bool {
        // In a real implementation, this would check against completed tasks
        // For now, assume dependencies are satisfied if empty
        self.dependencies.is_empty()
    }

    /// Start the task
    pub fn start(&mut self) {
        self.started_at = Some(SystemTime::now());
        self.current_attempt += 1;
    }

    /// Complete the task successfully
    pub fn complete_success(&mut self, outputs: HashMap<String, serde_json::Value>) {
        self.completed_at = Some(SystemTime::now());
        self.success = Some(true);
        self.metadata
            .insert("outputs".to_string(), serde_json::json!(outputs));
    }

    /// Complete the task with failure
    pub fn complete_failure(&mut self, error_message: String) {
        self.completed_at = Some(SystemTime::now());
        self.success = Some(false);
        self.error_message = Some(error_message);
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
        self.current_attempt < self.max_attempts && self.success != Some(true)
    }

    /// Reset task for retry
    pub fn reset_for_retry(&mut self) {
        self.started_at = None;
        self.completed_at = None;
        self.success = None;
        self.error_message = None;
    }

    /// Get execution duration
    pub fn get_execution_duration(&self) -> Option<Duration> {
        if let (Some(start), Some(end)) = (self.started_at, self.completed_at) {
            end.duration_since(start).ok()
        } else if let Some(start) = self.started_at {
            SystemTime::now().duration_since(start).ok()
        } else {
            None
        }
    }

    /// Check if task has timed out
    pub fn is_timed_out(&self) -> bool {
        if let (Some(duration), Some(start)) = (self.estimated_duration, self.started_at) {
            SystemTime::now().duration_since(start).unwrap_or_default() > duration * 2
        } else {
            false
        }
    }

    /// Get task complexity based on type and criteria
    pub fn get_complexity(&self) -> TaskComplexity {
        let criteria_count = self.success_criteria.len();
        let has_dependencies = !self.dependencies.is_empty();

        match self.task_type {
            TaskType::CodeGeneration => {
                if criteria_count > 3 || has_dependencies {
                    TaskComplexity::High
                } else if criteria_count > 1 {
                    TaskComplexity::Medium
                } else {
                    TaskComplexity::Low
                }
            }
            TaskType::CodeAnalysis | TaskType::Research => {
                if criteria_count > 2 {
                    TaskComplexity::Medium
                } else {
                    TaskComplexity::Low
                }
            }
            TaskType::FileOperation | TaskType::Communication => TaskComplexity::Low,
            _ => TaskComplexity::Medium,
        }
    }

    /// Validate task completeness
    pub fn validate(&self) -> Result<(), TaskValidationError> {
        if self.description.is_empty() {
            return Err(TaskValidationError::EmptyDescription);
        }

        if self.description.len() < 3 {
            return Err(TaskValidationError::DescriptionTooShort);
        }

        if self.max_attempts == 0 {
            return Err(TaskValidationError::InvalidMaxAttempts);
        }

        Ok(())
    }

    /// Get task summary
    pub fn get_summary(&self) -> String {
        format!(
            "Task: {} ({}), Status: {:?}, Priority: {:?}",
            self.description,
            format!("{:?}", self.task_type),
            self.get_status(),
            self.priority
        )
    }

    /// Add input to the task
    pub fn add_input(&mut self, key: String, value: serde_json::Value) {
        self.inputs.insert(key, value);
    }

    /// Get input value
    pub fn get_input(&self, key: &str) -> Option<&serde_json::Value> {
        self.inputs.get(key)
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }
}

/// Complexity levels for tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskComplexity {
    Low,
    Medium,
    High,
}

/// Errors that can occur during task validation
#[derive(Debug, Clone, PartialEq)]
pub enum TaskValidationError {
    EmptyDescription,
    DescriptionTooShort,
    InvalidMaxAttempts,
}

impl std::fmt::Display for TaskValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskValidationError::EmptyDescription => write!(f, "Task description cannot be empty"),
            TaskValidationError::DescriptionTooShort => write!(f, "Task description is too short"),
            TaskValidationError::InvalidMaxAttempts => {
                write!(f, "Max attempts must be greater than 0")
            }
        }
    }
}

impl std::error::Error for TaskValidationError {}

impl TaskBuilder {
    /// Set task priority
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.task.priority = priority;
        self
    }

    /// Add dependencies
    pub fn dependencies(mut self, dependencies: Vec<String>) -> Self {
        self.task.dependencies = dependencies;
        self
    }

    /// Add a single dependency
    pub fn dependency(mut self, dependency: String) -> Self {
        self.task.dependencies.push(dependency);
        self
    }

    /// Add inputs
    pub fn inputs(mut self, inputs: HashMap<String, serde_json::Value>) -> Self {
        self.task.inputs = inputs;
        self
    }

    /// Add a single input
    pub fn input(mut self, key: String, value: serde_json::Value) -> Self {
        self.task.inputs.insert(key, value);
        self
    }

    /// Set expected outputs
    pub fn expected_outputs(mut self, outputs: Vec<String>) -> Self {
        self.task.expected_outputs = outputs;
        self
    }

    /// Add expected output
    pub fn expected_output(mut self, output: String) -> Self {
        self.task.expected_outputs.push(output);
        self
    }

    /// Set success criteria
    pub fn success_criteria(mut self, criteria: Vec<String>) -> Self {
        self.task.success_criteria = criteria;
        self
    }

    /// Add success criterion
    pub fn success_criterion(mut self, criterion: String) -> Self {
        self.task.success_criteria.push(criterion);
        self
    }

    /// Set estimated duration
    pub fn estimated_duration(mut self, duration: Duration) -> Self {
        self.task.estimated_duration = Some(duration);
        self
    }

    /// Set max attempts
    pub fn max_attempts(mut self, max_attempts: u32) -> Self {
        self.task.max_attempts = max_attempts;
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.task.metadata.insert(key, value);
        self
    }

    /// Build the task
    pub fn build(self) -> Result<Task, TaskValidationError> {
        self.task.validate()?;
        Ok(self.task)
    }

    /// Build the task without validation (for testing)
    pub fn build_unchecked(self) -> Task {
        self.task
    }
}

/// Common task templates for quick creation
pub struct TaskTemplates;

impl TaskTemplates {
    /// Create a code generation task
    pub fn code_generation(
        description: String,
        language: String,
        requirements: Vec<String>,
    ) -> Task {
        let mut criteria = vec![
            format!("Generate valid {} code", language),
            "Code meets requirements".to_string(),
        ];
        criteria.extend(requirements);

        Task::builder(description, TaskType::CodeGeneration)
            .priority(TaskPriority::High)
            .success_criteria(criteria)
            .expected_output("Generated code".to_string())
            .estimated_duration(Duration::from_secs(180))
            .metadata("language".to_string(), serde_json::json!(language))
            .build_unchecked()
    }

    /// Create a file operation task
    pub fn file_operation(operation: String, file_path: String) -> Task {
        Task::builder(
            format!("{} file: {}", operation, file_path),
            TaskType::FileOperation,
        )
        .priority(TaskPriority::Medium)
        .success_criterion(format!("Successfully {} file", operation))
        .input("operation".to_string(), serde_json::json!(operation))
        .input("file_path".to_string(), serde_json::json!(file_path))
        .expected_output("Operation result".to_string())
        .estimated_duration(Duration::from_secs(30))
        .build_unchecked()
    }

    /// Create a tool execution task
    pub fn tool_execution(
        tool_name: String,
        parameters: HashMap<String, serde_json::Value>,
    ) -> Task {
        Task::builder(
            format!("Execute tool: {}", tool_name),
            TaskType::ToolExecution,
        )
        .priority(TaskPriority::Medium)
        .success_criterion("Tool executes successfully".to_string())
        .input("tool_name".to_string(), serde_json::json!(tool_name))
        .inputs(parameters)
        .expected_output("Tool output".to_string())
        .estimated_duration(Duration::from_secs(60))
        .build_unchecked()
    }

    /// Create a testing task
    pub fn testing(component: String, test_type: String) -> Task {
        Task::builder(
            format!("Test {} with {}", component, test_type),
            TaskType::Testing,
        )
        .priority(TaskPriority::High)
        .success_criteria(vec![
            "Tests execute successfully".to_string(),
            "All tests pass".to_string(),
        ])
        .input("component".to_string(), serde_json::json!(component))
        .input("test_type".to_string(), serde_json::json!(test_type))
        .expected_output("Test results".to_string())
        .estimated_duration(Duration::from_secs(120))
        .build_unchecked()
    }

    /// Create a validation task
    pub fn validation(subject: String, validation_rules: Vec<String>) -> Task {
        Task::builder(format!("Validate {}", subject), TaskType::Validation)
            .priority(TaskPriority::Medium)
            .success_criteria(validation_rules.clone())
            .input("subject".to_string(), serde_json::json!(subject))
            .input("rules".to_string(), serde_json::json!(validation_rules))
            .expected_output("Validation results".to_string())
            .estimated_duration(Duration::from_secs(90))
            .build_unchecked()
    }

    /// Create an analysis task
    pub fn analysis(target: String, analysis_type: String) -> Task {
        Task::builder(
            format!("Analyze {} for {}", target, analysis_type),
            TaskType::CodeAnalysis,
        )
        .priority(TaskPriority::Medium)
        .success_criteria(vec![
            "Complete analysis".to_string(),
            "Generate insights".to_string(),
        ])
        .input("target".to_string(), serde_json::json!(target))
        .input(
            "analysis_type".to_string(),
            serde_json::json!(analysis_type),
        )
        .expected_output("Analysis report".to_string())
        .estimated_duration(Duration::from_secs(150))
        .build_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task".to_string(), TaskType::CodeGeneration);

        assert!(!task.task_id.is_empty());
        assert_eq!(task.description, "Test task");
        assert!(matches!(task.task_type, TaskType::CodeGeneration));
        assert_eq!(task.priority, TaskPriority::Medium);
        assert_eq!(task.get_status(), TaskStatus::Created);
    }

    #[test]
    fn test_task_builder() {
        let task = Task::builder("Test task".to_string(), TaskType::Testing)
            .priority(TaskPriority::High)
            .success_criterion("Test passes".to_string())
            .dependency("previous_task".to_string())
            .estimated_duration(Duration::from_secs(120))
            .build()
            .unwrap();

        assert_eq!(task.description, "Test task");
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.success_criteria.len(), 1);
        assert_eq!(task.dependencies.len(), 1);
        assert_eq!(task.estimated_duration, Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_task_lifecycle() {
        let mut task = Task::new("Test task".to_string(), TaskType::FileOperation);

        assert_eq!(task.get_status(), TaskStatus::Created);

        task.start();
        assert_eq!(task.get_status(), TaskStatus::Running);
        assert_eq!(task.current_attempt, 1);

        task.complete_success(HashMap::new());
        assert_eq!(task.get_status(), TaskStatus::Completed);
        assert_eq!(task.success, Some(true));
    }

    #[test]
    fn test_task_retry() {
        let mut task = Task::new("Test task".to_string(), TaskType::ToolExecution);
        task.max_attempts = 3;

        task.start();
        task.complete_failure("Test error".to_string());

        assert!(task.can_retry());
        assert_eq!(task.current_attempt, 1);

        task.reset_for_retry();
        assert_eq!(task.get_status(), TaskStatus::Created);
        assert!(task.success.is_none());
    }

    #[test]
    fn test_task_templates() {
        let code_task = TaskTemplates::code_generation(
            "Generate function".to_string(),
            "Rust".to_string(),
            vec!["Must be async".to_string()],
        );

        assert!(matches!(code_task.task_type, TaskType::CodeGeneration));
        assert_eq!(code_task.priority, TaskPriority::High);
        assert!(code_task.success_criteria.len() >= 2);
        assert_eq!(
            code_task.get_metadata("language"),
            Some(&serde_json::json!("Rust"))
        );
    }

    #[test]
    fn test_task_complexity() {
        let simple_task = Task::builder("Simple task".to_string(), TaskType::FileOperation)
            .success_criterion("Complete".to_string())
            .build()
            .unwrap();

        assert_eq!(simple_task.get_complexity(), TaskComplexity::Low);

        let complex_task = Task::builder("Complex task".to_string(), TaskType::CodeGeneration)
            .success_criteria(vec![
                "Criterion 1".to_string(),
                "Criterion 2".to_string(),
                "Criterion 3".to_string(),
                "Criterion 4".to_string(),
            ])
            .dependency("dep1".to_string())
            .build()
            .unwrap();

        assert_eq!(complex_task.get_complexity(), TaskComplexity::High);
    }
}
