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
    /// Progress-specific data for long-running task checkpoints
    #[serde(default)]
    pub progress_data: Option<ProgressData>,
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
    /// Progress checkpoint for long-running tasks - saved periodically and to disk
    Progress,
    /// Milestone checkpoint at significant progress points (25%, 50%, 75%)
    Milestone,
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

/// Progress-specific data for tracking long-running task advancement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    /// Current iteration number
    pub current_iteration: u32,
    /// Maximum iterations allowed (if known)
    pub max_iterations: Option<u32>,
    /// Estimated completion percentage (0.0 - 100.0)
    pub estimated_completion_percentage: f64,
    /// Number of successful actions
    pub successful_actions: u32,
    /// Number of failed actions
    pub failed_actions: u32,
    /// Success rate (successful / total)
    pub success_rate: f64,
    /// Total tokens consumed (for cost tracking)
    pub tokens_used: u64,
    /// Total API calls made
    pub api_calls_made: u32,
    /// Elapsed time in seconds
    pub elapsed_seconds: u64,
    /// Last successful action description
    pub last_successful_action: Option<String>,
    /// Milestone reached (if any)
    pub milestone: Option<ProgressMilestone>,
    /// Recovery hint for resumption
    pub recovery_hint: Option<String>,
}

/// Milestone markers for progress tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProgressMilestone {
    Started,
    Quarter,       // 25%
    Half,          // 50%
    ThreeQuarters, // 75%
    NearComplete,  // 90%+
    Completed,
}

/// Recovery information for resuming from a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressRecoveryInfo {
    /// Checkpoint ID to resume from
    pub checkpoint_id: String,
    /// Iteration at checkpoint
    pub iteration_at_checkpoint: u32,
    /// Completion percentage at checkpoint
    pub completion_percentage: f64,
    /// Variables to restore
    pub recovered_variables: HashMap<String, String>,
    /// Last successful action
    pub last_successful_action: Option<String>,
    /// Recommended resumption strategy
    pub resumption_strategy: ResumptionStrategy,
}

