//! Agent Control Channel for Human-in-the-Loop Interaction
//!
//! This module provides bidirectional communication between the TUI (human interface)
//! and the agent orchestrator, enabling real-time human intervention, approvals,
//! guidance, and collaborative decision-making.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Capacity for control message channels
const CONTROL_CHANNEL_CAPACITY: usize = 100;
const STATE_CHANNEL_CAPACITY: usize = 1000;

/// Agent control channel for bidirectional communication
#[derive(Clone)]
pub struct AgentControlChannel {
    /// Send control messages from TUI to agent
    pub control_tx: mpsc::Sender<ControlMessage>,
    /// Receive control messages in agent
    pub control_rx_handle: ControlRxHandle,
    /// Send state updates from agent to TUI
    pub state_tx: mpsc::Sender<StateUpdate>,
    /// Receive state updates in TUI
    pub state_rx_handle: StateRxHandle,
}

/// Handle for receiving control messages (clone-safe)
#[derive(Clone)]
pub struct ControlRxHandle {
    rx: std::sync::Arc<tokio::sync::Mutex<mpsc::Receiver<ControlMessage>>>,
}

impl ControlRxHandle {
    pub async fn recv(&self) -> Option<ControlMessage> {
        self.rx.lock().await.recv().await
    }

    pub async fn try_recv(&self) -> Result<Option<ControlMessage>, mpsc::error::TryRecvError> {
        match self.rx.lock().await.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Handle for receiving state updates (clone-safe)
#[derive(Clone)]
pub struct StateRxHandle {
    rx: std::sync::Arc<tokio::sync::Mutex<mpsc::Receiver<StateUpdate>>>,
}

impl StateRxHandle {
    pub async fn recv(&self) -> Option<StateUpdate> {
        self.rx.lock().await.recv().await
    }

    pub async fn try_recv(&self) -> Result<Option<StateUpdate>, mpsc::error::TryRecvError> {
        match self.rx.lock().await.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl AgentControlChannel {
    /// Create a new agent control channel
    pub fn new() -> Self {
        let (control_tx, control_rx) = mpsc::channel(CONTROL_CHANNEL_CAPACITY);
        let (state_tx, state_rx) = mpsc::channel(STATE_CHANNEL_CAPACITY);

        Self {
            control_tx,
            control_rx_handle: ControlRxHandle {
                rx: std::sync::Arc::new(tokio::sync::Mutex::new(control_rx)),
            },
            state_tx,
            state_rx_handle: StateRxHandle {
                rx: std::sync::Arc::new(tokio::sync::Mutex::new(state_rx)),
            },
        }
    }

    /// Send a control message from TUI to agent
    pub async fn send_control(&self, message: ControlMessage) -> Result<()> {
        self.control_tx
            .send(message)
            .await
            .map_err(|e| anyhow!("Failed to send control message: {}", e))
    }

    /// Send a state update from agent to TUI
    pub async fn send_state(&self, update: StateUpdate) -> Result<()> {
        self.state_tx
            .send(update)
            .await
            .map_err(|e| anyhow!("Failed to send state update: {}", e))
    }

    /// Get control receiver handle
    pub fn control_receiver(&self) -> ControlRxHandle {
        self.control_rx_handle.clone()
    }

    /// Get state receiver handle
    pub fn state_receiver(&self) -> StateRxHandle {
        self.state_rx_handle.clone()
    }
}

impl Default for AgentControlChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// Control message from human to agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMessage {
    pub id: Uuid,
    pub timestamp: SystemTime,
    pub message_type: ControlMessageType,
    pub metadata: HashMap<String, String>,
}

/// Types of control messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessageType {
    /// Pause agent execution
    Pause,
    /// Resume agent execution
    Resume,
    /// Approve pending action
    Approve {
        approval_id: Uuid,
        comment: Option<String>,
    },
    /// Reject pending action
    Reject {
        approval_id: Uuid,
        reason: String,
        alternative: Option<String>,
    },
    /// Provide guidance or input
    Input {
        context: String,
        guidance: String,
        apply_to_future: bool,
    },
    /// Modify current goal
    ModifyGoal {
        new_goal: String,
        keep_context: bool,
    },
    /// Modify agent strategy or parameters
    ModifyStrategy { strategy_update: StrategyUpdate },
    /// Request detailed explanation
    RequestExplanation { context: String },
    /// Emergency stop
    EmergencyStop { reason: String },
    /// Request agent state snapshot
    RequestStateSnapshot,
    /// Checkpoint current state
    CreateCheckpoint { name: String },
}

/// Strategy update parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyUpdate {
    pub max_iterations: Option<u32>,
    pub confidence_threshold: Option<f64>,
    pub risk_tolerance: Option<RiskTolerance>,
    pub tools_enabled: Option<bool>,
    pub reflection_enabled: Option<bool>,
    pub custom_parameters: HashMap<String, serde_json::Value>,
}

/// Risk tolerance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskTolerance {
    Conservative, // Require approval for most actions
    Moderate,     // Default behavior
    Aggressive,   // Minimize approval requests
}

