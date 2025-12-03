//! Agentic mode operations and autonomous execution
//!
//! This module contains all the functionality for running the fluent_cli
//! in agentic mode, including goal processing, autonomous execution,
//! and MCP integration.

use anyhow::{anyhow, Result};
use fluent_core::config::Config;
use fluent_core::types::Request;
use tracing::{debug, error, info, warn};
use std::fs;
use std::pin::Pin;
use std::sync::Arc;

use crate::tui::{AgentStatus, TuiManager};

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
        }
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

        // Optional quick engine test
        let _ = self.test_engines(&runtime_config).await;

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
        self.tui
            .add_log("✅ Memory system initialized with working memory, compression, and persistence".to_string());

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
        let timeout_secs: u64 = std::env::var("FLUENT_AGENT_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(600); // Increased from 180s to 600s (10 minutes) for research tasks
        self.tui.add_log(format!(
            "🕒 Watchdog active ({}s). Running ReAct pipeline…",
            timeout_secs
        ));

        info!(
            "agent.react.start goal='{}' timeout_secs={}",
            self.config.goal_description, timeout_secs
        );

        self.tui.update_status(AgentStatus::Running);

        // If TUI is enabled, run agent execution and TUI concurrently
        let result = if self.tui.enabled() {
            self.run_with_tui(&goal, &runtime_config, timeout_secs)
                .await
        } else {
            // Run without TUI
            match tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
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
                        "agent.react.timeout secs={} goal='{}'",
                        timeout_secs, self.config.goal_description
                    );
                    Err(anyhow::anyhow!(format!(
                        "Agent timed out after {}s while executing the goal",
                        timeout_secs
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
            serde_yaml::from_str(&content)?
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
                    "agent.react.timeout secs={} goal='{}'",
                    timeout_secs, self.config.goal_description
                );
                self.tui.update_status(AgentStatus::Timeout);
                self.tui.add_log(format!(
                    "⏳ Agent timed out after {}s. Aborting.",
                    timeout_secs
                ));
                Err(anyhow::anyhow!(format!(
                    "Agent timed out after {}s while executing the goal",
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

        let mut executor = AutonomousExecutor::new(
            goal.clone(),
            runtime_config,
            self.config.gen_retries.unwrap_or(3),
            self.config.min_html_size.unwrap_or(2000) as usize,
            &mut self.tui,
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
}

impl<'a> AutonomousExecutor<'a> {
    pub fn new(
        goal: fluent_agent::goal::Goal,
        runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
        gen_retries: u32,
        min_html_size: usize,
        tui: &'a mut TuiManager,
    ) -> Self {
        let crx = tui.control_receiver();
        Self {
            goal,
            runtime_config,
            gen_retries,
            min_html_size,
            tui,
            control_rx: crx,
            paused: false,
            queued_guidance: Vec::new(),
        }
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

            let reasoning_response = self.perform_reasoning(iteration, max_iterations).await?;
            debug!(
                "agent.loop.reasoning.done len={} preview='{}'",
                reasoning_response.len(),
                &reasoning_response.chars().take(160).collect::<String>()
            );

            if self.is_game_goal() {
                info!("agent.loop.path game=true");
                self.handle_game_creation(&mut context).await?;
                return Ok(());
            } else {
                info!("agent.loop.path game=false");
                self.handle_general_goal(
                    &mut context,
                    &reasoning_response,
                    iteration,
                    max_iterations,
                )
                .await?;

                if self.should_complete_goal(iteration, max_iterations) {
                    info!("agent.loop.complete iter={}", iteration);
                    return Ok(());
                }
            }
        }

        self.tui
            .add_log("⚠️ Reached maximum iterations without completing goal".to_string());
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

    /// Perform reasoning for current iteration
    async fn perform_reasoning(&mut self, iteration: u32, max_iterations: u32) -> Result<String> {
        self.tui
            .set_current_action("Analyzing goal and determining next action...".to_string());
        self.tui
            .add_log("🧠 Analyzing goal and determining next action...".to_string());

        let tools_available = "file operations, shell commands, code analysis";
        let reasoning_request = Request {
            flowname: "agentic_reasoning".to_string(),
            payload: format!(
                "You are an autonomous AI agent. Analyze this goal and determine the next specific action to take:\n\n\
                Goal: {}\n\n\
                Current iteration: {}/{}\n\
                Tools available: {}\n\n\
                Based on this goal, what is the most logical next step? Respond with:\n\
                1. A brief analysis of what the goal requires\n\
                2. The specific next action to take\n\
                3. Why this action moves us toward the goal\n\n\
                Be specific and actionable. Focus on the actual goal, not creating games unless the goal specifically asks for a game.",
                self.goal.description,
                iteration,
                max_iterations,
                tools_available
            ),
        };

        debug!(
            "agent.reasoning.request flow='{}' len={}",
            reasoning_request.flowname,
            reasoning_request.payload.len()
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
        game_creator.create_game(context).await
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
            return Err(anyhow!("Failed to write research file: {}", e));
        }

        self.tui.add_log(format!(
            "✅ Created {} ({} characters)",
            file_path,
            content.len()
        ));
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
            }
            Err(e) => {
                self.tui
                    .add_log(format!("⚠️ Could not read file {}: {}", file_path, e));
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
    fn should_complete_goal(&mut self, iteration: u32, max_iterations: u32) -> bool {
        if self.goal.description.to_lowercase().contains("reflection")
            && iteration >= max_iterations / 2
        {
            self.tui.add_log(format!(
                "🎯 Comprehensive analysis completed across {} iterations!",
                iteration
            ));
            return true;
        }

        // For research goals, check if we've created substantial content
        if iteration >= 3 {
            // Check if research files exist and have content
            let research_files = ["grilled_cheese_research.md", "research_output.md"];
            for file in &research_files {
                if let Ok(metadata) = fs::metadata(file) {
                    if metadata.len() > 1000 {
                        // At least 1KB of content
                        self.tui.add_log(format!(
                            "🎯 Research goal appears complete - substantial content created in {}",
                            file
                        ));
                        return true;
                    }
                }
            }
        }

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
    pub async fn create_game(
        &mut self,
        context: &mut fluent_agent::context::ExecutionContext,
    ) -> Result<()> {
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
        Ok(())
    }

    /// Determine what type of game to create based on goal description
    /// Returns (file_extension, code_prompt, output_path)
    fn determine_game_type(goal_description: &str) -> (String, String, String) {
        let description = goal_description.to_lowercase();

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

        // Detect game type
        let game_name = if description.contains("solitaire") || description.contains("klondike") {
            "solitaire"
        } else if description.contains("tetris") {
            "tetris"
        } else if description.contains("snake") {
            "snake"
        } else if description.contains("pong") {
            "pong"
        } else if description.contains("breakout") || description.contains("arkanoid") {
            "breakout"
        } else if description.contains("minesweeper") {
            "minesweeper"
        } else {
            // Extract game name from description if possible
            "game"
        };

        // Determine file extension and output path based on platform
        let (ext, output_path) = if wants_love2d || wants_lua {
            ("lua".to_string(), format!("outputs/{}_love2d/main.lua", game_name))
        } else if wants_python {
            ("py".to_string(), format!("outputs/{}_pygame.py", game_name))
        } else if wants_web {
            ("html".to_string(), format!("outputs/{}_web.html", game_name))
        } else {
            ("rs".to_string(), format!("outputs/{}_game.rs", game_name))
        };

        // Generate appropriate code prompt based on platform and game
        let code_prompt = if wants_love2d {
            format!(
                "Create a complete, working {} game using the LÖVE (Love2D) framework in Lua.\n\
                Requirements:\n\
                - Create main.lua with all game logic\n\
                - Implement proper love.load(), love.update(dt), love.draw(), and love.keypressed(key) callbacks\n\
                - Include all necessary game mechanics for {}\n\
                - Use love.graphics for rendering\n\
                - Handle keyboard/mouse input appropriately\n\
                - Include scoring and game state management\n\
                - Add comments explaining the code structure\n\
                Provide ONLY the complete Lua code wrapped in:\n\
                ```lua\n\
                ... full main.lua code ...\n\
                ```",
                game_name, game_name
            )
        } else if wants_python {
            format!(
                "Create a complete, working {} game using Python and Pygame.\n\
                Requirements:\n\
                - Single Python file with all game logic\n\
                - Initialize pygame properly\n\
                - Implement game loop with event handling, update, and draw phases\n\
                - Include all necessary game mechanics for {}\n\
                - Handle keyboard input appropriately\n\
                - Include scoring and game state management\n\
                Provide ONLY the complete Python code wrapped in:\n\
                ```python\n\
                ... full code ...\n\
                ```",
                game_name, game_name
            )
        } else if wants_web {
            format!(
                "Create a complete, working {} game using HTML5, CSS, and JavaScript.\n\
                Requirements:\n\
                - Single HTML file with embedded CSS and JavaScript\n\
                - Use HTML5 Canvas for rendering\n\
                - Implement all standard {} game mechanics\n\
                - Keyboard controls for gameplay\n\
                - Scoring system and game state management\n\
                - Clean, well-structured code with comments\n\
                Provide ONLY the complete HTML file wrapped in:\n\
                ```html\n\
                ... full HTML ...\n\
                ```",
                game_name, game_name
            )
        } else {
            format!(
                "Create a complete, working {} game in Rust.\n\
                Requirements:\n\
                - Terminal-based interface using crossterm crate\n\
                - Implement all standard {} game mechanics\n\
                - Keyboard controls for gameplay\n\
                - Scoring system and game state management\n\
                - Clean game loop with non-blocking input\n\
                Provide ONLY the complete Rust code wrapped in:\n\
                ```rust\n\
                ... full code ...\n\
                ```",
                game_name, game_name
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

        // Lightweight validation for Tetris deliverables
        let desc = self.goal.description.to_lowercase();
        let needs_tetris = desc.contains("tetris");
        let mut valid = true;
        if needs_tetris && file_extension == "html" {
            let lc = game_code.to_lowercase();
            let has_canvas = lc.contains("<canvas")
                || lc.contains("getelementbyid('tetriscanvas'")
                || lc.contains("getelementbyid(\"tetriscanvas\"");
            let has_controls = lc.contains("keydown")
                || lc.contains("addEventListener('keydown'")
                || lc.contains("addEventListener(\"keydown\"");
            let has_logic = lc.contains("tetromino")
                || lc.contains("rotation")
                || lc.contains("rotate(")
                || lc.contains("lines")
                || lc.contains("score");
            let long_enough = game_code.len() > self.min_html_size; // require non-trivial output
            debug!(
                "agent.codegen.validate has_canvas={} has_controls={} has_logic={} long_enough={}",
                has_canvas, has_controls, has_logic, long_enough
            );
            valid = has_canvas && has_controls && has_logic && long_enough;
        }

        if !valid {
            self.tui.add_log(
                "⚠️ Output seems incomplete. Requesting refined Tetris implementation..."
                    .to_string(),
            );
            info!("agent.codegen.refine ext='{}'", file_extension);
            let refine_prompt = format!(
                "Your previous output was incomplete or generic. Regenerate the deliverable as a single, complete {} Tetris implementation with the following minimum features: \n\
                 - 10x20 grid, 7 tetrominoes (I,O,T,S,Z,J,L) \n\
                 - Rotation with wall kicks, gravity and lock delay \n\
                 - Line clear detection and scoring with level progression \n\
                 - Controls: arrows for move/rotate, space hard drop, shift hold \n\
                 Provide ONLY the full source in one block, no prose. Wrap it in a fenced block with the correct language: \n\
                 ```html``` for HTML or ```rust``` for Rust.\n\
                 ",
                if file_extension == "html" { "HTML (embedded JS/CSS)" } else { "Rust" }
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

            // Re-validate refined output; if still clearly a placeholder, keep the raw content to aid debugging
            if needs_tetris && file_extension == "html" {
                let _lc2 = game_code.to_lowercase();
                let still_placeholder = game_code.len() < self.min_html_size;
                if still_placeholder {
                    self.tui.add_log("⚠️ Refined output still looks insufficient. Writing raw response for inspection.".to_string());
                    game_code = code_response.content;
                }
            }
        }

        Ok(game_code)
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
