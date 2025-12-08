use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::timeout;

/// Default timeout for acquiring locks to prevent deadlocks
const LOCK_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of retries for reasoning engine calls
const MAX_REASONING_RETRIES: u32 = 3;

/// Base delay between reasoning retries (doubles each retry)
const REASONING_RETRY_BASE_DELAY: Duration = Duration::from_secs(2);

/// Number of consecutive similar iterations before detecting convergence
const CONVERGENCE_THRESHOLD: usize = 3;

/// Minimum similarity ratio (0.0-1.0) to consider outputs as "similar"
const SIMILARITY_THRESHOLD: f64 = 0.85;
// use uuid::Uuid;
use strum_macros::{Display, EnumString};

use crate::action::{ActionExecutor, ActionPlanner, ActionResult as DetailedActionResult};
use crate::autonomy::{AutonomySupervisor, GuardrailDecision, RiskAssessment, SupervisorStage};
use crate::config::AgentRuntimeConfig;
use crate::context::{CheckpointType, ExecutionContext};
use crate::goal::{Goal, GoalResult};
use crate::memory::MemorySystem;
use crate::monitoring::{AdaptiveStrategySystem, PerformanceMetrics};
use crate::observation::ObservationProcessor;
use crate::planning::DynamicReplanner;
use crate::reasoning::enhanced_multi_modal::{EnhancedMultiModalEngine, EnhancedReasoningConfig};
use crate::reasoning::{ReasoningCapability, ReasoningEngine};
use crate::reflection_engine::ReflectionEngine;
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
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    autonomy_supervisor: Option<Arc<AutonomySupervisor>>,
    dynamic_replanner: Option<Arc<DynamicReplanner>>,
    adaptive_strategy: Option<Arc<AdaptiveStrategySystem>>,
}

/// Manages the execution state and context throughout the agent workflow
#[allow(dead_code)]
pub struct StateManager {
    current_state: tokio::sync::RwLock<AgentState>,
    state_history: tokio::sync::RwLock<Vec<AgentState>>,
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
    pub execution_result: Option<crate::action::ActionResult>,
    pub duration: Option<Duration>,
}

/// Types of actions the agent can take
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumString)]
pub enum ActionType {
    ToolExecution,
    CodeGeneration,
    FileOperation,
    Analysis,
    Communication,
    Planning,
}

/// Result of a reasoning step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    pub reasoning_output: String,
    pub confidence_score: f64,
    pub goal_achieved_confidence: f64,
    pub next_actions: Vec<String>,
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

/// Simple result of action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleActionResult {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tracks recent outputs to detect when the agent is stuck in a loop
#[derive(Debug, Default)]
struct ConvergenceTracker {
    recent_reasoning: Vec<String>,
    recent_actions: Vec<String>,
    similar_count: usize,
}

impl ConvergenceTracker {
    fn new() -> Self {
        Self::default()
    }

    /// Record a reasoning output and check for convergence
    fn record_reasoning(&mut self, output: &str) -> bool {
        let normalized = Self::normalize_output(output);

        // Check similarity with recent outputs
        if self.recent_reasoning.iter().any(|prev| Self::similarity(prev, &normalized) >= SIMILARITY_THRESHOLD) {
            self.similar_count += 1;
        } else {
            self.similar_count = 0;
        }

        // Keep only the last few outputs
        self.recent_reasoning.push(normalized);
        if self.recent_reasoning.len() > CONVERGENCE_THRESHOLD + 1 {
            self.recent_reasoning.remove(0);
        }

        self.similar_count >= CONVERGENCE_THRESHOLD
    }

    /// Record an action and check for convergence
    fn record_action(&mut self, action: &str) {
        let normalized = Self::normalize_output(action);
        self.recent_actions.push(normalized);
        if self.recent_actions.len() > CONVERGENCE_THRESHOLD + 1 {
            self.recent_actions.remove(0);
        }
    }

