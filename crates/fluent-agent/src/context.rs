use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::fs;

use crate::goal::Goal;
use crate::orchestrator::Observation;
use crate::task::Task;

/// Checkpoint of execution context at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCheckpoint {
    pub checkpoint_id: String,
    pub timestamp: SystemTime,
    pub iteration_count: u32,
    pub checkpoint_type: CheckpointType,
    pub context_snapshot: ExecutionContextSnapshot,
    pub description: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type of checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointType {
    Manual,
    Automatic,
    BeforeAction,
    AfterAction,
    BeforeReflection,
    OnError,
    OnSuccess,
}

/// Lightweight snapshot of execution context for checkpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContextSnapshot {
    pub context_id: String,
    pub goal_description: Option<String>,
    pub active_task_count: usize,
    pub completed_task_count: usize,
    pub observation_count: usize,
    pub variable_count: usize,
    pub iteration_count: u32,
    pub key_variables: HashMap<String, String>, // Only important variables
    pub last_action_summary: Option<String>,
    pub progress_summary: String,
}

/// Execution context that maintains state throughout agent execution
///
/// The execution context serves as the central state container for agent operations,
/// tracking the current goal, active tasks, observations, variables, and execution history.
/// It provides a comprehensive view of the agent's current situation and progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub context_id: String,
    pub current_goal: Option<Goal>,
    pub active_tasks: Vec<Task>,
    pub completed_tasks: Vec<Task>,
    pub observations: Vec<Observation>,
    pub variables: HashMap<String, String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub execution_history: Vec<ExecutionEvent>,
    pub start_time: SystemTime,
    pub last_update: SystemTime,
    pub iteration_count: u32,
    pub available_tools: Vec<String>,
    pub strategy_adjustments: Vec<StrategyAdjustment>,
    pub checkpoints: Vec<ContextCheckpoint>,
    pub state_version: u32,
    pub persistence_enabled: bool,
    pub auto_checkpoint_interval: Option<u32>, // Checkpoint every N iterations
}

/// Event in the execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub event_id: String,
    pub timestamp: SystemTime,
    pub event_type: ExecutionEventType,
    pub description: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEventType {
    GoalSet,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    ObservationMade,
    VariableSet,
    StrategyAdjusted,
    ToolExecuted,
    ErrorOccurred,
    CheckpointCreated,
    ContextRestored,
    StateValidated,
}

/// Strategy adjustment made during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAdjustment {
    pub adjustment_id: String,
    pub timestamp: SystemTime,
    pub reason: String,
    pub adjustments: Vec<String>,
    pub expected_impact: String,
}

