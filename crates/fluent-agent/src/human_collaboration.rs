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

            // Record the response
            let record = InterventionRecord {
                intervention: intervention_clone.as_ref().unwrap().clone(),
                outcome: InterventionOutcome::Resolved,
                duration: intervention_clone
                    .as_ref()
                    .unwrap()
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
