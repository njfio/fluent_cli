//! Agentic mode operations and autonomous execution
//!
//! This module wires the fluent CLI into the `fluent-agent` orchestrator.
//!
//! Design goals:
//! - Single execution loop: `AgentOrchestrator::execute_goal` is the source of truth
//! - Generalist agent: no special-case goal branches (e.g., "game" mode)
//! - MCP is just another tool source (registered into the same ToolRegistry)

use anyhow::{anyhow, Result};
use fluent_core::config::Config;
use fluent_core::types::Request;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{error, info, warn};

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
    if lower.contains("rate limit") || lower.contains("too many requests") || lower.contains("429")
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
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Calculate delay for a given retry attempt (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        let delay_ms =
            (self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32)) as u64;
        let capped_delay_ms = delay_ms.min(self.max_delay_ms);
        std::time::Duration::from_millis(capped_delay_ms)
    }
}

/// Configuration for agentic mode execution
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
    /// Optional model override for default engines (e.g., gpt-4o, claude-3-5-sonnet-...)
    pub model_override: Option<String>,
    /// Optional max retries for LLM code generation
    pub gen_retries: Option<u32>,
    /// Optional minimum HTML size for validation (kept for backward compatibility)
    pub min_html_size: Option<u32>,
    /// If true, simulate actions without side effects
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
    #[allow(clippy::too_many_arguments)]
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

/// Agentic mode executor (CLI-facing wrapper around the fluent-agent orchestrator)
pub struct AgenticExecutor {
    config: AgenticConfig,
    tui: TuiManager,
}

impl AgenticExecutor {
    pub fn new(config: AgenticConfig, enable_tui: bool) -> Self {
        Self {
            config,
            tui: TuiManager::new(enable_tui),
        }
    }

    pub async fn run(&mut self, _fluent_config: &Config) -> Result<()> {
        if self.tui.enabled() {
            self.tui
                .add_log("🚀 AgenticExecutor::run() called".to_string());
        } else {
            println!("🚀 AgenticExecutor::run() called");
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
            }
            self.tui = TuiManager::new(false);
        }

        self.tui.set_goal(self.config.goal_description.clone());
        self.tui
            .set_features(self.config.enable_tools, self.config.enable_reflection);
        self.tui.update_status(AgentStatus::Initializing);
        self.tui.add_log("🤖 Starting Agentic Mode".to_string());

        // Spawn SimpleTUI in background if available
        let tui_handle = self.tui.spawn_simple_tui();
        if tui_handle.is_some() {
            self.tui
                .add_log("✅ SimpleTUI running in background - Press 'Q' to quit".to_string());
        }

        let agent_config = self.load_agent_configuration().await?;
        let credentials = self.load_and_validate_credentials(&agent_config).await?;
        let runtime_config = self
            .create_runtime_configuration(&agent_config, credentials)
            .await?;

        // Fail-fast-ish engine test (warn and continue)
        if let Err(e) = self.test_engines(&runtime_config).await {
            self.tui.add_log(format!("⚠️ Engine test failed: {}", e));
            warn!("agent.engine.test_failed error='{}'", e);
        }

        let mut goal = self.create_goal()?;

        // Build tool registry
        use fluent_agent::tools::ToolRegistry;
        let mut tool_registry = if self.config.enable_tools {
            ToolRegistry::with_standard_tools(&runtime_config.config.tools)
        } else {
            ToolRegistry::new()
        };

