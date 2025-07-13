use anyhow::{anyhow, Result};
use fluent_core::traits::Engine;
use fluent_core::types::Request;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

// Advanced agentic modules
pub mod action;
pub mod agent_with_mcp;
pub mod config;
pub mod context;
pub mod enhanced_mcp_client;
pub mod goal;
pub mod mcp_adapter;
pub mod mcp_client;
pub mod mcp_tool_registry;
pub mod mcp_resource_manager;
pub mod memory;
pub mod observation;
pub mod orchestrator;
pub mod performance;
pub mod profiling;
pub mod production_mcp;
pub mod reasoning;
pub mod reflection;
pub mod reflection_engine;
pub mod security;
pub mod state_manager;
pub mod task;
pub mod tools;
pub mod transport;
pub mod workflow;

// Re-export advanced agentic types
pub use action::{
    ActionExecutor, ActionPlanner, ComprehensiveActionExecutor, IntelligentActionPlanner,
};
pub use context::{ContextStats, ExecutionContext, ExecutionEvent};
pub use goal::{Goal, GoalPriority, GoalResult, GoalTemplates, GoalType};
pub use memory::{MemoryConfig, MemoryStats, MemorySystem};
pub use observation::{ComprehensiveObservationProcessor, ObservationProcessor};
pub use orchestrator::{AgentOrchestrator, AgentState as AdvancedAgentState, OrchestrationMetrics};
pub use production_mcp::{
    ProductionMcpManager, ProductionMcpConfig, McpError, HealthStatus, McpMetrics,
    initialize_production_mcp, initialize_production_mcp_with_config,
};
pub use reasoning::{LLMReasoningEngine, ReasoningCapability, ReasoningEngine};
pub use reflection_engine::{ReflectionEngine, ReflectionConfig, ReflectionResult, ReflectionType};
pub use state_manager::{StateManager, StateManagerConfig, StateRecoveryInfo};
pub use task::{Task, TaskPriority, TaskResult, TaskTemplates, TaskType};

/// Simple agent that keeps a history of prompt/response pairs.
pub struct Agent {
    engine: Box<dyn Engine>,
    history: Vec<(String, String)>,
}

impl Agent {
    /// Create a new agent from an engine.
    pub fn new(engine: Box<dyn Engine>) -> Self {
        Self {
            engine,
            history: Vec::new(),
        }
    }

    /// Send a prompt to the engine and store the response in history.
    pub async fn send(&mut self, prompt: &str) -> Result<String> {
        let request = Request {
            flowname: "agent".to_string(),
            payload: prompt.to_string(),
        };
        let response = Pin::from(self.engine.execute(&request)).await?;
        let content = response.content.clone();
        self.history.push((prompt.to_string(), content.clone()));
        Ok(content)
    }

    /// Read a file asynchronously.
    pub async fn read_file(&self, path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path).await?)
    }

    /// Write a file asynchronously.
    pub async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        fs::write(path, content).await.map_err(Into::into)
    }

    /// Run a shell command and capture stdout and stderr with security validation.
    pub async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        // Validate command against security policies
        Self::validate_command_security(cmd, args)?;

        let output = Command::new(cmd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear() // Clear environment for security
            .env("PATH", "/usr/bin:/bin:/usr/local/bin") // Minimal PATH
            .output()
            .await?;
        let mut result = String::from_utf8_lossy(&output.stdout).to_string();
        if !output.status.success() {
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }
        Ok(result)
    }

    /// Validate command and arguments against security policies
    fn validate_command_security(cmd: &str, args: &[&str]) -> Result<()> {
        // Get allowed commands from environment or use defaults
        let allowed_commands = Self::get_allowed_commands();

        // Check if command is in whitelist
        if !allowed_commands.contains(&cmd) {
            return Err(anyhow!("Command '{}' not in allowed list", cmd));
        }

        // Validate command name
        if cmd.len() > 100 {
            return Err(anyhow!("Command name too long"));
        }

        // Check for dangerous patterns in command
        let dangerous_patterns = ["../", "./", "/", "~", "$", "`", ";", "&", "|", ">", "<"];
        for pattern in &dangerous_patterns {
            if cmd.contains(pattern) {
                return Err(anyhow!("Command contains dangerous pattern: {}", pattern));
            }
        }

        // Validate arguments
        for arg in args {
            if arg.len() > 1000 {
                return Err(anyhow!("Argument too long"));
            }

            // Check for dangerous patterns in arguments
            for pattern in &dangerous_patterns {
                if arg.contains(pattern) {
                    return Err(anyhow!("Argument contains dangerous pattern: {}", pattern));
                }
            }
        }

        Ok(())
    }

    /// Get allowed commands from environment or defaults
    fn get_allowed_commands() -> Vec<&'static str> {
        // Check environment variable for custom allowed commands
        if let Ok(custom_commands) = std::env::var("FLUENT_ALLOWED_COMMANDS") {
            log::info!("Custom allowed commands: {}", custom_commands);
            // TODO: Parse and return custom commands with proper lifetime management
        }

        // Default allowed commands for agent operations
        vec![
            "cargo", "rustc", "git", "ls", "cat", "echo", "pwd", "which", "find"
        ]
    }

    /// Commit changes in the current git repository.
    pub async fn git_commit(&self, message: &str) -> Result<()> {
        self.run_command("git", &["add", "."]).await?;
        let status = Command::new("git")
            .args(["commit", "-m", message])
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow!("git commit failed"));
        }
        Ok(())
    }

    /// Run a simple plan -> generate -> test -> commit cycle using the engine.
    pub async fn run_cycle(&mut self, prompt: &str) -> Result<()> {
        let plan = self.send(&format!("Plan: {}", prompt)).await?;
        let _generation = self
            .send(&format!("Generate code based on plan:\n{}", plan))
            .await?;

        let test_output = self.run_command("cargo", &["test", "--quiet"]).await?;
        if !test_output.contains("0 failed") {
            return Err(anyhow!("tests failed"));
        }

        self.git_commit(prompt).await?;
        Ok(())
    }
}
