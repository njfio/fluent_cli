#![allow(unused)]
#![allow(ambiguous_glob_reexports)]

//! # Fluent Agent - Advanced Agentic Framework
//!
//! This crate provides advanced agentic capabilities for the Fluent CLI system,
//! including reasoning engines, action planning, memory systems, and Model Context Protocol (MCP) integration.
//!
//! ## ✅ Production Status
//!
//! Core framework components are production-ready with comprehensive error handling and security validation.
//! Advanced features continue to be developed with backward compatibility maintained.
//!
//! ## 🔒 Security Features
//!
//! Production-ready security implementations:
//! - **Command Validation**: Environment-configurable command whitelisting with security checks
//! - **File System Security**: Permission controls and path validation
//! - **Input Sanitization**: Comprehensive validation of user inputs and command arguments
//! - **Error Handling**: Zero unwrap() calls in production code, comprehensive Result types
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
// use security::security_framework::SecurityFramework;
use std::path::Path;
use std::pin::Pin;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

// Advanced agentic modules
pub mod action;
pub mod adapters;
pub mod advanced_tools;
pub mod agent_control;
pub mod agent_with_mcp;
pub mod autonomy;
pub mod benchmarks;
pub mod collaboration_bridge;
pub mod config;
pub mod context;
pub mod enhanced_mcp_client;
pub mod execution;
pub mod ethical_guardrails;
pub mod goal;
pub mod human_collaboration;
pub mod mcp_adapter;
pub mod mcp_client;
pub mod mcp_resource_manager;
pub mod mcp_tool_registry;
pub mod memory;
pub mod monitoring;
pub mod observation;
pub mod orchestrator;
pub mod performance;
pub mod planning;
pub mod production_mcp;
pub mod profiling;
pub mod prompts;
pub mod reasoning;
pub mod reflection;
pub mod reflection_engine;
pub mod security;
pub mod state_manager;
pub mod swarm_intelligence;
pub mod task;
pub mod tools;
pub mod transport;
pub mod web_dashboard;
pub mod workflow;