    /// Normalize output for comparison (lowercase, trim, remove extra whitespace)
    fn normalize_output(output: &str) -> String {
        output
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Calculate Jaccard similarity between two strings
    fn similarity(a: &str, b: &str) -> f64 {
        let words_a: std::collections::HashSet<_> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<_> = b.split_whitespace().collect();

        if words_a.is_empty() && words_b.is_empty() {
            return 1.0;
        }
        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        intersection as f64 / union as f64
    }
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
        performance_metrics: Arc<RwLock<PerformanceMetrics>>,
        autonomy_supervisor: Option<Arc<AutonomySupervisor>>,
        dynamic_replanner: Option<Arc<DynamicReplanner>>,
        adaptive_strategy: Option<Arc<AdaptiveStrategySystem>>,
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
            performance_metrics,
            autonomy_supervisor,
            dynamic_replanner,
            adaptive_strategy,
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
        autonomy_supervisor: Option<Arc<AutonomySupervisor>>,
        dynamic_replanner: Option<Arc<DynamicReplanner>>,
        adaptive_strategy: Option<Arc<AdaptiveStrategySystem>>,
    ) -> Result<Self> {
        // Get the base engine from runtime config or create a mock one
        let base_engine = runtime_config
            .get_base_engine()
            .unwrap_or_else(|| Arc::new(MockEngine));

        // Create enhanced multi-modal reasoning engine
        let enhanced_config = EnhancedReasoningConfig::default();
        let reasoning_engine: Box<dyn ReasoningEngine> =
            Box::new(EnhancedMultiModalEngine::new(base_engine, enhanced_config).await?);

        let performance_metrics = Arc::new(RwLock::new(PerformanceMetrics::default()));
        Ok(Self::new(
            reasoning_engine,
            action_planner,
            action_executor,
            observation_processor,
            memory_system,
            persistent_state_manager,
            reflection_engine,
            performance_metrics,
            autonomy_supervisor,
            dynamic_replanner,
            adaptive_strategy,
        )
        .await)
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
        self.persistent_state_manager
            .set_context(context.clone())
            .await?;

        // Create initial checkpoint
        self.persistent_state_manager
            .create_checkpoint(
                CheckpointType::BeforeAction,
                "Goal execution started".to_string(),
            )
            .await?;

        // Update metrics
        {
            let mut metrics = timeout(LOCK_TIMEOUT, self.metrics.write())
                .await
                .map_err(|_| anyhow!("Timeout acquiring metrics lock in execute_goal"))?;
            metrics.total_goals_processed += 1;
        }

        let mut iteration_count = 0;
        let max_iterations = goal.max_iterations.unwrap_or(50);
        let mut convergence_tracker = ConvergenceTracker::new();

        tracing::info!(
            "react.loop.begin goal='{}' max_iterations={}",
            goal.description,
            max_iterations
        );
        loop {
            // Track iterations locally and in the execution context
            iteration_count += 1;
            tracing::debug!("react.iteration.start iter={}", iteration_count);
            context.increment_iteration();

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
            tracing::debug!(
                "react.reasoning.begin context_len={}",
                context.get_summary().len()
            );

            // Retry reasoning with exponential backoff
            let reasoning_output = {
                let context_summary = context.get_summary();
                let mut last_error = None;
                let mut reasoning_result = None;

                for attempt in 0..MAX_REASONING_RETRIES {
                    match self.reasoning_engine.reason(&context_summary, &context).await {
                        Ok(output) => {
                            reasoning_result = Some(output);
                            break;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "react.reasoning.retry attempt={}/{} error={}",
                                attempt + 1,
                                MAX_REASONING_RETRIES,
                                e
                            );
                            last_error = Some(e);

                            if attempt + 1 < MAX_REASONING_RETRIES {
                                // Exponential backoff: 2s, 4s, 8s, ...
                                let delay = REASONING_RETRY_BASE_DELAY * (1 << attempt);
                                tokio::time::sleep(delay).await;
                            }
                        }
                    }
                }

                reasoning_result.ok_or_else(|| {
                    anyhow!(
                        "Reasoning failed after {} attempts: {}",
                        MAX_REASONING_RETRIES,
                        last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string())
                    )
                })?
            };

