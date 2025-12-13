//! Human-in-the-Loop Collaboration System
//!
//! This module implements real-time collaboration capabilities that enable
//! seamless interaction between human users and AI agents. It provides
//! feedback mechanisms, intervention points, approval workflows, and
//! collaborative decision-making processes.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::context::ExecutionContext;
use crate::goal::Goal;

/// Human collaboration coordinator
pub struct HumanCollaborationCoordinator {
    /// Active collaboration sessions
    sessions: Arc<RwLock<HashMap<Uuid, CollaborationSession>>>,
    /// Human users and their sessions
    users: Arc<RwLock<HashMap<String, UserProfile>>>,
    /// Communication channels
    channels: Arc<RwLock<CommunicationChannels>>,
    /// Feedback collection system
    feedback_system: Arc<RwLock<FeedbackSystem>>,
    /// Intervention management
    intervention_manager: Arc<RwLock<InterventionManager>>,
    /// Approval workflow system
    approval_system: Arc<RwLock<ApprovalSystem>>,
    /// Real-time event broadcaster
    event_broadcaster: broadcast::Sender<CollaborationEvent>,
}

/// Collaboration session between humans and agents
#[derive(Debug, Clone)]
pub struct CollaborationSession {
    /// Session unique identifier
    pub id: Uuid,
    /// Human participants
    pub human_participants: Vec<String>,
    /// Agent participants
    pub agent_participants: Vec<Uuid>,
    /// Current goal being worked on
    pub current_goal: Option<Goal>,
    /// Session status
    pub status: SessionStatus,
    /// Communication history
    pub message_history: Vec<CollaborationMessage>,
    /// Active interventions
    pub active_interventions: Vec<Intervention>,
    /// Pending approvals
    pub pending_approvals: Vec<ApprovalRequest>,
    /// Session start time
    pub started_at: SystemTime,
    /// Last activity
    pub last_activity: SystemTime,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    /// Session is being initialized
    Initializing,
    /// Session is active and running
    Active,
    /// Session is paused waiting for human input
    WaitingForHuman,
    /// Session has interventions pending
    InterventionsPending,
    /// Session has approvals pending
    ApprovalsPending,
    /// Session is completed
    Completed,
    /// Session was terminated
    Terminated,
}

/// Message in a collaboration session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationMessage {
    pub id: Uuid,
    pub sender: MessageSender,
    pub content: String,
    pub message_type: MessageType,
    pub timestamp: SystemTime,
    pub metadata: HashMap<String, String>,
}

/// Message sender types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSender {
    Human(String), // Human username
    Agent(Uuid),   // Agent ID
    System,        // System messages
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text,
    Command,
    Feedback,
    Approval,
    Intervention,
    StatusUpdate,
    Error,
}

/// User profile for human participants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub username: String,
    pub preferences: CollaborationPreferences,
    pub expertise_areas: Vec<String>,
    pub trust_level: f64,
    pub interaction_history: Vec<InteractionRecord>,
    pub last_active: SystemTime,
}

/// User collaboration preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPreferences {
    pub notification_level: NotificationLevel,
    pub intervention_points: Vec<InterventionTrigger>,
    pub approval_thresholds: HashMap<String, ApprovalThreshold>,
    pub communication_style: CommunicationStyle,
    pub auto_approval_rules: Vec<AutoApprovalRule>,
}

/// Notification levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationLevel {
    All,       // Notify on all events
    Important, // Only important events
    Critical,  // Only critical events
    None,      // No notifications
}

/// Intervention trigger points
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionTrigger {
    HighRiskActions,
    UncertainDecisions,
    MajorChanges,
    SecurityConcerns,
    PerformanceIssues,
    UserRequested,
}

/// Approval thresholds for different action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalThreshold {
    pub action_type: String,
    pub risk_level: RiskLevel,
    pub requires_approval: bool,
    pub auto_approve_below_confidence: f64,
}

/// Risk levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Communication style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationStyle {
    Technical, // Detailed technical explanations
    Concise,   // Brief, to-the-point
    Friendly,  // Friendly and approachable
    Formal,    // Formal business communication
}

/// Auto-approval rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApprovalRule {
    pub condition: String,
    pub action_pattern: String,
    pub max_frequency: u32,
    pub time_window: Duration,
}

/// Interaction record for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub session_id: Uuid,
    pub timestamp: SystemTime,
    pub interaction_type: InteractionType,
    pub outcome: InteractionOutcome,
    pub feedback_score: Option<f64>,
}

/// Types of interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    FeedbackProvided,
    InterventionRequested,
    ApprovalGiven,
    ApprovalDenied,
    GuidanceOffered,
    QuestionAsked,
}

/// Interaction outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionOutcome {
    Positive,
    Neutral,
    Negative,
    Resolved,
    Escalated,
}

/// Intervention outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionOutcome {
    Successful,
    PartiallySuccessful,
    Failed,
    Resolved,
    Escalated,
}

/// Communication channels for real-time interaction
pub struct CommunicationChannels {
    /// User message queues
    user_queues: HashMap<String, mpsc::UnboundedSender<CollaborationMessage>>,
    /// Agent message queues
    agent_queues: HashMap<Uuid, mpsc::UnboundedSender<CollaborationMessage>>,
    /// Session-specific channels
    session_channels: HashMap<Uuid, broadcast::Sender<CollaborationMessage>>,
}