// Re-export advanced agentic types
pub use action::{
    parse_structured_action, ActionExecutor, ActionPlanner, ComprehensiveActionExecutor,
    IntelligentActionPlanner, StructuredAction,
};
pub use advanced_tools::{
    AdvancedTool, AdvancedToolRegistry, ToolCategory, ToolParameters, ToolPriority, ToolResult,
};
pub use agent_control::{
    AgentControlChannel, ApprovalRequest, ApprovalResponse, ControlMessage, ControlMessageType,
    StateUpdate, StateUpdateType,
};
pub use autonomy::{
    AutonomySupervisor, AutonomySupervisorConfig, GuardrailDecision, RiskAssessment,
    SupervisorIncident, SupervisorStage,
};
pub use benchmarks::{AutonomousBenchmarkSuite, BenchmarkConfig, BenchmarkResult, BenchmarkType};
pub use collaboration_bridge::{ApprovalConfig, CollaborativeOrchestrator, ControlAction};
pub use context::{ContextStats, ExecutionContext, ExecutionEvent};
pub use ethical_guardrails::{
    EthicalEvaluation, EthicalGuardrailsSystem, EthicalRecommendation, FilterResult, HarmCategory,
    RiskLevel,
};
pub use goal::{Goal, GoalPriority, GoalResult, GoalTemplates, GoalType};
pub use human_collaboration::{
    ApprovalRequest as HumanApprovalRequest, ApprovalStatus, ApprovalType, CollaborationEvent,
    CollaborationMessage, CollaborationSession, CommunicationChannels, FeedbackEntry,
    FeedbackSystem, FeedbackType, HumanCollaborationCoordinator, HumanCollaborationInterface,
    Intervention, InterventionManager, InterventionOutcome, InterventionPriority,
    InterventionRequester, InterventionResponse, InterventionStatus, InterventionType,
    MessageSender, MessageType, SessionStatus, UserProfile,
};
pub use memory::{
    ContextCompressor, CrossSessionPersistence, IntegratedMemorySystem, MemoryConfig,
    MemoryContent, MemoryItem, MemoryStats, MemorySystem, WorkingMemory,
};
pub use monitoring::{
    AdaptiveStrategySystem, ErrorInstance, ErrorRecoverySystem, ErrorSeverity, ErrorType,
    PerformanceMetrics, PerformanceMonitor, QualityMetrics, RecoveryConfig, RecoveryResult,
};
pub use observation::{ComprehensiveObservationProcessor, ObservationProcessor};
pub use orchestrator::{AgentOrchestrator, AgentState as AdvancedAgentState, OrchestrationMetrics};
pub use planning::{
    CompletePlanningResult, CompositePlanner, DependencyAnalyzer, DynamicReplanner, HTNConfig,
    HTNPlanner, HTNResult,
};
pub use production_mcp::{
    initialize_production_mcp, initialize_production_mcp_with_config, HealthStatus, McpError,
    McpMetrics, ProductionMcpConfig, ProductionMcpManager,
};
pub use reasoning::{
    AudioData, BinaryData, ChainOfThoughtEngine, CoTConfig, CoTReasoningResult, CodeContent,
    CompositeReasoningEngine, CrossModalRelationship, ImageData, MetaConfig, MetaReasoningEngine,
    MetaReasoningResult, MultiModalInput, MultiModalReasoningEngine, MultiModalReasoningResult,
    ReasoningCapability, ReasoningEngine, StructuredData, ToTConfig, ToTReasoningResult,
    TreeOfThoughtEngine,
};
pub use reflection_engine::{ReflectionConfig, ReflectionEngine, ReflectionResult, ReflectionType};
pub use security::capability::CapabilityManager;
pub use state_manager::{StateManager, StateManagerConfig, StateRecoveryInfo};
pub use swarm_intelligence::{
    AgentPerformance, AgentSpecialization, AgentStatus, ConsensusEngine, Message, SwarmAgent,
    SwarmCoordinator, SwarmMetrics, SwarmResult, SwarmTask, TaskAllocator,
    TaskPriority as SwarmTaskPriority, TaskResult as SwarmTaskResult, Vote,
};
pub use task::{Task, TaskPriority, TaskResult, TaskTemplates, TaskType};
pub use web_dashboard::{DashboardConfig, WebDashboard};

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

    /// Run a shell command with security validation, timeout and output limits.
    pub async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        // Validate command against security policies using unified validator
        let validator = crate::security::command_validator::CommandValidator::from_environment();
        let args_string: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        validator.validate(cmd, &args_string)?;

        // Determine limits from environment or defaults
        let timeout_secs: u64 = std::env::var("FLUENT_CMD_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        let max_output_bytes: usize = std::env::var("FLUENT_CMD_MAX_OUTPUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(512 * 1024); // 512 KiB

        let mut child = Command::new(cmd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .env_clear()
            .env("PATH", "/usr/bin:/bin:/usr/local/bin")
            .spawn()?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture stdout"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("failed to capture stderr"))?;

        use tokio::io::AsyncReadExt;
        let mut out_buf: Vec<u8> = Vec::with_capacity(8 * 1024);
        let mut err_buf: Vec<u8> = Vec::with_capacity(8 * 1024);

        let read_fut = async {
            let mut tmp_out = [0u8; 8192];
            let mut tmp_err = [0u8; 8192];
            loop {
                tokio::select! {
                    read = stdout.read(&mut tmp_out) => {
                        let n = read?;
                        if n == 0 { break; }
                        let to_take = n.min(max_output_bytes.saturating_sub(out_buf.len()));
                        out_buf.extend_from_slice(&tmp_out[..to_take]);
                        if out_buf.len() >= max_output_bytes { break; }
                    }
                    read = stderr.read(&mut tmp_err) => {
                        let n = read?;
                        if n == 0 { break; }
                        let to_take = n.min(max_output_bytes.saturating_sub(err_buf.len()));
                        err_buf.extend_from_slice(&tmp_err[..to_take]);
                        if err_buf.len() >= max_output_bytes { break; }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        };

        // Apply timeout to child and reading
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), read_fut).await {
            Ok(r) => r?,
            Err(_) => {
                let _ = child.kill().await; // best-effort
                return Err(anyhow!("command timed out after {}s", timeout_secs));
            }
        }

        let status =
            match tokio::time::timeout(std::time::Duration::from_secs(2), child.wait()).await {
                Ok(r) => r?,
                Err(_) => {
                    let _ = child.kill().await;
                    return Err(anyhow!(
                        "command did not terminate promptly after output read"
                    ));
                }
            };

        let mut combined = String::new();
        combined.push_str(&String::from_utf8_lossy(&out_buf));
        if !status.success() {
            combined.push_str(&String::from_utf8_lossy(&err_buf));
        }

        if out_buf.len() >= max_output_bytes || err_buf.len() >= max_output_bytes {
            combined.push_str("\n[output truncated]\n");
        }

        Ok(combined)
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