            // Convert string output to ReasoningResult structure
            let reasoning_result = ReasoningResult {
                reasoning_output: reasoning_output.clone(),
                confidence_score: self.reasoning_engine.get_confidence().await,
                goal_achieved_confidence: if reasoning_output.to_lowercase().contains("complete")
                    || reasoning_output.to_lowercase().contains("achieved")
                {
                    0.9
                } else {
                    0.3
                },
                next_actions: vec!["Continue with planned action".to_string()],
            };

            tracing::debug!(
                "react.reasoning.end output_len={} conf={:.2} next_actions={}",
                reasoning_result.reasoning_output.len(),
                reasoning_result.confidence_score,
                reasoning_result.next_actions.len()
            );
            let reasoning_duration = reasoning_start.elapsed().unwrap_or_default();

            // Record reasoning step
            self.record_reasoning_step(reasoning_result.clone(), reasoning_duration)
                .await?;

            // Check for convergence (agent stuck in similar reasoning loop)
            if convergence_tracker.record_reasoning(&reasoning_result.reasoning_output) {
                tracing::warn!(
                    "react.convergence_detected iter={} similar_count={}",
                    iteration_count,
                    CONVERGENCE_THRESHOLD
                );

                // Try to break out by requesting a different approach
                context.add_context_item(
                    "system_warning".to_string(),
                    "CONVERGENCE DETECTED: Previous attempts have produced similar results. \
                     Please try a fundamentally different approach or reconsider the goal requirements."
                        .to_string(),
                );

                // If still stuck after additional iterations, fail gracefully
                if iteration_count > max_iterations / 2 {
                    tracing::error!(
                        "react.convergence_fatal iter={} max_iter={}",
                        iteration_count,
                        max_iterations
                    );
                    return Err(anyhow!(
                        "Agent appears stuck in a loop after {} iterations with similar outputs. \
                         Consider rephrasing the goal or breaking it into smaller tasks.",
                        iteration_count
                    ));
                }
            }

            // Check if goal is achieved
            if self.is_goal_achieved(&context, &reasoning_result).await? {
                tracing::info!(
                    "react.goal_achieved iter={} conf={:.2}",
                    iteration_count,
                    reasoning_result.goal_achieved_confidence
                );
                let final_result = self.finalize_goal_execution(&context, true).await?;
                self.update_success_metrics(start_time.elapsed().unwrap_or_default())
                    .await;
                return Ok(final_result);
            }

            // Planning Phase: Determine specific action to take
            let action_plan = self
                .action_planner
                .plan_action(reasoning_result.clone(), &context)
                .await?;
            // Create checkpoint before action execution
            self.persistent_state_manager
                .create_checkpoint(
                    CheckpointType::BeforeAction,
                    format!("Before action execution at iteration {}", iteration_count),
                )
                .await?;

            // Execution Phase: Execute the planned action
            let action_start = SystemTime::now();
            let action_execution_result = self
                .action_executor
                .execute(action_plan, &mut context)
                .await?;
            let action_duration = action_start.elapsed().unwrap_or_default();

            // Create checkpoint after action execution
            self.persistent_state_manager
                .create_checkpoint(
                    CheckpointType::AfterAction,
                    format!("After action execution at iteration {}", iteration_count),
                )
                .await?;

            let action_result = action_execution_result.clone();

            // Record action step
            self.record_action_step(
                SimpleActionResult {
                    success: action_result.success,
                    output: action_result.output.clone(),
                    error: action_result.error.clone(),
                    metadata: action_result.metadata.clone(),
                },
                action_duration,
            )
            .await?;

            // Track action for convergence detection
            if let Some(ref output) = action_result.output {
                convergence_tracker.record_action(output);
            }

            // Observation Phase: Process results and update context
            let observation = self
                .observation_processor
                .process(action_execution_result, &context)
                .await?;

            // Record observation
            self.record_observation(observation.clone()).await?;

            if let Some(supervisor) = &self.autonomy_supervisor {
                let assessment = supervisor
                    .assess_post_action(&action_result, &observation)
                    .await?;
                self.apply_guardrail(SupervisorStage::PostAction, "post action", &assessment)
                    .await?;
            }

            context.add_observation(observation);
            // Update memory system with new learnings
            self.memory_system.update_memory(&context).await?;