/// Feedback collection and analysis system
pub struct FeedbackSystem {
    /// Collected feedback
    feedback_history: Vec<FeedbackEntry>,
    /// Feedback analysis results
    analysis_results: HashMap<String, FeedbackAnalysis>,
    /// Feedback patterns
    patterns: Vec<FeedbackPattern>,
}

/// Individual feedback entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub id: Uuid,
    pub user: String,
    pub session_id: Uuid,
    pub feedback_type: FeedbackType,
    pub content: String,
    pub rating: Option<f64>,
    pub timestamp: SystemTime,
    pub context: HashMap<String, String>,
}

/// Types of feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    General,
    ActionApproval,
    ActionRejection,
    Performance,
    Usability,
    Accuracy,
    Helpfulness,
}

/// Feedback analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackAnalysis {
    pub topic: String,
    pub average_rating: f64,
    pub common_themes: Vec<String>,
    pub improvement_suggestions: Vec<String>,
    pub trend_direction: TrendDirection,
}

/// Feedback trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Declining,
}

/// Identified feedback patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackPattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: u32,
    pub associated_actions: Vec<String>,
    pub recommended_response: String,
}

/// Intervention management system
pub struct InterventionManager {
    /// Active interventions
    active_interventions: HashMap<Uuid, Intervention>,
    /// Intervention templates
    templates: HashMap<String, InterventionTemplate>,
    /// Intervention history
    history: Vec<InterventionRecord>,
}

/// Intervention request or action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intervention {
    pub id: Uuid,
    pub session_id: Uuid,
    pub intervention_type: InterventionType,
    pub description: String,
    pub requested_by: InterventionRequester,
    pub priority: InterventionPriority,
    pub status: InterventionStatus,
    pub created_at: SystemTime,
    pub resolved_at: Option<SystemTime>,
}

/// Types of interventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionType {
    PauseExecution,
    RequestGuidance,
    OverrideDecision,
    ProvideContext,
    EscalateIssue,
    ModifyGoal,
}

/// Who requested the intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionRequester {
    Human(String),
    Agent(Uuid),
    System,
}

/// Intervention priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Intervention status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionStatus {
    Pending,
    InProgress,
    Resolved,
    Cancelled,
}

/// Intervention template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionTemplate {
    pub name: String,
    pub intervention_type: InterventionType,
    pub description: String,
    pub required_context: Vec<String>,
    pub suggested_actions: Vec<String>,
}

/// Intervention record for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionRecord {
    pub intervention: Intervention,
    pub outcome: InterventionOutcome,
    pub duration: Duration,
    pub effectiveness_score: f64,
}

/// Approval workflow system
pub struct ApprovalSystem {
    /// Pending approval requests
    pending_requests: HashMap<Uuid, ApprovalRequest>,
    /// Approval workflows
    workflows: HashMap<String, ApprovalWorkflow>,
    /// Approval history
    history: Vec<ApprovalRecord>,
}

/// Approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: Uuid,
    pub session_id: Uuid,
    pub request_type: ApprovalType,
    pub description: String,
    pub requested_by: ApprovalRequester,
    pub risk_assessment: RiskAssessment,
    pub alternatives: Vec<String>,
    pub deadline: Option<SystemTime>,
    pub status: ApprovalStatus,
    pub created_at: SystemTime,
}

/// Types of approvals needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalType {
    ActionExecution,
    CodeDeployment,
    ConfigurationChange,
    SecurityPolicyUpdate,
    ResourceAllocation,
    GoalModification,
}

/// Who requested approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalRequester {
    Agent(Uuid),
    System,
}

/// Risk assessment for approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub impact_description: String,
    pub mitigation_strategies: Vec<String>,
    pub confidence_score: f64,
}

/// Approval status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Escalated,
    Expired,
}

/// Approval workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalWorkflow {
    pub name: String,
    pub approval_type: ApprovalType,
    pub required_approvers: u32,
    pub escalation_rules: Vec<EscalationRule>,
    pub auto_approval_conditions: Vec<String>,
}

/// Escalation rule for approvals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    pub condition: String,
    pub escalate_to: Vec<String>, // Usernames or roles
    pub time_threshold: Duration,
}

/// Approval record for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub request: ApprovalRequest,
    pub approved: bool,
    pub approved_by: Option<String>,
    pub reasoning: String,
    pub timestamp: SystemTime,
}

/// Real-time collaboration events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationEvent {
    SessionStarted {
        session_id: Uuid,
    },
    SessionEnded {
        session_id: Uuid,
    },
    MessageReceived {
        session_id: Uuid,
        message: CollaborationMessage,
    },
    InterventionRequested {
        session_id: Uuid,
        intervention: Intervention,
    },
    ApprovalRequested {
        session_id: Uuid,
        approval: ApprovalRequest,
    },
    FeedbackReceived {
        session_id: Uuid,
        feedback: FeedbackEntry,
    },
    StatusChanged {
        session_id: Uuid,
        status: SessionStatus,
    },
}