/// State update from agent to TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdate {
    pub timestamp: SystemTime,
    pub update_type: StateUpdateType,
}

/// Types of state updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateUpdateType {
    /// Agent status changed
    StatusChange { status: AgentStatus },
    /// Iteration progress
    IterationUpdate {
        current: u32,
        max: u32,
        progress_percentage: u32,
    },
    /// Current action changed
    ActionUpdate {
        action_description: String,
        action_type: String,
        estimated_duration: Option<Duration>,
    },
    /// Approval requested
    ApprovalRequested { approval: ApprovalRequest },
    /// Approval processed
    ApprovalProcessed { approval_id: Uuid, approved: bool },
    /// Human guidance requested
    GuidanceRequested { request: GuidanceRequest },
    /// Log message
    LogMessage { level: LogLevel, message: String },
    /// Reasoning step completed
    ReasoningStep {
        step_description: String,
        confidence: f64,
        thought_process: String,
    },
    /// Error occurred
    Error {
        error: String,
        recoverable: bool,
        suggested_actions: Vec<String>,
    },
    /// Goal progress
    GoalProgress {
        goal_description: String,
        completion_percentage: f64,
        achieved_criteria: Vec<String>,
        remaining_criteria: Vec<String>,
    },
    /// Performance metrics
    PerformanceMetrics { metrics: HashMap<String, f64> },
    /// Memory state
    MemoryState {
        working_memory_items: usize,
        long_term_memory_items: usize,
        memory_usage_mb: f64,
    },
    /// State snapshot
    StateSnapshot { snapshot: AgentStateSnapshot },
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Initializing,
    Running,
    Paused,
    WaitingForApproval,
    WaitingForGuidance,
    Completed,
    Failed(String),
    Timeout,
}

/// Approval request from agent
#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub timestamp: SystemTime,
    pub action_type: String,
    pub action_description: String,
    pub risk_level: RiskLevel,
    pub risk_factors: Vec<String>,
    pub context: ApprovalContext,
    pub timeout: Option<Duration>,
    pub default_action: DefaultAction,
    /// Response channel for approval result
    #[serde(skip)]
    pub response_tx: Option<oneshot::Sender<ApprovalResponse>>,
}

impl Clone for ApprovalRequest {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            timestamp: self.timestamp,
            action_type: self.action_type.clone(),
            action_description: self.action_description.clone(),
            risk_level: self.risk_level.clone(),
            risk_factors: self.risk_factors.clone(),
            context: self.context.clone(),
            timeout: self.timeout,
            default_action: self.default_action.clone(),
            response_tx: None, // Can't clone oneshot::Sender
        }
    }
}

/// Context for approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalContext {
    pub affected_files: Vec<String>,
    pub command: Option<String>,
    pub code_changes: Option<CodeDiff>,
    pub reasoning: String,
    pub alternatives: Vec<String>,
    pub agent_recommendation: String,
}

/// Code diff for approval preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeDiff {
    pub file_path: String,
    pub old_content: String,
    pub new_content: String,
    pub diff_lines: Vec<DiffLine>,
}

/// Individual diff line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    pub line_number: usize,
    pub change_type: DiffChangeType,
    pub content: String,
}

/// Type of diff change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffChangeType {
    Added,
    Removed,
    Modified,
    Unchanged,
}

/// Risk level for actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
}

/// Default action when approval times out
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefaultAction {
    Approve,
    Reject,
    Pause,
}

/// Response to approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub approved: bool,
    pub comment: Option<String>,
    pub alternative: Option<String>,
    pub remember_decision: bool,
}

/// Guidance request from agent
#[derive(Debug, Serialize, Deserialize)]
pub struct GuidanceRequest {
    pub id: Uuid,
    pub timestamp: SystemTime,
    pub reason: GuidanceReason,
    pub context: String,
    pub options: Vec<String>,
    pub recommended_option: Option<usize>,
    pub urgency: RequestUrgency,
    /// Response channel for guidance
    #[serde(skip)]
    pub response_tx: Option<oneshot::Sender<GuidanceResponse>>,
}

impl Clone for GuidanceRequest {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            timestamp: self.timestamp,
            reason: self.reason.clone(),
            context: self.context.clone(),
            options: self.options.clone(),
            recommended_option: self.recommended_option,
            urgency: self.urgency.clone(),
            response_tx: None, // Can't clone oneshot::Sender
        }
    }
}

/// Reason for requesting guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuidanceReason {
    LowConfidence(f64),
    AmbiguousRequirement,
    MultipleValidApproaches,
    UncertaintyInData,
    ConflictingConstraints,
    NovelSituation,
}

/// Urgency of guidance request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestUrgency {
    Low,
    Medium,
    High,
    Critical,
}