            // Advanced Self-reflection: Evaluate progress and adjust strategy if needed
            let mut reflection_engine = timeout(LOCK_TIMEOUT, self.reflection_engine.write())
                .await
                .map_err(|_| anyhow!("Timeout acquiring reflection_engine lock"))?;
            if let Some(trigger) = reflection_engine.should_reflect(&context) {
                // Create checkpoint before reflection
                self.persistent_state_manager
                    .create_checkpoint(
                        CheckpointType::BeforeReflection,
                        format!(
                            "Before reflection at iteration {} (trigger: {:?})",
                            iteration_count, trigger
                        ),
                    )
                    .await?;

                // Perform comprehensive reflection
                let reflection_result = reflection_engine
                    .reflect(&context, self.reasoning_engine.as_ref(), trigger)
                    .await?;

                // Apply strategy adjustments
                if !reflection_result.strategy_adjustments.is_empty() {
                    self.apply_strategy_adjustments(
                        &mut context,
                        &reflection_result.strategy_adjustments,
                    )
                    .await?;
                }

                // Log reflection insights
                tracing::info!(
                    "Reflection completed: {} insights, {} adjustments, confidence: {:.2}",
                    reflection_result.learning_insights.len(),
                    reflection_result.strategy_adjustments.len(),
                    reflection_result.confidence_assessment
                );
            }
            drop(reflection_engine); // Release the lock

            // Update agent state
            self.update_state(&context, iteration_count).await?;

            // Update persistent state manager with current context
            self.persistent_state_manager
                .set_context(context.clone())
                .await?;
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

        let mut state = timeout(LOCK_TIMEOUT, self.state_manager.current_state.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring state lock in initialize_state"))?;
        *state = initial_state;

        Ok(())
    }

    /// Check if the goal has been achieved
    async fn is_goal_achieved(
        &self,
        context: &ExecutionContext,
        reasoning: &ReasoningResult,
    ) -> Result<bool> {
        // 1) Check explicit success criteria on the goal if provided
        if let Some(goal) = context.get_current_goal() {
            if !goal.success_criteria.is_empty() {
                if self
                    .check_success_criteria(context, &goal.success_criteria)
                    .await?
                {
                    return Ok(true);
                }
            }
        }

        // 2) Heuristic: if recent file write succeeded and is non-empty
        if let Some(obs) = context.get_latest_observation() {
            if obs.content.to_lowercase().contains("successfully wrote to") {
                // Extract path and verify non-empty
                if let Some(path) = obs
                    .content
                    .split_whitespace()
                    .last()
                    .map(|s| s.trim_matches('\"'))
                {
                    if self.non_empty_file_exists(path).await? {
                        return Ok(true);
                    }
                }
            }
        }

        // 3) Fall back to reasoning-provided confidence
        Ok(reasoning.goal_achieved_confidence > 0.8)
    }

