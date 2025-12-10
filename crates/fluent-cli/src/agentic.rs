//! Agentic mode operations and autonomous execution
//!
//! This module contains all the functionality for running the fluent_cli
//! in agentic mode, including goal processing, autonomous execution,
//! and MCP integration.

use anyhow::{anyhow, Result};
use fluent_agent::{parse_structured_action, StructuredAction};
use fluent_core::config::Config;
use fluent_core::types::Request;
use std::collections::HashMap;
use std::fs;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::tui::{AgentStatus, TuiManager};

/// Classification of API errors for graceful handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorKind {
    /// Non-recoverable errors (billing, auth) - exit immediately
    NonRecoverable,
    /// Transient errors (network, rate limit) - may retry
    Transient,
    /// Unknown errors - treat as transient
    Unknown,
}

/// Check if an error message indicates a non-recoverable API error
///
/// Non-recoverable errors include:
/// - Billing/credit issues (e.g., "credit balance is too low")
/// - Authentication failures (e.g., "invalid API key", "unauthorized")
/// - Account issues (e.g., "account suspended")
///
/// These errors should cause immediate exit rather than continuing to retry.
pub fn classify_api_error(error_msg: &str) -> ApiErrorKind {
    let lower = error_msg.to_lowercase();

    // Billing/credit issues - non-recoverable
    if lower.contains("credit balance")
        || lower.contains("billing")
        || lower.contains("payment")
        || lower.contains("quota exceeded")
        || lower.contains("insufficient funds")
        || lower.contains("purchase credits")
    {
        return ApiErrorKind::NonRecoverable;
    }

    // Authentication issues - non-recoverable
    if lower.contains("invalid api key")
        || lower.contains("invalid_api_key")
        || lower.contains("unauthorized")
        || lower.contains("authentication failed")
        || lower.contains("invalid bearer token")
        || lower.contains("api key not found")
        || lower.contains("permission denied")
    {
        return ApiErrorKind::NonRecoverable;
    }

    // Account issues - non-recoverable
    if lower.contains("account suspended")
        || lower.contains("account disabled")
        || lower.contains("access denied")
    {
        return ApiErrorKind::NonRecoverable;
    }

    // Rate limiting - transient (may recover after backoff)
    if lower.contains("rate limit")
        || lower.contains("too many requests")
        || lower.contains("429")
    {
        return ApiErrorKind::Transient;
    }

    // Network/timeout errors - transient
    if lower.contains("timeout")
        || lower.contains("connection refused")
        || lower.contains("network error")
        || lower.contains("connection reset")
    {
        return ApiErrorKind::Transient;
    }

    ApiErrorKind::Unknown
}

/// Get a user-friendly message for non-recoverable errors
pub fn get_api_error_guidance(error_msg: &str) -> &'static str {
    let lower = error_msg.to_lowercase();

    if lower.contains("credit balance") || lower.contains("purchase credits") {
        "💳 API credits exhausted. Please add credits to your account and try again."
    } else if lower.contains("invalid api key") || lower.contains("invalid_api_key") {
        "🔑 Invalid API key. Please check your ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable."
    } else if lower.contains("unauthorized") || lower.contains("authentication") {
        "🔐 Authentication failed. Please verify your API credentials."
    } else if lower.contains("account suspended") || lower.contains("account disabled") {
        "⚠️ Account issue. Please check your account status with the API provider."
    } else {
        "❌ Non-recoverable API error. Please check your API configuration."
    }
}

/// Configuration for retry logic on transient errors
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay in milliseconds before first retry
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds between retries
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,   // 1 second
            max_delay_ms: 30000,      // 30 seconds
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given retry attempt (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let delay_ms = (self.initial_delay_ms as f64
            * self.backoff_multiplier.powi(attempt as i32)) as u64;
        let capped_delay_ms = delay_ms.min(self.max_delay_ms);
        std::time::Duration::from_millis(capped_delay_ms)
    }
}

/// Get user-friendly message for transient errors
pub fn get_transient_error_message(error_msg: &str) -> &'static str {
    let lower = error_msg.to_lowercase();

    if lower.contains("rate limit") || lower.contains("too many requests") || lower.contains("429")
    {
        "⏳ Rate limit hit. Waiting before retry..."
    } else if lower.contains("timeout") {
        "⏱️ Request timed out. Retrying..."
    } else if lower.contains("connection refused")
        || lower.contains("network error")
        || lower.contains("connection reset")
    {
        "🌐 Network error. Retrying..."
    } else {
        "🔄 Transient error. Retrying..."
    }
}

/// Status of a todo item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoStatus {
    /// Task is pending and hasn't been started yet
    Pending,
    /// Task is currently in progress
    InProgress,
    /// Task has been completed successfully
    Completed,
    /// Task has failed
    Failed,
}

impl TodoStatus {
    /// Get a display string with emoji prefix for the status
    pub fn display(&self) -> &'static str {
        match self {
            TodoStatus::Pending => "⏳ Pending",
            TodoStatus::InProgress => "🔄 In Progress",
            TodoStatus::Completed => "✅ Completed",
            TodoStatus::Failed => "❌ Failed",
        }
    }
}

/// A todo item tracking a specific task
#[derive(Debug, Clone)]
pub struct TodoItem {
    /// Description of the task to complete
    pub task: String,
    /// Current status of the task
    pub status: TodoStatus,
    /// When this todo was created
    pub created_at: Instant,
}

impl TodoItem {
    /// Create a new todo item
    pub fn new(task: String) -> Self {
        Self {
            task,
            status: TodoStatus::Pending,
            created_at: Instant::now(),
        }
    }

    /// Format the todo item for display
    pub fn display(&self) -> String {
        format!("{} - {}", self.status.display(), self.task)
    }
}

// Note: Goal completion is now handled dynamically via should_complete_goal()
// which tracks files created this session and todo completion status,
// rather than using hardcoded game types and file patterns.

/// Validate that generated game code matches the expected game type
fn validate_game_output(
    code_lower: &str,
    expected_game: &str,
    file_extension: &str,
    min_size: usize,
) -> bool {
    // Check minimum size
    if code_lower.len() < min_size {
        return false;
    }

    // Check for Love2D/Lua specific markers
    let has_love2d_markers = if file_extension == "lua" {
        code_lower.contains("love.load")
            || code_lower.contains("love.draw")
            || code_lower.contains("love.update")
    } else {
        true // Not Lua, skip this check
    };

    if !has_love2d_markers {
        return false;
    }

    // Check for game-specific markers
    match expected_game {
        "solitaire" => {
            // Must have card-game related terms
            let has_cards = code_lower.contains("card")
                || code_lower.contains("deck")
                || code_lower.contains("suit");
            let has_piles = code_lower.contains("pile")
                || code_lower.contains("tableau")
                || code_lower.contains("foundation")
                || code_lower.contains("stack");
            let no_wrong_game = !code_lower.contains("tetromino")
                && !code_lower.contains("snake")
                && !code_lower.contains("shooter")
                && !code_lower.contains("space");
            has_cards && has_piles && no_wrong_game
        }
        "tetris" => {
            let has_tetromino = code_lower.contains("tetromino")
                || code_lower.contains("piece")
                || code_lower.contains("block");
            let has_grid = code_lower.contains("grid")
                || code_lower.contains("board")
                || code_lower.contains("row");
            let has_rotation = code_lower.contains("rotat") || code_lower.contains("spin");
            has_tetromino && has_grid && has_rotation
        }
        "snake" => {
            let has_snake = code_lower.contains("snake") || code_lower.contains("segment");
            let has_food = code_lower.contains("food")
                || code_lower.contains("apple")
                || code_lower.contains("eat");
            let has_direction = code_lower.contains("direction")
                || code_lower.contains("up")
                || code_lower.contains("down");
            has_snake && has_food && has_direction
        }
        "pong" => {
            let has_paddle = code_lower.contains("paddle") || code_lower.contains("player");
            let has_ball = code_lower.contains("ball");
            let has_bounce = code_lower.contains("bounce")
                || code_lower.contains("velocity")
                || code_lower.contains("speed");
            has_paddle && has_ball && has_bounce
        }
        "breakout" => {
            let has_paddle = code_lower.contains("paddle");
            let has_ball = code_lower.contains("ball");
            let has_bricks = code_lower.contains("brick") || code_lower.contains("block");
            has_paddle && has_ball && has_bricks
        }
        "minesweeper" => {
            let has_mines = code_lower.contains("mine") || code_lower.contains("bomb");
            let has_grid = code_lower.contains("grid") || code_lower.contains("cell");
            let has_reveal = code_lower.contains("reveal")
                || code_lower.contains("flag")
                || code_lower.contains("click");
            has_mines && has_grid && has_reveal
        }
        _ => {
            // Generic game - just check for basic game elements
            let has_game_loop = code_lower.contains("update")
                || code_lower.contains("draw")
                || code_lower.contains("loop");
            let has_input = code_lower.contains("key")
                || code_lower.contains("mouse")
                || code_lower.contains("input");
            has_game_loop && has_input
        }
    }
}

/// Configuration for agentic mode execution
///
/// Contains all the settings needed to run the fluent_cli in autonomous
/// agentic mode, including goal description, iteration limits, and tool access.
///
/// # Examples
///
/// ```rust
/// use fluent_cli::agentic::AgenticConfig;
///
/// let config = AgenticConfig::new(
///     "Create a simple web game".to_string(),
///     "agent_config.json".to_string(),
///     10,
///     true,
///     "config.yaml".to_string(),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct AgenticConfig {
    /// The goal or task description for the agent to accomplish
    pub goal_description: String,
    /// Path to the agent configuration file
    pub agent_config_path: String,
    /// Maximum number of iterations before stopping
    pub max_iterations: u32,
    /// Whether to enable tool usage for autonomous operations
    pub enable_tools: bool,
    /// Whether to enable reflection mode
    pub enable_reflection: bool,
    /// Path to the main configuration file
    pub config_path: String,
    /// Optional model override for default engines (e.g., gpt-4o, claude-3-5-sonnet-20241022)
    pub model_override: Option<String>,
    /// Optional max retries for LLM code generation
    pub gen_retries: Option<u32>,
    /// Optional minimum HTML size for validation
    pub min_html_size: Option<u32>,
    pub dry_run: bool,
}

impl Default for AgenticConfig {
    fn default() -> Self {
        Self {
            goal_description: String::new(),
            agent_config_path: "agent_config.json".to_string(),
            max_iterations: 50,
            enable_tools: true,
            enable_reflection: false,
            config_path: "fluent_config.toml".to_string(),
            model_override: None,
            gen_retries: Some(3),
            min_html_size: Some(1000),
            dry_run: false,
        }
    }
}

impl AgenticConfig {
    /// Create a new agentic configuration
    ///
    /// # Arguments
    ///
    /// * `goal_description` - The goal or task for the agent to accomplish
    /// * `agent_config_path` - Path to the agent configuration file
    /// * `max_iterations` - Maximum number of iterations before stopping
    /// * `enable_tools` - Whether to enable tool usage
    /// * `config_path` - Path to the main configuration file
    ///
    /// # Returns
    ///
    /// A new `AgenticConfig` instance
    pub fn new(
        goal_description: String,
        agent_config_path: String,
        max_iterations: u32,
        enable_tools: bool,
        enable_reflection: bool,
        config_path: String,
        model_override: Option<String>,
        gen_retries: Option<u32>,
        min_html_size: Option<u32>,
    ) -> Self {
        Self {
            goal_description,
            agent_config_path,
            max_iterations,
            enable_tools,
            enable_reflection,
            config_path,
            model_override,
            gen_retries,
            min_html_size,
            dry_run: std::env::var("FLUENT_AGENT_DRY_RUN")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        }
    }
}