        // Workflow macro-tools (LLM-powered tools)
        if self.config.enable_tools {
            use fluent_agent::tools::ToolExecutionConfig;
            let workflow_config = ToolExecutionConfig {
                timeout_seconds: 60,
                max_output_size: 10 * 1024 * 1024, // 10MB
                allowed_paths: runtime_config
                    .config
                    .tools
                    .allowed_paths
                    .clone()
                    .unwrap_or_else(|| vec!["./".to_string()]),
                allowed_commands: vec![],
                read_only: self.config.dry_run,
            };

            let workflow_exec = Arc::new(fluent_agent::tools::WorkflowExecutor::new(
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
                // Attempt auto-connect from config file
                if let Err(e) =
                    Self::auto_connect_mcp_servers(&self.config.config_path, &manager).await
                {
                    self.tui
                        .add_log(format!("⚠️ MCP auto-connect skipped: {}", e));
                }

                use fluent_agent::tools::ToolExecutionConfig;
                let mcp_policy = ToolExecutionConfig {
                    timeout_seconds: 60,
                    max_output_size: 1024 * 1024,
                    allowed_paths: runtime_config
                        .config
                        .tools
                        .allowed_paths
                        .clone()
                        .unwrap_or_else(|| vec!["./".to_string()]),
                    allowed_commands: runtime_config
                        .config
                        .tools
                        .allowed_commands
                        .clone()
                        .unwrap_or_default(),
                    read_only: self.config.dry_run,
                };

                let mcp_exec = Arc::new(fluent_agent::adapters::McpRegistryExecutor::new(
                    manager.clone(),
                    mcp_policy,
                ));
                tool_registry.register("mcp".to_string(), mcp_exec);
                self.tui
                    .add_log("🔌 MCP integrated: remote tools available via registry".to_string());
            } else {
                self.tui
                    .add_log("⚠️ MCP integration skipped (initialization failed)".to_string());
            }
        }

        let arc_registry = Arc::new(tool_registry);

        // Inject dynamic tool info + project identity into goal metadata (used by orchestrator prompt building)
        {
            let tools = arc_registry.get_all_available_tools();
            let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
            let tool_md = render_tool_descriptions_markdown(&tools);
            goal.add_metadata("available_tools".to_string(), serde_json::json!(tool_names));
            goal.add_metadata(
                "tool_descriptions_markdown".to_string(),
                serde_json::json!(tool_md),
            );

            let ident = fluent_agent::project_identity::compute_project_identity();
            goal.add_metadata(
                "project_id".to_string(),
                serde_json::json!(ident.project_id),
            );
            goal.add_metadata(
                "project_id_source".to_string(),
                serde_json::json!(format!("{:?}", ident.source)),
            );
            if let Some(remote) = ident.git_remote {
                goal.add_metadata("git_remote".to_string(), serde_json::json!(remote));
            }
            if let Some(branch) = ident.git_branch {
                goal.add_metadata("git_branch".to_string(), serde_json::json!(branch));
            }
        }

        // Build orchestrator
        use fluent_agent::action::{
            ActionExecutor, ActionPlanner, ComprehensiveActionExecutor, IntelligentActionPlanner,
        };
        use fluent_agent::adapters::{
            CompositePlanner, FsFileManager, LlmCodeGenerator, RegistryToolAdapter,
            SimpleRiskAssessor,
        };
        use fluent_agent::observation::{
            BasicImpactAssessor, BasicLearningExtractor, BasicPatternDetector, BasicResultAnalyzer,
            ComprehensiveObservationProcessor,
        };
        use fluent_agent::{
            AgentOrchestrator, MemorySystem, ReflectionEngine, StateManager, StateManagerConfig,
        };

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

        // Memory and state (SQLite-first global persistence is implemented next)
        use fluent_agent::memory::{
            CompressorConfig, MemoryConfig, PersistenceConfig, WorkingMemoryConfig,
        };

        self.tui
            .add_log("🧠 Initializing memory system...".to_string());

        let memory_config = MemoryConfig {
            working_config: WorkingMemoryConfig {
                max_active_items: 50,
                max_memory_size: 1024 * 1024 * 100,
                attention_refresh_interval: 60,
                relevance_decay_rate: 0.1,
                enable_consolidation: true,
                consolidation_threshold: 0.8,
                enable_predictive_loading: true,
            },
            compressor_config: CompressorConfig {
                max_context_size: 10 * 1024 * 1024,
                target_compression_ratio: 0.3,
                enable_semantic_compression: true,
                enable_temporal_compression: true,
                min_information_retention: 0.8,
                analysis_window_size: 100,
            },
            persistence_config: {
                let env_off = std::env::var("FLUENT_AGENT_MEMORY")
                    .ok()
                    .map(|v| {
                        v.eq_ignore_ascii_case("off") || v == "0" || v.eq_ignore_ascii_case("false")
                    })
                    .unwrap_or(false);
                let enabled = runtime_config.config.memory_enabled && !env_off;
                let database_path = runtime_config.config.resolve_memory_db_path();

                if enabled {
                    self.tui.add_log(format!(
                        "💾 Memory persistence enabled (SQLite): {}",
                        database_path.display()
                    ));
                } else {
                    self.tui
                        .add_log("💾 Memory persistence disabled".to_string());
                }

                PersistenceConfig {
                    enabled,
                    database_path,
                    enable_automatic_save: true,
                    save_interval_secs: 300,
                    max_session_history: 100,
                    enable_compression: true,
                    enable_learning_persistence: true,
                    backup_retention_days: 30,
                }
            },
        };

        let memory = MemorySystem::from_components(
            runtime_config.reasoning_engine.clone(),
            memory_config.working_config,
            memory_config.compressor_config,
            memory_config.persistence_config,
        );
        memory.initialize().await?;
        self.tui.add_log(
            "✅ Memory system initialized with working memory, compression, and persistence"
                .to_string(),
        );

        let state_mgr = StateManager::new(StateManagerConfig::default()).await?;
        let reflection = ReflectionEngine::new();

        let orchestrator = AgentOrchestrator::from_config(
            runtime_config.clone(),
            planner,
            action_executor,
            obs,
            Arc::new(memory),
            Arc::new(state_mgr),
            reflection,
            None,
            None,
            None,
        )
        .await?;

        self.tui
            .add_log("🔁 Orchestrator constructed. Executing goal…".to_string());

        // Max total runtime - safety limit (default 1 hour)
        let max_runtime_secs: u64 = std::env::var("FLUENT_AGENT_MAX_RUNTIME_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600);

        info!(
            "agent.react.start goal='{}' max_runtime={}s",
            self.config.goal_description, max_runtime_secs
        );

        self.tui.update_status(AgentStatus::Running);

        let result = match tokio::time::timeout(
            std::time::Duration::from_secs(max_runtime_secs),
            orchestrator.execute_goal(goal),
        )
        .await
        {
            Ok(Ok(goal_result)) => {
                self.tui.update_status(AgentStatus::Completed);
                self.tui.add_log(format!(
                    "✅ Goal completed. Success: {} | Actions: {} | Reasoning steps: {}",
                    goal_result.success, goal_result.actions_taken, goal_result.reasoning_steps
                ));
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
                    max_runtime_secs, self.config.goal_description
                );
                self.tui.update_status(AgentStatus::Timeout);
                self.tui.add_log(format!(
                    "⏳ Agent exceeded max runtime of {}s (safety limit). Aborting.",
                    max_runtime_secs
                ));
                Err(anyhow!(format!(
                    "Agent exceeded max runtime of {}s",
                    max_runtime_secs
                )))
            }
        };

        self.tui.cleanup()?;
        result
    }

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

