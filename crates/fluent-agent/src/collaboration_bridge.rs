//! Collaboration Bridge for Human-in-the-Loop Agent Orchestration
//!
//! This module bridges the agent orchestrator with the human control channel,
//! enabling real-time intervention, approvals, and collaborative decision-making.

use anyhow::{anyhow, Result};
use similar::{ChangeTag, TextDiff};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{oneshot, RwLock};
use uuid::Uuid;

use crate::action::{ActionPlan, ActionResult};
use crate::context::ExecutionContext;
use crate::orchestrator::{ActionType, AgentState, ReasoningResult};

/// Control messages from human (local module)
pub use crate::agent_control::{
    AgentControlChannel, AgentStatus as ControlAgentStatus, ApprovalContext, ApprovalRequest,
    ApprovalResponse, CodeDiff, ControlMessage, ControlMessageType, DefaultAction, DiffChangeType,
    DiffLine, GuidanceRequest, GuidanceResponse, LogLevel, RiskLevel, StateUpdate, StateUpdateType,
    StrategyUpdate,
};

/// Generate a code diff between old and new content
fn generate_code_diff(file_path: &str, old_content: &str, new_content: &str) -> CodeDiff {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut diff_lines = Vec::new();
    let mut old_line_num = 1;
    let mut new_line_num = 1;

    for change in diff.iter_all_changes() {
        let content = change.to_string();

        match change.tag() {
            ChangeTag::Delete => {
                diff_lines.push(DiffLine {
                    line_number: old_line_num,
                    change_type: DiffChangeType::Removed,
                    content: content.trim_end().to_string(),
                });
                old_line_num += 1;
            }
            ChangeTag::Insert => {
                diff_lines.push(DiffLine {
                    line_number: new_line_num,
                    change_type: DiffChangeType::Added,
                    content: content.trim_end().to_string(),
                });
                new_line_num += 1;
            }
            ChangeTag::Equal => {
                diff_lines.push(DiffLine {
                    line_number: old_line_num,
                    change_type: DiffChangeType::Unchanged,
                    content: content.trim_end().to_string(),
                });
                old_line_num += 1;
                new_line_num += 1;
            }
        }
    }

    CodeDiff {
        file_path: file_path.to_string(),
        old_content: old_content.to_string(),
        new_content: new_content.to_string(),
        diff_lines,
    }
}

/// Orchestrator with human-in-the-loop capabilities
pub struct CollaborativeOrchestrator {
    /// Control channel for human interaction
    control_channel: Option<Arc<AgentControlChannel>>,
    /// Whether the agent is currently paused
    paused: Arc<RwLock<bool>>,
    /// Pending approvals awaiting human response
    pending_approvals: Arc<RwLock<Vec<ApprovalRequest>>>,
    /// Configuration for approval requirements
    approval_config: ApprovalConfig,
}

/// Configuration for approval requirements
#[derive(Debug, Clone)]
pub struct ApprovalConfig {
    /// Require approval for file write operations
    pub require_file_write_approval: bool,
    /// Require approval for shell commands
    pub require_shell_command_approval: bool,
    /// Require approval for code generation
    pub require_code_generation_approval: bool,
    /// Risk level threshold for automatic approval
    pub auto_approve_below_risk: RiskLevel,
    /// Timeout for approval requests
    pub approval_timeout: Duration,
    /// Default action when approval times out
    pub timeout_default_action: DefaultAction,
}

impl Default for ApprovalConfig {
    fn default() -> Self {
        Self {
            require_file_write_approval: true,
            require_shell_command_approval: true,
            require_code_generation_approval: false,
            auto_approve_below_risk: RiskLevel::Low,
            approval_timeout: Duration::from_secs(300), // 5 minutes
            timeout_default_action: DefaultAction::Reject,
        }
    }
}

impl CollaborativeOrchestrator {
    /// Create a new collaborative orchestrator
    pub fn new(
        control_channel: Option<Arc<AgentControlChannel>>,
        approval_config: ApprovalConfig,
    ) -> Self {
        Self {
            control_channel,
            paused: Arc::new(RwLock::new(false)),
            pending_approvals: Arc::new(RwLock::new(Vec::new())),
            approval_config,
        }
    }