impl Default for HumanCollaborationCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanCollaborationCoordinator {
    /// Create a new human collaboration coordinator
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(CommunicationChannels::new())),
            feedback_system: Arc::new(RwLock::new(FeedbackSystem::new())),
            intervention_manager: Arc::new(RwLock::new(InterventionManager::new())),
            approval_system: Arc::new(RwLock::new(ApprovalSystem::new())),
            event_broadcaster: event_tx,
        }
    }

    /// Start a new collaboration session
    pub async fn start_session(
        &self,
        human_participants: Vec<String>,
        agent_participants: Vec<Uuid>,
        initial_goal: Option<Goal>,
    ) -> Result<Uuid> {
        let session_id = Uuid::new_v4();

        let session = CollaborationSession {
            id: session_id,
            human_participants: human_participants.clone(),
            agent_participants: agent_participants.clone(),
            current_goal: initial_goal,
            status: SessionStatus::Initializing,
            message_history: Vec::new(),
            active_interventions: Vec::new(),
            pending_approvals: Vec::new(),
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
        };

        // Add session
        self.sessions.write().await.insert(session_id, session);

        // Initialize communication channels
        self.initialize_session_channels(session_id, &human_participants, &agent_participants)
            .await;

        // Broadcast session start event
        let _ = self
            .event_broadcaster
            .send(CollaborationEvent::SessionStarted { session_id });

        Ok(session_id)
    }

    /// Send a message in a collaboration session
    pub async fn send_message(
        &self,
        session_id: Uuid,
        sender: MessageSender,
        content: String,
        message_type: MessageType,
    ) -> Result<()> {
        let message = CollaborationMessage {
            id: Uuid::new_v4(),
            sender: sender.clone(),
            content,
            message_type,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        };

        // Add to session history
        if let Some(session) = self.sessions.write().await.get_mut(&session_id) {
            session.message_history.push(message.clone());
            session.last_activity = SystemTime::now();
        }

        // Broadcast to session participants
        let channels = self.channels.read().await;
        if let Some(session_tx) = channels.session_channels.get(&session_id) {
            let _ = session_tx.send(message.clone());
        }

        // Broadcast event
        let _ = self
            .event_broadcaster
            .send(CollaborationEvent::MessageReceived {
                session_id,
                message,
            });

        Ok(())
    }

    /// Request human intervention
    pub async fn request_intervention(
        &self,
        session_id: Uuid,
        intervention_type: InterventionType,
        description: String,
        requester: InterventionRequester,
        priority: InterventionPriority,
    ) -> Result<Uuid> {
        let intervention = Intervention {
            id: Uuid::new_v4(),
            session_id,
            intervention_type,
            description,
            requested_by: requester,
            priority,
            status: InterventionStatus::Pending,
            created_at: SystemTime::now(),
            resolved_at: None,
        };

        // Add to intervention manager
        self.intervention_manager
            .write()
            .await
            .add_intervention(intervention.clone());

        // Add to session
        if let Some(session) = self.sessions.write().await.get_mut(&session_id) {
            session.active_interventions.push(intervention.clone());
            session.status = SessionStatus::InterventionsPending;
        }

        // Broadcast event
        let intervention_id = intervention.id;
        let _ = self
            .event_broadcaster
            .send(CollaborationEvent::InterventionRequested {
                session_id,
                intervention,
            });

        Ok(intervention_id)
    }

    /// Request approval for an action
    pub async fn request_approval(
        &self,
        session_id: Uuid,
        approval_type: ApprovalType,
        description: String,
        requester: ApprovalRequester,
        risk_assessment: RiskAssessment,
    ) -> Result<Uuid> {
        let approval = ApprovalRequest {
            id: Uuid::new_v4(),
            session_id,
            request_type: approval_type,
            description,
            requested_by: requester,
            risk_assessment,
            alternatives: Vec::new(),
            deadline: None,
            status: ApprovalStatus::Pending,
            created_at: SystemTime::now(),
        };

        // Add to approval system
        self.approval_system
            .write()
            .await
            .add_request(approval.clone());

        // Add to session
        if let Some(session) = self.sessions.write().await.get_mut(&session_id) {
            session.pending_approvals.push(approval.clone());
            session.status = SessionStatus::ApprovalsPending;
        }

        // Broadcast event
        let approval_id = approval.id;
        let _ = self
            .event_broadcaster
            .send(CollaborationEvent::ApprovalRequested {
                session_id,
                approval,
            });

        Ok(approval_id)
    }

    /// Submit feedback
    pub async fn submit_feedback(
        &self,
        session_id: Uuid,
        user: String,
        feedback_type: FeedbackType,
        content: String,
        rating: Option<f64>,
        context: HashMap<String, String>,
    ) -> Result<()> {
        let feedback = FeedbackEntry {
            id: Uuid::new_v4(),
            user: user.clone(),
            session_id,
            feedback_type,
            content,
            rating,
            timestamp: SystemTime::now(),
            context,
        };

        // Add to feedback system
        self.feedback_system
            .write()
            .await
            .add_feedback(feedback.clone());

        // Update user interaction history
        if let Some(user_profile) = self.users.write().await.get_mut(&user) {
            user_profile.interaction_history.push(InteractionRecord {
                session_id,
                timestamp: SystemTime::now(),
                interaction_type: InteractionType::FeedbackProvided,
                outcome: InteractionOutcome::Neutral,
                feedback_score: rating,
            });
        }

        // Broadcast event
        let _ = self
            .event_broadcaster
            .send(CollaborationEvent::FeedbackReceived {
                session_id,
                feedback,
            });

        Ok(())
    }

    /// Get real-time event stream
    pub fn get_event_stream(&self) -> broadcast::Receiver<CollaborationEvent> {
        self.event_broadcaster.subscribe()
    }

    /// Initialize communication channels for a session
    async fn initialize_session_channels(
        &self,
        session_id: Uuid,
        human_participants: &[String],
        agent_participants: &[Uuid],
    ) {
        let mut channels = self.channels.write().await;
        let (session_tx, _) = broadcast::channel(100);
        channels.session_channels.insert(session_id, session_tx);
    }
}

