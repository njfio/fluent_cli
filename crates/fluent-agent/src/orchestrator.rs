//! Agent orchestration implementing the ReAct (Reasoning, Acting, Observing) pattern.
//!
//! This module contains the core [`AgentOrchestrator`] that coordinates all agent
//! activities including goal decomposition, task execution, and state management.
//!
//! # Architecture
//!
//! The orchestrator follows the ReAct pattern:
//!
//! 1. **Reasoning**: Analyze current state, plan next actions via the reasoning engine
//! 2. **Acting**: Execute planned actions through tools (file ops, shell, etc.)
//! 3. **Observing**: Process action results, update context and memory
//!
//! # Components
//!
//! - **ReasoningEngine**: Multi-modal reasoning with chain-of-thought
//! - **ActionPlanner/Executor**: Convert reasoning to concrete tool calls
//! - **ObservationProcessor**: Extract insights from action results
//! - **MemorySystem**: Short-term working memory and long-term persistence
//! - **ReflectionEngine**: Self-evaluation and strategy adjustment
//!
//! # Usage
//!
//! ```rust,ignore
//! use fluent_agent::orchestrator::AgentOrchestrator;
//!
//! let orchestrator = AgentOrchestrator::new(config).await?;
//! let result = orchestrator.execute_goal(goal).await?;
//! ```

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

/// Maximum number of reasoning steps to retain in history
const MAX_REASONING_HISTORY_SIZE: usize = 500;

/// Maximum number of observations to retain in history
const MAX_OBSERVATIONS_SIZE: usize = 1000;

/// Maximum number of completed tasks to retain
const MAX_COMPLETED_TASKS_SIZE: usize = 200;
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
use crate::reasoning::{ReasoningCapability, ReasoningEngine, StructuredReasoningOutput};
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