/// Strategy for resuming from a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResumptionStrategy {
    /// Resume exactly where execution left off
    ContinueFromCheckpoint,
    /// Last action may have failed, retry it
    RetryLastAction,
    /// Last phase complete, move to next
    SkipToNextPhase,
    /// Reconstruct context from observations
    RebuildContext,
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
    pub context_data: HashMap<String, String>,
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
    // Progress tracking fields for long-running tasks
    /// Maximum iterations allowed (for calculating completion %)
    #[serde(default)]
    pub max_iterations: Option<u32>,
    /// Number of successful actions completed
    #[serde(default)]
    pub successful_actions: u32,
    /// Number of failed actions
    #[serde(default)]
    pub failed_actions: u32,
    /// Total tokens used (for cost tracking)
    #[serde(default)]
    pub tokens_used: u64,
    /// Total API calls made
    #[serde(default)]
    pub api_calls_made: u32,
    /// Interval for progress checkpoint persistence (in iterations)
    #[serde(default)]
    pub progress_checkpoint_interval: Option<u32>,
    /// Last milestone reached
    #[serde(default)]
    pub last_milestone: Option<ProgressMilestone>,
    /// Description of last successful action
    #[serde(default)]
    pub last_successful_action: Option<String>,
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
            context_data: HashMap::new(),
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
            // Progress tracking fields
            max_iterations: goal.max_iterations,
            successful_actions: 0,
            failed_actions: 0,
            tokens_used: 0,
            api_calls_made: 0,
            progress_checkpoint_interval: Some(10), // Progress checkpoint every 10 iterations
            last_milestone: None,
            last_successful_action: None,
        }
    }

    /// Create a new context without a goal (for backward compatibility)
    pub fn new_default() -> Self {
        let context_id = uuid::Uuid::new_v4().to_string();
        let now = SystemTime::now();

        Self {
            context_id: context_id.clone(),
            current_goal: None,
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            observations: Vec::new(),
            variables: HashMap::new(),
            metadata: HashMap::new(),
            context_data: HashMap::new(),
            execution_history: Vec::new(),
            start_time: now,
            last_update: now,
            iteration_count: 0,
            available_tools: Vec::new(),
            strategy_adjustments: Vec::new(),
            checkpoints: Vec::new(),
            state_version: 1,
            persistence_enabled: true,
            auto_checkpoint_interval: Some(5),
            // Progress tracking fields
            max_iterations: None,
            successful_actions: 0,
            failed_actions: 0,
            tokens_used: 0,
            api_calls_made: 0,
            progress_checkpoint_interval: Some(10),
            last_milestone: None,
            last_successful_action: None,
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

    /// Add a context item
    pub fn add_context_item(&mut self, key: String, value: String) {
        self.context_data.insert(key, value);
        self.last_update = SystemTime::now();
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
        self.current_goal
            .as_ref()
            .is_none_or(|goal| goal.description.len() < 10 || goal.success_criteria.is_empty())
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
    pub fn create_checkpoint(
        &mut self,
        checkpoint_type: CheckpointType,
        description: String,
    ) -> String {
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
            progress_data: None,
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
            self.iteration_count > 0 && self.iteration_count.is_multiple_of(interval)
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
        self.checkpoints
            .iter()
            .find(|cp| cp.checkpoint_id == checkpoint_id)
    }

    /// Get all checkpoints of a specific type
    pub fn get_checkpoints_by_type(
        &self,
        checkpoint_type: &CheckpointType,
    ) -> Vec<&ContextCheckpoint> {
        self.checkpoints
            .iter()
            .filter(|cp| {
                std::mem::discriminant(&cp.checkpoint_type)
                    == std::mem::discriminant(checkpoint_type)
            })
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
    pub async fn save_checkpoint_to_disk<P: AsRef<Path>>(
        &self,
        checkpoint_id: &str,
        path: P,
    ) -> Result<()> {
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
            description: format!(
                "Context restored from checkpoint: {}",
                checkpoint.checkpoint_id
            ),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert(
                    "checkpoint_id".to_string(),
                    serde_json::json!(checkpoint.checkpoint_id),
                );
                meta.insert(
                    "checkpoint_type".to_string(),
                    serde_json::json!(checkpoint.checkpoint_type),
                );
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
            return Err(anyhow::anyhow!(
                "Execution history should not be empty with non-zero iterations"
            ));
        }

        // Check that completed tasks have completion timestamps
        for task in &self.completed_tasks {
            if task.completed_at.is_none() {
                return Err(anyhow::anyhow!(
                    "Completed task missing completion timestamp: {}",
                    task.task_id
                ));
            }
        }

        // Check checkpoint consistency
        for checkpoint in &self.checkpoints {
            if checkpoint.checkpoint_id.is_empty() {
                return Err(anyhow::anyhow!("Checkpoint ID cannot be empty"));
            }
            if checkpoint.iteration_count > self.iteration_count {
                return Err(anyhow::anyhow!(
                    "Checkpoint iteration count cannot exceed current iteration count"
                ));
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

    // =========================================================================
    // Progress Tracking Methods
    // =========================================================================

    /// Record a successful action
    pub fn record_action_success(&mut self, action_description: &str) {
        self.successful_actions += 1;
        self.last_successful_action = Some(action_description.to_string());
        self.last_update = SystemTime::now();

        // Check for milestone updates
        self.update_milestone();

        // Check if we should create a progress checkpoint
        if self.should_create_progress_checkpoint() {
            let milestone_str = self
                .last_milestone
                .as_ref()
                .map(|m| format!("{:?}", m))
                .unwrap_or_else(|| "none".to_string());
            let description = format!(
                "Progress checkpoint at iteration {} (milestone: {})",
                self.iteration_count, milestone_str
            );
            self.create_progress_checkpoint(description);
        }
    }

    /// Record a failed action
    pub fn record_action_failure(&mut self) {
        self.failed_actions += 1;
        self.last_update = SystemTime::now();
    }

    /// Record an API call with optional token count
    pub fn record_api_call(&mut self, tokens: Option<u64>) {
        self.api_calls_made += 1;
        if let Some(t) = tokens {
            self.tokens_used += t;
        }
        self.last_update = SystemTime::now();
    }

    /// Record tokens used
    pub fn record_tokens_used(&mut self, tokens: u64) {
        self.tokens_used += tokens;
        self.last_update = SystemTime::now();
    }

    /// Set maximum iterations (for completion percentage calculation)
    pub fn set_max_iterations(&mut self, max: u32) {
        self.max_iterations = Some(max);
        self.last_update = SystemTime::now();
    }

    /// Set progress checkpoint interval
    pub fn set_progress_checkpoint_interval(&mut self, interval: Option<u32>) {
        self.progress_checkpoint_interval = interval;
        self.last_update = SystemTime::now();
    }

    /// Calculate completion percentage based on iterations and max_iterations
    pub fn calculate_completion_percentage(&self) -> f64 {
        match self.max_iterations {
            Some(max) if max > 0 => (self.iteration_count as f64 / max as f64 * 100.0).min(100.0),
            _ => {
                // Estimate based on successful actions if no max_iterations
                // Assume ~20 actions for a typical goal
                let estimated_total = 20.0;
                ((self.successful_actions as f64 / estimated_total) * 100.0).min(100.0)
            }
        }
    }

    /// Calculate current milestone based on completion percentage
    pub fn calculate_milestone(&self) -> ProgressMilestone {
        let pct = self.calculate_completion_percentage();
        if pct >= 100.0 {
            ProgressMilestone::Completed
        } else if pct >= 90.0 {
            ProgressMilestone::NearComplete
        } else if pct >= 75.0 {
            ProgressMilestone::ThreeQuarters
        } else if pct >= 50.0 {
            ProgressMilestone::Half
        } else if pct >= 25.0 {
            ProgressMilestone::Quarter
        } else {
            ProgressMilestone::Started
        }
    }

    /// Update milestone if we've reached a new one
    fn update_milestone(&mut self) {
        let current_milestone = self.calculate_milestone();
        let should_update = match (&self.last_milestone, &current_milestone) {
            (None, _) => true,
            (Some(last), curr) => milestone_rank(curr) > milestone_rank(last),
        };

        if should_update {
            self.last_milestone = Some(current_milestone);
        }
    }

    /// Check if we should create a progress checkpoint
    pub fn should_create_progress_checkpoint(&self) -> bool {
        if let Some(interval) = self.progress_checkpoint_interval {
            self.iteration_count > 0 && self.iteration_count.is_multiple_of(interval)
        } else {
            false
        }
    }

    /// Build progress data from current context state
    pub fn build_progress_data(&self) -> ProgressData {
        let total_actions = self.successful_actions + self.failed_actions;
        let success_rate = if total_actions > 0 {
            self.successful_actions as f64 / total_actions as f64
        } else {
            1.0
        };

        let elapsed_seconds = self.start_time.elapsed().map(|d| d.as_secs()).unwrap_or(0);

        ProgressData {
            current_iteration: self.iteration_count,
            max_iterations: self.max_iterations,
            estimated_completion_percentage: self.calculate_completion_percentage(),
            successful_actions: self.successful_actions,
            failed_actions: self.failed_actions,
            success_rate,
            tokens_used: self.tokens_used,
            api_calls_made: self.api_calls_made,
            elapsed_seconds,
            last_successful_action: self.last_successful_action.clone(),
            milestone: self.last_milestone.clone(),
            recovery_hint: self.build_recovery_hint(),
        }
    }

    /// Build a recovery hint for resumption
    fn build_recovery_hint(&self) -> Option<String> {
        // Build hint based on last successful action and current state
        let mut hints = Vec::new();

        if let Some(action) = &self.last_successful_action {
            hints.push(format!("Last action: {}", action));
        }

        if !self.active_tasks.is_empty() {
            let task_names: Vec<_> = self
                .active_tasks
                .iter()
                .take(3)
                .map(|t| t.description.as_str())
                .collect();
            hints.push(format!("Active tasks: {}", task_names.join(", ")));
        }

        if hints.is_empty() {
            None
        } else {
            Some(hints.join("; "))
        }
    }

    /// Create a progress checkpoint with full progress data
    pub fn create_progress_checkpoint(&mut self, description: String) -> String {
        let checkpoint_id = uuid::Uuid::new_v4().to_string();
        let progress_data = self.build_progress_data();

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

        // Determine checkpoint type based on milestone
        let checkpoint_type = if progress_data.milestone.is_some() {
            CheckpointType::Milestone
        } else {
            CheckpointType::Progress
        };

        let checkpoint = ContextCheckpoint {
            checkpoint_id: checkpoint_id.clone(),
            timestamp: SystemTime::now(),
            iteration_count: self.iteration_count,
            checkpoint_type,
            context_snapshot: snapshot,
            description,
            metadata: HashMap::new(),
            progress_data: Some(progress_data),
        };

        self.checkpoints.push(checkpoint);
        self.state_version += 1;
        self.last_update = SystemTime::now();

        // Keep only the last 20 checkpoints for progress (more than regular checkpoints)
        let progress_count = self
            .checkpoints
            .iter()
            .filter(|c| {
                matches!(
                    c.checkpoint_type,
                    CheckpointType::Progress | CheckpointType::Milestone
                )
            })
            .count();
        if progress_count > 20 {
            // Remove oldest progress checkpoint
            if let Some(idx) = self.checkpoints.iter().position(|c| {
                matches!(
                    c.checkpoint_type,
                    CheckpointType::Progress | CheckpointType::Milestone
                )
            }) {
                self.checkpoints.remove(idx);
            }
        }

        checkpoint_id
    }

    /// Get the latest progress checkpoint
    pub fn get_latest_progress_checkpoint(&self) -> Option<&ContextCheckpoint> {
        self.checkpoints.iter().rev().find(|c| {
            matches!(
                c.checkpoint_type,
                CheckpointType::Progress | CheckpointType::Milestone
            )
        })
    }

    /// Build recovery info from a progress checkpoint
    pub fn build_recovery_info(&self, checkpoint: &ContextCheckpoint) -> ProgressRecoveryInfo {
        let progress_data = checkpoint.progress_data.as_ref();

        // Determine resumption strategy based on context
        let resumption_strategy = if self.failed_actions > 0
            && self.failed_actions as f64 / (self.successful_actions + self.failed_actions) as f64
                > 0.5
        {
            ResumptionStrategy::RebuildContext
        } else if checkpoint.context_snapshot.last_action_summary.is_some() {
            ResumptionStrategy::ContinueFromCheckpoint
        } else {
            ResumptionStrategy::SkipToNextPhase
        };

        ProgressRecoveryInfo {
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            iteration_at_checkpoint: checkpoint.iteration_count,
            completion_percentage: progress_data
                .map(|p| p.estimated_completion_percentage)
                .unwrap_or(0.0),
            recovered_variables: checkpoint.context_snapshot.key_variables.clone(),
            last_successful_action: progress_data.and_then(|p| p.last_successful_action.clone()),
            resumption_strategy,
        }
    }

    /// Resume execution from a progress checkpoint
    pub fn resume_from_progress_checkpoint(&mut self, checkpoint: &ContextCheckpoint) {
        // Restore key state from checkpoint snapshot
        let snapshot = &checkpoint.context_snapshot;

        // Update variables with key variables from checkpoint
        for (key, value) in &snapshot.key_variables {
            self.variables.insert(key.clone(), value.clone());
        }

        // Restore progress tracking state if available
        if let Some(progress) = &checkpoint.progress_data {
            self.iteration_count = progress.current_iteration;
            self.successful_actions = progress.successful_actions;
            self.failed_actions = progress.failed_actions;
            self.tokens_used = progress.tokens_used;
            self.api_calls_made = progress.api_calls_made;
            self.last_successful_action = progress.last_successful_action.clone();
            self.last_milestone = progress.milestone.clone();
        }

        // Add a restoration event to history
        self.execution_history.push(ExecutionEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            event_type: ExecutionEventType::ContextRestored,
            description: format!(
                "Context resumed from progress checkpoint: {} (iteration {})",
                checkpoint.checkpoint_id, checkpoint.iteration_count
            ),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert(
                    "checkpoint_id".to_string(),
                    serde_json::json!(checkpoint.checkpoint_id),
                );
                meta.insert("checkpoint_type".to_string(), serde_json::json!("progress"));
                if let Some(progress) = &checkpoint.progress_data {
                    meta.insert(
                        "completion_percentage".to_string(),
                        serde_json::json!(progress.estimated_completion_percentage),
                    );
                }
                meta
            },
        });

        self.state_version += 1;
        self.last_update = SystemTime::now();
    }

    /// Save the latest progress checkpoint to disk
    pub async fn save_progress_checkpoint_to_disk<P: AsRef<Path>>(
        &self,
        dir_path: P,
    ) -> Result<Option<String>> {
        if let Some(checkpoint) = self.get_latest_progress_checkpoint() {
            let file_name = format!("progress_checkpoint_{}.json", checkpoint.checkpoint_id);
            let file_path = dir_path.as_ref().join(&file_name);
            let json_data = serde_json::to_string_pretty(checkpoint)?;
            fs::write(&file_path, json_data).await?;
            Ok(Some(file_path.to_string_lossy().to_string()))
        } else {
            Ok(None)
        }
    }

    /// Get progress statistics summary
    pub fn get_progress_stats(&self) -> String {
        let pct = self.calculate_completion_percentage();
        let total_actions = self.successful_actions + self.failed_actions;
        let success_rate = if total_actions > 0 {
            self.successful_actions as f64 / total_actions as f64 * 100.0
        } else {
            100.0
        };

        format!(
            "Progress: {:.1}% | Actions: {} ({:.0}% success) | API calls: {} | Tokens: {}",
            pct, total_actions, success_rate, self.api_calls_made, self.tokens_used
        )
    }
}

/// Helper function to rank milestones for comparison
fn milestone_rank(milestone: &ProgressMilestone) -> u8 {
    match milestone {
        ProgressMilestone::Started => 0,
        ProgressMilestone::Quarter => 1,
        ProgressMilestone::Half => 2,
        ProgressMilestone::ThreeQuarters => 3,
        ProgressMilestone::NearComplete => 4,
        ProgressMilestone::Completed => 5,
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
            context_data: HashMap::new(),
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
            // Progress tracking fields
            max_iterations: None,
            successful_actions: 0,
            failed_actions: 0,
            tokens_used: 0,
            api_calls_made: 0,
            progress_checkpoint_interval: None,
            last_milestone: None,
            last_successful_action: None,
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
        let checkpoint_id =
            context.create_checkpoint(CheckpointType::Manual, "Test checkpoint".to_string());

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

    // =========================================================================
    // Progress Checkpoint Tests
    // =========================================================================

    #[test]
    fn test_progress_tracking_basic() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::CodeGeneration,
            priority: GoalPriority::High,
            success_criteria: Vec::new(),
            max_iterations: Some(100),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        // Record some actions
        context.record_action_success("Created file");
        context.record_action_success("Wrote content");
        context.record_action_failure();

        assert_eq!(context.successful_actions, 2);
        assert_eq!(context.failed_actions, 1);
        assert_eq!(
            context.last_successful_action,
            Some("Wrote content".to_string())
        );
    }

    #[test]
    fn test_completion_percentage_with_max_iterations() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: Some(100),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        // At 0 iterations, should be 0%
        assert_eq!(context.calculate_completion_percentage(), 0.0);

        // At 25 iterations, should be 25%
        context.iteration_count = 25;
        assert_eq!(context.calculate_completion_percentage(), 25.0);

        // At 50 iterations, should be 50%
        context.iteration_count = 50;
        assert_eq!(context.calculate_completion_percentage(), 50.0);

        // At 100 iterations, should be 100%
        context.iteration_count = 100;
        assert_eq!(context.calculate_completion_percentage(), 100.0);
    }

    #[test]
    fn test_milestone_calculation() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::FileOperation,
            priority: GoalPriority::Low,
            success_criteria: Vec::new(),
            max_iterations: Some(100),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);

        // Started milestone
        assert_eq!(context.calculate_milestone(), ProgressMilestone::Started);

        // Quarter milestone (25%)
        context.iteration_count = 25;
        assert_eq!(context.calculate_milestone(), ProgressMilestone::Quarter);

        // Half milestone (50%)
        context.iteration_count = 50;
        assert_eq!(context.calculate_milestone(), ProgressMilestone::Half);

        // Three-quarters milestone (75%)
        context.iteration_count = 75;
        assert_eq!(
            context.calculate_milestone(),
            ProgressMilestone::ThreeQuarters
        );

        // Near complete (90%)
        context.iteration_count = 90;
        assert_eq!(
            context.calculate_milestone(),
            ProgressMilestone::NearComplete
        );

        // Completed (100%)
        context.iteration_count = 100;
        assert_eq!(context.calculate_milestone(), ProgressMilestone::Completed);
    }

    #[test]
    fn test_api_call_tracking() {
        let mut context = ExecutionContext::default();

        context.record_api_call(Some(500));
        context.record_api_call(Some(1000));
        context.record_api_call(None);

        assert_eq!(context.api_calls_made, 3);
        assert_eq!(context.tokens_used, 1500);
    }

    #[test]
    fn test_progress_checkpoint_creation() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: Some(50),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);
        context.iteration_count = 25;
        context.record_action_success("Test action");
        context.record_api_call(Some(1000));

        let checkpoint_id =
            context.create_progress_checkpoint("Test progress checkpoint".to_string());

        assert!(!checkpoint_id.is_empty());

        let checkpoint = context.get_latest_progress_checkpoint().unwrap();
        assert!(checkpoint.progress_data.is_some());

        let progress = checkpoint.progress_data.as_ref().unwrap();
        assert_eq!(progress.current_iteration, 25);
        assert_eq!(progress.successful_actions, 1);
        assert_eq!(progress.tokens_used, 1000);
        assert_eq!(progress.estimated_completion_percentage, 50.0);
    }

    #[test]
    fn test_progress_data_build() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::CodeGeneration,
            priority: GoalPriority::High,
            success_criteria: Vec::new(),
            max_iterations: Some(100),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);
        context.iteration_count = 30;
        context.successful_actions = 10;
        context.failed_actions = 2;
        context.tokens_used = 5000;
        context.api_calls_made = 15;
        context.last_successful_action = Some("File written".to_string());

        let progress = context.build_progress_data();

        assert_eq!(progress.current_iteration, 30);
        assert_eq!(progress.max_iterations, Some(100));
        assert_eq!(progress.successful_actions, 10);
        assert_eq!(progress.failed_actions, 2);
        assert_eq!(progress.tokens_used, 5000);
        assert_eq!(progress.api_calls_made, 15);
        // Success rate = 10 / 12 ≈ 0.833
        assert!((progress.success_rate - 0.833).abs() < 0.01);
    }

    #[test]
    fn test_resume_from_progress_checkpoint() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: Some(100),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal.clone());
        context.iteration_count = 50;
        context.successful_actions = 20;
        context.failed_actions = 3;
        context.tokens_used = 10000;
        context.api_calls_made = 25;
        context.last_successful_action = Some("Analyzed data".to_string());

        // Create checkpoint
        context.create_progress_checkpoint("Mid-execution checkpoint".to_string());
        let checkpoint = context.get_latest_progress_checkpoint().unwrap().clone();

        // Create new context and resume from checkpoint
        let mut new_context = ExecutionContext::new(goal);
        new_context.resume_from_progress_checkpoint(&checkpoint);

        assert_eq!(new_context.iteration_count, 50);
        assert_eq!(new_context.successful_actions, 20);
        assert_eq!(new_context.failed_actions, 3);
        assert_eq!(new_context.tokens_used, 10000);
        assert_eq!(new_context.api_calls_made, 25);
        assert_eq!(
            new_context.last_successful_action,
            Some("Analyzed data".to_string())
        );
    }

    #[test]
    fn test_progress_stats_summary() {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::FileOperation,
            priority: GoalPriority::Low,
            success_criteria: Vec::new(),
            max_iterations: Some(100),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);
        context.iteration_count = 50;
        context.successful_actions = 18;
        context.failed_actions = 2;
        context.tokens_used = 5000;
        context.api_calls_made = 20;

        let stats = context.get_progress_stats();

        assert!(stats.contains("50.0%"));
        assert!(stats.contains("Actions: 20"));
        assert!(stats.contains("90%")); // 18/20 = 90% success
        assert!(stats.contains("API calls: 20"));
        assert!(stats.contains("Tokens: 5000"));
    }

    #[test]
    fn test_milestone_rank() {
        assert!(
            milestone_rank(&ProgressMilestone::Started)
                < milestone_rank(&ProgressMilestone::Quarter)
        );
        assert!(
            milestone_rank(&ProgressMilestone::Quarter) < milestone_rank(&ProgressMilestone::Half)
        );
        assert!(
            milestone_rank(&ProgressMilestone::Half)
                < milestone_rank(&ProgressMilestone::ThreeQuarters)
        );
        assert!(
            milestone_rank(&ProgressMilestone::ThreeQuarters)
                < milestone_rank(&ProgressMilestone::NearComplete)
        );
        assert!(
            milestone_rank(&ProgressMilestone::NearComplete)
                < milestone_rank(&ProgressMilestone::Completed)
        );
    }

    #[tokio::test]
    async fn test_progress_checkpoint_persistence() {
        use tempfile::tempdir;

        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::High,
            success_criteria: Vec::new(),
            max_iterations: Some(100),
            timeout: None,
            metadata: HashMap::new(),
        };

        let mut context = ExecutionContext::new(goal);
        context.iteration_count = 25;
        context.record_action_success("Test action");

        // Create progress checkpoint
        context.create_progress_checkpoint("Test checkpoint".to_string());

        // Save to disk
        let temp_dir = tempdir().unwrap();
        let result = context
            .save_progress_checkpoint_to_disk(temp_dir.path())
            .await
            .unwrap();

        assert!(result.is_some());
        let file_path = result.unwrap();
        assert!(file_path.contains("progress_checkpoint_"));
    }
}