impl CommunicationChannels {
    fn new() -> Self {
        Self {
            user_queues: HashMap::new(),
            agent_queues: HashMap::new(),
            session_channels: HashMap::new(),
        }
    }
}

impl FeedbackSystem {
    fn new() -> Self {
        Self {
            feedback_history: Vec::new(),
            analysis_results: HashMap::new(),
            patterns: Vec::new(),
        }
    }

    fn add_feedback(&mut self, feedback: FeedbackEntry) {
        self.feedback_history.push(feedback);
        // In a real implementation, this would trigger analysis
    }
}

impl InterventionManager {
    fn new() -> Self {
        Self {
            active_interventions: HashMap::new(),
            templates: HashMap::new(),
            history: Vec::new(),
        }
    }

    fn add_intervention(&mut self, intervention: Intervention) {
        self.active_interventions
            .insert(intervention.id, intervention);
    }
}

impl ApprovalSystem {
    fn new() -> Self {
        Self {
            pending_requests: HashMap::new(),
            workflows: HashMap::new(),
            history: Vec::new(),
        }
    }

    fn add_request(&mut self, request: ApprovalRequest) {
        self.pending_requests.insert(request.id, request);
    }
}

impl Default for CollaborationPreferences {
    fn default() -> Self {
        Self {
            notification_level: NotificationLevel::Important,
            intervention_points: vec![
                InterventionTrigger::HighRiskActions,
                InterventionTrigger::SecurityConcerns,
            ],
            approval_thresholds: HashMap::new(),
            communication_style: CommunicationStyle::Concise,
            auto_approval_rules: Vec::new(),
        }
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            username: String::new(),
            preferences: CollaborationPreferences::default(),
            expertise_areas: Vec::new(),
            trust_level: 0.5,
            interaction_history: Vec::new(),
            last_active: SystemTime::now(),
        }
    }
}

/// Human-AI collaboration interface
#[async_trait]
pub trait HumanCollaborationInterface {
    /// Connect a human user to the collaboration system
    async fn connect_user(
        &self,
        username: String,
        preferences: CollaborationPreferences,
    ) -> Result<()>;

    /// Get pending interventions for a user
    async fn get_pending_interventions(&self, username: &str) -> Result<Vec<Intervention>>;

    /// Get pending approvals for a user
    async fn get_pending_approvals(&self, username: &str) -> Result<Vec<ApprovalRequest>>;

    /// Respond to an intervention
    async fn respond_to_intervention(
        &self,
        intervention_id: Uuid,
        response: InterventionResponse,
        username: &str,
    ) -> Result<()>;

    /// Respond to an approval request
    async fn respond_to_approval(
        &self,
        approval_id: Uuid,
        approved: bool,
        reasoning: String,
        username: &str,
    ) -> Result<()>;
}

/// Response to an intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterventionResponse {
    Acknowledge,
    ProvideGuidance(String),
    Override(String),
    Escalate(String),
    Cancel,
}

#[async_trait]
impl HumanCollaborationInterface for HumanCollaborationCoordinator {
    async fn connect_user(
        &self,
        username: String,
        preferences: CollaborationPreferences,
    ) -> Result<()> {
        let user_profile = UserProfile {
            username: username.clone(),
            preferences,
            expertise_areas: Vec::new(),
            trust_level: 0.5,
            interaction_history: Vec::new(),
            last_active: SystemTime::now(),
        };

        self.users.write().await.insert(username, user_profile);
        Ok(())
    }

    async fn get_pending_interventions(&self, username: &str) -> Result<Vec<Intervention>> {
        let interventions = self.intervention_manager.read().await;
        let sessions = self.sessions.read().await;

        let mut user_interventions = Vec::new();

        for session in sessions.values() {
            if session.human_participants.contains(&username.to_string()) {
                for intervention in &session.active_interventions {
                    if matches!(intervention.status, InterventionStatus::Pending) {
                        user_interventions.push(intervention.clone());
                    }
                }
            }
        }

        Ok(user_interventions)
    }

    async fn get_pending_approvals(&self, username: &str) -> Result<Vec<ApprovalRequest>> {
        let approvals = self.approval_system.read().await;
        let sessions = self.sessions.read().await;

        let mut user_approvals = Vec::new();

        for session in sessions.values() {
            if session.human_participants.contains(&username.to_string()) {
                for approval in &session.pending_approvals {
                    if matches!(approval.status, ApprovalStatus::Pending) {
                        user_approvals.push(approval.clone());
                    }
                }
            }
        }

        Ok(user_approvals)
    }

    async fn respond_to_intervention(
        &self,
        intervention_id: Uuid,
        response: InterventionResponse,
        username: &str,
    ) -> Result<()> {
        let session_id;
        let intervention_clone;
        {
            let mut intervention_manager = self.intervention_manager.write().await;

            if let Some(intervention) = intervention_manager
                .active_interventions
                .get_mut(&intervention_id)
            {
                intervention.status = InterventionStatus::Resolved;
                intervention.resolved_at = Some(SystemTime::now());

                session_id = intervention.session_id;
                intervention_clone = Some(intervention.clone());
            } else {
                return Err(anyhow!("Intervention not found"));
            }

            // Record the response - intervention_clone is guaranteed to be Some here
            // because we would have returned an error above if not found
            let resolved_intervention = intervention_clone
                .as_ref()
                .expect("intervention_clone should be Some after successful lookup");

            let record = InterventionRecord {
                intervention: resolved_intervention.clone(),
                outcome: InterventionOutcome::Resolved,
                duration: resolved_intervention
                    .created_at
                    .elapsed()
                    .unwrap_or(Duration::from_secs(0)),
                effectiveness_score: 0.8, // Placeholder
            };

            intervention_manager.history.push(record);
        }

        // Update session status
        if let Some(session) = self.sessions.write().await.get_mut(&session_id) {
            session
                .active_interventions
                .retain(|i| i.id != intervention_id);
            if session.active_interventions.is_empty() && session.pending_approvals.is_empty() {
                session.status = SessionStatus::Active;
            }
        }

        Ok(())
    }