/// Signals collected for multi-signal goal achievement detection
#[derive(Debug, Clone, Default)]
struct GoalAchievementSignals {
    /// Confidence from reasoning engine (0.0-1.0)
    reasoning_confidence: f64,
    /// Assessment from structured reasoning output (0.0-1.0)
    structured_assessment: f64,
    /// Evidence from file creation/modification (0.0-1.0)
    file_evidence: f64,
    /// Success patterns in command execution (0.0-1.0)
    execution_success: f64,
    /// Progress trend over iterations (0.0-1.0)
    progress_trend: f64,
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
        if self
            .recent_reasoning
            .iter()
            .any(|prev| Self::similarity(prev, &normalized) >= SIMILARITY_THRESHOLD)
        {
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
    #[allow(clippy::too_many_arguments)]
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
    #[allow(clippy::too_many_arguments)]
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

        // Tool metadata from the caller (CLI)
        let tool_descriptions_markdown = goal
            .metadata
            .get("tool_descriptions_markdown")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(tools) = goal
            .metadata
            .get("available_tools")
            .and_then(|v| v.as_array())
        {
            context.available_tools = tools
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(project_id) = goal.metadata.get("project_id").and_then(|v| v.as_str()) {
            context.set_variable("project_id".to_string(), project_id.to_string());
        }

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
                // Build the full prompt (system + user prompt) so the model has
                // consistent ReAct instructions and an up-to-date tool list.
                let tools_md = tool_descriptions_markdown.clone().unwrap_or_else(|| {
                    if context.available_tools.is_empty() {
                        "(no tools available)".to_string()
                    } else {
                        context.available_tools.join("\n")
                    }
                });

                let recent_observations: Vec<String> = context
                    .observations
                    .iter()
                    .rev()
                    .take(5)
                    .map(|o| o.content.clone())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();

                let user_prompt = crate::prompts::format_reasoning_prompt(
                    &goal.description,
                    iteration_count,
                    max_iterations,
                    &recent_observations,
                    &tools_md,
                );
                let full_prompt = format!(
                    "{}\n\n---\n\n{}",
                    crate::prompts::AGENT_SYSTEM_PROMPT,
                    user_prompt
                );

                let mut last_error = None;
                let mut reasoning_result = None;

                for attempt in 0..MAX_REASONING_RETRIES {
                    match self.reasoning_engine.reason(&full_prompt, &context).await {
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
                        last_error
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "Unknown error".to_string())
                    )
                })?
            };

            // Parse raw output into structured format with schema validation
            let structured_output = StructuredReasoningOutput::from_raw_output(&reasoning_output);

            // Log structured output details for debugging
            tracing::debug!(
                "react.structured_reasoning summary='{}' thoughts={} actions={} progress={:.1}% achieved={}",
                structured_output.summary.chars().take(50).collect::<String>(),
                structured_output.reasoning_chain.len(),
                structured_output.proposed_actions.len(),
                structured_output.goal_assessment.progress_percentage * 100.0,
                structured_output.goal_assessment.is_achieved
            );

            // Convert to legacy ReasoningResult for compatibility
            // TODO: Eventually migrate fully to StructuredReasoningOutput
            let reasoning_result = ReasoningResult {
                reasoning_output: reasoning_output.clone(),
                confidence_score: structured_output.confidence,
                goal_achieved_confidence: structured_output.goal_assessment.achievement_confidence,
                next_actions: structured_output
                    .proposed_actions
                    .iter()
                    .map(|a| a.description.clone())
                    .collect(),
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

    /// Check if the goal has been achieved using multi-signal detection
    ///
    /// This uses a weighted scoring system combining multiple signals:
    /// 1. Explicit success criteria (if defined)
    /// 2. Structured reasoning assessment
    /// 3. File creation/modification evidence
    /// 4. Command execution success patterns
    /// 5. Observation history analysis
    async fn is_goal_achieved(
        &self,
        context: &ExecutionContext,
        reasoning: &ReasoningResult,
    ) -> Result<bool> {
        // 1) Check explicit success criteria on the goal if provided (highest priority)
        if let Some(goal) = context.get_current_goal() {
            if !goal.success_criteria.is_empty()
                && self
                    .check_success_criteria(context, &goal.success_criteria)
                    .await?
            {
                tracing::info!("react.goal_check explicit_criteria=passed");
                return Ok(true);
            }
        }

        // 2) Multi-signal weighted scoring
        let signals = self.collect_achievement_signals(context, reasoning).await;
        let weighted_score = self.calculate_weighted_achievement_score(&signals);

        tracing::debug!(
            "react.goal_check signals={:?} weighted_score={:.2}",
            signals,
            weighted_score
        );

        // Require high confidence from multiple signals
        Ok(weighted_score >= 0.75)
    }

    /// Collect all signals that indicate goal achievement
    async fn collect_achievement_signals(
        &self,
        context: &ExecutionContext,
        reasoning: &ReasoningResult,
    ) -> GoalAchievementSignals {
        let mut signals = GoalAchievementSignals {
            reasoning_confidence: reasoning.goal_achieved_confidence,
            ..Default::default()
        };

        // Signal 2: Parse structured output for assessment
        let structured = StructuredReasoningOutput::from_raw_output(&reasoning.reasoning_output);
        signals.structured_assessment = if structured.goal_assessment.is_achieved {
            structured.goal_assessment.achievement_confidence
        } else {
            structured.goal_assessment.progress_percentage * 0.5
        };

        // Signal 3: Recent file write success
        if let Some(obs) = context.get_latest_observation() {
            if obs.content.to_lowercase().contains("successfully wrote to")
                || obs.content.to_lowercase().contains("file created")
                || obs.content.to_lowercase().contains("saved to")
            {
                // Extract path and verify
                if let Some(path) = obs
                    .content
                    .split_whitespace()
                    .find(|s| s.contains('/') || s.contains('.'))
                    .map(|s| s.trim_matches(|c| c == '\"' || c == '\'' || c == '`'))
                {
                    if self.non_empty_file_exists(path).await.unwrap_or(false) {
                        signals.file_evidence = 1.0;
                    } else {
                        signals.file_evidence = 0.3; // Mentioned but not verified
                    }
                }
            }
        }

        // Signal 4: Command execution success patterns
        let recent_observations: Vec<_> = context.observations.iter().rev().take(5).collect();
        let success_patterns = [
            "successfully",
            "completed",
            "done",
            "finished",
            "created",
            "generated",
            "built",
            "compiled",
        ];
        let failure_patterns = ["error", "failed", "cannot", "unable", "exception", "panic"];

        let mut success_count = 0;
        let mut failure_count = 0;
        for obs in &recent_observations {
            let lower = obs.content.to_lowercase();
            for pattern in &success_patterns {
                if lower.contains(pattern) {
                    success_count += 1;
                    break;
                }
            }
            for pattern in &failure_patterns {
                if lower.contains(pattern) {
                    failure_count += 1;
                    break;
                }
            }
        }

        if success_count > 0 && failure_count == 0 {
            signals.execution_success =
                (success_count as f64 / recent_observations.len() as f64).min(1.0);
        } else if failure_count > success_count {
            signals.execution_success = 0.0;
        } else {
            signals.execution_success = 0.3;
        }

        // Signal 5: Progress trend (are we making progress?)
        let iteration = context.iteration_count();
        if iteration > 1 {
            // Simple heuristic: if we're on later iterations with high confidence, likely done
            signals.progress_trend = if iteration > 3 && signals.reasoning_confidence > 0.7 {
                0.8
            } else {
                0.5
            };
        }

        signals
    }

    /// Calculate weighted achievement score from multiple signals
    fn calculate_weighted_achievement_score(&self, signals: &GoalAchievementSignals) -> f64 {
        // Weights for each signal (must sum to 1.0)
        const REASONING_WEIGHT: f64 = 0.25;
        const STRUCTURED_WEIGHT: f64 = 0.25;
        const FILE_WEIGHT: f64 = 0.20;
        const EXECUTION_WEIGHT: f64 = 0.20;
        const PROGRESS_WEIGHT: f64 = 0.10;

        let score = signals.reasoning_confidence * REASONING_WEIGHT
            + signals.structured_assessment * STRUCTURED_WEIGHT
            + signals.file_evidence * FILE_WEIGHT
            + signals.execution_success * EXECUTION_WEIGHT
            + signals.progress_trend * PROGRESS_WEIGHT;

        // Bonus: if multiple strong signals agree, boost confidence
        let strong_signals = [
            signals.reasoning_confidence > 0.8,
            signals.structured_assessment > 0.8,
            signals.file_evidence > 0.8,
            signals.execution_success > 0.8,
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        if strong_signals >= 3 {
            (score * 1.1).min(1.0) // 10% bonus for agreement
        } else {
            score
        }
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
            .map_err(|_| {
                anyhow!("Timeout acquiring performance_metrics lock in record_reasoning_step")
            })?;

        state.reasoning_history.push(step);
        // Enforce memory bounds: keep most recent entries, evict oldest
        if state.reasoning_history.len() > MAX_REASONING_HISTORY_SIZE {
            let drain_count = state.reasoning_history.len() - MAX_REASONING_HISTORY_SIZE;
            state.reasoning_history.drain(0..drain_count);
        }
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
            .map_err(|_| {
                anyhow!("Timeout acquiring performance_metrics lock in record_action_step")
            })?;

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
            .map_err(|_| {
                anyhow!("Timeout acquiring performance_metrics lock in record_observation")
            })?;

        state.observations.push(observation.clone());
        // Enforce memory bounds: keep most recent entries, evict oldest
        if state.observations.len() > MAX_OBSERVATIONS_SIZE {
            let drain_count = state.observations.len() - MAX_OBSERVATIONS_SIZE;
            state.observations.drain(0..drain_count);
        }
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
                tracing::warn!(
                    "Timeout acquiring state lock in get_current_state - returning default"
                );
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

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
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

    #[test]
    fn test_goal_achievement_signals_default() {
        let signals = GoalAchievementSignals::default();
        assert_eq!(signals.reasoning_confidence, 0.0);
        assert_eq!(signals.structured_assessment, 0.0);
        assert_eq!(signals.file_evidence, 0.0);
        assert_eq!(signals.execution_success, 0.0);
        assert_eq!(signals.progress_trend, 0.0);
    }

    #[test]
    fn test_weighted_score_all_high() {
        let signals = GoalAchievementSignals {
            reasoning_confidence: 0.9,
            structured_assessment: 0.9,
            file_evidence: 0.9,
            execution_success: 0.9,
            progress_trend: 0.8,
        };

        // With 4 strong signals (>0.8), should get 10% bonus
        // Base: 0.9*0.25 + 0.9*0.25 + 0.9*0.20 + 0.9*0.20 + 0.8*0.10 = 0.89
        // With bonus: 0.89 * 1.1 = 0.979
        let score = calculate_weighted_score_test(&signals);
        assert!(score > 0.95, "Score should be > 0.95, got {}", score);
    }

    #[test]
    fn test_weighted_score_mixed_signals() {
        let signals = GoalAchievementSignals {
            reasoning_confidence: 0.9,
            structured_assessment: 0.7,
            file_evidence: 0.0,
            execution_success: 0.5,
            progress_trend: 0.5,
        };

        // Base: 0.9*0.25 + 0.7*0.25 + 0.0*0.20 + 0.5*0.20 + 0.5*0.10 = 0.55
        // Only 1 strong signal, no bonus
        let score = calculate_weighted_score_test(&signals);
        assert!(
            score > 0.5 && score < 0.7,
            "Score should be ~0.55, got {}",
            score
        );
    }

    #[test]
    fn test_weighted_score_all_low() {
        let signals = GoalAchievementSignals {
            reasoning_confidence: 0.1,
            structured_assessment: 0.2,
            file_evidence: 0.0,
            execution_success: 0.1,
            progress_trend: 0.0,
        };

        let score = calculate_weighted_score_test(&signals);
        assert!(score < 0.2, "Score should be < 0.2, got {}", score);
    }

    /// Helper function for testing weighted score calculation
    /// (duplicates the logic from AgentOrchestrator::calculate_weighted_achievement_score)
    fn calculate_weighted_score_test(signals: &GoalAchievementSignals) -> f64 {
        const REASONING_WEIGHT: f64 = 0.25;
        const STRUCTURED_WEIGHT: f64 = 0.25;
        const FILE_WEIGHT: f64 = 0.20;
        const EXECUTION_WEIGHT: f64 = 0.20;
        const PROGRESS_WEIGHT: f64 = 0.10;

        let score = signals.reasoning_confidence * REASONING_WEIGHT
            + signals.structured_assessment * STRUCTURED_WEIGHT
            + signals.file_evidence * FILE_WEIGHT
            + signals.execution_success * EXECUTION_WEIGHT
            + signals.progress_trend * PROGRESS_WEIGHT;

        let strong_signals = [
            signals.reasoning_confidence > 0.8,
            signals.structured_assessment > 0.8,
            signals.file_evidence > 0.8,
            signals.execution_success > 0.8,
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        if strong_signals >= 3 {
            (score * 1.1).min(1.0)
        } else {
            score
        }
    }

    // ==================== Memory Bounds Tests ====================

    #[test]
    fn test_memory_bounds_constants() {
        // Verify reasonable bounds are set
        assert!(MAX_REASONING_HISTORY_SIZE > 0);
        assert!(MAX_OBSERVATIONS_SIZE > 0);
        assert!(MAX_COMPLETED_TASKS_SIZE > 0);
        // Ensure observations > reasoning since observations are more frequent
        assert!(MAX_OBSERVATIONS_SIZE >= MAX_REASONING_HISTORY_SIZE);
    }

    #[test]
    fn test_agent_state_vector_initialization() {
        let state = AgentState::default();
        // Vectors should start empty
        assert!(state.reasoning_history.is_empty());
        assert!(state.observations.is_empty());
        assert!(state.completed_tasks.is_empty());
    }

    #[test]
    fn test_reasoning_history_bounded_simulation() {
        // Simulate the bounds check logic
        let mut history: Vec<ReasoningStep> = Vec::new();

        // Add more than max items
        for i in 0..MAX_REASONING_HISTORY_SIZE + 100 {
            history.push(ReasoningStep {
                step_id: format!("step-{}", i),
                timestamp: SystemTime::now(),
                reasoning_type: ReasoningType::GoalAnalysis,
                input_context: "test".to_string(),
                reasoning_output: format!("output-{}", i),
                confidence_score: 0.8,
                next_action_plan: None,
            });

            // Apply bounds check (same logic as record_reasoning_step)
            if history.len() > MAX_REASONING_HISTORY_SIZE {
                let drain_count = history.len() - MAX_REASONING_HISTORY_SIZE;
                history.drain(0..drain_count);
            }
        }

        // Should be bounded
        assert_eq!(history.len(), MAX_REASONING_HISTORY_SIZE);
        // Most recent should be preserved
        assert!(history
            .last()
            .unwrap()
            .step_id
            .contains(&(MAX_REASONING_HISTORY_SIZE + 99).to_string()));
    }

    #[test]
    fn test_observations_bounded_simulation() {
        // Simulate the bounds check logic
        let mut observations: Vec<Observation> = Vec::new();

        // Add more than max items
        for i in 0..MAX_OBSERVATIONS_SIZE + 50 {
            observations.push(Observation {
                observation_id: format!("obs-{}", i),
                timestamp: SystemTime::now(),
                observation_type: ObservationType::ProgressUpdate,
                content: format!("content-{}", i),
                source: "test".to_string(),
                relevance_score: 0.5,
                impact_assessment: None,
            });

            // Apply bounds check (same logic as record_observation)
            if observations.len() > MAX_OBSERVATIONS_SIZE {
                let drain_count = observations.len() - MAX_OBSERVATIONS_SIZE;
                observations.drain(0..drain_count);
            }
        }

        // Should be bounded
        assert_eq!(observations.len(), MAX_OBSERVATIONS_SIZE);
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

// ============================================================================
// ExecutionLoop Implementation for AgentOrchestrator
// ============================================================================

use crate::execution::{ExecutionLoop, ExecutionState, ExecutionStatus, StepResult};

/// Adapter to run AgentOrchestrator through the unified ExecutionLoop interface
///
/// This adapter wraps an AgentOrchestrator and exposes its ReAct loop as
/// discrete steps that can be controlled by the UniversalExecutor.
pub struct OrchestratorExecutionAdapter {
    /// The underlying orchestrator (owned for step execution)
    orchestrator: AgentOrchestrator,
    /// Goal being executed
    goal: Goal,
    /// Unified execution state for the ExecutionLoop interface
    execution_state: ExecutionState,
    /// Execution context for this run
    context: ExecutionContext,
    /// Convergence tracker to detect stuck loops
    convergence_tracker: ConvergenceTracker,
    /// Last reasoning result for completion checking
    last_reasoning: Option<ReasoningResult>,
    /// Whether initialization has been called
    initialized: bool,
    /// Last error encountered (for retry logic)
    last_error: Option<String>,
    /// Start time for elapsed tracking
    start_time: std::time::Instant,
}

impl OrchestratorExecutionAdapter {
    /// Create a new adapter for running an orchestrator with a goal
    pub fn new(orchestrator: AgentOrchestrator, goal: Goal) -> Self {
        let max_iterations = goal.max_iterations.unwrap_or(50);
        Self {
            orchestrator,
            goal: goal.clone(),
            execution_state: ExecutionState::new(Some(max_iterations)),
            context: ExecutionContext::new(goal),
            convergence_tracker: ConvergenceTracker::new(),
            last_reasoning: None,
            initialized: false,
            last_error: None,
            start_time: std::time::Instant::now(),
        }
    }

    /// Get the final goal result after execution completes
    pub async fn get_result(&self) -> Result<GoalResult> {
        let success = matches!(self.execution_state.status, ExecutionStatus::Completed);
        self.orchestrator
            .finalize_goal_execution(&self.context, success)
            .await
    }

    /// List all available checkpoints for this execution
    pub fn list_checkpoints(&self) -> Vec<CheckpointInfo> {
        self.context
            .checkpoints
            .iter()
            .map(|cp| CheckpointInfo {
                checkpoint_id: format!(
                    "orchestrator-{}-{}",
                    self.context.context_id, cp.iteration_count
                ),
                context_id: self.context.context_id.clone(),
                checkpoint_type: format!("{:?}", cp.checkpoint_type),
                description: cp.description.clone(),
                created_at: cp.timestamp,
                iteration: cp.iteration_count,
            })
            .collect()
    }

    /// Get recovery information for the current execution
    pub async fn get_recovery_info(&self) -> Result<RecoveryInfo> {
        let state_recovery = self
            .orchestrator
            .persistent_state_manager
            .get_recovery_info(&self.context.context_id)
            .await?;

        let latest_checkpoint = self.context.checkpoints.last().map(|cp| CheckpointInfo {
            checkpoint_id: format!(
                "orchestrator-{}-{}",
                self.context.context_id, cp.iteration_count
            ),
            context_id: self.context.context_id.clone(),
            checkpoint_type: format!("{:?}", cp.checkpoint_type),
            description: cp.description.clone(),
            created_at: cp.timestamp,
            iteration: cp.iteration_count,
        });

        Ok(RecoveryInfo {
            context_id: self.context.context_id.clone(),
            current_iteration: self.execution_state.iteration,
            checkpoint_count: self.context.checkpoints.len(),
            latest_checkpoint,
            recovery_possible: state_recovery.recovery_possible,
            corruption_detected: state_recovery.corruption_detected,
            last_saved: state_recovery.last_saved,
        })
    }

    /// Create a named checkpoint for manual recovery points
    pub async fn create_named_checkpoint(&mut self, name: &str) -> Result<String> {
        let checkpoint_id = self
            .orchestrator
            .persistent_state_manager
            .create_checkpoint(
                CheckpointType::Manual,
                format!("{} at iteration {}", name, self.execution_state.iteration),
            )
            .await?;

        Ok(format!(
            "orchestrator-{}-{}",
            self.context.context_id, self.execution_state.iteration
        ))
    }

    /// Resume from the most recent checkpoint
    pub async fn resume_from_latest(&mut self) -> Result<()> {
        let latest = self
            .context
            .checkpoints
            .last()
            .ok_or_else(|| anyhow!("No checkpoints available to resume from"))?;

        let checkpoint_id = format!(
            "orchestrator-{}-{}",
            self.context.context_id, self.context.iteration_count
        );

        self.restore_checkpoint(&checkpoint_id).await
    }

    /// Check if recovery is possible from a previous state
    pub async fn can_recover(&self) -> bool {
        match self
            .orchestrator
            .persistent_state_manager
            .get_recovery_info(&self.context.context_id)
            .await
        {
            Ok(info) => info.recovery_possible && !info.corruption_detected,
            Err(_) => false,
        }
    }
}

/// Information about a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInfo {
    pub checkpoint_id: String,
    pub context_id: String,
    pub checkpoint_type: String,
    pub description: String,
    pub created_at: std::time::SystemTime,
    pub iteration: u32,
}

/// Information about recovery state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInfo {
    pub context_id: String,
    pub current_iteration: u32,
    pub checkpoint_count: usize,
    pub latest_checkpoint: Option<CheckpointInfo>,
    pub recovery_possible: bool,
    pub corruption_detected: bool,
    pub last_saved: std::time::SystemTime,
}

#[async_trait::async_trait]
impl ExecutionLoop for OrchestratorExecutionAdapter {
    type State = ExecutionState;

    async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Initialize orchestrator state
        self.orchestrator
            .initialize_state(self.goal.clone(), &self.context)
            .await?;

        // Set context in persistent state manager
        self.orchestrator
            .persistent_state_manager
            .set_context(self.context.clone())
            .await?;

        // Create initial checkpoint
        self.orchestrator
            .persistent_state_manager
            .create_checkpoint(
                CheckpointType::BeforeAction,
                "Goal execution started via ExecutionLoop".to_string(),
            )
            .await?;

        // Update metrics
        {
            let mut metrics = timeout(LOCK_TIMEOUT, self.orchestrator.metrics.write())
                .await
                .map_err(|_| anyhow!("Timeout acquiring metrics lock in initialize"))?;
            metrics.total_goals_processed += 1;
        }

        self.execution_state.status = ExecutionStatus::Running;
        self.initialized = true;

        tracing::info!(
            "execution_loop.orchestrator.init goal='{}' max_iterations={:?}",
            self.goal.description,
            self.execution_state.max_iterations
        );

        Ok(())
    }

    async fn execute_step(&mut self) -> Result<StepResult> {
        let step_start = std::time::Instant::now();
        self.execution_state.next_iteration();
        self.context.increment_iteration();

        let step_id = format!("react-{}", self.execution_state.iteration);
        self.execution_state.current_step = step_id.clone();

        tracing::debug!(
            "execution_loop.step.start iter={} step={}",
            self.execution_state.iteration,
            step_id
        );

        // ====== Reasoning Phase ======
        let reasoning_result = {
            let context_summary = self.context.get_summary();
            let mut last_error = None;
            let mut reasoning_result = None;

            for attempt in 0..MAX_REASONING_RETRIES {
                match self
                    .orchestrator
                    .reasoning_engine
                    .reason(&context_summary, &self.context)
                    .await
                {
                    Ok(output) => {
                        // Parse into ReasoningResult
                        let structured = StructuredReasoningOutput::from_raw_output(&output);
                        reasoning_result = Some(ReasoningResult {
                            reasoning_output: output,
                            confidence_score: structured.confidence,
                            goal_achieved_confidence: structured
                                .goal_assessment
                                .achievement_confidence,
                            next_actions: structured
                                .proposed_actions
                                .iter()
                                .map(|a| a.description.clone())
                                .collect(),
                        });
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "execution_loop.reasoning.retry attempt={}/{} error={}",
                            attempt + 1,
                            MAX_REASONING_RETRIES,
                            e
                        );
                        last_error = Some(e);

                        if attempt + 1 < MAX_REASONING_RETRIES {
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
                    last_error
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "Unknown error".to_string())
                )
            })?
        };

        // Check for convergence
        if self
            .convergence_tracker
            .record_reasoning(&reasoning_result.reasoning_output)
        {
            tracing::warn!(
                "execution_loop.convergence iter={} similar_count={}",
                self.execution_state.iteration,
                CONVERGENCE_THRESHOLD
            );

            self.context.add_context_item(
                "system_warning".to_string(),
                "CONVERGENCE DETECTED: Please try a fundamentally different approach.".to_string(),
            );

            if self.execution_state.iteration
                > self.execution_state.max_iterations.unwrap_or(50) / 2
            {
                return Ok(StepResult::failure(
                    step_id,
                    "Agent stuck in convergence loop",
                    step_start.elapsed(),
                ));
            }
        }

        // Store for completion checking
        self.last_reasoning = Some(reasoning_result.clone());

        // Record reasoning step
        self.orchestrator
            .record_reasoning_step(reasoning_result.clone(), step_start.elapsed())
            .await?;

        // ====== Planning Phase ======
        let action_plan = self
            .orchestrator
            .action_planner
            .plan_action(reasoning_result.clone(), &self.context)
            .await?;

        // Create checkpoint before action
        self.orchestrator
            .persistent_state_manager
            .create_checkpoint(
                CheckpointType::BeforeAction,
                format!(
                    "Before action at iteration {}",
                    self.execution_state.iteration
                ),
            )
            .await?;

        // ====== Execution Phase ======
        let action_start = std::time::Instant::now();
        let action_result = self
            .orchestrator
            .action_executor
            .execute(action_plan, &mut self.context)
            .await?;
        let action_duration = action_start.elapsed();

        // Create checkpoint after action
        self.orchestrator
            .persistent_state_manager
            .create_checkpoint(
                CheckpointType::AfterAction,
                format!(
                    "After action at iteration {}",
                    self.execution_state.iteration
                ),
            )
            .await?;

        // Record action step
        self.orchestrator
            .record_action_step(
                SimpleActionResult {
                    success: action_result.success,
                    output: action_result.output.clone(),
                    error: action_result.error.clone(),
                    metadata: action_result.metadata.clone(),
                },
                action_duration,
            )
            .await?;

        // Track for convergence
        if let Some(ref output) = action_result.output {
            self.convergence_tracker.record_action(output);
        }

        // ====== Observation Phase ======
        let observation = self
            .orchestrator
            .observation_processor
            .process(action_result.clone(), &self.context)
            .await?;

        self.orchestrator
            .record_observation(observation.clone())
            .await?;

        // Apply guardrails if supervisor present
        if let Some(supervisor) = &self.orchestrator.autonomy_supervisor {
            let assessment = supervisor
                .assess_post_action(&action_result, &observation)
                .await?;
            self.orchestrator
                .apply_guardrail(SupervisorStage::PostAction, "post action", &assessment)
                .await?;
        }

        self.context.add_observation(observation.clone());
        self.orchestrator
            .memory_system
            .update_memory(&self.context)
            .await?;

        // Add observation to execution state
        self.execution_state.add_observation(
            format!(
                "[{}] {}",
                observation.observation_type.as_str(),
                observation.content.chars().take(200).collect::<String>()
            ),
            10,
        );

        // Update persistent state
        self.orchestrator
            .persistent_state_manager
            .set_context(self.context.clone())
            .await?;

        // Build step result
        let step_result = if action_result.success {
            StepResult::success(
                step_id,
                action_result.output.unwrap_or_default(),
                step_start.elapsed(),
            )
        } else {
            StepResult::failure(
                step_id,
                action_result
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
                step_start.elapsed(),
            )
        };

        tracing::debug!(
            "execution_loop.step.end iter={} success={} duration_ms={}",
            self.execution_state.iteration,
            step_result.success,
            step_start.elapsed().as_millis()
        );

        Ok(step_result)
    }

    fn current_step_id(&self) -> String {
        self.execution_state.current_step.clone()
    }

    fn should_continue(&self) -> bool {
        // Continue if not at max iterations and not complete
        !self.execution_state.is_max_iterations_exceeded()
            && !matches!(
                self.execution_state.status,
                ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Terminated
            )
    }

    fn is_retryable_error(&self) -> bool {
        self.last_error.is_some()
    }

    fn is_complete(&self) -> Result<bool> {
        // Use the orchestrator's completion logic
        if let Some(ref reasoning) = self.last_reasoning {
            // Check explicit success criteria
            if let Some(goal) = self.context.get_current_goal() {
                if !goal.success_criteria.is_empty() {
                    // Use blocking check - this is called from sync context
                    // For now, use a simple heuristic based on reasoning confidence
                    if reasoning.goal_achieved_confidence >= 0.85 {
                        return Ok(true);
                    }
                }
            }

            // Multi-signal check using confidence
            if reasoning.goal_achieved_confidence >= 0.75 && reasoning.confidence_score >= 0.7 {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn should_terminate(&self) -> Result<bool> {
        // Check for timeout (default 30 minutes)
        let timeout = Duration::from_secs(30 * 60);
        if self.start_time.elapsed() > timeout {
            return Ok(true);
        }

        // Check for max iterations
        if self.execution_state.is_max_iterations_exceeded() {
            return Ok(true);
        }

        Ok(false)
    }

    fn get_state(&self) -> &Self::State {
        &self.execution_state
    }

    fn get_state_mut(&mut self) -> &mut Self::State {
        &mut self.execution_state
    }

    async fn save_checkpoint(&self) -> Result<String> {
        let checkpoint_id = format!(
            "orchestrator-{}-{}",
            self.context.context_id, self.execution_state.iteration
        );

        self.orchestrator
            .persistent_state_manager
            .create_checkpoint(
                CheckpointType::Manual,
                format!(
                    "ExecutionLoop checkpoint at iteration {}",
                    self.execution_state.iteration
                ),
            )
            .await?;

        Ok(checkpoint_id)
    }

    async fn restore_checkpoint(&mut self, id: &str) -> Result<()> {
        // Parse checkpoint ID format: orchestrator-{context_id}-{iteration}
        let parts: Vec<&str> = id.splitn(3, '-').collect();
        if parts.len() < 3 || parts[0] != "orchestrator" {
            return Err(anyhow!("Invalid checkpoint ID format: expected 'orchestrator-<context_id>-<iteration>', got '{}'", id));
        }

        let context_id = parts[1];
        let saved_iteration: u32 = parts[2]
            .parse()
            .map_err(|_| anyhow!("Invalid iteration in checkpoint ID: '{}'", parts[2]))?;

        // Find the checkpoint ID from the context's checkpoints
        // The checkpoint was created at this iteration
        let checkpoint_id = {
            let context = self
                .orchestrator
                .persistent_state_manager
                .load_context(context_id)
                .await?;

            // Find checkpoint created at or near this iteration
            let checkpoint = context
                .checkpoints
                .iter()
                .find(|cp| {
                    cp.description
                        .contains(&format!("iteration {}", saved_iteration))
                        || cp.checkpoint_id.contains(&saved_iteration.to_string())
                })
                .or_else(|| context.checkpoints.last());

            match checkpoint {
                Some(cp) => cp.checkpoint_id.clone(),
                None => {
                    return Err(anyhow!(
                        "No checkpoint found for iteration {}",
                        saved_iteration
                    ))
                }
            }
        };

        // Use StateManager's restore_from_checkpoint for proper restoration
        self.orchestrator
            .persistent_state_manager
            .restore_from_checkpoint(context_id, &checkpoint_id)
            .await?;

        // Load the restored context
        self.context = self
            .orchestrator
            .persistent_state_manager
            .load_context(context_id)
            .await?;

        // Restore execution state from context
        self.execution_state.iteration = self.context.iteration_count;
        self.execution_state.status = ExecutionStatus::Running;
        self.execution_state.current_step = "restored".to_string();

        // Copy recent observations from context
        self.execution_state.recent_observations = self
            .context
            .execution_history
            .iter()
            .rev()
            .take(10)
            .map(|e| format!("{:?}: {}", e.event_type, e.description))
            .collect();

        // Reset adapter state
        self.last_reasoning = None;
        self.last_error = None;
        self.initialized = true;
        self.convergence_tracker = ConvergenceTracker::new();

        tracing::info!(
            "execution_loop.checkpoint.restored checkpoint_id={} iteration={}",
            checkpoint_id,
            saved_iteration
        );

        Ok(())
    }

    fn iteration(&self) -> u32 {
        self.execution_state.iteration
    }

    fn max_iterations(&self) -> Option<u32> {
        self.execution_state.max_iterations
    }

    fn elapsed_time(&self) -> Duration {
        self.start_time.elapsed()
    }

    async fn handle_error(&mut self, error: anyhow::Error) -> Result<()> {
        self.last_error = Some(error.to_string());
        self.execution_state.error_count += 1;

        tracing::warn!(
            "execution_loop.error iter={} error={}",
            self.execution_state.iteration,
            error
        );

        // Create error checkpoint
        self.orchestrator
            .persistent_state_manager
            .create_checkpoint(
                CheckpointType::OnError,
                format!(
                    "Error at iteration {}: {}",
                    self.execution_state.iteration, error
                ),
            )
            .await?;

        Ok(())
    }

    fn reset_error_state(&mut self) {
        self.last_error = None;
    }

    fn get_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "iteration": self.execution_state.iteration,
            "max_iterations": self.execution_state.max_iterations,
            "error_count": self.execution_state.error_count,
            "retry_count": self.execution_state.retry_count,
            "status": format!("{:?}", self.execution_state.status),
            "elapsed_ms": self.start_time.elapsed().as_millis(),
            "goal": self.goal.description,
        })
    }

    fn get_recent_observations(&self, n: usize) -> Vec<String> {
        self.execution_state
            .recent_observations
            .iter()
            .rev()
            .take(n)
            .cloned()
            .collect()
    }
}

/// Helper trait for observation type
impl ObservationType {
    fn as_str(&self) -> &'static str {
        match self {
            ObservationType::ActionResult => "action",
            ObservationType::EnvironmentChange => "env",
            ObservationType::UserFeedback => "user",
            ObservationType::SystemEvent => "system",
            ObservationType::ErrorOccurrence => "error",
            ObservationType::ProgressUpdate => "progress",
        }
    }
}

// ============================================================================
// End ExecutionLoop Implementation
// ============================================================================

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