impl ExecutionContext {
    /// Create a new execution context for a goal
    pub fn new(goal: Goal) -> Self {
        let context_id = uuid::Uuid::new_v4().to_string();
        let now = SystemTime::now();

        Self {
            context_id: context_id.clone(),
            current_goal: Some(goal.clone()),
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            observations: Vec::new(),
            variables: HashMap::new(),
            metadata: HashMap::new(),
            execution_history: vec![ExecutionEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                timestamp: now,
                event_type: ExecutionEventType::GoalSet,
                description: format!("Goal set: {}", goal.description),
                metadata: HashMap::new(),
            }],
            start_time: now,
            last_update: now,
            iteration_count: 0,
            available_tools: Vec::new(),
            strategy_adjustments: Vec::new(),
            checkpoints: Vec::new(),
            state_version: 1,
            persistence_enabled: true,
            auto_checkpoint_interval: Some(5), // Checkpoint every 5 iterations by default
        }
    }

    /// Create a new context for reflection
    pub fn new_for_reflection(base_context: &ExecutionContext) -> Self {
        let mut reflection_context = base_context.clone();
        reflection_context.context_id = uuid::Uuid::new_v4().to_string();
        reflection_context.metadata.insert(
            "reflection_base".to_string(),
            serde_json::json!(base_context.context_id),
        );
        reflection_context
    }

    /// Add an observation to the context
    pub fn add_observation(&mut self, observation: Observation) {
        self.observations.push(observation.clone());
        self.last_update = SystemTime::now();

        // Record event
        self.execution_history.push(ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::ObservationMade,
            description: format!("Observation: {}", observation.content),
            metadata: HashMap::new(),
        });
    }

    /// Set a variable in the context
    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key.clone(), value.clone());
        self.last_update = SystemTime::now();

        // Record event
        self.execution_history.push(ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::VariableSet,
            description: format!("Variable set: {} = {}", key, value),
            metadata: HashMap::new(),
        });
    }

    /// Add a strategy adjustment
    pub fn add_strategy_adjustment(&mut self, adjustments: Vec<String>) {
        let adjustment = StrategyAdjustment {
            adjustment_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            reason: "Reflection-based adjustment".to_string(),
            adjustments: adjustments.clone(),
            expected_impact: "Improved goal achievement".to_string(),
        };

        self.strategy_adjustments.push(adjustment);
        self.last_update = SystemTime::now();

        // Record event
        self.execution_history.push(ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::StrategyAdjusted,
            description: format!("Strategy adjusted: {:?}", adjustments),
            metadata: HashMap::new(),
        });
    }

    /// Start a new task
    pub fn start_task(&mut self, task: Task) {
        self.active_tasks.push(task.clone());
        self.last_update = SystemTime::now();

        // Record event
        self.execution_history.push(ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::TaskStarted,
            description: format!("Task started: {}", task.description),
            metadata: HashMap::new(),
        });
    }

    /// Complete a task
    pub fn complete_task(&mut self, task_id: &str, success: bool) {
        if let Some(index) = self.active_tasks.iter().position(|t| t.task_id == task_id) {
            let mut task = self.active_tasks.remove(index);
            task.completed_at = Some(SystemTime::now());
            task.success = Some(success);
            self.completed_tasks.push(task.clone());
            self.last_update = SystemTime::now();

            // Record event
            let event_type = if success {
                ExecutionEventType::TaskCompleted
            } else {
                ExecutionEventType::TaskFailed
            };
            self.execution_history.push(ExecutionEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now(),
                event_type,
                description: format!(
                    "Task {}: {}",
                    if success { "completed" } else { "failed" },
                    task.description
                ),
                metadata: HashMap::new(),
            });
        }
    }

    /// Get the current goal
    pub fn get_current_goal(&self) -> Option<&Goal> {
        self.current_goal.as_ref()
    }

    /// Get the current task
    pub fn get_current_task(&self) -> Option<&Task> {
        self.active_tasks.first()
    }

    /// Get a summary of the current context
    pub fn get_summary(&self) -> String {
        format!(
            "Context: Goal: {:?}, Active tasks: {}, Completed tasks: {}, Observations: {}, Variables: {}",
            self.current_goal.as_ref().map(|g| &g.description),
            self.active_tasks.len(),
            self.completed_tasks.len(),
            self.observations.len(),
            self.variables.len()
        )
    }

    /// Get available tools
    pub fn get_available_tools(&self) -> &[String] {
        &self.available_tools
    }

    /// Get recent actions from execution history
    pub fn get_recent_actions(&self) -> Vec<&ExecutionEvent> {
        self.execution_history.iter().rev().take(10).collect()
    }

    /// Get the latest observation
    pub fn get_latest_observation(&self) -> Option<Observation> {
        self.observations.last().cloned()
    }

    /// Get progress summary
    pub fn get_progress_summary(&self) -> String {
        let total_tasks = self.active_tasks.len() + self.completed_tasks.len();
        let completed_count = self.completed_tasks.len();
        let success_count = self
            .completed_tasks
            .iter()
            .filter(|t| t.success == Some(true))
            .count();

        format!(
            "Progress: {}/{} tasks completed, {} successful, {} iterations, {} observations",
            completed_count,
            total_tasks,
            success_count,
            self.iteration_count,
            self.observations.len()
        )
    }

    /// Get action history
    pub fn get_action_history(&self) -> Vec<String> {
        self.execution_history
            .iter()
            .map(|event| format!("{:?}: {}", event.event_type, event.description))
            .collect()
    }

    /// Get results summary
    pub fn get_results_summary(&self) -> String {
        let successful_tasks = self
            .completed_tasks
            .iter()
            .filter(|t| t.success == Some(true))
            .count();
        let failed_tasks = self
            .completed_tasks
            .iter()
            .filter(|t| t.success == Some(false))
            .count();
        let positive_observations = self
            .observations
            .iter()
            .filter(|o| o.content.contains("SUCCESS"))
            .count();

        format!(
            "Results: {} successful tasks, {} failed tasks, {} positive observations",
            successful_tasks, failed_tasks, positive_observations
        )
    }

    /// Get final output
    pub fn get_final_output(&self) -> Option<String> {
        // Look for output in recent observations or completed tasks
        self.observations
            .iter()
            .rev()
            .find(|obs| obs.content.contains("output") || obs.content.contains("result"))
            .map(|obs| obs.content.clone())
            .or_else(|| {
                self.completed_tasks
                    .iter()
                    .rev()
                    .find(|task| task.success == Some(true))
                    .map(|task| format!("Completed task: {}", task.description))
            })
    }

    /// Get tags for memory storage
    pub fn get_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();

        if let Some(goal) = &self.current_goal {
            tags.push(format!("goal_{:?}", goal.goal_type));
        }

        tags.push(format!("tasks_{}", self.completed_tasks.len()));
        tags.push(format!("observations_{}", self.observations.len()));

        if self.completed_tasks.iter().any(|t| t.success == Some(true)) {
            tags.push("has_success".to_string());
        }

        if self
            .completed_tasks
            .iter()
            .any(|t| t.success == Some(false))
        {
            tags.push("has_failure".to_string());
        }

        tags
    }

    /// Check if goal is unclear
    pub fn is_goal_unclear(&self) -> bool {
        self.current_goal.as_ref().map_or(true, |goal| {
            goal.description.len() < 10 || goal.success_criteria.is_empty()
        })
    }

    /// Check if task decomposition is needed
    pub fn needs_task_decomposition(&self) -> bool {
        self.active_tasks.is_empty() && self.current_goal.is_some()
    }

    /// Check if action planning is needed
    pub fn needs_action_planning(&self) -> bool {
        !self.active_tasks.is_empty() && self.execution_history.len() < 3
    }

    /// Get iteration count
    pub fn iteration_count(&self) -> u32 {
        self.iteration_count
    }

    /// Increment iteration count
    pub fn increment_iteration(&mut self) {
        self.iteration_count += 1;
        self.last_update = SystemTime::now();

        // Create automatic checkpoint if needed
        if self.should_create_auto_checkpoint() {
            let description = format!("Automatic checkpoint at iteration {}", self.iteration_count);
            self.create_checkpoint(CheckpointType::Automatic, description);
        }
    }

    /// Set available tools
    pub fn set_available_tools(&mut self, tools: Vec<String>) {
        self.available_tools = tools;
        self.last_update = SystemTime::now();
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: serde_json::Value) {
        self.metadata.insert(key, value);
        self.last_update = SystemTime::now();
    }

    /// Get execution duration
    pub fn get_execution_duration(&self) -> Duration {
        self.last_update
            .duration_since(self.start_time)
            .unwrap_or_default()
    }

    /// Check if context is stale (hasn't been updated recently)
    pub fn is_stale(&self, threshold: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.last_update)
            .unwrap_or_default()
            > threshold
    }

    /// Create a checkpoint of the current context state
    pub fn create_checkpoint(&mut self, checkpoint_type: CheckpointType, description: String) -> String {
        let checkpoint_id = uuid::Uuid::new_v4().to_string();

        let snapshot = ExecutionContextSnapshot {
            context_id: self.context_id.clone(),
            goal_description: self.current_goal.as_ref().map(|g| g.description.clone()),
            active_task_count: self.active_tasks.len(),
            completed_task_count: self.completed_tasks.len(),
            observation_count: self.observations.len(),
            variable_count: self.variables.len(),
            iteration_count: self.iteration_count,
            key_variables: self.get_key_variables(),
            last_action_summary: self.get_last_action_summary(),
            progress_summary: self.get_progress_summary(),
        };

        let checkpoint = ContextCheckpoint {
            checkpoint_id: checkpoint_id.clone(),
            timestamp: SystemTime::now(),
            iteration_count: self.iteration_count,
            checkpoint_type,
            context_snapshot: snapshot,
            description,
            metadata: HashMap::new(),
        };

        self.checkpoints.push(checkpoint);
        self.state_version += 1;
        self.last_update = SystemTime::now();

        // Keep only the last 10 checkpoints to prevent memory bloat
        if self.checkpoints.len() > 10 {
            self.checkpoints.remove(0);
        }

        checkpoint_id
    }

    /// Get key variables (those that have been accessed recently or are important)
    fn get_key_variables(&self) -> HashMap<String, String> {
        // For now, return all variables. In the future, we could implement
        // importance scoring based on usage frequency, recency, etc.
        self.variables.clone()
    }

    /// Get a summary of the last action taken
    fn get_last_action_summary(&self) -> Option<String> {
        self.execution_history
            .iter()
            .rev()
            .find(|event| matches!(event.event_type, ExecutionEventType::ToolExecuted))
            .map(|event| event.description.clone())
    }

    /// Check if an automatic checkpoint should be created
    pub fn should_create_auto_checkpoint(&self) -> bool {
        if let Some(interval) = self.auto_checkpoint_interval {
            self.iteration_count > 0 && self.iteration_count % interval == 0
        } else {
            false
        }
    }

    /// Get the most recent checkpoint
    pub fn get_latest_checkpoint(&self) -> Option<&ContextCheckpoint> {
        self.checkpoints.last()
    }

    /// Get checkpoint by ID
    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Option<&ContextCheckpoint> {
        self.checkpoints.iter().find(|cp| cp.checkpoint_id == checkpoint_id)
    }

    /// Get all checkpoints of a specific type
    pub fn get_checkpoints_by_type(&self, checkpoint_type: &CheckpointType) -> Vec<&ContextCheckpoint> {
        self.checkpoints
            .iter()
            .filter(|cp| std::mem::discriminant(&cp.checkpoint_type) == std::mem::discriminant(checkpoint_type))
            .collect()
    }

    /// Save the execution context to disk
    pub async fn save_to_disk<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if !self.persistence_enabled {
            return Ok(()); // Skip saving if persistence is disabled
        }

        let json_data = serde_json::to_string_pretty(self)?;
        fs::write(path, json_data).await?;
        Ok(())
    }

    /// Load execution context from disk
    pub async fn load_from_disk<P: AsRef<Path>>(path: P) -> Result<Self> {
        let json_data = fs::read_to_string(path).await?;
        let context: ExecutionContext = serde_json::from_str(&json_data)?;
        Ok(context)
    }

    /// Save a checkpoint to disk
    pub async fn save_checkpoint_to_disk<P: AsRef<Path>>(&self, checkpoint_id: &str, path: P) -> Result<()> {
        if let Some(checkpoint) = self.get_checkpoint(checkpoint_id) {
            let json_data = serde_json::to_string_pretty(checkpoint)?;
            fs::write(path, json_data).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Checkpoint not found: {}", checkpoint_id))
        }
    }

    /// Load a checkpoint from disk
    pub async fn load_checkpoint_from_disk<P: AsRef<Path>>(path: P) -> Result<ContextCheckpoint> {
        let json_data = fs::read_to_string(path).await?;
        let checkpoint: ContextCheckpoint = serde_json::from_str(&json_data)?;
        Ok(checkpoint)
    }

    /// Restore context from a checkpoint (partial restoration)
    pub fn restore_from_checkpoint(&mut self, checkpoint: &ContextCheckpoint) {
        // Restore key state from checkpoint snapshot
        let snapshot = &checkpoint.context_snapshot;

        // Update variables with key variables from checkpoint
        for (key, value) in &snapshot.key_variables {
            self.variables.insert(key.clone(), value.clone());
        }

        // Add a restoration event to history
        self.execution_history.push(ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::ContextRestored,
            description: format!("Context restored from checkpoint: {}", checkpoint.checkpoint_id),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("checkpoint_id".to_string(), serde_json::json!(checkpoint.checkpoint_id));
                meta.insert("checkpoint_type".to_string(), serde_json::json!(checkpoint.checkpoint_type));
                meta
            },
        });

        self.state_version += 1;
        self.last_update = SystemTime::now();
    }

    /// Enable or disable persistence
    pub fn set_persistence_enabled(&mut self, enabled: bool) {
        self.persistence_enabled = enabled;
        self.last_update = SystemTime::now();
    }

    /// Set auto-checkpoint interval
    pub fn set_auto_checkpoint_interval(&mut self, interval: Option<u32>) {
        self.auto_checkpoint_interval = interval;
        self.last_update = SystemTime::now();
    }

    /// Get state version (increments on each significant change)
    pub fn get_state_version(&self) -> u32 {
        self.state_version
    }

    /// Validate context state consistency
    pub fn validate_state(&self) -> Result<()> {
        // Check basic consistency
        if self.context_id.is_empty() {
            return Err(anyhow::anyhow!("Context ID cannot be empty"));
        }

        if self.iteration_count > 0 && self.execution_history.is_empty() {
            return Err(anyhow::anyhow!("Execution history should not be empty with non-zero iterations"));
        }

        // Check that completed tasks have completion timestamps
        for task in &self.completed_tasks {
            if task.completed_at.is_none() {
                return Err(anyhow::anyhow!("Completed task missing completion timestamp: {}", task.task_id));
            }
        }

        // Check checkpoint consistency
        for checkpoint in &self.checkpoints {
            if checkpoint.checkpoint_id.is_empty() {
                return Err(anyhow::anyhow!("Checkpoint ID cannot be empty"));
            }
            if checkpoint.iteration_count > self.iteration_count {
                return Err(anyhow::anyhow!("Checkpoint iteration count cannot exceed current iteration count"));
            }
        }

        Ok(())
    }

    /// Get context statistics
    pub fn get_stats(&self) -> ContextStats {
        ContextStats {
            total_observations: self.observations.len(),
            active_tasks: self.active_tasks.len(),
            completed_tasks: self.completed_tasks.len(),
            variables_count: self.variables.len(),
            execution_events: self.execution_history.len(),
            strategy_adjustments: self.strategy_adjustments.len(),
            execution_duration: self.get_execution_duration(),
            iteration_count: self.iteration_count,
        }
    }
}