    async fn respond_to_approval(
        &self,
        approval_id: Uuid,
        approved: bool,
        reasoning: String,
        username: &str,
    ) -> Result<()> {
        let mut approval_system = self.approval_system.write().await;

        let approval_record =
            if let Some(approval) = approval_system.pending_requests.get_mut(&approval_id) {
                approval.status = if approved {
                    ApprovalStatus::Approved
                } else {
                    ApprovalStatus::Denied
                };

                // Record the approval
                Some((
                    ApprovalRecord {
                        request: approval.clone(),
                        approved,
                        approved_by: Some(username.to_string()),
                        reasoning: reasoning.clone(),
                        timestamp: SystemTime::now(),
                    },
                    approval.session_id,
                ))
            } else {
                None
            };

        if let Some((record, session_id)) = approval_record {
            approval_system.history.push(record);

            // Update session status
            if let Some(session) = self.sessions.write().await.get_mut(&session_id) {
                session.pending_approvals.retain(|a| a.id != approval_id);
                if session.active_interventions.is_empty() && session.pending_approvals.is_empty() {
                    session.status = SessionStatus::Active;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Session Status Tests ==========

    #[test]
    fn test_session_status_variants() {
        let statuses = vec![
            SessionStatus::Initializing,
            SessionStatus::Active,
            SessionStatus::WaitingForHuman,
            SessionStatus::InterventionsPending,
            SessionStatus::ApprovalsPending,
            SessionStatus::Completed,
            SessionStatus::Terminated,
        ];
        assert_eq!(statuses.len(), 7);
    }

    #[test]
    fn test_session_status_equality() {
        assert_eq!(SessionStatus::Active, SessionStatus::Active);
        assert_ne!(SessionStatus::Active, SessionStatus::Completed);
    }

    // ========== Message Types Tests ==========

    #[test]
    fn test_message_sender_variants() {
        let human = MessageSender::Human("user1".to_string());
        let agent = MessageSender::Agent(Uuid::new_v4());
        let system = MessageSender::System;

        assert!(matches!(human, MessageSender::Human(_)));
        assert!(matches!(agent, MessageSender::Agent(_)));
        assert!(matches!(system, MessageSender::System));
    }

    #[test]
    fn test_message_type_variants() {
        let types = vec![
            MessageType::Text,
            MessageType::Command,
            MessageType::Feedback,
            MessageType::Approval,
            MessageType::Intervention,
            MessageType::StatusUpdate,
            MessageType::Error,
        ];
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn test_collaboration_message_creation() {
        let msg = CollaborationMessage {
            id: Uuid::new_v4(),
            sender: MessageSender::Human("alice".to_string()),
            content: "Hello!".to_string(),
            message_type: MessageType::Text,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        };

        assert_eq!(msg.content, "Hello!");
        assert!(matches!(msg.message_type, MessageType::Text));
    }

    // ========== User Profile Tests ==========

    #[test]
    fn test_user_profile_default() {
        let profile = UserProfile::default();

        assert_eq!(profile.username, "");
        assert!((profile.trust_level - 0.5).abs() < f64::EPSILON);
        assert!(profile.expertise_areas.is_empty());
        assert!(profile.interaction_history.is_empty());
    }

    #[test]
    fn test_collaboration_preferences_default() {
        let prefs = CollaborationPreferences::default();

        assert!(matches!(
            prefs.notification_level,
            NotificationLevel::Important
        ));
        assert!(matches!(
            prefs.communication_style,
            CommunicationStyle::Concise
        ));
        assert_eq!(prefs.intervention_points.len(), 2);
        assert!(prefs.auto_approval_rules.is_empty());
    }

    #[test]
    fn test_notification_level_variants() {
        let levels = vec![
            NotificationLevel::All,
            NotificationLevel::Important,
            NotificationLevel::Critical,
            NotificationLevel::None,
        ];
        assert_eq!(levels.len(), 4);
    }

    #[test]
    fn test_communication_style_variants() {
        let styles = vec![
            CommunicationStyle::Technical,
            CommunicationStyle::Concise,
            CommunicationStyle::Friendly,
            CommunicationStyle::Formal,
        ];
        assert_eq!(styles.len(), 4);
    }

    // ========== Intervention Tests ==========

    #[test]
    fn test_intervention_type_variants() {
        let types = vec![
            InterventionType::PauseExecution,
            InterventionType::RequestGuidance,
            InterventionType::OverrideDecision,
            InterventionType::ProvideContext,
            InterventionType::EscalateIssue,
            InterventionType::ModifyGoal,
        ];
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_intervention_priority_variants() {
        let priorities = vec![
            InterventionPriority::Low,
            InterventionPriority::Medium,
            InterventionPriority::High,
            InterventionPriority::Critical,
        ];
        assert_eq!(priorities.len(), 4);
    }

    #[test]
    fn test_intervention_status_variants() {
        let statuses = vec![
            InterventionStatus::Pending,
            InterventionStatus::InProgress,
            InterventionStatus::Resolved,
            InterventionStatus::Cancelled,
        ];
        assert_eq!(statuses.len(), 4);
    }

    #[test]
    fn test_intervention_creation() {
        let intervention = Intervention {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            intervention_type: InterventionType::RequestGuidance,
            description: "Need help with decision".to_string(),
            requested_by: InterventionRequester::Agent(Uuid::new_v4()),
            priority: InterventionPriority::High,
            status: InterventionStatus::Pending,
            created_at: SystemTime::now(),
            resolved_at: None,
        };

        assert!(matches!(
            intervention.intervention_type,
            InterventionType::RequestGuidance
        ));
        assert!(matches!(intervention.priority, InterventionPriority::High));
        assert!(intervention.resolved_at.is_none());
    }

    #[test]
    fn test_intervention_response_variants() {
        let responses = vec![
            InterventionResponse::Acknowledge,
            InterventionResponse::ProvideGuidance("Do this".to_string()),
            InterventionResponse::Override("Different approach".to_string()),
            InterventionResponse::Escalate("Need manager".to_string()),
            InterventionResponse::Cancel,
        ];
        assert_eq!(responses.len(), 5);
    }

    // ========== Approval Tests ==========

    #[test]
    fn test_approval_type_variants() {
        let types = vec![
            ApprovalType::ActionExecution,
            ApprovalType::CodeDeployment,
            ApprovalType::ConfigurationChange,
            ApprovalType::SecurityPolicyUpdate,
            ApprovalType::ResourceAllocation,
            ApprovalType::GoalModification,
        ];
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_approval_status_variants() {
        let statuses = vec![
            ApprovalStatus::Pending,
            ApprovalStatus::Approved,
            ApprovalStatus::Denied,
            ApprovalStatus::Escalated,
            ApprovalStatus::Expired,
        ];
        assert_eq!(statuses.len(), 5);
    }

    #[test]
    fn test_risk_assessment_creation() {
        let assessment = RiskAssessment {
            risk_level: RiskLevel::Medium,
            impact_description: "Moderate impact on system".to_string(),
            mitigation_strategies: vec!["Backup data".to_string(), "Test in staging".to_string()],
            confidence_score: 0.75,
        };

        assert!(matches!(assessment.risk_level, RiskLevel::Medium));
        assert_eq!(assessment.mitigation_strategies.len(), 2);
    }

    #[test]
    fn test_approval_request_creation() {
        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            request_type: ApprovalType::CodeDeployment,
            description: "Deploy to production".to_string(),
            requested_by: ApprovalRequester::Agent(Uuid::new_v4()),
            risk_assessment: RiskAssessment {
                risk_level: RiskLevel::High,
                impact_description: "Production deployment".to_string(),
                mitigation_strategies: vec!["Rollback plan".to_string()],
                confidence_score: 0.85,
            },
            alternatives: vec!["Deploy to staging first".to_string()],
            deadline: None,
            status: ApprovalStatus::Pending,
            created_at: SystemTime::now(),
        };

        assert!(matches!(request.request_type, ApprovalType::CodeDeployment));
        assert!(matches!(request.status, ApprovalStatus::Pending));
    }

    // ========== Feedback Tests ==========

    #[test]
    fn test_feedback_type_variants() {
        let types = vec![
            FeedbackType::General,
            FeedbackType::ActionApproval,
            FeedbackType::ActionRejection,
            FeedbackType::Performance,
            FeedbackType::Usability,
            FeedbackType::Accuracy,
            FeedbackType::Helpfulness,
        ];
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn test_trend_direction_variants() {
        let directions = vec![
            TrendDirection::Improving,
            TrendDirection::Stable,
            TrendDirection::Declining,
        ];
        assert_eq!(directions.len(), 3);
    }

    #[test]
    fn test_feedback_entry_creation() {
        let feedback = FeedbackEntry {
            id: Uuid::new_v4(),
            user: "bob".to_string(),
            session_id: Uuid::new_v4(),
            feedback_type: FeedbackType::Performance,
            content: "System is responsive".to_string(),
            rating: Some(4.5),
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };

        assert_eq!(feedback.user, "bob");
        assert_eq!(feedback.rating, Some(4.5));
    }

    #[test]
    fn test_feedback_analysis_creation() {
        let analysis = FeedbackAnalysis {
            topic: "performance".to_string(),
            average_rating: 4.2,
            common_themes: vec!["fast".to_string(), "responsive".to_string()],
            improvement_suggestions: vec!["Add caching".to_string()],
            trend_direction: TrendDirection::Improving,
        };

        assert!((analysis.average_rating - 4.2).abs() < f64::EPSILON);
        assert!(matches!(
            analysis.trend_direction,
            TrendDirection::Improving
        ));
    }

    // ========== Interaction Tests ==========

    #[test]
    fn test_interaction_type_variants() {
        let types = vec![
            InteractionType::FeedbackProvided,
            InteractionType::InterventionRequested,
            InteractionType::ApprovalGiven,
            InteractionType::ApprovalDenied,
            InteractionType::GuidanceOffered,
            InteractionType::QuestionAsked,
        ];
        assert_eq!(types.len(), 6);
    }

    #[test]
    fn test_interaction_outcome_variants() {
        let outcomes = vec![
            InteractionOutcome::Positive,
            InteractionOutcome::Neutral,
            InteractionOutcome::Negative,
            InteractionOutcome::Resolved,
            InteractionOutcome::Escalated,
        ];
        assert_eq!(outcomes.len(), 5);
    }

    #[test]
    fn test_intervention_outcome_variants() {
        let outcomes = vec![
            InterventionOutcome::Successful,
            InterventionOutcome::PartiallySuccessful,
            InterventionOutcome::Failed,
            InterventionOutcome::Resolved,
            InterventionOutcome::Escalated,
        ];
        assert_eq!(outcomes.len(), 5);
    }

    // ========== Collaboration Event Tests ==========

    #[test]
    fn test_collaboration_event_session_started() {
        let event = CollaborationEvent::SessionStarted {
            session_id: Uuid::new_v4(),
        };
        assert!(matches!(event, CollaborationEvent::SessionStarted { .. }));
    }

    #[test]
    fn test_collaboration_event_message_received() {
        let msg = CollaborationMessage {
            id: Uuid::new_v4(),
            sender: MessageSender::System,
            content: "Test".to_string(),
            message_type: MessageType::StatusUpdate,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        };
        let event = CollaborationEvent::MessageReceived {
            session_id: Uuid::new_v4(),
            message: msg,
        };
        assert!(matches!(event, CollaborationEvent::MessageReceived { .. }));
    }

    // ========== System Tests ==========

    #[test]
    fn test_communication_channels_new() {
        let channels = CommunicationChannels::new();
        assert!(channels.user_queues.is_empty());
        assert!(channels.agent_queues.is_empty());
        assert!(channels.session_channels.is_empty());
    }

    #[test]
    fn test_feedback_system_new() {
        let system = FeedbackSystem::new();
        assert!(system.feedback_history.is_empty());
        assert!(system.analysis_results.is_empty());
        assert!(system.patterns.is_empty());
    }

    #[test]
    fn test_intervention_manager_new() {
        let manager = InterventionManager::new();
        assert!(manager.active_interventions.is_empty());
        assert!(manager.templates.is_empty());
        assert!(manager.history.is_empty());
    }

    #[test]
    fn test_approval_system_new() {
        let system = ApprovalSystem::new();
        assert!(system.pending_requests.is_empty());
        assert!(system.workflows.is_empty());
        assert!(system.history.is_empty());
    }

    #[test]
    fn test_feedback_system_add_feedback() {
        let mut system = FeedbackSystem::new();
        let feedback = FeedbackEntry {
            id: Uuid::new_v4(),
            user: "test".to_string(),
            session_id: Uuid::new_v4(),
            feedback_type: FeedbackType::General,
            content: "Good work".to_string(),
            rating: Some(5.0),
            timestamp: SystemTime::now(),
            context: HashMap::new(),
        };

        system.add_feedback(feedback);
        assert_eq!(system.feedback_history.len(), 1);
    }

    #[test]
    fn test_intervention_manager_add_intervention() {
        let mut manager = InterventionManager::new();
        let intervention = Intervention {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            intervention_type: InterventionType::PauseExecution,
            description: "Need to pause".to_string(),
            requested_by: InterventionRequester::System,
            priority: InterventionPriority::Medium,
            status: InterventionStatus::Pending,
            created_at: SystemTime::now(),
            resolved_at: None,
        };

        let id = intervention.id;
        manager.add_intervention(intervention);
        assert!(manager.active_interventions.contains_key(&id));
    }

    #[test]
    fn test_approval_system_add_request() {
        let mut system = ApprovalSystem::new();
        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            request_type: ApprovalType::ActionExecution,
            description: "Execute action".to_string(),
            requested_by: ApprovalRequester::System,
            risk_assessment: RiskAssessment {
                risk_level: RiskLevel::Low,
                impact_description: "Low impact".to_string(),
                mitigation_strategies: Vec::new(),
                confidence_score: 0.9,
            },
            alternatives: Vec::new(),
            deadline: None,
            status: ApprovalStatus::Pending,
            created_at: SystemTime::now(),
        };

        let id = request.id;
        system.add_request(request);
        assert!(system.pending_requests.contains_key(&id));
    }

    // ========== Async Coordinator Tests ==========

    #[tokio::test]
    async fn test_coordinator_new() {
        let coordinator = HumanCollaborationCoordinator::new();
        // Verify coordinator creates without panic
        let _ = coordinator.get_event_stream();
    }

    #[tokio::test]
    async fn test_coordinator_start_session() {
        let coordinator = HumanCollaborationCoordinator::new();

        let session_id = coordinator
            .start_session(vec!["alice".to_string()], vec![Uuid::new_v4()], None)
            .await
            .unwrap();

        // Verify session was created
        let sessions = coordinator.sessions.read().await;
        assert!(sessions.contains_key(&session_id));
    }

    #[tokio::test]
    async fn test_coordinator_send_message() {
        let coordinator = HumanCollaborationCoordinator::new();

        let session_id = coordinator
            .start_session(vec!["alice".to_string()], vec![], None)
            .await
            .unwrap();

        let result = coordinator
            .send_message(
                session_id,
                MessageSender::Human("alice".to_string()),
                "Hello world".to_string(),
                MessageType::Text,
            )
            .await;

        assert!(result.is_ok());

        // Verify message was added to session
        let sessions = coordinator.sessions.read().await;
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.message_history.len(), 1);
    }

    #[tokio::test]
    async fn test_coordinator_request_intervention() {
        let coordinator = HumanCollaborationCoordinator::new();

        let session_id = coordinator
            .start_session(vec!["bob".to_string()], vec![], None)
            .await
            .unwrap();

        let intervention_id = coordinator
            .request_intervention(
                session_id,
                InterventionType::RequestGuidance,
                "Need help".to_string(),
                InterventionRequester::System,
                InterventionPriority::High,
            )
            .await
            .unwrap();

        // Verify intervention was added
        let manager = coordinator.intervention_manager.read().await;
        assert!(manager.active_interventions.contains_key(&intervention_id));

        // Verify session status was updated
        let sessions = coordinator.sessions.read().await;
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.status, SessionStatus::InterventionsPending);
    }

    #[tokio::test]
    async fn test_coordinator_request_approval() {
        let coordinator = HumanCollaborationCoordinator::new();

        let session_id = coordinator
            .start_session(vec!["carol".to_string()], vec![], None)
            .await
            .unwrap();

        let approval_id = coordinator
            .request_approval(
                session_id,
                ApprovalType::CodeDeployment,
                "Deploy to prod".to_string(),
                ApprovalRequester::System,
                RiskAssessment {
                    risk_level: RiskLevel::High,
                    impact_description: "Prod deployment".to_string(),
                    mitigation_strategies: vec!["Rollback".to_string()],
                    confidence_score: 0.8,
                },
            )
            .await
            .unwrap();

        // Verify approval was added
        let approval_system = coordinator.approval_system.read().await;
        assert!(approval_system.pending_requests.contains_key(&approval_id));

        // Verify session status was updated
        let sessions = coordinator.sessions.read().await;
        let session = sessions.get(&session_id).unwrap();
        assert_eq!(session.status, SessionStatus::ApprovalsPending);
    }

    #[tokio::test]
    async fn test_coordinator_submit_feedback() {
        let coordinator = HumanCollaborationCoordinator::new();

        let session_id = coordinator
            .start_session(vec!["dave".to_string()], vec![], None)
            .await
            .unwrap();

        let result = coordinator
            .submit_feedback(
                session_id,
                "dave".to_string(),
                FeedbackType::Helpfulness,
                "Very helpful!".to_string(),
                Some(5.0),
                HashMap::new(),
            )
            .await;

        assert!(result.is_ok());

        // Verify feedback was added
        let feedback_system = coordinator.feedback_system.read().await;
        assert_eq!(feedback_system.feedback_history.len(), 1);
    }

    #[tokio::test]
    async fn test_coordinator_connect_user() {
        let coordinator = HumanCollaborationCoordinator::new();

        let result = coordinator
            .connect_user("eve".to_string(), CollaborationPreferences::default())
            .await;

        assert!(result.is_ok());

        // Verify user was added
        let users = coordinator.users.read().await;
        assert!(users.contains_key("eve"));
    }

    #[tokio::test]
    async fn test_coordinator_get_pending_interventions() {
        let coordinator = HumanCollaborationCoordinator::new();

        let session_id = coordinator
            .start_session(vec!["frank".to_string()], vec![], None)
            .await
            .unwrap();

        // Add an intervention
        coordinator
            .request_intervention(
                session_id,
                InterventionType::PauseExecution,
                "Test intervention".to_string(),
                InterventionRequester::System,
                InterventionPriority::Medium,
            )
            .await
            .unwrap();

        // Get pending interventions
        let interventions = coordinator
            .get_pending_interventions("frank")
            .await
            .unwrap();
        assert_eq!(interventions.len(), 1);
    }

    #[tokio::test]
    async fn test_coordinator_get_pending_approvals() {
        let coordinator = HumanCollaborationCoordinator::new();

        let session_id = coordinator
            .start_session(vec!["grace".to_string()], vec![], None)
            .await
            .unwrap();

        // Add an approval
        coordinator
            .request_approval(
                session_id,
                ApprovalType::ActionExecution,
                "Test approval".to_string(),
                ApprovalRequester::System,
                RiskAssessment {
                    risk_level: RiskLevel::Low,
                    impact_description: "Test".to_string(),
                    mitigation_strategies: Vec::new(),
                    confidence_score: 0.9,
                },
            )
            .await
            .unwrap();

        // Get pending approvals
        let approvals = coordinator.get_pending_approvals("grace").await.unwrap();
        assert_eq!(approvals.len(), 1);
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: SessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SessionStatus::Active);
    }

    #[test]
    fn test_collaboration_message_serialization() {
        let msg = CollaborationMessage {
            id: Uuid::new_v4(),
            sender: MessageSender::System,
            content: "Test".to_string(),
            message_type: MessageType::Text,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: CollaborationMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "Test");
    }

    #[test]
    fn test_intervention_serialization() {
        let intervention = Intervention {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            intervention_type: InterventionType::PauseExecution,
            description: "Test".to_string(),
            requested_by: InterventionRequester::System,
            priority: InterventionPriority::High,
            status: InterventionStatus::Pending,
            created_at: SystemTime::now(),
            resolved_at: None,
        };

        let json = serde_json::to_string(&intervention).unwrap();
        let deserialized: Intervention = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.description, "Test");
    }

    #[test]
    fn test_approval_request_serialization() {
        let request = ApprovalRequest {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            request_type: ApprovalType::ActionExecution,
            description: "Test".to_string(),
            requested_by: ApprovalRequester::System,
            risk_assessment: RiskAssessment {
                risk_level: RiskLevel::Low,
                impact_description: "Low".to_string(),
                mitigation_strategies: Vec::new(),
                confidence_score: 0.9,
            },
            alternatives: Vec::new(),
            deadline: None,
            status: ApprovalStatus::Pending,
            created_at: SystemTime::now(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ApprovalRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.description, "Test");
    }
}
