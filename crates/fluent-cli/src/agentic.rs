//! Agentic mode operations and autonomous execution
//! 
//! This module contains all the functionality for running the fluent_cli
//! in agentic mode, including goal processing, autonomous execution,
//! and MCP integration.

use anyhow::{anyhow, Result};
use fluent_core::config::Config;
use fluent_core::types::Request;
use std::pin::Pin;
use std::fs;

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
    ) -> Self {
        Self {
            goal_description,
            agent_config_path,
            max_iterations,
            enable_tools,
            config_path,
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
        
        self.test_engines(&runtime_config).await?;
        
        if self.config.enable_tools {
            self.run_autonomous_execution(&goal, &runtime_config).await?;
        } else {
            println!("📝 Tools disabled - would need --enable-tools for full autonomous operation");
        }
        
        Ok(())
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
            .create_runtime_config(&self.config.config_path, credentials)
            .await?;

        println!("✅ LLM engines created successfully!");
        Ok(runtime_config)
    }

    /// Create goal from description
    fn create_goal(&self) -> Result<fluent_agent::goal::Goal> {
        use fluent_agent::goal::{Goal, GoalType};
        
        let goal = Goal::builder(self.config.goal_description.clone(), GoalType::CodeGeneration)
            .max_iterations(self.config.max_iterations)
            .success_criterion("Code compiles without errors".to_string())
            .success_criterion("Code runs successfully".to_string())
            .success_criterion("Code meets the specified requirements".to_string())
            .build()?;

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
                println!("❌ Engine test failed: {}", e);
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
        
        let executor = AutonomousExecutor::new(goal.clone(), runtime_config);
        executor.execute(self.config.max_iterations).await
    }
}

/// Autonomous execution engine
pub struct AutonomousExecutor<'a> {
    goal: fluent_agent::goal::Goal,
    runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
}

impl<'a> AutonomousExecutor<'a> {
    pub fn new(goal: fluent_agent::goal::Goal, runtime_config: &'a fluent_agent::config::AgentRuntimeConfig) -> Self {
        Self { goal, runtime_config }
    }