/// Statistics about the execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    pub total_observations: usize,
    pub active_tasks: usize,
    pub completed_tasks: usize,
    pub variables_count: usize,
    pub execution_events: usize,
    pub strategy_adjustments: usize,
    pub execution_duration: Duration,
    pub iteration_count: u32,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        let now = SystemTime::now();
        Self {
            context_id: uuid::Uuid::new_v4().to_string(),
            current_goal: None,
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            observations: Vec::new(),
            variables: HashMap::new(),
            metadata: HashMap::new(),
            execution_history: Vec::new(),
            start_time: now,
            last_update: now,
            iteration_count: 0,
            available_tools: Vec::new(),
            strategy_adjustments: Vec::new(),
            checkpoints: Vec::new(),
            state_version: 1,
            persistence_enabled: false, // Disabled by default in Default impl
            auto_checkpoint_interval: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{Goal, GoalPriority, GoalType};

    #[test]
    fn test_execution_context_creation() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::CodeGeneration,
            priority: GoalPriority::High,
            success_criteria: vec!["Complete successfully".to_string()],
            max_iterations: Some(10),
            timeout: None,
            metadata: HashMap::new(),
        };

        let context = ExecutionContext::new(goal.clone());

        assert!(!context.context_id.is_empty());
        assert_eq!(context.current_goal.as_ref().unwrap().goal_id, "test-goal");
        assert_eq!(context.iteration_count, 0);
        assert!(context.active_tasks.is_empty());
        assert_eq!(context.execution_history.len(), 1); // Goal set event
    }

    #[test]
    fn test_context_variable_management() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        context.set_variable("test_key".to_string(), "test_value".to_string());

        assert_eq!(
            context.variables.get("test_key"),
            Some(&"test_value".to_string())
        );
        assert!(context
            .execution_history
            .iter()
            .any(|e| matches!(e.event_type, ExecutionEventType::VariableSet)));
    }

    #[test]
    fn test_context_summary() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::FileOperation,
            priority: GoalPriority::Low,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let context = ExecutionContext::new(goal);
        let summary = context.get_summary();

        assert!(summary.contains("Goal: Some(\"Test goal\")"));
        assert!(summary.contains("Active tasks: 0"));
        assert!(summary.contains("Completed tasks: 0"));
    }

    #[test]
    fn test_context_stats() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Communication,
            priority: GoalPriority::High,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let context = ExecutionContext::new(goal);
        let stats = context.get_stats();

        assert_eq!(stats.total_observations, 0);
        assert_eq!(stats.active_tasks, 0);
        assert_eq!(stats.completed_tasks, 0);
        assert_eq!(stats.iteration_count, 0);
        assert_eq!(stats.execution_events, 1); // Goal set event
    }

    #[test]
    fn test_checkpoint_creation() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        // Create a manual checkpoint
        let checkpoint_id = context.create_checkpoint(
            CheckpointType::Manual,
            "Test checkpoint".to_string()
        );

        assert!(!checkpoint_id.is_empty());
        assert_eq!(context.checkpoints.len(), 1);
        assert_eq!(context.state_version, 2); // Incremented from initial 1

        let checkpoint = context.get_checkpoint(&checkpoint_id).unwrap();
        assert_eq!(checkpoint.description, "Test checkpoint");
        assert!(matches!(checkpoint.checkpoint_type, CheckpointType::Manual));
    }

    #[test]
    fn test_auto_checkpoint() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::FileOperation,
            priority: GoalPriority::High,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);
        context.set_auto_checkpoint_interval(Some(3)); // Checkpoint every 3 iterations

        // Increment iterations
        for i in 1..=6 {
            context.increment_iteration();

            if i % 3 == 0 {
                // Should have created automatic checkpoints at iterations 3 and 6
                let auto_checkpoints = context.get_checkpoints_by_type(&CheckpointType::Automatic);
                assert_eq!(auto_checkpoints.len(), i / 3);
            }
        }

        assert_eq!(context.iteration_count, 6);
        assert_eq!(context.checkpoints.len(), 2); // Two automatic checkpoints
    }

    #[test]
    fn test_state_validation() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::CodeGeneration,
            priority: GoalPriority::Low,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let context = ExecutionContext::new(goal);

        // Valid context should pass validation
        assert!(context.validate_state().is_ok());

        // Test with invalid context
        let mut invalid_context = context.clone();
        invalid_context.context_id = String::new(); // Empty context ID should fail
        assert!(invalid_context.validate_state().is_err());
    }

    #[tokio::test]
    async fn test_context_persistence() {
        use tempfile::tempdir;

        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);
        context.set_variable("test_key".to_string(), "test_value".to_string());
        context.increment_iteration();

        // Save to temporary file
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("context.json");

        context.save_to_disk(&file_path).await.unwrap();

        // Load from file
        let loaded_context = ExecutionContext::load_from_disk(&file_path).await.unwrap();

        assert_eq!(context.context_id, loaded_context.context_id);
        assert_eq!(context.iteration_count, loaded_context.iteration_count);
        assert_eq!(context.variables, loaded_context.variables);
        assert_eq!(context.state_version, loaded_context.state_version);
    }
}