    /// Check control channel for human intervention before proceeding
    pub async fn check_control_channel(&self) -> Result<ControlAction> {
        // If paused, wait for resume
        if *self.paused.read().await {
            return Ok(ControlAction::WaitForResume);
        }

        let Some(ref channel) = self.control_channel else {
            return Ok(ControlAction::Continue);
        };

        // Check for pending control messages
        match channel.control_receiver().try_recv().await {
            Ok(Some(msg)) => {
                tracing::info!("Received control message: {:?}", msg.message_type);
                self.handle_control_message(msg).await
            }
            Ok(None) => Ok(ControlAction::Continue),
            Err(e) => {
                tracing::error!("Error receiving control message: {:?}", e);
                Ok(ControlAction::Continue)
            }
        }
    }

    /// Handle a control message from human
    async fn handle_control_message(&self, msg: ControlMessage) -> Result<ControlAction> {
        match msg.message_type {
            ControlMessageType::Pause => {
                *self.paused.write().await = true;
                self.send_state_update(StateUpdate::status_change(ControlAgentStatus::Paused))
                    .await?;
                tracing::info!("Agent paused by human");
                Ok(ControlAction::Pause)
            }

            ControlMessageType::Resume => {
                *self.paused.write().await = false;
                self.send_state_update(StateUpdate::status_change(ControlAgentStatus::Running))
                    .await?;
                tracing::info!("Agent resumed by human");
                Ok(ControlAction::Continue)
            }

            ControlMessageType::Approve {
                approval_id,
                comment,
            } => {
                self.handle_approval(approval_id, true, comment, None)
                    .await?;
                Ok(ControlAction::Continue)
            }

            ControlMessageType::Reject {
                approval_id,
                reason,
                alternative,
            } => {
                self.handle_approval(approval_id, false, Some(reason), alternative)
                    .await?;
                Ok(ControlAction::Continue)
            }

            ControlMessageType::Input {
                context,
                guidance,
                apply_to_future,
            } => {
                tracing::info!(
                    "Received human guidance: {} (apply_to_future: {})",
                    guidance,
                    apply_to_future
                );
                Ok(ControlAction::ApplyGuidance {
                    context,
                    guidance,
                    apply_to_future,
                })
            }

            ControlMessageType::ModifyGoal {
                new_goal,
                keep_context,
            } => {
                tracing::info!("Goal modification requested: {}", new_goal);
                Ok(ControlAction::ModifyGoal {
                    new_goal,
                    keep_context,
                })
            }

            ControlMessageType::ModifyStrategy { strategy_update } => {
                tracing::info!("Strategy modification requested");
                Ok(ControlAction::ModifyStrategy(strategy_update))
            }

            ControlMessageType::EmergencyStop { reason } => {
                tracing::warn!("Emergency stop requested: {}", reason);
                Ok(ControlAction::EmergencyStop(reason))
            }

            ControlMessageType::RequestExplanation { context } => {
                tracing::info!("Explanation requested for: {}", context);
                Ok(ControlAction::ProvideExplanation(context))
            }

            ControlMessageType::RequestStateSnapshot => {
                tracing::info!("State snapshot requested");
                Ok(ControlAction::SendStateSnapshot)
            }

            ControlMessageType::CreateCheckpoint { name } => {
                tracing::info!("Checkpoint creation requested: {}", name);
                Ok(ControlAction::CreateCheckpoint(name))
            }
        }
    }