        let required_engines = vec![
            agent_config.reasoning_engine.clone(),
            agent_config.action_engine.clone(),
            agent_config.reflection_engine.clone(),
        ];
        credentials::validate_credentials(&credentials, &required_engines)?;

        Ok(credentials)
    }

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

    fn create_goal(&mut self) -> Result<fluent_agent::goal::Goal> {
        use fluent_agent::goal::{Goal, GoalType};

        let mut builder = Goal::builder(
            self.config.goal_description.clone(),
            GoalType::CodeGeneration,
        )
        .max_iterations(self.config.max_iterations);

        // Load success criteria from env (optional)
        if let Ok(sc) = std::env::var("FLUENT_AGENT_SUCCESS_CRITERIA") {
            for criterion in sc.split("||").filter(|s| !s.is_empty()) {
                builder = builder.success_criterion(criterion.to_string());
            }
        } else {
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

    async fn test_engines(
        &mut self,
        runtime_config: &fluent_agent::config::AgentRuntimeConfig,
    ) -> Result<()> {
        self.tui
            .add_log("🧠 Testing reasoning engine...".to_string());

        let test_request = Request {
            flowname: "agentic_test".to_string(),
            payload: "Hello! Please respond with 'Agentic mode is working!' to confirm the engine is operational."
                .to_string(),
        };

        match Pin::from(runtime_config.reasoning_engine.execute(&test_request)).await {
            Ok(response) => {
                self.tui.add_log(format!(
                    "✅ Reasoning engine response: {}",
                    response.content
                ));
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

    /// Attempt to auto-connect MCP servers based on entries in the main config file.
    /// Supported schema (TOML via fluent_config.toml converted to JSON Value):
    /// mcp:
    ///   servers:
    ///     - { name: "search", command: "my-mcp-server", args: ["--stdio"] }
    ///     - "search:my-mcp-server --stdio"
    async fn auto_connect_mcp_servers(
        config_path: &str,
        manager: &Arc<fluent_agent::production_mcp::ProductionMcpManager>,
    ) -> anyhow::Result<()> {
        use serde_json::Value;

        info!("agent.mcp.autoconnect.config path='{}'", config_path);
        let content = tokio::fs::read_to_string(config_path).await?;
        let root: Value = if content.trim_start().starts_with('{') {
            serde_json::from_str(&content)?
        } else {
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
                            let mut parts = s.splitn(2, ':');
                            let name = parts.next().unwrap_or("");
                            let cmd_and_args = parts.next().unwrap_or("").trim();
                            if name.is_empty() || cmd_and_args.is_empty() {
                                continue;
                            }
                            let mut split = cmd_and_args.split_whitespace();
                            if let Some(command) = split.next() {
                                let args: Vec<String> = split.map(|x| x.to_string()).collect();
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
}

fn render_tool_descriptions_markdown(tools: &[fluent_agent::tools::ToolInfo]) -> String {
    let mut out = String::new();
    out.push_str("## Available Tools\n\n");
    out.push_str("| Tool | Executor | Description |\n");
    out.push_str("|------|----------|-------------|\n");
    for t in tools {
        let desc = t.description.replace('\n', " ").replace('|', "\\|");
        out.push_str(&format!("| {} | {} | {} |\n", t.name, t.executor, desc));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_api_error_billing() {
        assert_eq!(
            classify_api_error("Your credit balance is too low to access the Anthropic API"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Please go to Plans & Billing to purchase credits"),
            ApiErrorKind::NonRecoverable
        );
    }

    #[test]
    fn test_classify_api_error_auth() {
        assert_eq!(
            classify_api_error("Invalid API key"),
            ApiErrorKind::NonRecoverable
        );
        assert_eq!(
            classify_api_error("Unauthorized"),
            ApiErrorKind::NonRecoverable
        );
    }

    #[test]
    fn test_classify_api_error_rate_limit() {
        assert_eq!(
            classify_api_error("Rate limit exceeded"),
            ApiErrorKind::Transient
        );
        assert_eq!(classify_api_error("429"), ApiErrorKind::Transient);
    }
}