/// Response to guidance request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidanceResponse {
    pub selected_option: Option<usize>,
    pub custom_guidance: Option<String>,
    pub confidence: f64,
    pub apply_to_similar_situations: bool,
}

/// Log level for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Snapshot of agent state for inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateSnapshot {
    pub timestamp: SystemTime,
    pub iteration: u32,
    pub current_goal: Option<String>,
    pub active_tasks: Vec<String>,
    pub completed_tasks: Vec<String>,
    pub reasoning_history: Vec<String>,
    pub confidence_score: f64,
    pub memory_stats: MemoryStats,
    pub performance_stats: PerformanceStats,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub working_memory_size: usize,
    pub long_term_memory_size: usize,
    pub cache_hit_rate: f64,
    pub memory_usage_bytes: usize,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub average_iteration_time: Duration,
    pub total_actions: u64,
    pub successful_actions: u64,
    pub failed_actions: u64,
    pub average_confidence: f64,
}

impl ControlMessage {
    /// Create a new control message
    pub fn new(message_type: ControlMessageType) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            message_type,
            metadata: HashMap::new(),
        }
    }

    /// Create a pause message
    pub fn pause() -> Self {
        Self::new(ControlMessageType::Pause)
    }

    /// Create a resume message
    pub fn resume() -> Self {
        Self::new(ControlMessageType::Resume)
    }

    /// Create an approve message
    pub fn approve(approval_id: Uuid, comment: Option<String>) -> Self {
        Self::new(ControlMessageType::Approve {
            approval_id,
            comment,
        })
    }

    /// Create a reject message
    pub fn reject(approval_id: Uuid, reason: String, alternative: Option<String>) -> Self {
        Self::new(ControlMessageType::Reject {
            approval_id,
            reason,
            alternative,
        })
    }

    /// Create an input message
    pub fn input(context: String, guidance: String, apply_to_future: bool) -> Self {
        Self::new(ControlMessageType::Input {
            context,
            guidance,
            apply_to_future,
        })
    }

    /// Create an emergency stop message
    pub fn emergency_stop(reason: String) -> Self {
        Self::new(ControlMessageType::EmergencyStop { reason })
    }
}

impl StateUpdate {
    /// Create a new state update
    pub fn new(update_type: StateUpdateType) -> Self {
        Self {
            timestamp: SystemTime::now(),
            update_type,
        }
    }

    /// Create a status change update
    pub fn status_change(status: AgentStatus) -> Self {
        Self::new(StateUpdateType::StatusChange { status })
    }

    /// Create an iteration update
    pub fn iteration_update(current: u32, max: u32, progress_percentage: u32) -> Self {
        Self::new(StateUpdateType::IterationUpdate {
            current,
            max,
            progress_percentage,
        })
    }

    /// Create an action update
    pub fn action_update(action_description: String, action_type: String) -> Self {
        Self::new(StateUpdateType::ActionUpdate {
            action_description,
            action_type,
            estimated_duration: None,
        })
    }

    /// Create a log message update
    pub fn log(level: LogLevel, message: String) -> Self {
        Self::new(StateUpdateType::LogMessage { level, message })
    }

    /// Create an error update
    pub fn error(error: String, recoverable: bool) -> Self {
        Self::new(StateUpdateType::Error {
            error,
            recoverable,
            suggested_actions: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_control_channel_creation() {
        let channel = AgentControlChannel::new();
        assert!(channel.control_tx.capacity() > 0);
        assert!(channel.state_tx.capacity() > 0);
    }

    #[tokio::test]
    async fn test_send_control_message() {
        let channel = AgentControlChannel::new();
        let message = ControlMessage::pause();

        let result = channel.send_control(message.clone()).await;
        assert!(result.is_ok());

        let rx = channel.control_receiver();
        let received = rx.recv().await;
        assert!(received.is_some());
    }

    #[tokio::test]
    async fn test_send_state_update() {
        let channel = AgentControlChannel::new();
        let update = StateUpdate::status_change(AgentStatus::Running);

        let result = channel.send_state(update).await;
        assert!(result.is_ok());

        let rx = channel.state_receiver();
        let received = rx.recv().await;
        assert!(received.is_some());
    }

    #[tokio::test]
    async fn test_approval_request_creation() {
        let approval = ApprovalRequest {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            action_type: "file_write".to_string(),
            action_description: "Write to config file".to_string(),
            risk_level: RiskLevel::Medium,
            risk_factors: vec!["Overwrites existing file".to_string()],
            context: ApprovalContext {
                affected_files: vec!["config.toml".to_string()],
                command: None,
                code_changes: None,
                reasoning: "Need to update configuration".to_string(),
                alternatives: vec!["Create backup first".to_string()],
                agent_recommendation: "Proceed with backup".to_string(),
            },
            timeout: Some(Duration::from_secs(60)),
            default_action: DefaultAction::Reject,
            response_tx: None,
        };

        assert_eq!(approval.risk_level, RiskLevel::Medium);
        assert_eq!(approval.action_type, "file_write");
    }
}