    /// Evaluate simple, common success criteria patterns
    async fn check_success_criteria(
        &self,
        context: &ExecutionContext,
        criteria: &[String],
    ) -> Result<bool> {
        for crit in criteria {
            if let Some(rest) = crit.strip_prefix("file_exists:") {
                if !Path::new(rest.trim()).exists() {
                    return Ok(false);
                }
            } else if let Some(rest) = crit.strip_prefix("non_empty_file:") {
                if !self.non_empty_file_exists(rest.trim()).await? {
                    return Ok(false);
                }
            } else if let Some(substr) = crit.strip_prefix("observation_contains:") {
                let substr = substr.trim().to_lowercase();
                let found = context
                    .get_recent_actions()
                    .iter()
                    .any(|e| e.description.to_lowercase().contains(&substr))
                    || context
                        .observations
                        .iter()
                        .rev()
                        .take(10)
                        .any(|o| o.content.to_lowercase().contains(&substr));
                if !found {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    async fn non_empty_file_exists(&self, path: &str) -> Result<bool> {
        if !Path::new(path).exists() {
            return Ok(false);
        }
        if let Ok(meta) = fs::metadata(path).await {
            return Ok(meta.len() > 0);
        }
        Ok(false)
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
                adjustment.adjustment_type, adjustment.description, adjustment.expected_impact
            );

            context.add_strategy_adjustment(vec![adjustment_description]);

            // Log the adjustment
            tracing::info!(
                "Applied strategy adjustment: {} - {}",
                adjustment.adjustment_id,
                adjustment.description
            );

            // Create checkpoint after applying adjustment
            self.persistent_state_manager
                .create_checkpoint(
                    CheckpointType::AfterAction,
                    format!(
                        "After applying strategy adjustment: {}",
                        adjustment.adjustment_id
                    ),
                )
                .await?;
        }

        Ok(())
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
            reasoning_type: ReasoningType::GoalAnalysis,
            input_context: "context summary".to_string(),
            reasoning_output: reasoning.reasoning_output.clone(),
            confidence_score: reasoning.confidence_score,
            next_action_plan: reasoning.next_actions.first().cloned(),
        };

        // DEADLOCK PREVENTION: Acquire locks in consistent order with timeout
        let mut state = timeout(LOCK_TIMEOUT, self.state_manager.current_state.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring state lock in record_reasoning_step"))?;
        let mut metrics = timeout(LOCK_TIMEOUT, self.metrics.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring metrics lock in record_reasoning_step"))?;
        let mut perf = timeout(LOCK_TIMEOUT, self.performance_metrics.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring performance_metrics lock in record_reasoning_step"))?;

        state.reasoning_history.push(step);
        metrics.total_reasoning_steps += 1;
        metrics.average_reasoning_time = (metrics.average_reasoning_time
            * (metrics.total_reasoning_steps - 1) as f64
            + duration.as_millis() as f64)
            / metrics.total_reasoning_steps as f64;

        perf.execution_metrics.total_execution_time += duration;
        perf.execution_metrics.tasks_completed += 1;
        perf.execution_metrics.success_rate = (perf.execution_metrics.tasks_completed as f64)
            / (perf.execution_metrics.tasks_completed + perf.execution_metrics.tasks_failed) as f64;

        Ok(())
    }

    /// Record an action step for analysis and debugging
    async fn record_action_step(
        &self,
        action: SimpleActionResult,
        duration: Duration,
    ) -> Result<()> {
        let step = ActionStep {
            action_id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            action_type: ActionType::ToolExecution,
            parameters: HashMap::new(),
            execution_result: Some(DetailedActionResult {
                action_id: uuid::Uuid::new_v4().to_string(),
                action_type: ActionType::ToolExecution,
                parameters: HashMap::new(),
                result: serde_json::Value::Null,
                execution_time: Duration::from_secs(0),
                success: action.success,
                output: action.output.clone(),
                error: action.error.clone(),
                metadata: action.metadata.clone(),
                side_effects: Vec::new(),
                verification: None,
            }),
            duration: Some(duration),
        };

        let mut state = timeout(LOCK_TIMEOUT, self.state_manager.current_state.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring state lock in record_action_step"))?;
        let mut metrics = timeout(LOCK_TIMEOUT, self.metrics.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring metrics lock in record_action_step"))?;
        let mut perf = timeout(LOCK_TIMEOUT, self.performance_metrics.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring performance_metrics lock in record_action_step"))?;

        state.last_action = Some(step);
        metrics.total_actions_taken += 1;
        metrics.average_action_time = (metrics.average_action_time
            * (metrics.total_actions_taken - 1) as f64
            + duration.as_millis() as f64)
            / metrics.total_actions_taken as f64;

        perf.execution_metrics.total_execution_time += duration;
        if action.success {
            perf.execution_metrics.tasks_completed += 1;
        } else {
            perf.execution_metrics.tasks_failed += 1;
        }
        perf.execution_metrics.success_rate = (perf.execution_metrics.tasks_completed as f64)
            / (perf.execution_metrics.tasks_completed + perf.execution_metrics.tasks_failed) as f64;

        Ok(())
    }

    /// Record an observation for analysis and learning
    async fn record_observation(&self, observation: Observation) -> Result<()> {
        let mut state = timeout(LOCK_TIMEOUT, self.state_manager.current_state.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring state lock in record_observation"))?;
        let mut metrics = timeout(LOCK_TIMEOUT, self.metrics.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring metrics lock in record_observation"))?;
        let mut perf = timeout(LOCK_TIMEOUT, self.performance_metrics.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring performance_metrics lock in record_observation"))?;

        state.observations.push(observation.clone());
        metrics.total_observations_made += 1;

        if observation.content.to_lowercase().contains("error") {
            perf.reliability_metrics.error_recovery_rate *= 0.95;
        }

        Ok(())
    }

    /// Update the current agent state
    async fn update_state(&self, context: &ExecutionContext, iteration_count: u32) -> Result<()> {
        let mut state = timeout(LOCK_TIMEOUT, self.state_manager.current_state.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring state lock in update_state"))?;
        state.current_context = context.clone();
        state.iteration_count = iteration_count;
        state.last_update = SystemTime::now();

        let mut perf = timeout(LOCK_TIMEOUT, self.performance_metrics.write())
            .await
            .map_err(|_| anyhow!("Timeout acquiring performance_metrics lock in update_state"))?;
        perf.execution_metrics.queue_length = context.active_tasks.len() as u32;
        perf.execution_metrics.active_tasks = context.active_tasks.len() as u32;

        Ok(())
    }

    /// Finalize goal execution and generate result
    async fn finalize_goal_execution(
        &self,
        context: &ExecutionContext,
        success: bool,
    ) -> Result<GoalResult> {
        let state = timeout(LOCK_TIMEOUT, self.state_manager.current_state.read())
            .await
            .map_err(|_| anyhow!("Timeout acquiring state lock in finalize_goal_execution"))?;

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
        match timeout(LOCK_TIMEOUT, self.metrics.write()).await {
            Ok(mut metrics) => {
                metrics.successful_goals += 1;
                metrics.average_goal_completion_time = (metrics.average_goal_completion_time
                    * (metrics.successful_goals - 1) as f64
                    + duration.as_millis() as f64)
                    / metrics.successful_goals as f64;
                metrics.success_rate =
                    metrics.successful_goals as f64 / metrics.total_goals_processed as f64;
            }
            Err(_) => {
                tracing::warn!("Timeout acquiring metrics lock in update_success_metrics - metrics may be stale");
            }
        }
    }

    /// Get current orchestration metrics
    pub async fn get_metrics(&self) -> OrchestrationMetrics {
        match timeout(LOCK_TIMEOUT, self.metrics.read()).await {
            Ok(metrics) => metrics.clone(),
            Err(_) => {
                tracing::warn!("Timeout acquiring metrics lock in get_metrics - returning default");
                OrchestrationMetrics::default()
            }
        }
    }

    /// Get current agent state
    pub async fn get_current_state(&self) -> AgentState {
        match timeout(LOCK_TIMEOUT, self.state_manager.current_state.read()).await {
            Ok(state) => state.clone(),
            Err(_) => {
                tracing::warn!("Timeout acquiring state lock in get_current_state - returning default");
                AgentState::default()
            }
        }
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
        let context = self
            .persistent_state_manager
            .load_context(context_id)
            .await?;
        self.persistent_state_manager.set_context(context).await
    }

    /// Apply guardrail based on supervisor assessment
    pub async fn apply_guardrail(
        &self,
        stage: SupervisorStage,
        context: &str,
        assessment: &RiskAssessment,
    ) -> Result<()> {
        match assessment.recommended_action {
            GuardrailDecision::Allow => {
                // Continue execution
            }
            GuardrailDecision::Review => {
                // Log warning but continue
                println!("Review required at stage {:?}: {}", stage, context);
            }
            GuardrailDecision::Mitigate => {
                // Log warning but continue
                println!("Mitigation required at stage {:?}: {}", stage, context);
            }
            GuardrailDecision::Block => {
                return Err(anyhow::anyhow!(
                    "Execution blocked by guardrail at stage {:?}: {}",
                    stage,
                    context
                ));
            }
            GuardrailDecision::Escalate => {
                return Err(anyhow::anyhow!(
                    "Escalation required at stage {:?}: {}",
                    stage,
                    context
                ));
            }
        }
        Ok(())
    }
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            current_state: tokio::sync::RwLock::new(AgentState::default()),
            state_history: tokio::sync::RwLock::new(Vec::new()),
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

    #[test]
    fn test_convergence_tracker_no_convergence() {
        let mut tracker = ConvergenceTracker::new();
        // Different inputs should not trigger convergence
        assert!(!tracker.record_reasoning("This is the first unique reasoning output"));
        assert!(!tracker.record_reasoning("A completely different second output"));
        assert!(!tracker.record_reasoning("Yet another unique third output"));
    }

    #[test]
    fn test_convergence_tracker_detects_convergence() {
        let mut tracker = ConvergenceTracker::new();
        // Similar inputs should trigger convergence after threshold
        assert!(!tracker.record_reasoning("The agent should write a file to disk"));
        assert!(!tracker.record_reasoning("The agent should write a file to disk now"));
        assert!(!tracker.record_reasoning("The agent should write a file to disk please"));
        // Third similar output triggers convergence (threshold is 3)
        assert!(tracker.record_reasoning("The agent should write a file to disk again"));
    }

    #[test]
    fn test_convergence_similarity_function() {
        // Identical strings
        assert!((ConvergenceTracker::similarity("hello world", "hello world") - 1.0).abs() < 0.01);
        // Similar strings
        let sim = ConvergenceTracker::similarity("the quick brown fox", "the quick brown dog");
        assert!(sim > 0.5 && sim < 1.0);
        // Completely different strings
        let sim = ConvergenceTracker::similarity("hello", "goodbye world");
        assert!(sim < 0.5);
        // Empty strings
        assert!((ConvergenceTracker::similarity("", "") - 1.0).abs() < 0.01);
    }
}

/// Mock reasoning engine for testing and basic functionality
struct MockReasoningEngine;

#[async_trait::async_trait]
impl ReasoningEngine for MockReasoningEngine {
    async fn reason(&self, prompt: &str, _context: &ExecutionContext) -> Result<String> {
        Ok(format!(
            "Mock reasoning response for: {}",
            prompt.chars().take(50).collect::<String>()
        ))
    }

    async fn get_capabilities(&self) -> Vec<ReasoningCapability> {
        vec![
            ReasoningCapability::GoalDecomposition,
            ReasoningCapability::TaskPlanning,
        ]
    }

    async fn get_confidence(&self) -> f64 {
        0.8
    }
}

/// Mock engine for testing and configuration fallback
struct MockEngine;

impl fluent_core::traits::Engine for MockEngine {
    fn execute<'a>(
        &'a self,
        _request: &'a fluent_core::types::Request,
    ) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::Response>> + Send + 'a>
    {
        Box::new(async move {
            Ok(fluent_core::types::Response {
                content: "Mock engine response".to_string(),
                usage: fluent_core::types::Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                model: "mock-model".to_string(),
                finish_reason: Some("stop".to_string()),
                cost: fluent_core::types::Cost {
                    prompt_cost: 0.001,
                    completion_cost: 0.002,
                    total_cost: 0.003,
                },
            })
        })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a fluent_core::types::UpsertRequest,
    ) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::UpsertResponse>> + Send + 'a>
    {
        Box::new(async move {
            Ok(fluent_core::types::UpsertResponse {
                processed_files: vec!["mock-file".to_string()],
                errors: Vec::new(),
            })
        })
    }

    fn get_neo4j_client(&self) -> Option<&std::sync::Arc<fluent_core::neo4j_client::Neo4jClient>> {
        None
    }

    fn get_session_id(&self) -> Option<String> {
        None
    }

    fn extract_content(
        &self,
        _value: &serde_json::Value,
    ) -> Option<fluent_core::types::ExtractedContent> {
        None
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a std::path::Path,
    ) -> Box<dyn std::future::Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move { Ok("mock-file-id".to_string()) })
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a fluent_core::types::Request,
        _file_path: &'a std::path::Path,
    ) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::Response>> + Send + 'a>
    {
        self.execute(request)
    }
}