/// Agentic mode executor
///
/// The main executor for autonomous agentic operations. This struct
/// orchestrates the entire agentic workflow including goal processing,
/// LLM interactions, tool usage, and iterative execution.
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_cli::agentic::{AgenticConfig, AgenticExecutor};
/// use fluent_core::config::Config;
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = AgenticConfig::new(
///     "Create a simple game".to_string(),
///     "agent_config.json".to_string(),
///     5,
///     true,
///     "config.yaml".to_string(),
/// );
///
/// let executor = AgenticExecutor::new(config);
/// let fluent_config = Config::default();
/// executor.run(&fluent_config).await?;
/// # Ok(())
/// # }
/// ```
pub struct AgenticExecutor {
    config: AgenticConfig,
    tui: TuiManager,
    /// Recent observations for feedback into reasoning loop
    recent_observations: Vec<String>,
    /// Tool registry for structured action execution (set during init)
    tool_registry: Option<Arc<fluent_agent::tools::ToolRegistry>>,
}

impl AgenticExecutor {
    /// Create a new agentic executor
    ///
    /// # Arguments
    ///
    /// * `config` - The agentic configuration to use
    /// * `enable_tui` - Whether to enable the TUI interface
    ///
    /// # Returns
    ///
    /// A new `AgenticExecutor` instance
    pub fn new(config: AgenticConfig, enable_tui: bool) -> Self {
        Self {
            config,
            tui: TuiManager::new(enable_tui),
            recent_observations: Vec::new(),
            tool_registry: None,
        }
    }

    /// Store an observation for feedback into reasoning
    fn store_observation(&mut self, observation: String) {
        // Keep only last 5 observations
        if self.recent_observations.len() >= 5 {
            self.recent_observations.remove(0);
        }
        self.recent_observations.push(observation);
        debug!(
            "Stored observation, total: {}",
            self.recent_observations.len()
        );
    }

    /// Main entry point for agentic mode execution
    pub async fn run(&mut self, _fluent_config: &Config) -> Result<()> {
        if self.tui.enabled() {
            self.tui
                .add_log("🚀 AgenticExecutor::run() called".to_string());
            self.tui.add_log("🔧 Initializing TUI...".to_string());
        } else {
            println!("🚀 AgenticExecutor::run() called");
            println!("🔧 Initializing TUI...");
        }
        if let Err(e) = self.tui.init() {
            if self.tui.enabled() {
                self.tui
                    .add_log(format!("❌ TUI initialization failed: {}", e));
                self.tui
                    .add_log("💡 Falling back to non-TUI mode".to_string());
            } else {
                println!("❌ TUI initialization failed: {}", e);
                println!("💡 Falling back to non-TUI mode");
                println!("💡 To use TUI, try:");
                println!("   - Use a different terminal emulator (iTerm2, Alacritty, etc.)");
                println!("   - Make sure you're in an interactive terminal session");
                println!("   - Check that your terminal supports ANSI escape sequences");
            }
            // Fall back to non-TUI mode by disabling TUI
            self.tui = TuiManager::new(false);
        } else {
            if self.tui.enabled() {
                self.tui
                    .add_log("✅ TUI initialized successfully".to_string());
            } else {
                println!("✅ TUI initialized successfully");
            }
            self.tui.set_goal(self.config.goal_description.clone());
            self.tui
                .set_features(self.config.enable_tools, self.config.enable_reflection);
            self.tui.update_status(AgentStatus::Initializing);
            self.tui.add_log("🤖 Starting Agentic Mode".to_string());
        }

        // Spawn SimpleTUI in background if it's available
        let tui_handle = self.tui.spawn_simple_tui();
        if tui_handle.is_some() {
            if self.tui.enabled() {
                self.tui
                    .add_log("✅ SimpleTUI running in background - Press 'Q' to quit".to_string());
            } else {
                println!("✅ SimpleTUI running in background - Press 'Q' to quit");
            }
        }

        let agent_config = self.load_agent_configuration().await?;
        let credentials = self.load_and_validate_credentials(&agent_config).await?;
        let runtime_config = self
            .create_runtime_configuration(&agent_config, credentials)
            .await?;
        let goal = self.create_goal()?;

        // Engine test - fail fast if engines aren't working
        if let Err(e) = self.test_engines(&runtime_config).await {
            self.tui.add_log(format!("⚠️ Engine test failed: {}", e));
            // Continue anyway but warn - user may want to abort
            warn!("agent.engine.test_failed error='{}'", e);
        }

        self.print_startup_info();

        // Build autonomous orchestrator with tools/memory
        use fluent_agent::action::{
            ActionExecutor, ActionPlanner, ComprehensiveActionExecutor, IntelligentActionPlanner,
        };
        use fluent_agent::adapters::CompositePlanner;
        use fluent_agent::adapters::{
            FsFileManager, LlmCodeGenerator, RegistryToolAdapter, SimpleRiskAssessor,
        };
        use fluent_agent::observation::{
            BasicImpactAssessor, BasicLearningExtractor, BasicPatternDetector, BasicResultAnalyzer,
            ComprehensiveObservationProcessor,
        };
        use fluent_agent::tools::ToolRegistry;
        use fluent_agent::{
            AgentOrchestrator, MemorySystem, ReflectionEngine, StateManager, StateManagerConfig,
        };

        let mut tool_registry = if self.config.enable_tools {
            ToolRegistry::with_standard_tools(&runtime_config.config.tools)
        } else {
            ToolRegistry::new()
        };

        // Workflow macro-tools (LLM-powered tools)
        if self.config.enable_tools {
            // Create tool execution config for workflow executor
            use fluent_agent::tools::ToolExecutionConfig;
            let workflow_config = ToolExecutionConfig {
                timeout_seconds: 60,
                max_output_size: 10 * 1024 * 1024, // 10MB for workflow outputs
                allowed_paths: runtime_config
                    .config
                    .tools
                    .allowed_paths
                    .clone()
                    .unwrap_or_else(|| {
                        vec![
                            "./".to_string(),
                            "./src".to_string(),
                            "./examples".to_string(),
                            "./crates".to_string(),
                        ]
                    }),
                allowed_commands: vec![], // Workflow doesn't execute commands
                read_only: false,
            };

            let workflow_exec = std::sync::Arc::new(fluent_agent::tools::WorkflowExecutor::new(
                runtime_config.reasoning_engine.clone(),
                workflow_config,
            ));
            tool_registry.register("workflow".to_string(), workflow_exec);
            self.tui.add_log(
                "🧰 Registered workflow macro-tools (outline/toc/assemble/research)".to_string(),
            );
        }

        // Optional MCP integration: initialize and register MCP tool executor
        if self.config.enable_tools {
            use fluent_agent::production_mcp::initialize_production_mcp;
            if let Ok(manager) = initialize_production_mcp().await {
                // Attempt auto-connect from config file (config_path)
                if let Err(e) =
                    Self::auto_connect_mcp_servers(&self.config.config_path, &manager).await
                {
                    self.tui
                        .add_log(format!("⚠️ MCP auto-connect skipped: {}", e));
                }

                let mcp_exec = std::sync::Arc::new(
                    fluent_agent::adapters::McpRegistryExecutor::new(manager.clone()),
                );
                tool_registry.register("mcp".to_string(), mcp_exec);
                self.tui
                    .add_log("🔌 MCP integrated: remote tools available via registry".to_string());
            } else {
                self.tui
                    .add_log("⚠️ MCP integration skipped (initialization failed)".to_string());
            }
        }

        // Finalize registry, then create shared Arc for adapters/planners
        let arc_registry = Arc::new(tool_registry);
        // Store for use in AutonomousExecutor
        self.tool_registry = Some(arc_registry.clone());
        let tool_adapter = Box::new(RegistryToolAdapter::new(arc_registry.clone()));
        let codegen = Box::new(LlmCodeGenerator::new(
            runtime_config.reasoning_engine.clone(),
        ));
        let filemgr = Box::new(FsFileManager::new());
        let base_executor: Box<dyn ActionExecutor> = Box::new(ComprehensiveActionExecutor::new(
            tool_adapter,
            codegen,
            filemgr,
        ));
        let action_executor: Box<dyn ActionExecutor> = if self.config.dry_run {
            self.tui
                .add_log("🧪 Dry-run mode: no side effects will be executed".to_string());
            Box::new(fluent_agent::adapters::DryRunActionExecutor)
        } else {
            base_executor
        };

        // Planner and observation (adaptive + reflective)
        let base_planner: Box<dyn ActionPlanner> =
            Box::new(IntelligentActionPlanner::new(Box::new(SimpleRiskAssessor)));
        let planner: Box<dyn ActionPlanner> = Box::new(CompositePlanner::new_with_registry(
            base_planner,
            arc_registry.clone(),
        ));
        let obs = Box::new(ComprehensiveObservationProcessor::new(
            Box::new(BasicResultAnalyzer),
            Box::new(BasicPatternDetector),
            Box::new(BasicImpactAssessor),
            Box::new(BasicLearningExtractor),
        ));

        // Memory and state
        use fluent_agent::memory::{
            CompressorConfig, MemoryConfig, PersistenceConfig, WorkingMemoryConfig,
        };

        // Initialize comprehensive memory system with proper configuration
        self.tui
            .add_log("🧠 Initializing memory system...".to_string());

        let memory_config = MemoryConfig {
            working_config: WorkingMemoryConfig {
                max_active_items: 50,
                max_memory_size: 1024 * 1024 * 100, // 100MB
                attention_refresh_interval: 60,     // 1 minute
                relevance_decay_rate: 0.1,
                enable_consolidation: true,
                consolidation_threshold: 0.8,
                enable_predictive_loading: true,
            },
            compressor_config: CompressorConfig {
                max_context_size: 10 * 1024 * 1024, // 10MB
                target_compression_ratio: 0.3,
                enable_semantic_compression: true,
                enable_temporal_compression: true,
                min_information_retention: 0.8,
                analysis_window_size: 100,
            },
            persistence_config: PersistenceConfig {
                storage_path: std::path::PathBuf::from("./fluent_persistence"),
                enable_automatic_save: true,
                save_interval_secs: 300, // 5 minutes
                max_session_history: 100,
                enable_compression: true,
                enable_learning_persistence: true,
                backup_retention_days: 30,
            },
        };

        let memory = MemorySystem::new(memory_config).await?;
        self.tui.add_log(
            "✅ Memory system initialized with working memory, compression, and persistence"
                .to_string(),
        );

        let state_mgr = StateManager::new(StateManagerConfig::default()).await?;
        let reflection = ReflectionEngine::new();

        // Orchestrate
        // Build orchestrator without moving runtime_config so we can run explicit loop
        let _orchestrator = AgentOrchestrator::from_config(
            runtime_config.clone(),
            planner,
            action_executor,
            obs,
            Arc::new(memory),
            Arc::new(state_mgr),
            reflection,
            None, // supervisor: Option<Arc<AutonomySupervisor>>
            None, // dynamic_replanner: Option<Arc<DynamicReplanner>>
            None, // adaptive_strategy: Option<Arc<AdaptiveStrategySystem>>
        )
        .await?;

        self.tui
            .add_log("🔁 Orchestrator constructed. Entering autonomous loop…".to_string());

        // Max total runtime - safety limit (default 1 hour)
        // The real timeout is activity-based: agent times out after inactivity, not total time
        let max_runtime_secs: u64 = std::env::var("FLUENT_AGENT_MAX_RUNTIME_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600); // 1 hour max total runtime

        // Inactivity timeout - resets on each LLM response/tool execution
        let inactivity_secs: u64 = std::env::var("FLUENT_AGENT_INACTIVITY_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120); // 2 min of no activity = timeout

        self.tui.add_log(format!(
            "🕒 Activity watchdog: {}s inactivity timeout, {}s max runtime",
            inactivity_secs, max_runtime_secs
        ));

        info!(
            "agent.react.start goal='{}' max_runtime={}s inactivity_timeout={}s",
            self.config.goal_description, max_runtime_secs, inactivity_secs
        );

        self.tui.update_status(AgentStatus::Running);

        // If TUI is enabled, run agent execution and TUI concurrently
        let result = if self.tui.enabled() {
            self.run_with_tui(&goal, &runtime_config, max_runtime_secs)
                .await
        } else {
            // Run without TUI - max_runtime is safety limit, inactivity handles real timeout
            match tokio::time::timeout(
                std::time::Duration::from_secs(max_runtime_secs),
                self.run_autonomous_execution(&goal, &runtime_config),
            )
            .await
            {
                Ok(Ok(())) => {
                    info!("agent.react.done success=true explicit_autonomous_loop=true");
                    Ok(())
                }
                Ok(Err(e)) => {
                    error!("agent.react.error err={}", e);
                    Err(e)
                }
                Err(_) => {
                    error!(
                        "agent.react.max_runtime_exceeded secs={} goal='{}'",
                        max_runtime_secs, self.config.goal_description
                    );
                    Err(anyhow::anyhow!(format!(
                        "Agent exceeded max runtime of {}s (this is a safety limit, not inactivity)",
                        max_runtime_secs
                    )))
                }
            }
        };

        // Cleanup TUI
        self.tui.cleanup()?;

        result
    }

    /// Attempt to auto-connect MCP servers based on entries in the main config file.
    /// Supported schema (YAML or JSON):
    /// mcp:
    ///   servers:
    ///     - name: search
    ///       command: my-mcp-server
    ///       args: ["--stdio"]
    ///     - "search:my-mcp-server --stdio"
    async fn auto_connect_mcp_servers(
        config_path: &str,
        manager: &std::sync::Arc<fluent_agent::production_mcp::ProductionMcpManager>,
    ) -> anyhow::Result<()> {
        use serde_json::Value;

        info!("agent.mcp.autoconnect.config path='{}'", config_path);
        let content = tokio::fs::read_to_string(config_path).await?;
        let root: Value = if content.trim_start().starts_with('{') {
            serde_json::from_str(&content)?
        } else {
            // Parse as TOML and convert to JSON Value
            let toml_value: toml::Value = toml::from_str(&content)?;
            fluent_core::config::toml_to_json(toml_value)?
        };

        let servers = root
            .get("mcp")
            .and_then(|m| m.get("servers"))
            .ok_or_else(|| anyhow::anyhow!("No mcp.servers section found"))?;

        match servers {
            Value::Array(arr) => {
                for item in arr {
                    match item {
                        Value::String(s) => {
                            // Format: "name:command [args...]"
                            let mut parts = s.splitn(2, ':');
                            let name = parts.next().unwrap_or("");
                            let cmd_and_args = parts.next().unwrap_or("").trim();
                            if name.is_empty() || cmd_and_args.is_empty() {
                                continue;
                            }
                            let mut split = cmd_and_args.split_whitespace();
                            if let Some(command) = split.next() {
                                let args: Vec<String> = split.map(|x| x.to_string()).collect();
                                info!(
                                    "agent.mcp.server.connect name='{}' command='{}' args={}",
                                    name,
                                    command,
                                    args.len()
                                );
                                info!(
                                    "agent.mcp.server.connect name='{}' command='{}' args={}",
                                    name,
                                    command,
                                    args.len()
                                );
                                let _ = manager
                                    .client_manager()
                                    .connect_server(name.to_string(), command.to_string(), args)
                                    .await;
                            }
                        }
                        Value::Object(map) => {
                            let name = map.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let command = map.get("command").and_then(|v| v.as_str()).unwrap_or("");
                            let args: Vec<String> = map
                                .get("args")
                                .and_then(|v| v.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();
                            if !name.is_empty() && !command.is_empty() {
                                info!(
                                    "agent.mcp.server.connect name='{}' command='{}' args={}",
                                    name,
                                    command,
                                    args.len()
                                );
                                info!(
                                    "agent.mcp.server.connect name='{}' command='{}' args={}",
                                    name,
                                    command,
                                    args.len()
                                );
                                let _ = manager
                                    .client_manager()
                                    .connect_server(name.to_string(), command.to_string(), args)
                                    .await;
                            }
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            _ => Err(anyhow::anyhow!("mcp.servers must be an array")),
        }
    }

    /// Print startup information
    fn print_startup_info(&mut self) {
        // TUI handles the display, just log to TUI
        self.tui
            .add_log(format!("Max iterations: {}", self.config.max_iterations));
        self.tui
            .add_log(format!("Tools enabled: {}", self.config.enable_tools));
        self.tui.add_log(format!(
            "Reflection enabled: {}",
            self.config.enable_reflection
        ));
    }

    /// Load agent configuration from file
    async fn load_agent_configuration(
        &mut self,
    ) -> Result<fluent_agent::config::AgentEngineConfig> {
        use fluent_agent::config::AgentEngineConfig;

        let agent_config = AgentEngineConfig::load_from_file(&self.config.agent_config_path)
            .await
            .map_err(|e| anyhow!("Failed to load agent config: {}", e))?;

        self.tui
            .add_log("✅ Agent configuration loaded".to_string());

        Ok(agent_config)
    }

    /// Load and validate credentials
    async fn load_and_validate_credentials(
        &mut self,
        agent_config: &fluent_agent::config::AgentEngineConfig,
    ) -> Result<std::collections::HashMap<String, String>> {
        use fluent_agent::config::credentials;

        let credentials = credentials::load_from_environment();
        self.tui.add_log(format!(
            "🔑 Loaded {} credential(s) from environment",
            credentials.len()
        ));

        // Validate required credentials
        let required_engines = vec![
            agent_config.reasoning_engine.clone(),
            agent_config.action_engine.clone(),
            agent_config.reflection_engine.clone(),
        ];
        credentials::validate_credentials(&credentials, &required_engines)?;

        Ok(credentials)
    }

    /// Create runtime configuration with engines
    async fn create_runtime_configuration(
        &mut self,
        agent_config: &fluent_agent::config::AgentEngineConfig,
        credentials: std::collections::HashMap<String, String>,
    ) -> Result<fluent_agent::config::AgentRuntimeConfig> {
        self.tui.add_log("🔧 Creating LLM engines...".to_string());

        let runtime_config = agent_config
            .create_runtime_config(
                &self.config.config_path,
                credentials,
                self.config.model_override.as_deref(),
            )
            .await?;

        self.tui
            .add_log("✅ LLM engines created successfully!".to_string());
        Ok(runtime_config)
    }

    /// Create goal from description
    fn create_goal(&mut self) -> Result<fluent_agent::goal::Goal> {
        use fluent_agent::goal::{Goal, GoalType};

        let mut builder = Goal::builder(
            self.config.goal_description.clone(),
            GoalType::CodeGeneration,
        )
        .max_iterations(self.config.max_iterations);

        // Load success criteria from env if provided by --goal-file path
        if let Ok(sc) = std::env::var("FLUENT_AGENT_SUCCESS_CRITERIA") {
            for criterion in sc.split("||").filter(|s| !s.is_empty()) {
                builder = builder.success_criterion(criterion.to_string());
            }
        } else {
            // Reasonable defaults for code-oriented tasks if nothing else provided
            builder = builder
                .success_criterion("Code compiles without errors".to_string())
                .success_criterion("Code runs successfully".to_string())
                .success_criterion("Code meets the specified requirements".to_string());
        }

        let goal = builder.build()?;

        self.tui.add_log(format!("🎯 Goal: {}", goal.description));
        self.tui
            .add_log(format!("🔄 Max iterations: {:?}", goal.max_iterations));

        Ok(goal)
    }

    /// Test engines to ensure they're working
    async fn test_engines(
        &mut self,
        runtime_config: &fluent_agent::config::AgentRuntimeConfig,
    ) -> Result<()> {
        self.tui
            .add_log("🧠 Testing reasoning engine...".to_string());

        let test_request = Request {
            flowname: "agentic_test".to_string(),
            payload: "Hello! Please respond with 'Agentic mode is working!' to confirm the engine is operational.".to_string(),
        };

        match Pin::from(runtime_config.reasoning_engine.execute(&test_request)).await {
            Ok(response) => {
                self.tui.add_log(format!(
                    "✅ Reasoning engine response: {}",
                    response.content
                ));
                self.print_operational_status();
                Ok(())
            }
            Err(e) => {
                self.tui.add_log(format!("❌ Engine test failed: {}", e));
                self.tui
                    .add_log("🔧 Please check your API keys and configuration".to_string());
                Err(anyhow!("Engine test failed: {}", e))
            }
        }
    }

    /// Print operational status
    fn print_operational_status(&mut self) {
        self.tui
            .add_log("🚀 AGENTIC MODE IS FULLY OPERATIONAL!".to_string());
        self.tui.add_log("🔧 All systems ready:".to_string());
        self.tui
            .add_log("   ✅ LLM engines connected and tested".to_string());
        self.tui
            .add_log("   ✅ Configuration system integrated".to_string());
        self.tui
            .add_log("   ✅ Credential management working".to_string());
        self.tui
            .add_log("   ✅ Goal system operational".to_string());

        if self.config.enable_tools {
            self.tui.add_log("   ✅ Tool execution enabled".to_string());
        } else {
            self.tui.add_log(
                "   ⚠️  Tool execution disabled (use --enable-tools to enable)".to_string(),
            );
        }

        self.tui.add_log(
            "🎉 The agentic coding platform is ready for autonomous operation!".to_string(),
        );
    }

    /// Show a mock TUI for demonstration when real TUI is not available
    #[allow(dead_code)]
    fn show_mock_tui(&self) {
        println!(
            "\n╔══════════════════════════════════════════════════════════════════════════════╗"
        );
        println!(
            "║                              🤖 FLUENT AGENTIC MODE                              ║"
        );
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!("║ Goal: {:<68} ║", self.config.goal_description);
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║ Status: 🔄 Initializing                  │ Iter: 0/{} │ Elapsed: 00:00:00    ║",
            self.config.max_iterations
        );
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!("║ Progress: [░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] ║");
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!("║ Current Action: Initializing agentic framework...                          ║");
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!("║ Logs:                                                                       ║");
        println!("║ 🤖 Starting Agentic Mode                                                   ║");
        println!("║ 🔧 Initializing LLM engines...                                             ║");
        println!("║ ✅ Configuration loaded                                                     ║");
        println!("║ 🔑 Credentials validated                                                    ║");
        println!("║ 🎯 Goal processing started                                                  ║");
        println!("║                                                                             ║");
        println!("║                                                                             ║");
        println!("║                                                                             ║");
        println!("║                                                                             ║");
        println!("║                                                                             ║");
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!(
            "║ Features: Tools: {} │ Reflection: {} │ Model: Default                        ║",
            if self.config.enable_tools {
                "✅"
            } else {
                "❌"
            },
            if self.config.enable_reflection {
                "✅"
            } else {
                "❌"
            }
        );
        println!(
            "╠══════════════════════════════════════════════════════════════════════════════╣"
        );
        println!("║ Press 'q' to quit │ ↑/↓ scroll logs │ PgUp/PgDn for faster scrolling        ║");
        println!(
            "╚══════════════════════════════════════════════════════════════════════════════╝\n"
        );
    }

    /// Run agent execution with TUI concurrently
    async fn run_with_tui(
        &mut self,
        goal: &fluent_agent::goal::Goal,
        runtime_config: &fluent_agent::config::AgentRuntimeConfig,
        timeout_secs: u64,
    ) -> Result<()> {
        // Update TUI to show we're starting execution
        self.tui.update_status(AgentStatus::Running);
        self.tui
            .add_log("🚀 Starting agent execution...".to_string());

        // For ASCII TUI, display current state immediately
        if self.tui.is_fallback_mode() {
            let _ = self.tui.force_display();
        }

        // Run agent execution
        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            self.run_autonomous_execution(goal, runtime_config),
        )
        .await
        {
            Ok(Ok(())) => {
                info!("agent.react.done success=true explicit_autonomous_loop=true");
                self.tui.update_status(AgentStatus::Completed);
                self.tui
                    .add_log("✅ Goal execution finished. Success: true".to_string());
                Ok(())
            }
            Ok(Err(e)) => {
                error!("agent.react.error err={}", e);
                self.tui.update_status(AgentStatus::Failed(e.to_string()));
                self.tui.add_log(format!("❌ Orchestrator error: {}", e));
                Err(e)
            }
            Err(_) => {
                error!(
                    "agent.react.max_runtime_exceeded secs={} goal='{}'",
                    timeout_secs, self.config.goal_description
                );
                self.tui.update_status(AgentStatus::Timeout);
                self.tui.add_log(format!(
                    "⏳ Agent exceeded max runtime of {}s (safety limit). Aborting.",
                    timeout_secs
                ));
                Err(anyhow::anyhow!(format!(
                    "Agent exceeded max runtime of {}s (safety limit, not inactivity)",
                    timeout_secs
                )))
            }
        };

        // Show completion and allow user interaction
        self.tui
            .add_log("🎯 Agent execution completed. Press 'q' to exit.".to_string());

        // For ASCII TUI, display final state immediately
        if self.tui.is_fallback_mode() {
            let _ = self.tui.run_event_loop().await;
        } else {
            // For full TUI, run event loop
            let _ = self.tui.run_event_loop().await;
        }

        result
    }

    /// Run autonomous execution loop
    async fn run_autonomous_execution(
        &mut self,
        goal: &fluent_agent::goal::Goal,
        runtime_config: &fluent_agent::config::AgentRuntimeConfig,
    ) -> Result<()> {
        self.tui
            .add_log("🚀 Starting autonomous execution...".to_string());

        // Get tool registry, falling back to empty if not initialized
        let registry = self
            .tool_registry
            .clone()
            .unwrap_or_else(|| Arc::new(fluent_agent::tools::ToolRegistry::new()));

        let mut executor = AutonomousExecutor::new(
            goal.clone(),
            runtime_config,
            self.config.gen_retries.unwrap_or(3),
            self.config.min_html_size.unwrap_or(2000) as usize,
            &mut self.tui,
            registry,
        );
        executor.execute(self.config.max_iterations).await
    }
}

/// Autonomous execution engine
pub struct AutonomousExecutor<'a> {
    goal: fluent_agent::goal::Goal,
    runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
    gen_retries: u32,
    min_html_size: usize,
    tui: &'a mut TuiManager,
    control_rx: Option<fluent_agent::agent_control::ControlRxHandle>,
    paused: bool,
    queued_guidance: Vec<String>,
    /// Recent observations from the last 3-5 iterations to feed back into reasoning
    recent_observations: Vec<String>,
    /// List of todos tracking progress toward the goal
    todo_list: Vec<TodoItem>,
    /// Tool registry for executing structured actions
    tool_registry: Arc<fluent_agent::tools::ToolRegistry>,
    /// Files created during this session (for completion tracking)
    files_created_this_session: Vec<String>,
    /// Last time the agent made progress (for activity-based watchdog)
    last_activity: Instant,
    /// Inactivity timeout in seconds (watchdog resets on each LLM response)
    inactivity_timeout_secs: u64,
}

/// Result of executing a structured action
struct ActionExecutionResult {
    observation: String,
    success: bool,
}

impl<'a> AutonomousExecutor<'a> {
    pub fn new(
        goal: fluent_agent::goal::Goal,
        runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
        gen_retries: u32,
        min_html_size: usize,
        tui: &'a mut TuiManager,
        tool_registry: Arc<fluent_agent::tools::ToolRegistry>,
    ) -> Self {
        let crx = tui.control_receiver();
        // Inactivity timeout - agent times out if no progress for this duration
        // Default 120s (2 min) of inactivity, not total runtime
        let inactivity_timeout_secs: u64 = std::env::var("FLUENT_AGENT_INACTIVITY_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120);

        Self {
            goal,
            runtime_config,
            gen_retries,
            min_html_size,
            tui,
            control_rx: crx,
            paused: false,
            queued_guidance: Vec::new(),
            recent_observations: Vec::new(),
            todo_list: Vec::new(),
            tool_registry,
            files_created_this_session: Vec::new(),
            last_activity: Instant::now(),
            inactivity_timeout_secs,
        }
    }

    /// Reset the activity timer - call this on every LLM response or tool completion
    fn reset_activity_timer(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if the agent has been inactive for too long
    fn is_inactive_timeout(&self) -> bool {
        self.last_activity.elapsed().as_secs() > self.inactivity_timeout_secs
    }

    /// Execute a structured action using the tool registry
    ///
    /// Returns the observation and whether it succeeded.
    async fn execute_structured_action(
        &mut self,
        action: &StructuredAction,
    ) -> ActionExecutionResult {
        use fluent_agent::prompts::format_observation;

        let tool_name = action.get_tool_name().unwrap_or_else(|| {
            // Infer tool from action type
            match action.action_type.to_lowercase().as_str() {
                "file" | "fileoperation" | "file_operation" => "file_system".to_string(),
                "shell" | "command" | "run" => "shell".to_string(),
                "code" | "codegeneration" | "code_generation" => "file_system".to_string(),
                _ => "file_system".to_string(),
            }
        });

        self.tui.add_log(format!(
            "🔧 Executing tool: {} with {} parameters",
            tool_name,
            action.parameters.len()
        ));

        debug!(
            "agent.tool.execute tool='{}' params={:?}",
            tool_name, action.parameters
        );

        // Execute via tool registry
        match self
            .tool_registry
            .execute_tool(&tool_name, &action.parameters)
            .await
        {
            Ok(output) => {
                // Track files created this session for dynamic completion checking
                if tool_name == "write_file" || tool_name == "file_system" {
                    if let Some(serde_json::Value::String(path)) = action.parameters.get("path") {
                        if !self.files_created_this_session.contains(path) {
                            self.files_created_this_session.push(path.clone());
                            debug!("agent.session.file_created path='{}'", path);
                        }
                    }
                }

                let truncated_output = if output.len() > 1000 {
                    format!(
                        "{}... (truncated {} chars)",
                        &output[..1000],
                        output.len() - 1000
                    )
                } else {
                    output.clone()
                };
                let observation = format_observation(
                    &action.action_type,
                    &tool_name,
                    true,
                    &truncated_output,
                    None,
                );
                self.tui.add_log(format!("✅ Tool {} succeeded", tool_name));
                info!(
                    "agent.tool.success tool='{}' output_len={}",
                    tool_name,
                    output.len()
                );
                ActionExecutionResult {
                    observation,
                    success: true,
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                let observation = format_observation(
                    &action.action_type,
                    &tool_name,
                    false,
                    "",
                    Some(&error_msg),
                );
                self.tui
                    .add_log(format!("❌ Tool {} failed: {}", tool_name, e));
                warn!("agent.tool.error tool='{}' error={}", tool_name, e);
                ActionExecutionResult {
                    observation,
                    success: false,
                }
            }
        }
    }

    /// Add a new todo item
    pub fn add_todo(&mut self, task: String) {
        let todo = TodoItem::new(task.clone());
        self.todo_list.push(todo);
        self.tui.add_log(format!("📋 Todo added: {}", task));
        debug!("agent.todo.added task='{}'", task);
    }

    /// Update the status of a todo item by index
    pub fn update_todo_status(&mut self, index: usize, status: TodoStatus) -> Result<()> {
        if index >= self.todo_list.len() {
            return Err(anyhow!("Todo index {} out of bounds", index));
        }

        let todo = &mut self.todo_list[index];
        let old_status = todo.status;
        todo.status = status;

        self.tui.add_log(format!(
            "📋 Todo updated: {} -> {}",
            todo.task,
            status.display()
        ));
        debug!(
            "agent.todo.updated index={} task='{}' old_status={:?} new_status={:?}",
            index, todo.task, old_status, status
        );

        Ok(())
    }

    /// Find a todo that matches the given action
    /// Returns the index of a matching pending todo, or None if no match
    fn find_matching_todo(&self, action: &StructuredAction) -> Option<usize> {
        let tool_name = action.get_tool_name().unwrap_or_default().to_lowercase();
        let action_type = action.action_type.to_lowercase();

        // Extract file path if present
        let file_path = action
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        for (idx, todo) in self.todo_list.iter().enumerate() {
            if todo.status != TodoStatus::Pending {
                continue;
            }

            let task_lower = todo.task.to_lowercase();

            // Match write_file to "write" or "create" todos
            if tool_name == "write_file"
                || (tool_name == "file_system" && action.parameters.contains_key("content"))
            {
                if task_lower.contains("write")
                    || task_lower.contains("create")
                    || task_lower.contains("generate")
                    || task_lower.contains("output")
                    || task_lower.contains("save")
                {
                    return Some(idx);
                }
            }

            // Match read_file to "read" or "examine" or "understand" todos
            if tool_name == "read_file"
                || (tool_name == "file_system" && !action.parameters.contains_key("content"))
            {
                if task_lower.contains("read")
                    || task_lower.contains("examine")
                    || task_lower.contains("understand")
                    || task_lower.contains("analyze")
                    || task_lower.contains("check")
                    || task_lower.contains("review")
                {
                    return Some(idx);
                }
            }

            // Match create_directory to "directory" or "folder" todos
            if tool_name == "create_directory" || tool_name.contains("mkdir") {
                if task_lower.contains("directory")
                    || task_lower.contains("folder")
                    || task_lower.contains("structure")
                {
                    return Some(idx);
                }
            }

            // Match shell/run_command to "run" or "execute" or "build" or "test" todos
            if tool_name == "shell" || tool_name == "run_command" || action_type.contains("shell") {
                if task_lower.contains("run")
                    || task_lower.contains("execute")
                    || task_lower.contains("build")
                    || task_lower.contains("test")
                    || task_lower.contains("compile")
                {
                    return Some(idx);
                }
            }

            // Match based on file path appearing in todo
            if !file_path.is_empty() && task_lower.contains(&file_path.to_lowercase()) {
                return Some(idx);
            }
        }

        // No semantic match found - return None (don't mark any todo)
        None
    }

    /// Get all pending todos
    pub fn get_pending_todos(&self) -> Vec<&TodoItem> {
        self.todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::Pending)
            .collect()
    }

    /// Display todo list summary to TUI
    pub fn display_todo_summary(&mut self) {
        let total = self.todo_list.len();
        let completed = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::Completed)
            .count();
        let in_progress = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();
        let pending = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::Pending)
            .count();
        let failed = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::Failed)
            .count();

        self.tui.add_log(format!(
            "📊 Todo Summary: {} total | ✅ {} completed | 🔄 {} in progress | ⏳ {} pending | ❌ {} failed",
            total, completed, in_progress, pending, failed
        ));
    }

    /// Display all todos to TUI
    pub fn display_todos(&mut self) {
        if self.todo_list.is_empty() {
            self.tui.add_log("📋 No todos yet".to_string());
            return;
        }

        self.tui.add_log("📋 Current Todos:".to_string());
        for (idx, todo) in self.todo_list.iter().enumerate() {
            self.tui
                .add_log(format!("  {}. {}", idx + 1, todo.display()));
        }
    }

    /// Parse the goal description into initial todos
    fn parse_goal_into_todos(&mut self) {
        let description = self.goal.description.to_lowercase();

        // Detect goal type and create appropriate todos
        if description.contains("game") {
            if description.contains("tetris") {
                self.add_todo("Generate Tetris game code".to_string());
                self.add_todo("Validate game has tetromino pieces".to_string());
                self.add_todo("Validate game has grid/board".to_string());
                self.add_todo("Write game to output file".to_string());
            } else if description.contains("solitaire") {
                self.add_todo("Generate Solitaire game code".to_string());
                self.add_todo("Validate game has card deck and piles".to_string());
                self.add_todo("Write game to output file".to_string());
            } else if description.contains("snake") {
                self.add_todo("Generate Snake game code".to_string());
                self.add_todo("Validate game has snake and food mechanics".to_string());
                self.add_todo("Write game to output file".to_string());
            } else {
                self.add_todo("Determine game type to create".to_string());
                self.add_todo("Generate game code".to_string());
                self.add_todo("Validate game mechanics".to_string());
                self.add_todo("Write game to output file".to_string());
            }
        } else if description.contains("reflection") || description.contains("analysis") {
            self.add_todo("Analyze target system".to_string());
            self.add_todo("Generate comprehensive report".to_string());
            self.add_todo("Write analysis to file".to_string());
        } else if description.contains("research") {
            self.add_todo("Determine research scope".to_string());
            self.add_todo("Generate research content".to_string());
            self.add_todo("Write research to file".to_string());
        } else {
            // Generic goal breakdown
            self.add_todo("Analyze goal requirements".to_string());
            self.add_todo("Plan approach".to_string());
            self.add_todo("Execute planned actions".to_string());
            self.add_todo("Validate results".to_string());
        }

        self.tui.add_log(format!(
            "📋 Created {} initial todos from goal",
            self.todo_list.len()
        ));
        self.display_todos();
    }

    /// Execute autonomous loop
    pub async fn execute(&mut self, max_iterations: u32) -> Result<()> {
        use fluent_agent::context::ExecutionContext;

        self.tui.add_log(format!(
            "🎯 Starting autonomous execution for goal: {}",
            self.goal.description
        ));
        info!(
            "agent.loop.begin goal='{}' max_iterations={}",
            self.goal.description, max_iterations
        );

        // Parse goal into initial todos
        self.parse_goal_into_todos();
        self.display_todo_summary();

        let mut context = ExecutionContext::new(self.goal.clone());

        for iteration in 1..=max_iterations {
            self.process_controls(&mut context).await?;
            while self.paused {
                if let Some(rx) = &self.control_rx {
                    if let Some(msg) = rx.recv().await {
                        self.handle_control_message(&mut context, msg).await?;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }

            if !self.queued_guidance.is_empty() {
                for (idx, g) in std::mem::take(&mut self.queued_guidance)
                    .into_iter()
                    .enumerate()
                {
                    context.set_variable(
                        format!(
                            "queued_guidance_{}",
                            idx + context.iteration_count() as usize
                        ),
                        g.clone(),
                    );
                    self.tui
                        .add_log(format!("💬 Queued guidance applied: {}", g));
                }
            }
            self.tui.update_iteration(iteration, max_iterations);
            self.tui
                .add_log(format!("🔄 Iteration {}/{}", iteration, max_iterations));
            debug!("agent.loop.iteration start iter={}", iteration);

            // Check for inactivity timeout before proceeding
            if self.is_inactive_timeout() {
                error!(
                    "agent.loop.inactivity_timeout secs={} last_activity={}s ago",
                    self.inactivity_timeout_secs,
                    self.last_activity.elapsed().as_secs()
                );
                return Err(anyhow!(
                    "Agent timed out after {}s of inactivity (no LLM response or tool execution)",
                    self.inactivity_timeout_secs
                ));
            }

            // Perform reasoning with retry logic for transient errors
            let retry_config = RetryConfig::default();
            let mut last_error: Option<anyhow::Error> = None;

            let reasoning_response = 'retry_loop: {
                for attempt in 0..=retry_config.max_retries {
                    match self.perform_reasoning(iteration, max_iterations).await {
                        Ok(response) => break 'retry_loop response,
                        Err(e) => {
                            let error_msg = e.to_string();
                            let error_kind = classify_api_error(&error_msg);

                            match error_kind {
                                ApiErrorKind::NonRecoverable => {
                                    // Log and exit immediately for non-recoverable errors
                                    let guidance = get_api_error_guidance(&error_msg);
                                    error!(
                                        "agent.api.non_recoverable error='{}' guidance='{}'",
                                        error_msg, guidance
                                    );
                                    self.tui.add_log(format!("🛑 {}", guidance));
                                    self.tui.add_log(
                                        "❌ Agent stopping immediately due to non-recoverable API error".to_string()
                                    );
                                    return Err(anyhow!(
                                        "Non-recoverable API error: {}. {}",
                                        error_msg,
                                        guidance
                                    ));
                                }
                                ApiErrorKind::Transient | ApiErrorKind::Unknown => {
                                    // For transient errors, retry with exponential backoff
                                    if attempt < retry_config.max_retries {
                                        let delay = retry_config.delay_for_attempt(attempt);
                                        let message = get_transient_error_message(&error_msg);
                                        warn!(
                                            "agent.api.transient attempt={}/{} delay_ms={} error='{}'",
                                            attempt + 1,
                                            retry_config.max_retries,
                                            delay.as_millis(),
                                            error_msg
                                        );
                                        self.tui.add_log(format!(
                                            "{} (attempt {}/{}, waiting {}s)",
                                            message,
                                            attempt + 1,
                                            retry_config.max_retries,
                                            delay.as_secs()
                                        ));
                                        tokio::time::sleep(delay).await;
                                        last_error = Some(e);
                                        continue;
                                    } else {
                                        // Exhausted all retries
                                        error!(
                                            "agent.api.retry_exhausted attempts={} error='{}'",
                                            retry_config.max_retries + 1,
                                            error_msg
                                        );
                                        self.tui.add_log(format!(
                                            "❌ Exhausted {} retry attempts. Last error: {}",
                                            retry_config.max_retries + 1,
                                            error_msg
                                        ));
                                        return Err(e);
                                    }
                                }
                            }
                        }
                    }
                }
                // Should not reach here, but handle edge case
                return Err(last_error.unwrap_or_else(|| anyhow!("Unknown error during retry")));
            };

            // Reset activity timer - we got an LLM response
            self.reset_activity_timer();

            debug!(
                "agent.loop.reasoning.done len={} preview='{}'",
                reasoning_response.len(),
                &reasoning_response.chars().take(160).collect::<String>()
            );

            // Try to parse structured action from reasoning response
            let action_result = parse_structured_action(&reasoning_response);

            let observation = match action_result {
                Ok(action) => {
                    info!(
                        "agent.loop.structured_action tool={:?} type='{}'",
                        action.get_tool_name(),
                        action.action_type
                    );

                    // Find a todo that semantically matches this action
                    let matching_todo_idx = self.find_matching_todo(&action);

                    // Mark matching todo as in-progress (if found)
                    if let Some(idx) = matching_todo_idx {
                        let _ = self.update_todo_status(idx, TodoStatus::InProgress);
                    } else {
                        // No matching todo - log but continue (action may still be useful)
                        debug!(
                            "agent.todo.no_match tool={:?} - action doesn't match any pending todo",
                            action.get_tool_name()
                        );
                    }

                    // Execute the structured action via tool registry
                    let result = self.execute_structured_action(&action).await;

                    // Reset activity timer - tool execution completed
                    self.reset_activity_timer();

                    // Update todo status based on actual success/failure
                    // Only update the todo we marked in-progress (if any)
                    if let Some(idx) = matching_todo_idx {
                        if result.success {
                            let _ = self.update_todo_status(idx, TodoStatus::Completed);
                        } else {
                            let _ = self.update_todo_status(idx, TodoStatus::Failed);
                        }
                    }

                    result.observation
                }
                Err(_) => {
                    // Fallback: No structured action parsed, use legacy paths
                    debug!("agent.loop.fallback no_structured_action");

                    if self.is_game_goal() {
                        info!("agent.loop.path game=true (legacy)");
                        // Legacy game handling - but now continues loop instead of returning
                        match self.handle_game_creation(&mut context).await {
                            Ok(()) => {
                                // Mark todos complete and check if we should exit
                                for idx in 0..self.todo_list.len() {
                                    if self.todo_list[idx].status != TodoStatus::Completed {
                                        let _ = self.update_todo_status(idx, TodoStatus::Completed);
                                    }
                                }
                                format!(
                                    "Iteration {}: Game creation completed successfully",
                                    iteration
                                )
                            }
                            Err(e) => {
                                format!("Iteration {}: Game creation failed: {}", iteration, e)
                            }
                        }
                    } else {
                        info!("agent.loop.path general=true (legacy)");
                        // Legacy general goal handling
                        match self
                            .handle_general_goal(
                                &mut context,
                                &reasoning_response,
                                iteration,
                                max_iterations,
                            )
                            .await
                        {
                            Ok(()) => {
                                format!("Iteration {}: General goal step completed", iteration)
                            }
                            Err(e) => {
                                format!("Iteration {}: General goal step failed: {}", iteration, e)
                            }
                        }
                    }
                }
            };

            // Store the observation from this iteration
            self.store_observation(observation.clone());
            self.display_todo_summary();

            // Check goal completion (todos + file verification)
            // Note: We don't early-exit on "all todos complete" because we need to verify
            // that files were actually created for file-producing goals
            let goal_met = self.should_complete_goal(iteration, max_iterations);
            info!(
                "agent.loop.goal_check goal_met={} iter={}",
                goal_met, iteration
            );

            if goal_met {
                info!("agent.loop.complete criteria_met iter={}", iteration);
                return Ok(());
            }
        }

        self.tui
            .add_log("⚠️ Reached maximum iterations without completing goal".to_string());
        self.display_todo_summary();
        Ok(())
    }

    async fn process_controls(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
    ) -> Result<()> {
        if let Some(rx) = &self.control_rx {
            let mut msgs = Vec::new();
            loop {
                match rx.try_recv().await {
                    Ok(Some(msg)) => msgs.push(msg),
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            for msg in msgs {
                self.handle_control_message(context, msg).await?;
            }
        }
        Ok(())
    }

    async fn handle_control_message(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
        msg: fluent_agent::agent_control::ControlMessage,
    ) -> Result<()> {
        use fluent_agent::agent_control::ControlMessageType;
        match msg.message_type {
            ControlMessageType::Pause => {
                self.paused = true;
                self.tui.update_status(crate::tui::AgentStatus::Paused);
                self.tui.add_log("⏸️ Paused by user".to_string());
            }
            ControlMessageType::Resume => {
                self.paused = false;
                self.tui.update_status(crate::tui::AgentStatus::Running);
                self.tui.add_log("▶️ Resumed by user".to_string());
            }
            ControlMessageType::Input {
                context: _ctx,
                guidance,
                apply_to_future,
            } => {
                if apply_to_future {
                    self.queued_guidance.push(guidance.clone());
                    self.tui
                        .add_log(format!("💬 Guidance queued: {}", guidance));
                } else {
                    context.set_variable("human_guidance".to_string(), guidance.clone());
                    self.tui
                        .add_log(format!("💬 Guidance applied: {}", guidance));
                }
            }
            ControlMessageType::ModifyGoal {
                new_goal,
                keep_context: _,
            } => {
                context.add_context_item("goal_modified".to_string(), new_goal.clone());
                self.tui.set_goal(new_goal.clone());
                self.tui.add_log(format!("🎯 Goal modified by user"));
            }
            _ => {}
        }
        Ok(())
    }

    /// Store observation with sliding window (keep last 5)
    fn store_observation(&mut self, observation: String) {
        const MAX_OBSERVATIONS: usize = 5;
        self.recent_observations.push(observation);

        // Keep only the last MAX_OBSERVATIONS
        if self.recent_observations.len() > MAX_OBSERVATIONS {
            self.recent_observations.remove(0);
        }

        debug!(
            "agent.observations.stored count={} total_stored={}",
            self.recent_observations.len(),
            self.recent_observations.len()
        );
    }

    /// Perform reasoning for current iteration
    async fn perform_reasoning(&mut self, iteration: u32, max_iterations: u32) -> Result<String> {
        use fluent_agent::prompts::{
            format_reasoning_prompt, AGENT_SYSTEM_PROMPT, TOOL_DESCRIPTIONS,
        };

        self.tui
            .set_current_action("Analyzing goal and determining next action...".to_string());
        self.tui
            .add_log("🧠 Analyzing goal and determining next action...".to_string());

        // Get the last 3-5 observations for context
        let observation_window = 5;
        let recent_obs_slice = if self.recent_observations.len() > observation_window {
            &self.recent_observations[self.recent_observations.len() - observation_window..]
        } else {
            &self.recent_observations[..]
        };

        // Use the centralized reasoning prompt with observation feedback
        let user_prompt = format_reasoning_prompt(
            &self.goal.description,
            iteration,
            max_iterations,
            recent_obs_slice,
            TOOL_DESCRIPTIONS,
        );

        // CRITICAL: Include the full system prompt so the LLM knows HOW to reason
        // The system prompt defines the ReAct algorithm and output format
        let full_payload = format!("{}\n\n---\n\n{}", AGENT_SYSTEM_PROMPT, user_prompt);

        let reasoning_request = Request {
            flowname: "agentic_reasoning".to_string(),
            payload: full_payload,
        };

        debug!(
            "agent.reasoning.request flow='{}' len={} observations={}",
            reasoning_request.flowname,
            reasoning_request.payload.len(),
            recent_obs_slice.len()
        );
        match Pin::from(
            self.runtime_config
                .reasoning_engine
                .execute(&reasoning_request),
        )
        .await
        {
            Ok(response) => {
                self.tui
                    .add_log(format!("🤖 Agent reasoning: {}", response.content));
                debug!(
                    "agent.reasoning.response len={} preview='{}'",
                    response.content.len(),
                    &response.content.chars().take(200).collect::<String>()
                );
                Ok(response.content)
            }
            Err(e) => {
                self.tui.add_log(format!("❌ Reasoning failed: {}", e));
                error!("agent.reasoning.error {}", e);
                Err(anyhow!("Reasoning failed: {}", e))
            }
        }
    }

    /// Check if this is a game creation goal
    fn is_game_goal(&self) -> bool {
        let description = self.goal.description.to_lowercase();
        description.contains("game")
            || description.contains("tetris")
            || description.contains("javascript")
            || description.contains("html")
    }

    /// Handle game creation goals
    async fn handle_game_creation(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
    ) -> Result<()> {
        self.tui
            .add_log("🎮 Agent decision: Create the game now!".to_string());

        let mut game_creator = GameCreator::new(
            &self.goal,
            self.runtime_config,
            self.gen_retries,
            self.min_html_size,
            self.tui,
        );
        let file_path = game_creator.create_game(context).await?;

        // Track the created file for completion checking
        if !self.files_created_this_session.contains(&file_path) {
            self.files_created_this_session.push(file_path.clone());
            debug!(
                "agent.session.file_created path='{}' (via legacy game creator)",
                file_path
            );
        }

        Ok(())
    }

    /// Handle general (non-game) goals
    async fn handle_general_goal(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
        reasoning_response: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<()> {
        self.tui
            .set_current_action("Processing complex analytical goal...".to_string());
        self.tui
            .add_log("🔍 Processing complex analytical goal...".to_string());

        let action_response = self
            .plan_action(reasoning_response, iteration, max_iterations)
            .await?;

        if self.goal.description.to_lowercase().contains("reflection") {
            self.handle_reflection_analysis(context, &action_response, iteration, max_iterations)
                .await?;
        } else {
            self.tui.add_log(format!(
                "🔧 Processing general goal: {}",
                self.goal.description
            ));
            self.execute_planned_action(context, &action_response, iteration, max_iterations)
                .await?;
        }

        Ok(())
    }

    /// Plan specific action based on reasoning
    async fn plan_action(
        &mut self,
        reasoning_response: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<String> {
        self.tui
            .set_current_action("Planning specific action...".to_string());

        let action_request = Request {
            flowname: "action_planning".to_string(),
            payload: format!(
                "Based on this goal and reasoning, determine the specific action to take:\n\n\
                Goal: {}\n\
                Reasoning: {}\n\
                Iteration: {}/{}\n\n\
                What specific file should be analyzed, created, or modified? \
                Respond with just the file path and a brief description of what to do with it.",
                self.goal.description, reasoning_response, iteration, max_iterations
            ),
        };

        debug!(
            "agent.action.request flow='{}' len={}",
            action_request.flowname,
            action_request.payload.len()
        );
        match Pin::from(
            self.runtime_config
                .reasoning_engine
                .execute(&action_request),
        )
        .await
        {
            Ok(response) => {
                self.tui
                    .add_log(format!("📋 Planned action: {}", response.content));
                info!(
                    "agent.action.planned first_line='{}'",
                    response.content.lines().next().unwrap_or("")
                );
                Ok(response.content)
            }
            Err(e) => {
                self.tui
                    .add_log(format!("❌ Action planning failed: {}", e));
                error!("agent.action.error {}", e);
                Err(anyhow!("Action planning failed: {}", e))
            }
        }
    }

    /// Handle reflection system analysis
    async fn handle_reflection_analysis(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
        action_response: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<()> {
        let analysis_file = "analysis/reflection_system_analysis.md";

        // Create analysis directory
        if let Err(e) = fs::create_dir_all("analysis") {
            self.tui
                .add_log(format!("⚠️ Could not create analysis directory: {}", e));
        }

        let analysis_response = self
            .perform_reflection_analysis(iteration, max_iterations)
            .await?;
        self.write_analysis_file(
            analysis_file,
            &analysis_response,
            action_response,
            iteration,
        )
        .await?;

        // Store observation for reflection analysis
        let obs = format!(
            "Reflection analysis completed: iteration {}/{} - File: {} - Preview: {}",
            iteration,
            max_iterations,
            analysis_file,
            analysis_response.chars().take(150).collect::<String>()
        );
        self.store_observation(obs);

        // Update context with progress
        context.set_variable("analysis_iteration".to_string(), iteration.to_string());
        context.set_variable("analysis_file".to_string(), analysis_file.to_string());
        context.increment_iteration();

        Ok(())
    }

    /// Perform reflection system analysis
    async fn perform_reflection_analysis(
        &mut self,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<String> {
        let analysis_request = Request {
            flowname: "reflection_analysis".to_string(),
            payload: format!(
                "Conduct a comprehensive analysis of the fluent_cli self-reflection system. \
                Focus on iteration {iteration}/{max_iterations}.\n\n\
                Analyze the following aspects:\n\
                1. Architecture and design patterns\n\
                2. Performance characteristics\n\
                3. Memory usage patterns\n\
                4. Potential bottlenecks\n\
                5. Optimization opportunities\n\n\
                Provide a detailed technical analysis with specific recommendations."
            ),
        };

        match Pin::from(
            self.runtime_config
                .reasoning_engine
                .execute(&analysis_request),
        )
        .await
        {
            Ok(response) => Ok(response.content),
            Err(e) => {
                self.tui.add_log(format!("❌ Analysis failed: {}", e));
                Err(anyhow!("Analysis failed: {}", e))
            }
        }
    }

    /// Write analysis to file
    async fn write_analysis_file(
        &mut self,
        analysis_file: &str,
        analysis_response: &str,
        action_response: &str,
        iteration: u32,
    ) -> Result<()> {
        let analysis_content = format!(
            "# Reflection System Analysis - Iteration {}\n\n\
            Generated: {}\n\n\
            ## Goal\n{}\n\n\
            ## Analysis\n{}\n\n\
            ## Action Taken\n{}\n\n",
            iteration,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            self.goal.description,
            analysis_response,
            action_response
        );

        if let Err(e) = fs::write(analysis_file, &analysis_content) {
            self.tui
                .add_log(format!("❌ Failed to write analysis: {}", e));
            Err(anyhow!("Failed to write analysis: {}", e))
        } else {
            self.tui
                .add_log(format!("✅ Analysis written to: {}", analysis_file));
            self.tui.add_log(format!(
                "📝 Analysis length: {} characters",
                analysis_content.len()
            ));
            Ok(())
        }
    }

    /// Execute the planned action
    async fn execute_planned_action(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
        action_response: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<()> {
        // Parse action response to extract file path and action type
        let (file_path, action_description) = self.parse_action_response(action_response);

        self.tui
            .set_current_action(format!("Executing: {}", action_description));

        if action_description.to_lowercase().contains("create")
            || action_description.to_lowercase().contains("write")
        {
            // Create or write a file
            self.create_research_file(&file_path, &action_description, iteration, max_iterations)
                .await?;
        } else if action_description.to_lowercase().contains("analyze")
            || action_description.to_lowercase().contains("read")
        {
            // Analyze an existing file
            self.analyze_file(&file_path, &action_description).await?;
        } else {
            // For other actions, just log them
            self.tui.add_log(format!(
                "ℹ️ Action type not implemented: {}",
                action_description
            ));
        }

        // Update context
        context.set_variable("last_action".to_string(), action_response.to_string());
        context.set_variable("iteration".to_string(), iteration.to_string());
        context.increment_iteration();

        Ok(())
    }

    /// Parse action response to extract file path and description
    fn parse_action_response(&self, action_response: &str) -> (String, String) {
        // Expected format: "filename.ext - Description of action"
        if let Some(dash_pos) = action_response.find(" - ") {
            let file_path = action_response[..dash_pos].trim().to_string();
            let description = action_response[dash_pos + 3..].trim().to_string();
            (file_path, description)
        } else {
            // Fallback: assume the whole response is the description
            (
                "research_output.md".to_string(),
                action_response.to_string(),
            )
        }
    }

    /// Create a research file with generated content
    async fn create_research_file(
        &mut self,
        file_path: &str,
        description: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<()> {
        self.tui
            .add_log(format!("📝 Creating research file: {}", file_path));

        // Generate content based on the goal and description
        let content = self
            .generate_research_content(description, iteration, max_iterations)
            .await?;

        // Ensure parent directories exist
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                self.tui
                    .add_log(format!("⚠️ Could not create directory {:?}: {}", parent, e));
            }
        }

        // Write to file
        if let Err(e) = fs::write(file_path, &content) {
            self.tui
                .add_log(format!("❌ Failed to write file {}: {}", file_path, e));

            // Store failure observation
            let obs = format!("File creation FAILED: {} - Error: {}", file_path, e);
            self.store_observation(obs);

            return Err(anyhow!("Failed to write research file: {}", e));
        }

        self.tui.add_log(format!(
            "✅ Created {} ({} characters)",
            file_path,
            content.len()
        ));

        // Store success observation
        let obs = format!(
            "File created SUCCESS: {} ({} characters) - {}",
            file_path,
            content.len(),
            description
        );
        self.store_observation(obs);

        Ok(())
    }

    /// Generate research content using LLM
    async fn generate_research_content(
        &mut self,
        description: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<String> {
        let research_request = Request {
            flowname: "research_generation".to_string(),
            payload: format!(
                "You are conducting research on: {}\n\n\
                Goal: {}\n\n\
                Current iteration: {}/{}\n\n\
                Task: {}\n\n\
                Provide comprehensive, well-structured content for this research file. \
                Include relevant facts, analysis, and insights. Be thorough but concise. \
                Format with proper markdown structure including headers, lists, and sections as appropriate.",
                self.goal.description, self.goal.description, iteration, max_iterations, description
            ),
        };

        match Pin::from(
            self.runtime_config
                .reasoning_engine
                .execute(&research_request),
        )
        .await
        {
            Ok(response) => Ok(response.content),
            Err(e) => {
                self.tui
                    .add_log(format!("❌ Research content generation failed: {}", e));
                Err(anyhow!("Research content generation failed: {}", e))
            }
        }
    }

    /// Analyze an existing file
    async fn analyze_file(&mut self, file_path: &str, description: &str) -> Result<()> {
        self.tui
            .add_log(format!("🔍 Analyzing file: {}", file_path));

        // Try to read the file
        match fs::read_to_string(file_path) {
            Ok(content) => {
                self.tui.add_log(format!(
                    "📖 Read {} characters from {}",
                    content.len(),
                    file_path
                ));

                // Generate analysis
                let analysis = self.analyze_content(&content, description).await?;
                self.tui.add_log(format!("📊 Analysis: {}", analysis));

                // Store observation
                let obs = format!(
                    "File analysis SUCCESS: {} ({} chars) - Analysis: {}",
                    file_path,
                    content.len(),
                    analysis.chars().take(150).collect::<String>()
                );
                self.store_observation(obs);
            }
            Err(e) => {
                self.tui
                    .add_log(format!("⚠️ Could not read file {}: {}", file_path, e));

                // Store observation for file not found
                let obs = format!(
                    "File read FAILED: {} - Error: {} - Creating new file",
                    file_path, e
                );
                self.store_observation(obs);

                // Create the file if it doesn't exist
                self.create_research_file(
                    file_path,
                    &format!("Create analysis file: {}", description),
                    1,
                    1,
                )
                .await?;
            }
        }

        Ok(())
    }

    /// Analyze content using LLM
    async fn analyze_content(&mut self, content: &str, description: &str) -> Result<String> {
        let analysis_request = Request {
            flowname: "content_analysis".to_string(),
            payload: format!(
                "Analyze the following content in the context of the goal: {}\n\n\
                Analysis task: {}\n\n\
                Content to analyze:\n{}\n\n\
                Provide a brief analysis focusing on relevance, completeness, and next steps.",
                self.goal.description, description, content
            ),
        };

        match Pin::from(
            self.runtime_config
                .reasoning_engine
                .execute(&analysis_request),
        )
        .await
        {
            Ok(response) => Ok(response.content),
            Err(e) => {
                self.tui
                    .add_log(format!("❌ Content analysis failed: {}", e));
                Err(anyhow!("Content analysis failed: {}", e))
            }
        }
    }

    /// Check if goal should be completed
    ///
    /// Uses dynamic, session-aware completion checking instead of hardcoded patterns.
    /// The agent is complete when:
    /// 1. All todos are completed (primary indicator)
    /// 2. At least one file was created in this session (for file-producing goals)
    /// 3. No todos have failed status
    fn should_complete_goal(&mut self, iteration: u32, _max_iterations: u32) -> bool {
        // Count todo statuses
        let total_todos = self.todo_list.len();
        let completed_todos = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::Completed)
            .count();
        let failed_todos = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::Failed)
            .count();
        let pending_todos = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::Pending)
            .count();
        let in_progress_todos = self
            .todo_list
            .iter()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();

        // Log completion check status
        info!(
            "agent.completion.check iteration={} todos={{total={}, completed={}, failed={}, pending={}, in_progress={}}} files_created={}",
            iteration, total_todos, completed_todos, failed_todos, pending_todos, in_progress_todos, self.files_created_this_session.len()
        );

        // If there are failed todos, we're not complete
        if failed_todos > 0 {
            debug!(
                "agent.completion.blocked reason='failed_todos' count={}",
                failed_todos
            );
            return false;
        }

        // If there are still pending or in-progress todos, we're not complete
        if pending_todos > 0 || in_progress_todos > 0 {
            debug!(
                "agent.completion.blocked reason='incomplete_todos' pending={} in_progress={}",
                pending_todos, in_progress_todos
            );
            return false;
        }

        // All todos must be completed
        if total_todos > 0 && completed_todos == total_todos {
            // Verify we actually created something this session
            if !self.files_created_this_session.is_empty() {
                // Verify created files exist and have content
                for file_path in &self.files_created_this_session {
                    if let Ok(metadata) = fs::metadata(file_path) {
                        if metadata.len() > 100 {
                            self.tui.add_log(format!(
                                "✅ Goal complete: All {} todos done, created {} ({} bytes)",
                                total_todos,
                                file_path,
                                metadata.len()
                            ));
                            info!(
                                "agent.completion.success todos={} files_created={} primary_file='{}' size={}",
                                completed_todos, self.files_created_this_session.len(), file_path, metadata.len()
                            );
                            return true;
                        }
                    }
                }
                // Files were tracked but may not exist (write failed)
                debug!("agent.completion.blocked reason='files_not_verified'");
                return false;
            } else {
                // No files created - check if this is a file-producing goal
                let goal_lower = self.goal.description.to_lowercase();
                let requires_files = goal_lower.contains("create")
                    || goal_lower.contains("write")
                    || goal_lower.contains("build")
                    || goal_lower.contains("make")
                    || goal_lower.contains("implement")
                    || goal_lower.contains("game")
                    || goal_lower.contains("code");

                if requires_files {
                    // Goal requires files but none were created - not complete
                    debug!(
                        "agent.completion.blocked reason='file_producing_goal_no_files' goal='{}'",
                        self.goal.description
                    );
                    self.tui
                        .add_log("⏳ Waiting for file creation...".to_string());
                    return false;
                } else {
                    // Non-file-producing goal (analysis, research, etc.)
                    // Complete if we've done at least 2 iterations of work
                    if iteration >= 2 {
                        self.tui.add_log(format!(
                            "✅ Goal complete: All {} todos done (no files required)",
                            total_todos
                        ));
                        return true;
                    }
                }
            }
        }

        // Not complete yet
        false
    }
}

/// Game creation handler
pub struct GameCreator<'a> {
    goal: &'a fluent_agent::goal::Goal,
    runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
    gen_retries: u32,
    min_html_size: usize,
    tui: &'a mut TuiManager,
}

impl<'a> GameCreator<'a> {
    pub fn new(
        goal: &'a fluent_agent::goal::Goal,
        runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
        gen_retries: u32,
        min_html_size: usize,
        tui: &'a mut TuiManager,
    ) -> Self {
        Self {
            goal,
            runtime_config,
            gen_retries,
            min_html_size,
            tui,
        }
    }

    /// Create game based on goal description
    /// Create the game and return the file path where it was written
    pub async fn create_game(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
    ) -> Result<String> {
        let (file_extension, code_prompt, file_path) =
            Self::determine_game_type(&self.goal.description);

        self.tui.add_log(format!(
            "🎮 Creating {} game: {}",
            file_extension.to_uppercase(),
            file_path
        ));

        info!(
            "agent.codegen.select type='{}' path='{}'",
            file_extension, file_path
        );

        // Generate game code
        let game_code = self
            .generate_game_code(&code_prompt, &file_extension)
            .await?;
        debug!(
            "agent.codegen.generated len={} ext='{}'",
            game_code.len(),
            file_extension
        );

        // Ensure output directory exists
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write game file
        self.write_game_file(&file_path, &game_code)?;

        // Update context
        self.update_context(context, &file_path, &file_extension);

        // Log success
        self.tui.add_log(format!(
            "🎉 Goal achieved! {} game created at {}!",
            file_extension.to_uppercase(),
            file_path
        ));
        Ok(file_path)
    }

    /// Determine what type of game to create based on goal description
    /// Returns (file_extension, code_prompt, output_path)
    fn determine_game_type(goal_description: &str) -> (String, String, String) {
        let description = goal_description.to_lowercase();

        /// Get game-specific requirements to help LLM produce correct game type
        fn get_game_specific_requirements(game_name: &str) -> &'static str {
            match game_name {
                "solitaire" => {
                    "\
                    - Implement Klondike Solitaire (the classic single-player card game)\n\
                    - Use a standard 52-card deck with 4 suits (hearts, diamonds, clubs, spades)\n\
                    - Create 7 tableau piles, 4 foundation piles, and a stock/waste pile\n\
                    - Cards alternate red/black in tableau, same suit ascending in foundations\n\
                    - Allow dragging cards between piles with mouse click/drag\n\
                    - Deal 3 cards at a time from stock to waste\n\
                    - Win condition: all cards moved to foundations (Ace to King)"
                }
                "tetris" => {
                    "\
                    - Standard 10x20 playing field\n\
                    - 7 tetromino pieces: I, O, T, S, Z, J, L\n\
                    - Piece rotation with wall kicks\n\
                    - Gravity/falling pieces with increasing speed\n\
                    - Line clear detection and scoring\n\
                    - Ghost piece showing where piece will land\n\
                    - Next piece preview"
                }
                "snake" => {
                    "\
                    - Snake that grows when eating food\n\
                    - Arrow keys or WASD for direction control\n\
                    - Random food spawning\n\
                    - Game over on wall or self collision\n\
                    - Score based on food eaten\n\
                    - Increasing speed as snake grows"
                }
                "pong" => {
                    "\
                    - Two paddles (left/right or top/bottom)\n\
                    - Ball bouncing off paddles and walls\n\
                    - Score tracking for both players\n\
                    - Ball speed increases over time\n\
                    - Player vs CPU or 2-player mode\n\
                    - Win condition (first to score X points)"
                }
                "breakout" => {
                    "\
                    - Paddle at bottom controlled by mouse/keyboard\n\
                    - Ball bouncing off paddle, walls, and bricks\n\
                    - Grid of breakable bricks\n\
                    - Different brick types (colors, hit points)\n\
                    - Power-ups dropping from bricks\n\
                    - Multiple lives, score tracking"
                }
                "minesweeper" => {
                    "\
                    - Grid of cells with hidden mines\n\
                    - Left-click to reveal, right-click to flag\n\
                    - Numbers showing adjacent mine count\n\
                    - Cascade reveal for zero-adjacent cells\n\
                    - Win by revealing all non-mine cells\n\
                    - Lose by clicking a mine\n\
                    - Timer and mine counter display"
                }
                "tower" => {
                    "\
                    - Tower defense game with waves of enemies\n\
                    - Path that enemies follow from start to end\n\
                    - Multiple tower types with different stats (damage, range, fire rate)\n\
                    - Tower placement system using mouse click\n\
                    - Projectiles that towers fire at enemies\n\
                    - Money system for buying towers, earned from kills\n\
                    - Lives that decrease when enemies reach the end\n\
                    - Wave system with increasing difficulty"
                }
                "space" | "shooter" => {
                    "\
                    - Player-controlled ship or character\n\
                    - Shooting mechanics with projectiles\n\
                    - Enemies that spawn and move\n\
                    - Collision detection for bullets and enemies\n\
                    - Score tracking and lives system\n\
                    - Increasing difficulty over time"
                }
                _ => {
                    "\
                    - Complete, playable game implementation\n\
                    - Clear game mechanics and rules\n\
                    - User input handling\n\
                    - Score tracking and game over conditions\n\
                    - Visual feedback for game state"
                }
            }
        }

        // Detect target platform/language
        let wants_web = description.contains("javascript")
            || description.contains("html")
            || description.contains("web")
            || description.contains("browser");
        let wants_love2d = description.contains("love2d")
            || description.contains("löve")
            || description.contains("love 2d");
        let wants_lua = description.contains("lua") || wants_love2d;
        let wants_python = description.contains("python") || description.contains("pygame");

        // Detect game type (with common typo tolerance)
        let game_name = if description.contains("solitaire")
            || description.contains("solitare")
            || description.contains("klondike")
        {
            "solitaire"
        } else if description.contains("tetris") || description.contains("tetros") {
            "tetris"
        } else if description.contains("snake") {
            "snake"
        } else if description.contains("pong") {
            "pong"
        } else if description.contains("breakout")
            || description.contains("arkanoid")
            || description.contains("brick")
        {
            "breakout"
        } else if description.contains("minesweeper") || description.contains("mine sweeper") {
            "minesweeper"
        } else if description.contains("tower") || description.contains("defense") {
            "tower"
        } else if description.contains("space") || description.contains("shooter") {
            "space"
        } else {
            // Extract game name from description if possible
            "game"
        };

        // Determine file extension and output path based on platform
        let (ext, output_path) = if wants_love2d || wants_lua {
            (
                "lua".to_string(),
                format!("outputs/{}_love2d/main.lua", game_name),
            )
        } else if wants_python {
            ("py".to_string(), format!("outputs/{}_pygame.py", game_name))
        } else if wants_web {
            (
                "html".to_string(),
                format!("outputs/{}_web.html", game_name),
            )
        } else {
            ("rs".to_string(), format!("outputs/{}_game.rs", game_name))
        };

        // Get game-specific requirements to help LLM stay on track
        let game_requirements = get_game_specific_requirements(game_name);

        // Generate appropriate code prompt based on platform and game
        let code_prompt = if wants_love2d {
            format!(
                "IMPORTANT: You MUST create a {} game. Do NOT create any other type of game.\n\n\
                Create a complete, working {} game using the LÖVE (Love2D) framework in Lua.\n\n\
                Game-Specific Requirements for {}:\n\
                {}\n\n\
                Technical Requirements:\n\
                - Create main.lua with all game logic\n\
                - Implement proper love.load(), love.update(dt), love.draw(), and love.keypressed(key)/love.mousepressed(x,y,button) callbacks\n\
                - Use love.graphics for rendering cards/pieces/game elements\n\
                - Handle keyboard AND mouse input appropriately\n\
                - Include scoring and game state management\n\
                - Add comments explaining the code structure\n\n\
                CRITICAL: This MUST be a {} game. Provide ONLY the complete Lua code wrapped in:\n\
                ```lua\n\
                ... full main.lua code for {} ...\n\
                ```",
                game_name.to_uppercase(), game_name, game_name, game_requirements, game_name, game_name
            )
        } else if wants_python {
            format!(
                "IMPORTANT: You MUST create a {} game. Do NOT create any other type of game.\n\n\
                Create a complete, working {} game using Python and Pygame.\n\n\
                Game-Specific Requirements for {}:\n\
                {}\n\n\
                Technical Requirements:\n\
                - Single Python file with all game logic\n\
                - Initialize pygame properly\n\
                - Implement game loop with event handling, update, and draw phases\n\
                - Handle keyboard AND mouse input appropriately\n\
                - Include scoring and game state management\n\n\
                CRITICAL: This MUST be a {} game. Provide ONLY the complete Python code wrapped in:\n\
                ```python\n\
                ... full code for {} ...\n\
                ```",
                game_name.to_uppercase(), game_name, game_name, game_requirements, game_name, game_name
            )
        } else if wants_web {
            format!(
                "IMPORTANT: You MUST create a {} game. Do NOT create any other type of game.\n\n\
                Create a complete, working {} game using HTML5, CSS, and JavaScript.\n\n\
                Game-Specific Requirements for {}:\n\
                {}\n\n\
                Technical Requirements:\n\
                - Single HTML file with embedded CSS and JavaScript\n\
                - Use HTML5 Canvas for rendering\n\
                - Handle keyboard AND mouse input appropriately\n\
                - Scoring system and game state management\n\
                - Clean, well-structured code with comments\n\n\
                CRITICAL: This MUST be a {} game. Provide ONLY the complete HTML file wrapped in:\n\
                ```html\n\
                ... full HTML for {} ...\n\
                ```",
                game_name.to_uppercase(),
                game_name,
                game_name,
                game_requirements,
                game_name,
                game_name
            )
        } else {
            format!(
                "IMPORTANT: You MUST create a {} game. Do NOT create any other type of game.\n\n\
                Create a complete, working {} game in Rust.\n\n\
                Game-Specific Requirements for {}:\n\
                {}\n\n\
                Technical Requirements:\n\
                - Terminal-based interface using crossterm crate\n\
                - Keyboard controls for gameplay\n\
                - Scoring system and game state management\n\
                - Clean game loop with non-blocking input\n\n\
                CRITICAL: This MUST be a {} game. Provide ONLY the complete Rust code wrapped in:\n\
                ```rust\n\
                ... full code for {} ...\n\
                ```",
                game_name.to_uppercase(),
                game_name,
                game_name,
                game_requirements,
                game_name,
                game_name
            )
        };

        (ext, code_prompt, output_path)
    }

    /// Generate game code using LLM
    async fn generate_game_code(
        &mut self,
        code_prompt: &str,
        file_extension: &str,
    ) -> Result<String> {
        info!(
            "agent.codegen.start ext='{}' retries={}",
            file_extension, self.gen_retries
        );
        let code_request = Request {
            flowname: "code_generation".to_string(),
            payload: code_prompt.to_string(),
        };

        self.tui.add_log(format!(
            "🧠 Generating {} game code with selected LLM...",
            file_extension.to_uppercase()
        ));

        // Helper: try execute with retry/backoff
        async fn try_execute_with_retry(
            engine: &Box<dyn fluent_core::traits::Engine>,
            req: &Request,
            attempts: u32,
            tui: &mut TuiManager,
        ) -> Result<fluent_core::types::Response> {
            let mut delay = 500u64;
            let max_attempts = attempts.max(1);
            let mut last_err: Option<anyhow::Error> = None;
            for attempt in 1..=max_attempts {
                debug!("agent.codegen.attempt {} of {}", attempt, max_attempts);
                match Pin::from(engine.execute(req)).await {
                    Ok(resp) => return Ok(resp),
                    Err(e) => {
                        last_err = Some(e);
                        if attempt < max_attempts {
                            tui.add_log(format!(
                                "⚠️ LLM request failed (attempt {}/{}). Retrying in {}ms...",
                                attempt, max_attempts, delay
                            ));
                            warn!(
                                "agent.codegen.retry attempt={}/{} delay_ms={}",
                                attempt, max_attempts, delay
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                            delay *= 2;
                        }
                    }
                }
            }
            Err(anyhow::anyhow!(format!(
                "LLM request failed after retries: {}",
                last_err.unwrap_or_else(|| anyhow::anyhow!("unknown error"))
            )))
        }

        // First attempt with retry
        let mut code_response = try_execute_with_retry(
            self.runtime_config.reasoning_engine.as_ref(),
            &code_request,
            self.gen_retries,
            self.tui,
        )
        .await?;
        let mut game_code = crate::utils::extract_code(&code_response.content, file_extension);
        debug!(
            "agent.codegen.extracted len={} ext='{}'",
            game_code.len(),
            file_extension
        );

        // Validate game output matches expected game type
        let desc = self.goal.description.to_lowercase();
        let lc = game_code.to_lowercase();

        // Detect expected game type
        let expected_game = if desc.contains("solitaire") || desc.contains("klondike") {
            "solitaire"
        } else if desc.contains("tetris") {
            "tetris"
        } else if desc.contains("snake") {
            "snake"
        } else if desc.contains("pong") {
            "pong"
        } else if desc.contains("breakout") || desc.contains("arkanoid") {
            "breakout"
        } else if desc.contains("minesweeper") {
            "minesweeper"
        } else {
            "game"
        };

        // Check if generated code matches expected game type
        let valid = validate_game_output(&lc, expected_game, file_extension, self.min_html_size);
        debug!(
            "agent.codegen.validate expected='{}' ext='{}' valid={}",
            expected_game, file_extension, valid
        );

        if !valid {
            self.tui.add_log(format!(
                "⚠️ Output doesn't match expected {} game. Requesting refinement...",
                expected_game.to_uppercase()
            ));
            info!(
                "agent.codegen.refine ext='{}' expected_game='{}'",
                file_extension, expected_game
            );

            // Get the game-specific requirements for the refinement prompt
            let game_requirements = Self::get_refinement_requirements(expected_game);
            let lang_hint = match file_extension {
                "lua" => "```lua```",
                "html" => "```html```",
                "py" | "python" => "```python```",
                "rs" | "rust" => "```rust```",
                _ => "```",
            };

            let refine_prompt = format!(
                "CRITICAL ERROR: Your previous output was NOT a {} game. You MUST regenerate as a {} game.\n\n\
                 The output MUST be a {} game with these specific features:\n\
                 {}\n\n\
                 Do NOT create any other type of game (no shooters, no space games, no action games unless that is what was requested).\n\n\
                 Provide ONLY the full source in one block, no prose. Wrap it in a fenced block: {}\n",
                expected_game.to_uppercase(),
                expected_game,
                expected_game,
                game_requirements,
                lang_hint
            );

            let refine_request = Request {
                flowname: "code_generation_refine".to_string(),
                payload: refine_prompt,
            };
            code_response = try_execute_with_retry(
                self.runtime_config.reasoning_engine.as_ref(),
                &refine_request,
                self.gen_retries,
                self.tui,
            )
            .await?;
            game_code = crate::utils::extract_code(&code_response.content, file_extension);

            // Re-validate refined output
            let lc2 = game_code.to_lowercase();
            let still_invalid =
                !validate_game_output(&lc2, expected_game, file_extension, self.min_html_size);
            if still_invalid {
                self.tui.add_log("⚠️ Refined output still doesn't match expected game. Writing raw response for inspection.".to_string());
                warn!(
                    "agent.codegen.refine_failed expected_game='{}' writing_raw=true",
                    expected_game
                );
                game_code = code_response.content;
            }
        }

        Ok(game_code)
    }

    /// Get game-specific requirements for refinement prompt
    fn get_refinement_requirements(game_name: &str) -> &'static str {
        match game_name {
            "solitaire" => {
                "\
                - Klondike Solitaire card game\n\
                - 52-card deck, 4 suits (hearts, diamonds, clubs, spades)\n\
                - 7 tableau piles, 4 foundation piles, stock and waste\n\
                - Card dragging with mouse\n\
                - Deal 3 cards from stock\n\
                - Alternating colors in tableau, same suit in foundations"
            }
            "tetris" => {
                "\
                - 10x20 grid, 7 tetrominoes (I, O, T, S, Z, J, L)\n\
                - Piece rotation with wall kicks\n\
                - Gravity and lock delay\n\
                - Line clear detection and scoring\n\
                - Arrow keys for movement, up for rotate"
            }
            "snake" => {
                "\
                - Snake that grows when eating food\n\
                - Arrow keys or WASD for direction\n\
                - Random food spawning\n\
                - Game over on wall or self collision\n\
                - Score display"
            }
            "pong" => {
                "\
                - Two paddles, left and right\n\
                - Ball bouncing off paddles and walls\n\
                - Score tracking for both players\n\
                - W/S and Up/Down for controls\n\
                - AI opponent option"
            }
            "breakout" => {
                "\
                - Paddle at bottom\n\
                - Ball bouncing\n\
                - Grid of breakable bricks\n\
                - Mouse or arrow keys for paddle\n\
                - Multiple lives, score tracking"
            }
            "minesweeper" => {
                "\
                - Grid of cells with hidden mines\n\
                - Left-click to reveal, right-click to flag\n\
                - Numbers showing adjacent mine count\n\
                - Cascade reveal for zero-adjacent cells\n\
                - Win/lose conditions"
            }
            _ => {
                "\
                - Complete, playable game\n\
                - Clear game mechanics\n\
                - User input handling\n\
                - Score tracking"
            }
        }
    }

    /// Write game code to file
    fn write_game_file(&mut self, file_path: &str, game_code: &str) -> Result<()> {
        fs::write(file_path, game_code)?;
        info!(
            "agent.codegen.file_written path='{}' bytes={}",
            file_path,
            game_code.len()
        );
        self.tui
            .add_log(format!("✅ Created game at: {}", file_path));
        self.tui.add_log(format!(
            "📝 Game code length: {} characters",
            game_code.len()
        ));
        Ok(())
    }

    /// Update execution context with game creation info
    fn update_context(
        &self,
        context: &mut fluent_agent::context::ExecutionContext,
        file_path: &str,
        file_extension: &str,
    ) {
        context.set_variable("game_created".to_string(), "true".to_string());
        context.set_variable("game_path".to_string(), file_path.to_string());
        context.set_variable("game_type".to_string(), file_extension.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_api_error_billing() {
        // Credit balance errors
        assert_eq!(
            classify_api_error("Your credit balance is too low to access the Anthropic API"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Please go to Plans & Billing to purchase credits"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Quota exceeded for your organization"),
            ApiErrorKind::NonRecoverable
        );
    }

    #[test]
    fn test_classify_api_error_auth() {
        // Authentication errors
        assert_eq!(
            classify_api_error("Invalid API key provided"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Unauthorized: invalid_api_key"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Authentication failed: invalid bearer token"),
            ApiErrorKind::NonRecoverable
        );
    }

    #[test]
    fn test_classify_api_error_account() {
        // Account issues (note: patterns are "account suspended", "account disabled", "access denied")
        assert_eq!(
            classify_api_error("Your account suspended for policy violation"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Account disabled due to terms of service"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Access denied: insufficient permissions"),
            ApiErrorKind::NonRecoverable
        );
    }

    #[test]
    fn test_classify_api_error_transient() {
        // Rate limiting - transient
        assert_eq!(
            classify_api_error("Rate limit exceeded, please retry"),
            ApiErrorKind::Transient
        );
        assert_eq!(
            classify_api_error("429 Too Many Requests"),
            ApiErrorKind::Transient
        );

        // Network errors - transient (note: patterns are "timeout", "connection refused", "network error")
        assert_eq!(
            classify_api_error("Request timeout after 30 seconds"),
            ApiErrorKind::Transient
        );
        assert_eq!(
            classify_api_error("Connection refused by remote server"),
            ApiErrorKind::Transient
        );
        assert_eq!(
            classify_api_error("A network error occurred"),
            ApiErrorKind::Transient
        );
    }

    #[test]
    fn test_classify_api_error_unknown() {
        // Unknown errors
        assert_eq!(
            classify_api_error("Some random error message"),
            ApiErrorKind::Unknown
        );
        assert_eq!(
            classify_api_error("Internal server error"),
            ApiErrorKind::Unknown
        );
    }

    #[test]
    fn test_classify_api_error_case_insensitive() {
        // Should be case insensitive
        assert_eq!(
            classify_api_error("CREDIT BALANCE is too low"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("INVALID API KEY"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("RATE LIMIT exceeded"),
            ApiErrorKind::Transient
        );
    }

    #[test]
    fn test_get_api_error_guidance() {
        // Test guidance messages
        assert!(get_api_error_guidance("credit balance is too low").contains("credits"));
        assert!(get_api_error_guidance("invalid api key").contains("API key"));
        assert!(get_api_error_guidance("unauthorized").contains("Authentication"));
        assert!(get_api_error_guidance("account suspended").contains("Account"));
        assert!(get_api_error_guidance("unknown error").contains("API"));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retry_config_exponential_backoff() {
        let config = RetryConfig::default();

        // First attempt: 1000ms
        let delay0 = config.delay_for_attempt(0);
        assert_eq!(delay0.as_millis(), 1000);

        // Second attempt: 2000ms (1000 * 2)
        let delay1 = config.delay_for_attempt(1);
        assert_eq!(delay1.as_millis(), 2000);

        // Third attempt: 4000ms (1000 * 2^2)
        let delay2 = config.delay_for_attempt(2);
        assert_eq!(delay2.as_millis(), 4000);

        // Fourth attempt: 8000ms (1000 * 2^3)
        let delay3 = config.delay_for_attempt(3);
        assert_eq!(delay3.as_millis(), 8000);
    }

    #[test]
    fn test_retry_config_max_delay_cap() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay_ms: 1000,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        };

        // After many retries, delay should cap at max_delay_ms
        let delay10 = config.delay_for_attempt(10);
        assert_eq!(delay10.as_millis(), 5000); // Capped at max
    }

    #[test]
    fn test_get_transient_error_message_rate_limit() {
        assert!(get_transient_error_message("rate limit exceeded").contains("Rate limit"));
        assert!(get_transient_error_message("too many requests").contains("Rate limit"));
        assert!(get_transient_error_message("Error 429: too many requests").contains("Rate limit"));
    }

    #[test]
    fn test_get_transient_error_message_timeout() {
        assert!(get_transient_error_message("request timeout").contains("timed out"));
        assert!(get_transient_error_message("connection timeout").contains("timed out"));
    }

    #[test]
    fn test_get_transient_error_message_network() {
        assert!(get_transient_error_message("connection refused").contains("Network"));
        assert!(get_transient_error_message("network error").contains("Network"));
        assert!(get_transient_error_message("connection reset by peer").contains("Network"));
    }

    #[test]
    fn test_get_transient_error_message_unknown() {
        // Unknown transient errors should get a generic retry message
        assert!(get_transient_error_message("some unknown error").contains("Transient"));
    }
}
