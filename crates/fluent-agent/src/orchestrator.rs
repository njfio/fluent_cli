use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use crate::action::{ActionExecutor, ActionPlanner};
use crate::config::AgentRuntimeConfig;
use crate::context::{ExecutionContext, CheckpointType};
use crate::goal::{Goal, GoalResult};
use crate::memory::MemorySystem;
use crate::observation::ObservationProcessor;
use crate::reasoning::{LLMReasoningEngine, ReasoningEngine};
use crate::reflection::ReflectionEngine;
use crate::state_manager::StateManager as PersistentStateManager;
use crate::task::{Task, TaskResult};

/// Core agent orchestrator implementing the ReAct (Reasoning, Acting, Observing) pattern
///
/// This is the central component that coordinates all agent activities:
/// - Reasoning: Analyzes current state and plans next actions
/// - Acting: Executes planned actions through tools and engines
/// - Observing: Processes results and updates context
///
/// The orchestrator maintains the overall goal, decomposes it into tasks,
/// and manages the execution state throughout the workflow.
pub struct AgentOrchestrator {
    reasoning_engine: Box<dyn ReasoningEngine>,
    action_planner: Box<dyn ActionPlanner>,
    action_executor: Box<dyn ActionExecutor>,
    observation_processor: Box<dyn ObservationProcessor>,
    memory_system: Arc<MemorySystem>,
    state_manager: Arc<StateManager>,
    persistent_state_manager: Arc<PersistentStateManager>,
    reflection_engine: Arc<RwLock<ReflectionEngine>>,
    metrics: Arc<RwLock<OrchestrationMetrics>>,
}

/// Manages the execution state and context throughout the agent workflow
#[allow(dead_code)]
pub struct StateManager {
    current_state: Arc<RwLock<AgentState>>,
    state_history: Arc<RwLock<Vec<AgentState>>>,
}

/// Current state of the agent during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub current_goal: Option<Goal>,
    pub active_tasks: Vec<Task>,
    pub completed_tasks: Vec<TaskResult>,
    pub current_context: ExecutionContext,
    pub reasoning_history: Vec<ReasoningStep>,
    pub last_action: Option<ActionStep>,
    pub observations: Vec<Observation>,
    pub iteration_count: u32,
    pub start_time: SystemTime,
    pub last_update: SystemTime,
}

/// Individual reasoning step in the agent's thought process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_id: String,
    pub timestamp: SystemTime,
    pub reasoning_type: ReasoningType,
    pub input_context: String,
    pub reasoning_output: String,
    pub confidence_score: f64,
    pub next_action_plan: Option<String>,
}

/// Types of reasoning the agent can perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningType {
    GoalAnalysis,
    TaskDecomposition,
    ActionPlanning,
    ContextAnalysis,
    ProblemSolving,
    SelfReflection,
    StrategyAdjustment,
}

/// Action step taken by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub action_id: String,
    pub timestamp: SystemTime,
    pub action_type: ActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub execution_result: Option<ActionResult>,
    pub duration: Option<Duration>,
}

/// Types of actions the agent can take
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionType {
    ToolExecution,
    CodeGeneration,
    FileOperation,
    Analysis,
    Communication,
    Planning,
}

/// Result of an action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Observation made by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub observation_id: String,
    pub timestamp: SystemTime,
    pub observation_type: ObservationType,
    pub content: String,
    pub source: String,
    pub relevance_score: f64,
    pub impact_assessment: Option<String>,
}

/// Types of observations the agent can make
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObservationType {
    ActionResult,
    EnvironmentChange,
    UserFeedback,
    SystemEvent,
    ErrorOccurrence,
    ProgressUpdate,
}

