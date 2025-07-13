//! # Fluent Agent - Advanced Agentic Framework
//!
//! This crate provides advanced agentic capabilities for the Fluent CLI system,
//! including reasoning engines, action planning, memory systems, and Model Context Protocol (MCP) integration.
//!
//! ## ⚠️ Development Status
//!
//! This framework is under active development. While core functionality is stable,
//! some advanced features are experimental and should be thoroughly tested before production use.
//!
//! ## 🔒 Security Considerations
//!
//! This crate includes security-sensitive components:
//! - Command execution with validation and sandboxing
//! - File system operations with permission controls
//! - MCP client/server implementations with transport security
//! - Memory systems with data persistence
//!
//! Always review security configurations before deployment and follow the security
//! guidelines provided in individual module documentation.
//!
//! ## 🏗️ Architecture
//!
//! The agent framework is built around several core components:
//! - **Reasoning Engine**: LLM-powered decision making
//! - **Action Planning**: Task decomposition and execution planning
//! - **Memory System**: Persistent storage for agent state and learning
//! - **Observation Processing**: Environment feedback analysis
//! - **Security Framework**: Comprehensive security controls and validation
//! - **MCP Integration**: Model Context Protocol client and server support

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
        if !allowed_commands.iter().any(|allowed| allowed == cmd) {
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
    fn get_allowed_commands() -> Vec<String> {
        // Check environment variable for custom allowed commands
        if let Ok(custom_commands) = std::env::var("FLUENT_ALLOWED_COMMANDS") {
            log::info!("Custom allowed commands: {}", custom_commands);

            // Parse comma-separated commands with proper validation
            let parsed_commands: Vec<String> = custom_commands
                .split(',')
                .map(|cmd| cmd.trim().to_string())
                .filter(|cmd| !cmd.is_empty() && Self::is_valid_command_name(cmd))
                .collect();

            if !parsed_commands.is_empty() {
                log::info!("Using {} custom allowed commands", parsed_commands.len());
                return parsed_commands;
            } else {
                log::warn!("No valid commands found in FLUENT_ALLOWED_COMMANDS, using defaults");
            }
        }

        // Default allowed commands for agent operations
        vec![
            "cargo".to_string(),
            "rustc".to_string(),
            "git".to_string(),
            "ls".to_string(),
            "cat".to_string(),
            "echo".to_string(),
            "pwd".to_string(),
            "which".to_string(),
            "find".to_string()
        ]
    }

    /// Validate that a command name is safe and reasonable
    fn is_valid_command_name(cmd: &str) -> bool {
        // Basic validation: alphanumeric, dash, underscore only
        // No paths, no shell metacharacters
        if cmd.is_empty() || cmd.len() > 50 {
            return false;
        }

        // Must start with alphanumeric
        if !cmd.chars().next().unwrap_or(' ').is_ascii_alphanumeric() {
            return false;
        }

        // Only allow safe characters
        cmd.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && !cmd.contains('/') // No paths
            && !cmd.contains('\\') // No Windows paths
            && !cmd.contains(' ') // No spaces
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
