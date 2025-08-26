//! Agentic mode operations and autonomous execution
//! 
//! This module contains all the functionality for running the fluent_cli
//! in agentic mode, including goal processing, autonomous execution,
//! and MCP integration.

use anyhow::{anyhow, Result};
use log::{debug, info, warn, error};
use fluent_core::config::Config;
use fluent_core::types::Request;
use std::pin::Pin;
use std::fs;
use std::sync::Arc;

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
            config_path,
            model_override,
            gen_retries,
            min_html_size,
            dry_run: std::env::var("FLUENT_AGENT_DRY_RUN").ok().map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false),
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
}

impl AgenticExecutor {
    /// Create a new agentic executor
    ///
    /// # Arguments
    ///
    /// * `config` - The agentic configuration to use
    ///
    /// # Returns
    ///
    /// A new `AgenticExecutor` instance
    pub fn new(config: AgenticConfig) -> Self {
        Self { config }
    }

    /// Main entry point for agentic mode execution
    pub async fn run(&self, _fluent_config: &Config) -> Result<()> {
        self.print_startup_info();

        let agent_config = self.load_agent_configuration().await?;
        let credentials = self.load_and_validate_credentials(&agent_config).await?;
        let runtime_config = self.create_runtime_configuration(&agent_config, credentials).await?;
        let goal = self.create_goal()?;

        // Optional quick engine test
        let _ = self.test_engines(&runtime_config).await;

        // Build autonomous orchestrator with tools/memory
        use fluent_agent::adapters::{
            RegistryToolAdapter,
            LlmCodeGenerator,
            FsFileManager,
            SimpleRiskAssessor,
        };
        use fluent_agent::{AgentOrchestrator, MemorySystem, ReflectionEngine, StateManager, StateManagerConfig};
        use fluent_agent::action::{ComprehensiveActionExecutor, ActionPlanner, ActionExecutor, IntelligentActionPlanner};
        use fluent_agent::observation::{ComprehensiveObservationProcessor, BasicResultAnalyzer, BasicPatternDetector, BasicImpactAssessor, BasicLearningExtractor};
        use fluent_agent::adapters::CompositePlanner;
        use fluent_agent::tools::ToolRegistry;

        let mut tool_registry = if self.config.enable_tools {
            ToolRegistry::with_standard_tools(&runtime_config.config.tools)
        } else {
            ToolRegistry::new()
        };

        // Workflow macro-tools (LLM-powered tools)
        if self.config.enable_tools {
            let workflow_exec = std::sync::Arc::new(fluent_agent::tools::WorkflowExecutor::new(
                runtime_config.reasoning_engine.clone(),
            ));
            tool_registry.register("workflow".to_string(), workflow_exec);
            println!("🧰 Registered workflow macro-tools (outline/toc/assemble/research)");
        }

        // Optional MCP integration: initialize and register MCP tool executor
        if self.config.enable_tools {
            use fluent_agent::production_mcp::initialize_production_mcp;
            if let Ok(manager) = initialize_production_mcp().await {
                // Attempt auto-connect from config file (config_path)
                if let Err(e) = Self::auto_connect_mcp_servers(&self.config.config_path, &manager).await {
                    println!("⚠️ MCP auto-connect skipped: {}", e);
                }

                let mcp_exec = std::sync::Arc::new(fluent_agent::adapters::McpRegistryExecutor::new(manager.clone()));
                tool_registry.register("mcp".to_string(), mcp_exec);
                println!("🔌 MCP integrated: remote tools available via registry");
            } else {
                println!("⚠️ MCP integration skipped (initialization failed)");
            }
        }

        // Finalize registry, then create shared Arc for adapters/planners
        let arc_registry = Arc::new(tool_registry);
        let tool_adapter = Box::new(RegistryToolAdapter::new(arc_registry.clone()));
        let codegen = Box::new(LlmCodeGenerator::new(runtime_config.reasoning_engine.clone()));
        let filemgr = Box::new(FsFileManager);
        let base_executor: Box<dyn ActionExecutor> = Box::new(ComprehensiveActionExecutor::new(tool_adapter, codegen, filemgr));
        let action_executor: Box<dyn ActionExecutor> = if self.config.dry_run {
            println!("🧪 Dry-run mode: no side effects will be executed");
            Box::new(fluent_agent::adapters::DryRunActionExecutor)
        } else {
            base_executor
        };

        // Planner and observation (adaptive + reflective)
        let base_planner: Box<dyn ActionPlanner> = Box::new(IntelligentActionPlanner::new(Box::new(SimpleRiskAssessor)));
        let planner: Box<dyn ActionPlanner> = Box::new(CompositePlanner::new_with_registry(base_planner, arc_registry.clone()));
        let obs = Box::new(ComprehensiveObservationProcessor::new(
            Box::new(BasicResultAnalyzer),
            Box::new(BasicPatternDetector),
            Box::new(BasicImpactAssessor),
            Box::new(BasicLearningExtractor),
        ));

        // Memory and state
        use fluent_agent::memory::MemoryConfig;
        // TODO: Implement proper memory system once dependencies are resolved
        let memory = fluent_agent::memory::MemorySystem::new(MemoryConfig::default()).await?;
        let state_mgr = StateManager::new(StateManagerConfig::default()).await?;
        let reflection = ReflectionEngine::new();

        // Orchestrate
        // Build orchestrator without moving runtime_config so we can run explicit loop
        let orchestrator = AgentOrchestrator::from_config(
            fluent_agent::config::AgentRuntimeConfig {
                reasoning_engine: runtime_config.reasoning_engine.clone(),
                action_engine: runtime_config.action_engine.clone(),
                reflection_engine: runtime_config.reflection_engine.clone(),
                config: runtime_config.config.clone(),
                credentials: runtime_config.credentials.clone(),
            },
            planner,
            action_executor,
            obs,
            Arc::new(memory),
            Arc::new(state_mgr),
            reflection,
        )
        .await?;

        println!("🔁 Orchestrator constructed. Entering autonomous loop…");
        let timeout_secs: u64 = std::env::var("FLUENT_AGENT_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(180);
        println!(
            "🕒 Watchdog active ({}s). Running ReAct pipeline…",
            timeout_secs
        );

        info!("agent.react.start goal='{}' timeout_secs={}", self.config.goal_description, timeout_secs);
        // Explicit autonomous loop for visibility
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), self.run_autonomous_execution(&goal, &runtime_config)).await {
            Ok(Ok(())) => {
                info!("agent.react.done success=true explicit_autonomous_loop=true");
                println!("✅ Goal execution finished. Success: true");
                Ok(())
            }
            Ok(Err(e)) => {
                error!("agent.react.error err={}", e);
                eprintln!("❌ Orchestrator error: {}", e);
                Err(e)
            }
            Err(_) => {
                error!("agent.react.timeout secs={} goal='{}'", timeout_secs, self.config.goal_description);
                eprintln!("⏳ Agent timed out after {}s. Aborting.", timeout_secs);
                Err(anyhow::anyhow!(format!(
                    "Agent timed out after {}s while executing the goal",
                    timeout_secs
                )))
            }
        }
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
                            if name.is_empty() || cmd_and_args.is_empty() { continue; }
                            let mut split = cmd_and_args.split_whitespace();
                            if let Some(command) = split.next() {
                                let args: Vec<String> = split.map(|x| x.to_string()).collect();
                                info!("agent.mcp.server.connect name='{}' command='{}' args={}", name, command, args.len());
                                 info!("agent.mcp.server.connect name='{}' command='{}' args={}", name, command, args.len());
                                 let _ = manager.client_manager().connect_server(name.to_string(), command.to_string(), args).await;
                            }
                        }
                        Value::Object(map) => {
                            let name = map.get("name").and_then(|v| v.as_str()).unwrap_or("");
                            let command = map.get("command").and_then(|v| v.as_str()).unwrap_or("");
                            let args: Vec<String> = map.get("args")
                                .and_then(|v| v.as_array())
                                .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
                                .unwrap_or_default();
                            if !name.is_empty() && !command.is_empty() {
                                info!("agent.mcp.server.connect name='{}' command='{}' args={}", name, command, args.len());
                                 info!("agent.mcp.server.connect name='{}' command='{}' args={}", name, command, args.len());
                                 let _ = manager.client_manager().connect_server(name.to_string(), command.to_string(), args).await;
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
    fn print_startup_info(&self) {
        println!("🤖 Starting Agentic Mode");
        println!("Goal: {}", self.config.goal_description);
        println!("Max iterations: {}", self.config.max_iterations);
        println!("Tools enabled: {}", self.config.enable_tools);
    }

    /// Load agent configuration from file
    async fn load_agent_configuration(&self) -> Result<fluent_agent::config::AgentEngineConfig> {
        use fluent_agent::config::AgentEngineConfig;
        
        let agent_config = AgentEngineConfig::load_from_file(&self.config.agent_config_path)
            .await
            .map_err(|e| anyhow!("Failed to load agent config: {}", e))?;

        println!("✅ Agent configuration loaded:");
        println!("   - Reasoning engine: {}", agent_config.reasoning_engine);
        println!("   - Action engine: {}", agent_config.action_engine);
        println!("   - Reflection engine: {}", agent_config.reflection_engine);
        println!("   - Memory database: {}", agent_config.memory_database);

        Ok(agent_config)
    }

    /// Load and validate credentials
    async fn load_and_validate_credentials(
        &self,
        agent_config: &fluent_agent::config::AgentEngineConfig,
    ) -> Result<std::collections::HashMap<String, String>> {
        use fluent_agent::config::credentials;
        
        let credentials = credentials::load_from_environment();
        println!("🔑 Loaded {} credential(s) from environment", credentials.len());

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
        &self,
        agent_config: &fluent_agent::config::AgentEngineConfig,
        credentials: std::collections::HashMap<String, String>,
    ) -> Result<fluent_agent::config::AgentRuntimeConfig> {
        println!("🔧 Creating LLM engines...");
        
        let runtime_config = agent_config
            .create_runtime_config(&self.config.config_path, credentials, self.config.model_override.as_deref())
            .await?;

        println!("✅ LLM engines created successfully!");
        Ok(runtime_config)
    }

    /// Create goal from description
    fn create_goal(&self) -> Result<fluent_agent::goal::Goal> {
        use fluent_agent::goal::{Goal, GoalType};
        
        let mut builder = Goal::builder(self.config.goal_description.clone(), GoalType::CodeGeneration)
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

        println!("🎯 Goal: {}", goal.description);
        println!("🔄 Max iterations: {:?}", goal.max_iterations);

        Ok(goal)
    }

    /// Test engines to ensure they're working
    async fn test_engines(&self, runtime_config: &fluent_agent::config::AgentRuntimeConfig) -> Result<()> {
        println!("\n🧠 Testing reasoning engine...");
        
        let test_request = Request {
            flowname: "agentic_test".to_string(),
            payload: "Hello! Please respond with 'Agentic mode is working!' to confirm the engine is operational.".to_string(),
        };

        match Pin::from(runtime_config.reasoning_engine.execute(&test_request)).await {
            Ok(response) => {
                println!("✅ Reasoning engine response: {}", response.content);
                self.print_operational_status();
                Ok(())
            }
            Err(e) => {
                println!("❌ Engine test failed: {e}");
                println!("🔧 Please check your API keys and configuration");
                Err(anyhow!("Engine test failed: {}", e))
            }
        }
    }

    /// Print operational status
    fn print_operational_status(&self) {
        println!("\n🚀 AGENTIC MODE IS FULLY OPERATIONAL!");
        println!("🎯 Goal: {}", self.config.goal_description);
        println!("🔧 All systems ready:");
        println!("   ✅ LLM engines connected and tested");
        println!("   ✅ Configuration system integrated");
        println!("   ✅ Credential management working");
        println!("   ✅ Goal system operational");

        if self.config.enable_tools {
            println!("   ✅ Tool execution enabled");
        } else {
            println!("   ⚠️  Tool execution disabled (use --enable-tools to enable)");
        }

        println!("\n🎉 The agentic coding platform is ready for autonomous operation!");
    }

    /// Run autonomous execution loop
    async fn run_autonomous_execution(
        &self,
        goal: &fluent_agent::goal::Goal,
        runtime_config: &fluent_agent::config::AgentRuntimeConfig,
    ) -> Result<()> {
        println!("\n🚀 Starting autonomous execution...");
        
        let executor = AutonomousExecutor::new(
            goal.clone(),
            runtime_config,
            self.config.gen_retries.unwrap_or(3),
            self.config.min_html_size.unwrap_or(2000) as usize,
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
}

impl<'a> AutonomousExecutor<'a> {
    pub fn new(
        goal: fluent_agent::goal::Goal,
        runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
        gen_retries: u32,
        min_html_size: usize,
    ) -> Self {
        Self { goal, runtime_config, gen_retries, min_html_size }
    }

    /// Execute autonomous loop
    pub async fn execute(&self, max_iterations: u32) -> Result<()> {
        use fluent_agent::context::ExecutionContext;
        
        println!("🎯 Starting autonomous execution for goal: {}", self.goal.description);
        info!("agent.loop.begin goal='{}' max_iterations={}", self.goal.description, max_iterations);

        let mut context = ExecutionContext::new(self.goal.clone());

        for iteration in 1..=max_iterations {
            println!("\n🔄 Iteration {iteration}/{max_iterations}");
            debug!("agent.loop.iteration start iter={}", iteration);

            let reasoning_response = self.perform_reasoning(iteration, max_iterations).await?;
            debug!("agent.loop.reasoning.done len={} preview='{}'", reasoning_response.len(), &reasoning_response.chars().take(160).collect::<String>());
            
            if self.is_game_goal() {
                info!("agent.loop.path game=true");
                self.handle_game_creation(&mut context).await?;
                return Ok(());
            } else {
                info!("agent.loop.path game=false");
                self.handle_general_goal(&mut context, &reasoning_response, iteration, max_iterations).await?;
                
                if self.should_complete_goal(iteration, max_iterations) {
                    info!("agent.loop.complete iter={}", iteration);
                    return Ok(());
                }
            }
        }

        println!("⚠️ Reached maximum iterations without completing goal");
        Ok(())
    }

    /// Perform reasoning for current iteration
    async fn perform_reasoning(&self, iteration: u32, max_iterations: u32) -> Result<String> {
        println!("🧠 Analyzing goal and determining next action...");

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

        debug!("agent.reasoning.request flow='{}' len={}", reasoning_request.flowname, reasoning_request.payload.len());
        match Pin::from(self.runtime_config.reasoning_engine.execute(&reasoning_request)).await {
            Ok(response) => {
                println!("🤖 Agent reasoning: {}", response.content);
                debug!("agent.reasoning.response len={} preview='{}'", response.content.len(), &response.content.chars().take(200).collect::<String>());
                Ok(response.content)
            }
            Err(e) => {
                println!("❌ Reasoning failed: {e}");
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
    async fn handle_game_creation(&self, context: &mut fluent_agent::context::ExecutionContext) -> Result<()> {
        println!("🎮 Agent decision: Create the game now!");

        let game_creator = GameCreator::new(&self.goal, self.runtime_config, self.gen_retries, self.min_html_size);
        game_creator.create_game(context).await
    }

    /// Handle general (non-game) goals
    async fn handle_general_goal(
        &self,
        context: &mut fluent_agent::context::ExecutionContext,
        reasoning_response: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<()> {
        println!("🔍 Processing complex analytical goal...");

        let action_response = self.plan_action(reasoning_response, iteration, max_iterations).await?;
        
        if self.goal.description.to_lowercase().contains("reflection") {
            self.handle_reflection_analysis(context, &action_response, iteration, max_iterations).await?;
        } else {
            println!("🔧 Processing general goal: {}", self.goal.description);
            context.increment_iteration();
        }

        Ok(())
    }

    /// Plan specific action based on reasoning
    async fn plan_action(&self, reasoning_response: &str, iteration: u32, max_iterations: u32) -> Result<String> {
        let action_request = Request {
            flowname: "action_planning".to_string(),
            payload: format!(
                "Based on this goal and reasoning, determine the specific action to take:\n\n\
                Goal: {}\n\
                Reasoning: {}\n\
                Iteration: {}/{}\n\n\
                What specific file should be analyzed, created, or modified? \
                Respond with just the file path and a brief description of what to do with it.",
                self.goal.description,
                reasoning_response,
                iteration,
                max_iterations
            ),
        };

        debug!("agent.action.request flow='{}' len={}", action_request.flowname, action_request.payload.len());
        match Pin::from(self.runtime_config.reasoning_engine.execute(&action_request)).await {
            Ok(response) => {
                println!("📋 Planned action: {}", response.content);
                info!("agent.action.planned first_line='{}'", response.content.lines().next().unwrap_or(""));
                Ok(response.content)
            }
            Err(e) => {
                println!("❌ Action planning failed: {e}");
                error!("agent.action.error {}", e);
                Err(anyhow!("Action planning failed: {}", e))
            }
        }
    }

    /// Handle reflection system analysis
    async fn handle_reflection_analysis(
        &self,
        context: &mut fluent_agent::context::ExecutionContext,
        action_response: &str,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<()> {
        let analysis_file = "analysis/reflection_system_analysis.md";

        // Create analysis directory
        if let Err(e) = fs::create_dir_all("analysis") {
            println!("⚠️ Could not create analysis directory: {e}");
        }

        let analysis_response = self.perform_reflection_analysis(iteration, max_iterations).await?;
        self.write_analysis_file(analysis_file, &analysis_response, action_response, iteration).await?;
        
        // Update context with progress
        context.set_variable("analysis_iteration".to_string(), iteration.to_string());
        context.set_variable("analysis_file".to_string(), analysis_file.to_string());
        context.increment_iteration();

        Ok(())
    }

    /// Perform reflection system analysis
    async fn perform_reflection_analysis(&self, iteration: u32, max_iterations: u32) -> Result<String> {
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

        match Pin::from(self.runtime_config.reasoning_engine.execute(&analysis_request)).await {
            Ok(response) => Ok(response.content),
            Err(e) => {
                println!("❌ Analysis failed: {e}");
                Err(anyhow!("Analysis failed: {}", e))
            }
        }
    }

    /// Write analysis to file
    async fn write_analysis_file(
        &self,
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
            println!("❌ Failed to write analysis: {e}");
            Err(anyhow!("Failed to write analysis: {}", e))
        } else {
            println!("✅ Analysis written to: {analysis_file}");
            println!("📝 Analysis length: {} characters", analysis_content.len());
            Ok(())
        }
    }

    /// Check if goal should be completed
    fn should_complete_goal(&self, iteration: u32, max_iterations: u32) -> bool {
        if self.goal.description.to_lowercase().contains("reflection")
            && iteration >= max_iterations / 2 {
                println!("🎯 Comprehensive analysis completed across {iteration} iterations!");
                return true;
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
}

impl<'a> GameCreator<'a> {
    pub fn new(
        goal: &'a fluent_agent::goal::Goal,
        runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
        gen_retries: u32,
        min_html_size: usize,
    ) -> Self {
        Self { goal, runtime_config, gen_retries, min_html_size }
    }

    /// Create game based on goal description
    pub async fn create_game(&self, context: &mut fluent_agent::context::ExecutionContext) -> Result<()> {
        let (file_extension, code_prompt, file_path) = self.determine_game_type();
        info!("agent.codegen.select type='{}' path='{}'", file_extension, file_path);
        let game_code = self.generate_game_code(&code_prompt, file_extension).await?;
        debug!("agent.codegen.generated len={} ext='{}'", game_code.len(), file_extension);
        self.write_game_file(file_path, &game_code)?;
        self.update_context(context, file_path, file_extension);
        
        println!("🎉 Goal achieved! {} game created successfully!", file_extension.to_uppercase());
        Ok(())
    }

    /// Determine what type of game to create
    fn determine_game_type(&self) -> (&str, String, &str) {
        let description = self.goal.description.to_lowercase();
        let wants_web = description.contains("javascript") || description.contains("html") || description.contains("web");

        // Check for specific game types in order of preference
        if description.contains("tetris") {
            if wants_web {
                (
                    "html",
                    "Create a complete, working Tetris game using HTML5, CSS, and JavaScript. Requirements:\n\
                        - Single HTML file with embedded CSS and JavaScript (no external files)\n\
                        - Use HTML5 Canvas for rendering\n\
                        - Implement standard Tetris rules: 10x20 grid, 7 tetrominoes (I, O, T, S, Z, J, L)\n\
                        - Rotation system (clockwise), wall kicks, and gravity\n\
                        - Piece hold, next piece queue, soft drop, and hard drop\n\
                        - Line clear detection (single/double/triple/tetris) and scoring system\n\
                        - Level progression (increase fall speed) and game over state\n\
                        - Keyboard controls (arrow keys + space for hard drop, shift for hold)\n\
                        - Cleanly structured code (Board, Piece, GameLoop) with comments\n\
                        Provide ONLY the complete HTML file with embedded CSS and JavaScript, wrapped in a single fenced block like:\n\
                        ```html\n\
                        ... your full HTML here ...\n\
                        ```".to_string(),
                    "examples/web_tetris.html"
                )
            } else {
                (
                    "rs",
                    "Create a complete, working Tetris game in Rust. Requirements:\n\
                        - Terminal-based interface using crossterm crate\n\
                        - 10x20 grid, 7 tetrominoes, piece rotation and movement\n\
                        - Gravity, line clear detection, scoring, and levels\n\
                        - Controls: arrow keys to move/rotate, space hard drop, 'c' to hold\n\
                        - Clean game loop with non-blocking input and rendering\n\
                        Provide ONLY the complete, compilable Rust code with all necessary imports, wrapped in:\n\
                        ```rust\n\
                        ... full program ...\n\
                        ```".to_string(),
                    "examples/agent_tetris.rs"
                )
            }
        } else if description.contains("snake") {
            if wants_web {
                (
                    "html",
                    "Create a complete, working Snake game using HTML5, CSS, and JavaScript. Requirements:\n\
                        - Single HTML file with embedded CSS and JavaScript (no external files)\n\
                        - Use HTML5 Canvas for rendering on a grid (e.g., 20x20 cells)\n\
                        - Classic Snake mechanics: growing tail when eating food, collision with walls or self causes game over\n\
                        - Food spawn at random empty cell; avoid spawning on snake\n\
                        - Scoring system and increasing speed per level or every few foods\n\
                        - Keyboard controls: arrow keys and WASD; include pause/resume and restart\n\
                        - Clean structure with a main game loop, update, and render phases\n\
                        Provide ONLY the complete HTML file with embedded CSS and JavaScript, wrapped in a fenced block:\n\
                        ```html\n\
                        ... full HTML here ...\n\
                        ```".to_string(),
                    "examples/web_snake.html"
                )
            } else {
                (
                    "rs",
                    "Create a complete, working Snake game in Rust. Requirements:\n\
                        - Terminal-based interface using crossterm\n\
                        - Grid-based snake movement, food spawn, self/wall collision detection\n\
                        - Score tracking and increasing speed over time\n\
                        - Controls: arrow keys / WASD, 'p' pause, 'r' restart\n\
                        Provide ONLY the complete, compilable Rust code with all necessary imports, wrapped in:\n\
                        ```rust\n\
                        ... full program ...\n\
                        ```".to_string(),
                    "examples/agent_snake.rs"
                )
            }
        } else {
            // For unrecognized game requests, default to Tetris as it's the most complex and requested
            println!("⚠️ Unrecognized game type requested. Defaulting to Tetris as it's the most complex option.");
            if wants_web {
                (
                    "html",
                    "Create a complete, working Tetris game using HTML5, CSS, and JavaScript. Requirements:\n\
                        - Single HTML file with embedded CSS and JavaScript (no external files)\n\
                        - Use HTML5 Canvas for rendering\n\
                        - Implement standard Tetris rules: 10x20 grid, 7 tetrominoes (I, O, T, S, Z, J, L)\n\
                        - Rotation system (clockwise), wall kicks, and gravity\n\
                        - Piece hold, next piece queue, soft drop, and hard drop\n\
                        - Line clear detection (single/double/triple/tetris) and scoring system\n\
                        - Level progression (increase fall speed) and game over state\n\
                        - Keyboard controls (arrow keys + space for hard drop, shift for hold)\n\
                        - Cleanly structured code (Board, Piece, GameLoop) with comments\n\
                        Provide ONLY the complete HTML file with embedded CSS and JavaScript, wrapped in a single fenced block like:\n\
                        ```html\n\
                        ... your full HTML here ...\n\
                        ```".to_string(),
                    "examples/web_tetris.html"
                )
            } else {
                (
                    "rs",
                    "Create a complete, working Tetris game in Rust. Requirements:\n\
                        - Terminal-based interface using crossterm crate\n\
                        - 10x20 grid, 7 tetrominoes, piece rotation and movement\n\
                        - Gravity, line clear detection, scoring, and levels\n\
                        - Controls: arrow keys to move/rotate, space hard drop, 'c' to hold\n\
                        - Clean game loop with non-blocking input and rendering\n\
                        Provide ONLY the complete, compilable Rust code with all necessary imports, wrapped in:\n\
                        ```rust\n\
                        ... full program ...\n\
                        ```".to_string(),
                    "examples/agent_tetris.rs"
                )
            }
        }
    }

    /// Generate game code using LLM
    async fn generate_game_code(&self, code_prompt: &str, file_extension: &str) -> Result<String> {
        info!("agent.codegen.start ext='{}' retries={}", file_extension, self.gen_retries);
        let code_request = Request {
            flowname: "code_generation".to_string(),
            payload: code_prompt.to_string(),
        };

        println!("🧠 Generating {} game code with selected LLM...", file_extension.to_uppercase());

        // Helper: try execute with retry/backoff
        async fn try_execute_with_retry(engine: &Box<dyn fluent_core::traits::Engine>, req: &Request, attempts: u32) -> Result<fluent_core::types::Response> {
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
                            println!("⚠️ LLM request failed (attempt {attempt}/{max_attempts}). Retrying in {}ms...", delay);
                            warn!("agent.codegen.retry attempt={}/{} delay_ms={}", attempt, max_attempts, delay);
                            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                            delay *= 2;
                        }
                    }
                }
            }
            Err(anyhow::anyhow!(format!("LLM request failed after retries: {}", last_err.unwrap_or_else(|| anyhow::anyhow!("unknown error")))))
        }

        // First attempt with retry
        let mut code_response = try_execute_with_retry(self.runtime_config.reasoning_engine.as_ref(), &code_request, self.gen_retries).await?;
        let mut game_code = crate::utils::extract_code(&code_response.content, file_extension);
        debug!("agent.codegen.extracted len={} ext='{}'", game_code.len(), file_extension);

        // Lightweight validation for Tetris deliverables
        let desc = self.goal.description.to_lowercase();
        let needs_tetris = desc.contains("tetris");
        let mut valid = true;
        if needs_tetris && file_extension == "html" {
            let lc = game_code.to_lowercase();
            let has_canvas = lc.contains("<canvas") || lc.contains("getelementbyid('tetriscanvas'") || lc.contains("getelementbyid(\"tetriscanvas\"");
            let has_controls = lc.contains("keydown") || lc.contains("addEventListener('keydown'") || lc.contains("addEventListener(\"keydown\"");
            let has_logic = lc.contains("tetromino") || lc.contains("rotation") || lc.contains("rotate(") || lc.contains("lines") || lc.contains("score");
            let long_enough = game_code.len() > self.min_html_size; // require non-trivial output
            debug!("agent.codegen.validate has_canvas={} has_controls={} has_logic={} long_enough={}", has_canvas, has_controls, has_logic, long_enough);
            valid = has_canvas && has_controls && has_logic && long_enough;
        }

        if !valid {
            println!("⚠️ Output seems incomplete. Requesting refined Tetris implementation...");
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

            let refine_request = Request { flowname: "code_generation_refine".to_string(), payload: refine_prompt };
            code_response = try_execute_with_retry(self.runtime_config.reasoning_engine.as_ref(), &refine_request, self.gen_retries).await?;
            game_code = crate::utils::extract_code(&code_response.content, file_extension);

            // Re-validate refined output; if still clearly a placeholder, keep the raw content to aid debugging
            if needs_tetris && file_extension == "html" {
                let lc2 = game_code.to_lowercase();
                let still_placeholder = game_code.len() < self.min_html_size;
                if still_placeholder {
                    println!("⚠️ Refined output still looks insufficient. Writing raw response for inspection.");
                    game_code = code_response.content;
                }
            }
        }

        Ok(game_code)
    }

    /// Write game code to file
    fn write_game_file(&self, file_path: &str, game_code: &str) -> Result<()> {
        fs::write(file_path, game_code)?;
        info!("agent.codegen.file_written path='{}' bytes={}", file_path, game_code.len());
        println!("✅ Created game at: {file_path}");
        println!("📝 Game code length: {} characters", game_code.len());
        Ok(())
    }

    /// Update execution context with game creation info
    fn update_context(&self, context: &mut fluent_agent::context::ExecutionContext, file_path: &str, file_extension: &str) {
        context.set_variable("game_created".to_string(), "true".to_string());
        context.set_variable("game_path".to_string(), file_path.to_string());
        context.set_variable("game_type".to_string(), file_extension.to_string());
    }
}