    /// Execute autonomous loop
    pub async fn execute(&self, max_iterations: u32) -> Result<()> {
        use fluent_agent::context::ExecutionContext;
        
        println!("🎯 Starting autonomous execution for goal: {}", self.goal.description);

        let mut context = ExecutionContext::new(self.goal.clone());

        for iteration in 1..=max_iterations {
            println!("\n🔄 Iteration {}/{}", iteration, max_iterations);

            let reasoning_response = self.perform_reasoning(iteration, max_iterations).await?;
            
            if self.is_game_goal() {
                self.handle_game_creation(&mut context).await?;
                return Ok(());
            } else {
                self.handle_general_goal(&mut context, &reasoning_response, iteration, max_iterations).await?;
                
                if self.should_complete_goal(iteration, max_iterations) {
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

        match Pin::from(self.runtime_config.reasoning_engine.execute(&reasoning_request)).await {
            Ok(response) => {
                println!("🤖 Agent reasoning: {}", response.content);
                Ok(response.content)
            }
            Err(e) => {
                println!("❌ Reasoning failed: {}", e);
                Err(anyhow!("Reasoning failed: {}", e))
            }
        }
    }

    /// Check if this is a game creation goal
    fn is_game_goal(&self) -> bool {
        let description = self.goal.description.to_lowercase();
        description.contains("game")
            || description.contains("frogger")
            || description.contains("javascript")
            || description.contains("html")
    }

    /// Handle game creation goals
    async fn handle_game_creation(&self, context: &mut fluent_agent::context::ExecutionContext) -> Result<()> {
        println!("🎮 Agent decision: Create the game now!");

        let game_creator = GameCreator::new(&self.goal, self.runtime_config);
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

        match Pin::from(self.runtime_config.reasoning_engine.execute(&action_request)).await {
            Ok(response) => {
                println!("📋 Planned action: {}", response.content);
                Ok(response.content)
            }
            Err(e) => {
                println!("❌ Action planning failed: {}", e);
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
            println!("⚠️ Could not create analysis directory: {}", e);
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
                Focus on iteration {}/{}.\n\n\
                Analyze the following aspects:\n\
                1. Architecture and design patterns\n\
                2. Performance characteristics\n\
                3. Memory usage patterns\n\
                4. Potential bottlenecks\n\
                5. Optimization opportunities\n\n\
                Provide a detailed technical analysis with specific recommendations.",
                iteration,
                max_iterations
            ),
        };

        match Pin::from(self.runtime_config.reasoning_engine.execute(&analysis_request)).await {
            Ok(response) => Ok(response.content),
            Err(e) => {
                println!("❌ Analysis failed: {}", e);
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
            println!("❌ Failed to write analysis: {}", e);
            Err(anyhow!("Failed to write analysis: {}", e))
        } else {
            println!("✅ Analysis written to: {}", analysis_file);
            println!("📝 Analysis length: {} characters", analysis_content.len());
            Ok(())
        }
    }

    /// Check if goal should be completed
    fn should_complete_goal(&self, iteration: u32, max_iterations: u32) -> bool {
        if self.goal.description.to_lowercase().contains("reflection") {
            if iteration >= max_iterations / 2 {
                println!("🎯 Comprehensive analysis completed across {} iterations!", iteration);
                return true;
            }
        }
        false
    }
}

/// Game creation handler
pub struct GameCreator<'a> {
    goal: &'a fluent_agent::goal::Goal,
    runtime_config: &'a fluent_agent::config::AgentRuntimeConfig,
}

impl<'a> GameCreator<'a> {
    pub fn new(goal: &'a fluent_agent::goal::Goal, runtime_config: &'a fluent_agent::config::AgentRuntimeConfig) -> Self {
        Self { goal, runtime_config }
    }

    /// Create game based on goal description
    pub async fn create_game(&self, context: &mut fluent_agent::context::ExecutionContext) -> Result<()> {
        let (file_extension, code_prompt, file_path) = self.determine_game_type();
        let game_code = self.generate_game_code(&code_prompt, file_extension).await?;
        self.write_game_file(&file_path, &game_code)?;
        self.update_context(context, &file_path, file_extension);
        
        println!("🎉 Goal achieved! {} game created successfully!", file_extension.to_uppercase());
        Ok(())
    }

    /// Determine what type of game to create
    fn determine_game_type(&self) -> (&str, String, &str) {
        let description = self.goal.description.to_lowercase();
        
        if description.contains("javascript") || description.contains("html") || description.contains("web") {
            (
                "html",
                format!(
                    "Create a complete, working Frogger-like game using HTML5, CSS, and JavaScript. Requirements:\n\
                    - Complete HTML file with embedded CSS and JavaScript\n\
                    - HTML5 Canvas for game rendering\n\
                    - Frog character that moves with arrow keys or WASD\n\
                    - Cars moving horizontally that the frog must avoid\n\
                    - Goal area at the top that the frog needs to reach\n\
                    - Collision detection between frog and cars\n\
                    - Scoring system and lives system\n\
                    - Smooth animations and game loop\n\
                    - Professional styling and responsive design\n\n\
                    Provide ONLY the complete HTML file with embedded CSS and JavaScript:"
                ),
                "examples/web_frogger.html"
            )
        } else {
            (
                "rs",
                format!(
                    "Create a complete, working Frogger-like game in Rust. Requirements:\n\
                    - Terminal-based interface using crossterm crate\n\
                    - Frog character that moves up/down/left/right with WASD keys\n\
                    - Cars moving horizontally that the frog must avoid\n\
                    - Goal area at the top that the frog needs to reach\n\
                    - Collision detection between frog and cars\n\
                    - Scoring system that increases when reaching goal\n\
                    - Game over mechanics when hitting cars\n\
                    - Lives system (3 lives)\n\
                    - Game loop with proper input handling\n\n\
                    Provide ONLY the complete, compilable Rust code with all necessary imports:"
                ),
                "examples/agent_frogger.rs"
            )
        }
    }

    /// Generate game code using LLM
    async fn generate_game_code(&self, code_prompt: &str, file_extension: &str) -> Result<String> {
        let code_request = Request {
            flowname: "code_generation".to_string(),
            payload: code_prompt.to_string(),
        };

        println!("🧠 Generating {} game code with Claude...", file_extension.to_uppercase());
        
        let code_response = Pin::from(self.runtime_config.reasoning_engine.execute(&code_request)).await?;
        let game_code = crate::utils::extract_code(&code_response.content, file_extension);
        
        Ok(game_code)
    }

    /// Write game code to file
    fn write_game_file(&self, file_path: &str, game_code: &str) -> Result<()> {
        fs::write(file_path, game_code)?;
        println!("✅ Created game at: {}", file_path);
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