/// Metrics for monitoring orchestration performance
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OrchestrationMetrics {
    pub total_goals_processed: u64,
    pub successful_goals: u64,
    pub failed_goals: u64,
    pub total_reasoning_steps: u64,
    pub total_actions_taken: u64,
    pub total_observations_made: u64,
    pub average_goal_completion_time: f64,
    pub average_reasoning_time: f64,
    pub average_action_time: f64,
    pub success_rate: f64,
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator with the specified components
    pub async fn new(
        reasoning_engine: Box<dyn ReasoningEngine>,
        action_planner: Box<dyn ActionPlanner>,
        action_executor: Box<dyn ActionExecutor>,
        observation_processor: Box<dyn ObservationProcessor>,
        memory_system: Arc<MemorySystem>,
        persistent_state_manager: Arc<PersistentStateManager>,
        reflection_engine: ReflectionEngine,
    ) -> Self {
        Self {
            reasoning_engine,
            action_planner,
            action_executor,
            observation_processor,
            memory_system,
            state_manager: Arc::new(StateManager::new()),
            persistent_state_manager,
            reflection_engine: Arc::new(RwLock::new(reflection_engine)),
            metrics: Arc::new(RwLock::new(OrchestrationMetrics::default())),
        }
    }

    /// Create a new agent orchestrator from runtime configuration
    pub async fn from_config(
        runtime_config: AgentRuntimeConfig,
        action_planner: Box<dyn ActionPlanner>,
        action_executor: Box<dyn ActionExecutor>,
        observation_processor: Box<dyn ObservationProcessor>,
        memory_system: Arc<MemorySystem>,
        persistent_state_manager: Arc<PersistentStateManager>,
        reflection_engine: ReflectionEngine,
    ) -> Result<Self> {
        let reasoning_engine = Box::new(LLMReasoningEngine::new(
            runtime_config.reasoning_engine.clone(),
        ));

        Ok(Self::new(
            reasoning_engine,
            action_planner,
            action_executor,
            observation_processor,
            memory_system,
            persistent_state_manager,
            reflection_engine,
        ).await)
    }

    /// Execute a goal using the ReAct pattern
    ///
    /// This is the main entry point for agent execution. It implements the core
    /// ReAct loop: Reasoning -> Acting -> Observing, with self-reflection and
    /// strategy adjustment capabilities.
    pub async fn execute_goal(&self, goal: Goal) -> Result<GoalResult> {
        let start_time = SystemTime::now();
        let mut context = ExecutionContext::new(goal.clone());

        // Initialize agent state
        self.initialize_state(goal.clone(), &context).await?;

        // Set context in persistent state manager
        self.persistent_state_manager.set_context(context.clone()).await?;

        // Create initial checkpoint
        self.persistent_state_manager.create_checkpoint(
            CheckpointType::BeforeAction,
            "Goal execution started".to_string()
        ).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_goals_processed += 1;
        }

        let mut iteration_count = 0;
        let max_iterations = goal.max_iterations.unwrap_or(50);

        loop {
            iteration_count += 1;

            // Safety check to prevent infinite loops
            if iteration_count > max_iterations {
                return Err(anyhow!(
                    "Maximum iterations ({}) exceeded for goal: {}",
                    max_iterations,
                    goal.description
                ));
            }

            // Reasoning Phase: Analyze current state and plan next action
            let reasoning_start = SystemTime::now();
            let reasoning_result = self.reasoning_engine.reason(&context).await?;
            let reasoning_duration = reasoning_start.elapsed().unwrap_or_default();

            // Record reasoning step
            self.record_reasoning_step(reasoning_result.clone(), reasoning_duration)
                .await?;

            // Check if goal is achieved
            if self.is_goal_achieved(&context, &reasoning_result).await? {
                let final_result = self.finalize_goal_execution(&context, true).await?;
                self.update_success_metrics(start_time.elapsed().unwrap_or_default())
                    .await;
                return Ok(final_result);
            }

            // Planning Phase: Determine specific action to take
            let action_plan = self
                .action_planner
                .plan_action(reasoning_result, &context)
                .await?;

            // Create checkpoint before action execution
            self.persistent_state_manager.create_checkpoint(
                CheckpointType::BeforeAction,
                format!("Before action execution at iteration {}", iteration_count)
            ).await?;

            // Execution Phase: Execute the planned action
            let action_start = SystemTime::now();
            let action_execution_result = self
                .action_executor
                .execute(action_plan, &mut context)
                .await?;
            let action_duration = action_start.elapsed().unwrap_or_default();

            // Create checkpoint after action execution
            self.persistent_state_manager.create_checkpoint(
                CheckpointType::AfterAction,
                format!("After action execution at iteration {}", iteration_count)
            ).await?;

            // Convert to orchestrator ActionResult for recording
            let action_result = ActionResult {
                success: action_execution_result.success,
                output: action_execution_result.output.clone(),
                error: action_execution_result.error.clone(),
                metadata: action_execution_result.metadata.clone(),
            };

            // Record action step
            self.record_action_step(action_result, action_duration)
                .await?;

            // Observation Phase: Process results and update context
            let observation = self
                .observation_processor
                .process(action_execution_result, &context)
                .await?;

            // Record observation
            self.record_observation(observation.clone()).await?;

            // Update context with new information
            context.add_observation(observation);

            // Update memory system with new learnings
            self.memory_system.update(&context).await?;

            // Advanced Self-reflection: Evaluate progress and adjust strategy if needed
            let mut reflection_engine = self.reflection_engine.write().await;
            if let Some(trigger) = reflection_engine.should_reflect(&context) {
                // Create checkpoint before reflection
                self.persistent_state_manager.create_checkpoint(
                    CheckpointType::BeforeReflection,
                    format!("Before reflection at iteration {} (trigger: {:?})", iteration_count, trigger)
                ).await?;

                // Perform comprehensive reflection
                let reflection_result = reflection_engine.reflect(
                    &context,
                    self.reasoning_engine.as_ref(),
                    trigger
                ).await?;

                // Apply strategy adjustments
                if !reflection_result.strategy_adjustments.is_empty() {
                    self.apply_strategy_adjustments(&mut context, &reflection_result.strategy_adjustments)
                        .await?;
                }

                // Log reflection insights
                log::info!("Reflection completed: {} insights, {} adjustments, confidence: {:.2}",
                          reflection_result.learning_insights.len(),
                          reflection_result.strategy_adjustments.len(),
                          reflection_result.confidence_assessment);
            }
            drop(reflection_engine); // Release the lock

            // Update agent state
            self.update_state(&context, iteration_count).await?;

            // Update persistent state manager with current context
            self.persistent_state_manager.set_context(context.clone()).await?;
        }
    }

    /// Initialize the agent state for goal execution
    async fn initialize_state(&self, goal: Goal, context: &ExecutionContext) -> Result<()> {
        let initial_state = AgentState {
            current_goal: Some(goal),
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            current_context: context.clone(),
            reasoning_history: Vec::new(),
            last_action: None,
            observations: Vec::new(),
            iteration_count: 0,
            start_time: SystemTime::now(),
            last_update: SystemTime::now(),
        };

        let mut state = self.state_manager.current_state.write().await;
        *state = initial_state;

        Ok(())
    }

    /// Check if the goal has been achieved
    async fn is_goal_achieved(
        &self,
        _context: &ExecutionContext,
        reasoning: &ReasoningResult,
    ) -> Result<bool> {
        // Implementation would check various criteria:
        // - Goal completion conditions met
        // - Success metrics achieved
        // - User satisfaction confirmed
        // - No critical errors present

        // For now, simplified check based on reasoning output
        Ok(reasoning.goal_achieved_confidence > 0.8)
    }

    /// Apply strategy adjustments from reflection results
    async fn apply_strategy_adjustments(
        &self,
        context: &mut ExecutionContext,
        adjustments: &[crate::reflection::StrategyAdjustment],
    ) -> Result<()> {
        for adjustment in adjustments {
            // Apply the adjustment to the context
            let adjustment_description = format!(
                "{}: {} (Expected impact: {:?})",
                adjustment.adjustment_type,
                adjustment.description,
                adjustment.expected_impact
            );

            context.add_strategy_adjustment(vec![adjustment_description]);

            // Log the adjustment
            log::info!("Applied strategy adjustment: {} - {}",
                      adjustment.adjustment_id, adjustment.description);

            // Create checkpoint after applying adjustment
            self.persistent_state_manager.create_checkpoint(
                CheckpointType::AfterAction,
                format!("After applying strategy adjustment: {}", adjustment.adjustment_id)
            ).await?;
        }

        Ok(())
    }

    /// Get reflection engine for external access
    pub fn get_reflection_engine(&self) -> Arc<RwLock<ReflectionEngine>> {
        self.reflection_engine.clone()
    }

    /// Trigger manual reflection
    pub async fn trigger_reflection(&self, context: &ExecutionContext, _reason: String) -> Result<crate::reflection::ReflectionResult> {
        let mut reflection_engine = self.reflection_engine.write().await;
        let trigger = crate::reflection::ReflectionTrigger::UserRequest;

        reflection_engine.reflect(
            context,
            self.reasoning_engine.as_ref(),
            trigger
        ).await
    }

    /// Record a reasoning step for analysis and debugging
    async fn record_reasoning_step(
        &self,
        reasoning: ReasoningResult,
        duration: Duration,
    ) -> Result<()> {
        let step = ReasoningStep {
            step_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            reasoning_type: reasoning.reasoning_type,
            input_context: reasoning.input_context,
            reasoning_output: reasoning.reasoning_output,
            confidence_score: reasoning.confidence_score,
            next_action_plan: reasoning.next_actions.first().cloned(),
        };

        let mut state = self.state_manager.current_state.write().await;
        state.reasoning_history.push(step);

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_reasoning_steps += 1;
        metrics.average_reasoning_time = (metrics.average_reasoning_time
            * (metrics.total_reasoning_steps - 1) as f64
            + duration.as_millis() as f64)
            / metrics.total_reasoning_steps as f64;

        Ok(())
    }

    /// Record an action step for analysis and debugging
    async fn record_action_step(&self, action: ActionResult, duration: Duration) -> Result<()> {
        let step = ActionStep {
            action_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            action_type: ActionType::ToolExecution, // Default type since we don't have it in ActionResult
            parameters: HashMap::new(), // Default empty since we don't have it in ActionResult
            execution_result: Some(action),
            duration: Some(duration),
        };

        let mut state = self.state_manager.current_state.write().await;
        state.last_action = Some(step);

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_actions_taken += 1;
        metrics.average_action_time = (metrics.average_action_time
            * (metrics.total_actions_taken - 1) as f64
            + duration.as_millis() as f64)
            / metrics.total_actions_taken as f64;

        Ok(())
    }

    /// Record an observation for analysis and learning
    async fn record_observation(&self, observation: Observation) -> Result<()> {
        let mut state = self.state_manager.current_state.write().await;
        state.observations.push(observation);

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_observations_made += 1;

        Ok(())
    }

    /// Update the current agent state
    async fn update_state(&self, context: &ExecutionContext, iteration_count: u32) -> Result<()> {
        let mut state = self.state_manager.current_state.write().await;
        state.current_context = context.clone();
        state.iteration_count = iteration_count;
        state.last_update = SystemTime::now();

        Ok(())
    }

    /// Finalize goal execution and generate result
    async fn finalize_goal_execution(
        &self,
        context: &ExecutionContext,
        success: bool,
    ) -> Result<GoalResult> {
        let state = self.state_manager.current_state.read().await;

        Ok(GoalResult {
            success,
            final_context: context.clone(),
            execution_summary: format!(
                "Goal execution completed in {} iterations",
                state.iteration_count
            ),
            reasoning_steps: state.reasoning_history.len(),
            actions_taken: state.observations.len(),
            total_duration: state.start_time.elapsed().unwrap_or_default(),
            final_output: context.get_final_output(),
        })
    }

    /// Update success metrics
    async fn update_success_metrics(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.successful_goals += 1;
        metrics.average_goal_completion_time = (metrics.average_goal_completion_time
            * (metrics.successful_goals - 1) as f64
            + duration.as_millis() as f64)
            / metrics.successful_goals as f64;
        metrics.success_rate =
            metrics.successful_goals as f64 / metrics.total_goals_processed as f64;
    }

    /// Get current orchestration metrics
    pub async fn get_metrics(&self) -> OrchestrationMetrics {
        self.metrics.read().await.clone()
    }

    /// Get current agent state
    pub async fn get_current_state(&self) -> AgentState {
        self.state_manager.current_state.read().await.clone()
    }

    /// Get the persistent state manager for advanced state operations
    pub fn get_persistent_state_manager(&self) -> Arc<PersistentStateManager> {
        self.persistent_state_manager.clone()
    }

    /// Save current execution state to disk
    pub async fn save_execution_state(&self) -> Result<()> {
        self.persistent_state_manager.save_context().await
    }

    /// Load execution state from disk
    pub async fn load_execution_state(&self, context_id: &str) -> Result<()> {
        let context = self.persistent_state_manager.load_context(context_id).await?;
        self.persistent_state_manager.set_context(context).await
    }
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            current_state: Arc::new(RwLock::new(AgentState::default())),
            state_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self {
            current_goal: None,
            active_tasks: Vec::new(),
            completed_tasks: Vec::new(),
            current_context: ExecutionContext::default(),
            reasoning_history: Vec::new(),
            last_action: None,
            observations: Vec::new(),
            iteration_count: 0,
            start_time: SystemTime::now(),
            last_update: SystemTime::now(),
        }
    }
}



/// Result of reasoning process
#[derive(Debug, Clone)]
pub struct ReasoningResult {
    pub reasoning_type: ReasoningType,
    pub input_context: String,
    pub reasoning_output: String,
    pub confidence_score: f64,
    pub goal_achieved_confidence: f64,
    pub next_actions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_creation() {
        let state = AgentState::default();
        assert_eq!(state.iteration_count, 0);
        assert!(state.current_goal.is_none());
        assert!(state.active_tasks.is_empty());
    }

    #[test]
    fn test_orchestration_metrics_default() {
        let metrics = OrchestrationMetrics::default();
        assert_eq!(metrics.total_goals_processed, 0);
        assert_eq!(metrics.success_rate, 0.0);
    }
}