    /// Handle approval response
    async fn handle_approval(
        &self,
        approval_id: Uuid,
        approved: bool,
        comment: Option<String>,
        alternative: Option<String>,
    ) -> Result<()> {
        let mut pending = self.pending_approvals.write().await;

        if let Some(pos) = pending.iter().position(|a| a.id == approval_id) {
            let mut approval = pending.remove(pos);

            // Send response through the oneshot channel
            if let Some(tx) = approval.response_tx.take() {
                let response = ApprovalResponse {
                    approved,
                    comment: comment.clone(),
                    alternative: alternative.clone(),
                    remember_decision: false,
                };

                if tx.send(response).is_err() {
                    tracing::error!("Failed to send approval response");
                }
            }

            // Send state update
            self.send_state_update(StateUpdate::new(StateUpdateType::ApprovalProcessed {
                approval_id,
                approved,
            }))
            .await?;

            tracing::info!(
                "Approval {} {}: {:?}",
                approval_id,
                if approved { "approved" } else { "rejected" },
                comment
            );
        } else {
            tracing::warn!("Approval {} not found in pending list", approval_id);
        }

        Ok(())
    }

    /// Request approval for an action
    pub async fn request_approval(
        &self,
        action_plan: &ActionPlan,
        context: &ExecutionContext,
    ) -> Result<bool> {
        // Check if approval is required for this action type
        if !self.requires_approval(action_plan) {
            return Ok(true); // Auto-approve
        }

        let Some(ref channel) = self.control_channel else {
            // No human in the loop, use default behavior
            return Ok(self.default_approval_decision(action_plan));
        };

        // Create approval request
        let (tx, rx) = oneshot::channel();
        let approval_id = Uuid::new_v4();

        let approval_request = ApprovalRequest {
            id: approval_id,
            timestamp: SystemTime::now(),
            action_type: format!("{:?}", action_plan.action_type),
            action_description: action_plan.description.clone(),
            risk_level: self.assess_risk(action_plan),
            risk_factors: self.identify_risk_factors(action_plan),
            context: self.create_approval_context(action_plan, context).await?,
            timeout: Some(self.approval_config.approval_timeout),
            default_action: self.approval_config.timeout_default_action.clone(),
            response_tx: Some(tx),
        };

        // Store in pending approvals
        {
            let mut pending = self.pending_approvals.write().await;
            pending.push(approval_request.clone());
        }

        // Send approval request to human
        let mut request_without_channel = approval_request.clone();
        request_without_channel.response_tx = None; // Can't serialize oneshot channel

        channel
            .send_state(StateUpdate::new(StateUpdateType::ApprovalRequested {
                approval: request_without_channel,
            }))
            .await?;

        // Update agent status
        self.send_state_update(StateUpdate::status_change(
            ControlAgentStatus::WaitingForApproval,
        ))
        .await?;

        // Wait for approval response with timeout
        match tokio::time::timeout(self.approval_config.approval_timeout, rx).await {
            Ok(Ok(response)) => {
                // Remove from pending
                let mut pending = self.pending_approvals.write().await;
                pending.retain(|a| a.id != approval_id);

                // Apply human's decision
                Ok(response.approved)
            }
            Ok(Err(_)) => {
                // Channel closed without response
                tracing::warn!("Approval channel closed without response");
                Ok(self.apply_default_action(&approval_request.default_action))
            }
            Err(_) => {
                // Timeout
                tracing::warn!(
                    "Approval timeout after {:?}, using default action",
                    self.approval_config.approval_timeout
                );
                Ok(self.apply_default_action(&self.approval_config.timeout_default_action))
            }
        }
    }

    /// Check if approval is required for this action
    fn requires_approval(&self, action_plan: &ActionPlan) -> bool {
        match action_plan.action_type {
            ActionType::FileOperation => self.approval_config.require_file_write_approval,
            ActionType::ToolExecution => {
                // Check if it's a shell command
                action_plan.description.to_lowercase().contains("shell")
                    || action_plan.description.to_lowercase().contains("command")
            }
            ActionType::CodeGeneration => self.approval_config.require_code_generation_approval,
            _ => false,
        }
    }

    /// Assess risk level for an action
    fn assess_risk(&self, action_plan: &ActionPlan) -> RiskLevel {
        // Risk assessment heuristics
        let mut risk_score = 0;

        // File operations
        if action_plan.action_type == ActionType::FileOperation {
            if action_plan.description.contains("delete") || action_plan.description.contains("rm")
            {
                risk_score += 3;
            } else if action_plan.description.contains("write")
                || action_plan.description.contains("modify")
            {
                risk_score += 2;
            }
        }

        // Shell commands
        if action_plan.description.to_lowercase().contains("shell") {
            risk_score += 2;
            if action_plan.description.contains("sudo") || action_plan.description.contains("rm") {
                risk_score += 3;
            }
        }

        // Convert score to risk level
        match risk_score {
            0..=1 => RiskLevel::Minimal,
            2..=3 => RiskLevel::Low,
            4..=5 => RiskLevel::Medium,
            6..=7 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    /// Identify specific risk factors
    fn identify_risk_factors(&self, action_plan: &ActionPlan) -> Vec<String> {
        let mut factors = Vec::new();

        if action_plan.description.contains("delete") {
            factors.push("Deletes existing data".to_string());
        }
        if action_plan.description.contains("overwrite") {
            factors.push("Overwrites existing file".to_string());
        }
        if action_plan.description.contains("shell") {
            factors.push("Executes shell command".to_string());
        }
        if action_plan.description.contains("sudo") {
            factors.push("Requires elevated privileges".to_string());
        }

        if factors.is_empty() {
            factors.push("Standard operation".to_string());
        }

        factors
    }

    /// Create approval context with details
    async fn create_approval_context(
        &self,
        action_plan: &ActionPlan,
        context: &ExecutionContext,
    ) -> Result<ApprovalContext> {
        // Extract alternative descriptions
        let alternatives: Vec<String> = action_plan
            .alternatives
            .iter()
            .map(|alt| alt.description.clone())
            .collect();

        // Generate code diff if old and new content are available
        let code_changes = self.extract_code_diff(action_plan);

        Ok(ApprovalContext {
            affected_files: self.extract_affected_files(action_plan),
            command: self.extract_command(action_plan),
            code_changes,
            reasoning: action_plan.description.clone(),
            alternatives,
            agent_recommendation: format!(
                "Proceed with action (confidence: {:.1}%)",
                action_plan.confidence_score * 100.0
            ),
        })
    }

    /// Extract and generate code diff from action plan parameters
    fn extract_code_diff(&self, action_plan: &ActionPlan) -> Option<CodeDiff> {
        // Extract file path
        let file_path = if let Some(path) = action_plan.parameters.get("path") {
            path.as_str()?.to_string()
        } else if let Some(file) = action_plan.parameters.get("file") {
            file.as_str()?.to_string()
        } else {
            return None;
        };

        // Extract old and new content
        let old_content = action_plan
            .parameters
            .get("old_content")
            .or_else(|| action_plan.parameters.get("previous_content"))
            .or_else(|| action_plan.parameters.get("original_content"))
            .and_then(|v| v.as_str())?;

        let new_content = action_plan
            .parameters
            .get("new_content")
            .or_else(|| action_plan.parameters.get("content"))
            .and_then(|v| v.as_str())?;

        // Generate and return the diff
        Some(generate_code_diff(&file_path, old_content, new_content))
    }

    /// Extract affected files from action plan
    fn extract_affected_files(&self, action_plan: &ActionPlan) -> Vec<String> {
        // Simple extraction - can be enhanced
        if let Some(path) = action_plan.parameters.get("path") {
            if let Some(path_str) = path.as_str() {
                return vec![path_str.to_string()];
            }
        }
        if let Some(file) = action_plan.parameters.get("file") {
            if let Some(file_str) = file.as_str() {
                return vec![file_str.to_string()];
            }
        }
        Vec::new()
    }

    /// Extract command from action plan
    fn extract_command(&self, action_plan: &ActionPlan) -> Option<String> {
        if let Some(cmd) = action_plan.parameters.get("command") {
            return cmd.as_str().map(|s| s.to_string());
        }
        None
    }

    /// Default approval decision without human input
    fn default_approval_decision(&self, action_plan: &ActionPlan) -> bool {
        let risk = self.assess_risk(action_plan);
        risk <= self.approval_config.auto_approve_below_risk
    }

    /// Apply default action when approval times out
    fn apply_default_action(&self, default_action: &DefaultAction) -> bool {
        match default_action {
            DefaultAction::Approve => true,
            DefaultAction::Reject => false,
            DefaultAction::Pause => {
                // Pause and wait - return false to block action
                false
            }
        }
    }

    /// Send state update to TUI
    async fn send_state_update(&self, update: StateUpdate) -> Result<()> {
        if let Some(ref channel) = self.control_channel {
            channel.send_state(update).await?;
        }
        Ok(())
    }

    /// Send reasoning step update
    pub async fn send_reasoning_update(
        &self,
        step_description: String,
        confidence: f64,
        thought_process: String,
    ) -> Result<()> {
        self.send_state_update(StateUpdate::new(StateUpdateType::ReasoningStep {
            step_description,
            confidence,
            thought_process,
        }))
        .await
    }

    /// Send action update
    pub async fn send_action_update(
        &self,
        action_description: String,
        action_type: String,
    ) -> Result<()> {
        self.send_state_update(StateUpdate::action_update(action_description, action_type))
            .await
    }

    /// Send iteration update
    pub async fn send_iteration_update(&self, current: u32, max: u32) -> Result<()> {
        let progress_percentage = if max > 0 {
            ((current as f32 / max as f32) * 100.0) as u32
        } else {
            0
        };
        self.send_state_update(StateUpdate::iteration_update(
            current,
            max,
            progress_percentage,
        ))
        .await
    }

    /// Send log message
    pub async fn send_log(&self, level: LogLevel, message: String) -> Result<()> {
        self.send_state_update(StateUpdate::log(level, message))
            .await
    }

    /// Send error update
    pub async fn send_error(&self, error: String, recoverable: bool) -> Result<()> {
        self.send_state_update(StateUpdate::error(error, recoverable))
            .await
    }

    /// Check if agent is paused
    pub async fn is_paused(&self) -> bool {
        *self.paused.read().await
    }

    /// Wait while paused
    pub async fn wait_while_paused(&self) -> Result<()> {
        while self.is_paused().await {
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Check for resume command
            if let Ok(ControlAction::Continue) = self.check_control_channel().await {
                break;
            }
        }
        Ok(())
    }
}

/// Action to take based on control message
#[derive(Debug)]
pub enum ControlAction {
    Continue,
    Pause,
    WaitForResume,
    ApplyGuidance {
        context: String,
        guidance: String,
        apply_to_future: bool,
    },
    ModifyGoal {
        new_goal: String,
        keep_context: bool,
    },
    ModifyStrategy(StrategyUpdate),
    EmergencyStop(String),
    ProvideExplanation(String),
    SendStateSnapshot,
    CreateCheckpoint(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_config_default() {
        let config = ApprovalConfig::default();
        assert!(config.require_file_write_approval);
        assert!(config.require_shell_command_approval);
    }

    #[tokio::test]
    async fn test_collaborative_orchestrator_creation() {
        let orchestrator = CollaborativeOrchestrator::new(None, ApprovalConfig::default());
        assert!(!orchestrator.is_paused().await);
    }

    #[tokio::test]
    async fn test_check_control_channel_without_channel() {
        let orchestrator = CollaborativeOrchestrator::new(None, ApprovalConfig::default());
        let action = orchestrator.check_control_channel().await.unwrap();
        assert!(matches!(action, ControlAction::Continue));
    }

    #[test]
    fn test_generate_code_diff_simple() {
        let old = "line1\nline2\nline3";
        let new = "line1\nmodified\nline3";

        let diff = generate_code_diff("test.rs", old, new);

        assert_eq!(diff.file_path, "test.rs");
        assert_eq!(diff.old_content, old);
        assert_eq!(diff.new_content, new);

        // Check that we have the expected diff lines
        let added_lines: Vec<_> = diff
            .diff_lines
            .iter()
            .filter(|line| matches!(line.change_type, DiffChangeType::Added))
            .collect();
        let removed_lines: Vec<_> = diff
            .diff_lines
            .iter()
            .filter(|line| matches!(line.change_type, DiffChangeType::Removed))
            .collect();

        assert_eq!(added_lines.len(), 1);
        assert_eq!(removed_lines.len(), 1);
        assert!(added_lines[0].content.contains("modified"));
        assert!(removed_lines[0].content.contains("line2"));
    }

    #[test]
    fn test_generate_code_diff_additions_only() {
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\nline4\n";

        let diff = generate_code_diff("test.rs", old, new);

        let added_lines: Vec<_> = diff
            .diff_lines
            .iter()
            .filter(|line| matches!(line.change_type, DiffChangeType::Added))
            .collect();

        assert_eq!(added_lines.len(), 2);
        assert!(added_lines[0].content.contains("line3"));
        assert!(added_lines[1].content.contains("line4"));
    }

    #[test]
    fn test_generate_code_diff_deletions_only() {
        let old = "line1\nline2\nline3\nline4\n";
        let new = "line1\nline2\n";

        let diff = generate_code_diff("test.rs", old, new);

        let removed_lines: Vec<_> = diff
            .diff_lines
            .iter()
            .filter(|line| matches!(line.change_type, DiffChangeType::Removed))
            .collect();

        assert_eq!(removed_lines.len(), 2);
        assert!(removed_lines[0].content.contains("line3"));
        assert!(removed_lines[1].content.contains("line4"));
    }

    #[test]
    fn test_generate_code_diff_no_changes() {
        let content = "line1\nline2\nline3";

        let diff = generate_code_diff("test.rs", content, content);

        let changed_lines: Vec<_> = diff
            .diff_lines
            .iter()
            .filter(|line| !matches!(line.change_type, DiffChangeType::Unchanged))
            .collect();

        assert_eq!(changed_lines.len(), 0);

        let unchanged_lines: Vec<_> = diff
            .diff_lines
            .iter()
            .filter(|line| matches!(line.change_type, DiffChangeType::Unchanged))
            .collect();

        assert_eq!(unchanged_lines.len(), 3);
    }

    #[test]
    fn test_extract_code_diff_with_parameters() {
        use std::collections::HashMap;

        let orchestrator = CollaborativeOrchestrator::new(None, ApprovalConfig::default());

        let mut parameters = HashMap::new();
        parameters.insert(
            "path".to_string(),
            serde_json::Value::String("test.rs".to_string()),
        );
        parameters.insert(
            "old_content".to_string(),
            serde_json::Value::String("old line".to_string()),
        );
        parameters.insert(
            "new_content".to_string(),
            serde_json::Value::String("new line".to_string()),
        );

        let action_plan = ActionPlan {
            action_id: "test".to_string(),
            action_type: ActionType::FileOperation,
            description: "Test action".to_string(),
            parameters,
            expected_outcome: "Test".to_string(),
            success_criteria: vec![],
            confidence_score: 0.9,
            estimated_duration: None,
            risk_level: crate::action::RiskLevel::Low,
            alternatives: vec![],
            prerequisites: vec![],
        };

        let diff = orchestrator.extract_code_diff(&action_plan);
        assert!(diff.is_some());

        let diff = diff.unwrap();
        assert_eq!(diff.file_path, "test.rs");
        assert_eq!(diff.old_content, "old line");
        assert_eq!(diff.new_content, "new line");
    }

    #[test]
    fn test_extract_code_diff_missing_parameters() {
        use std::collections::HashMap;

        let orchestrator = CollaborativeOrchestrator::new(None, ApprovalConfig::default());

        // Test with missing old_content
        let mut parameters = HashMap::new();
        parameters.insert(
            "path".to_string(),
            serde_json::Value::String("test.rs".to_string()),
        );
        parameters.insert(
            "new_content".to_string(),
            serde_json::Value::String("new line".to_string()),
        );

        let action_plan = ActionPlan {
            action_id: "test".to_string(),
            action_type: ActionType::FileOperation,
            description: "Test action".to_string(),
            parameters,
            expected_outcome: "Test".to_string(),
            success_criteria: vec![],
            confidence_score: 0.9,
            estimated_duration: None,
            risk_level: crate::action::RiskLevel::Low,
            alternatives: vec![],
            prerequisites: vec![],
        };

        let diff = orchestrator.extract_code_diff(&action_plan);
        assert!(diff.is_none());
    }
}
