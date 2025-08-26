//! Cross-Session Persistence System
//!
//! This module provides comprehensive state persistence across autonomous
//! task runs, enabling long-term learning and context maintenance.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::context::ExecutionContext;
use crate::goal::Goal;
use crate::task::Task;

/// Cross-session persistence manager
pub struct CrossSessionPersistence {
    config: PersistenceConfig,
    session_manager: Arc<RwLock<SessionManager>>,
    state_store: Arc<RwLock<StateStore>>,
    learning_repository: Arc<RwLock<LearningRepository>>,
}

/// Configuration for persistence system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub storage_path: PathBuf,
    pub enable_automatic_save: bool,
    pub save_interval_secs: u64,
    pub max_session_history: u32,
    pub enable_compression: bool,
    pub enable_learning_persistence: bool,
    pub backup_retention_days: u32,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./fluent_persistence"),
            enable_automatic_save: true,
            save_interval_secs: 300, // 5 minutes
            max_session_history: 100,
            enable_compression: true,
            enable_learning_persistence: true,
            backup_retention_days: 30,
        }
    }
}

/// Manager for session state and history
#[derive(Debug, Default)]
pub struct SessionManager {
    current_session: Option<SessionState>,
    session_history: Vec<SessionRecord>,
    active_sessions: HashMap<String, SessionState>,
}

/// State of a single execution session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub started_at: SystemTime,
    pub last_updated: SystemTime,
    pub session_type: SessionType,
    pub primary_goal: Option<Goal>,
    pub execution_context: SerializableContext,
    pub completed_tasks: Vec<Task>,
    pub failed_tasks: Vec<Task>,
    pub session_metrics: SessionMetrics,
    pub learned_insights: Vec<SessionInsight>,
    pub checkpoint_data: Vec<CheckpointData>,
}

/// Type of execution session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Interactive,
    Autonomous,
    Batch,
    Experimental,
    Recovery,
}

/// Serializable version of execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableContext {
    pub context_data: HashMap<String, String>,
    pub iteration_count: u32,
    pub goal_description: Option<String>,
    pub context_summary: String,
    pub key_decisions: Vec<String>,
    pub performance_indicators: HashMap<String, f64>,
}

/// Metrics about session execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_tasks: u32,
    pub successful_tasks: u32,
    pub failed_tasks: u32,
    pub total_duration: Duration,
    pub average_task_duration: Duration,
    pub efficiency_score: f64,
    pub adaptation_count: u32,
    pub resource_usage: ResourceUsage,
}

/// Resource usage during session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub peak_memory_mb: u64,
    pub total_api_calls: u32,
    pub files_accessed: u32,
    pub network_requests: u32,
    pub computation_time: Duration,
}

/// Insight learned during session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInsight {
    pub insight_id: String,
    pub insight_type: InsightType,
    pub description: String,
    pub confidence: f64,
    pub applicability: Vec<String>,
    pub learned_at: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    StrategyImprovement,
    ErrorPrevention,
    EfficiencyGain,
    PatternRecognition,
    ResourceOptimization,
    DecisionImprovement,
}

/// Checkpoint data for session recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointData {
    pub checkpoint_id: String,
    pub created_at: SystemTime,
    pub checkpoint_type: CheckpointType,
    pub state_snapshot: Vec<u8>,
    pub recovery_metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckpointType {
    Automatic,
    Manual,
    BeforeCriticalOperation,
    AfterMilestone,
    OnError,
}

/// Historical record of completed session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub completed_at: SystemTime,
    pub final_state: SessionState,
    pub outcome: SessionOutcome,
    pub archived_location: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionOutcome {
    Successful,
    PartiallySuccessful,
    Failed,
    Interrupted,
    Timeout,
}

/// Persistent state storage
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StateStore {
    persistent_state: HashMap<String, PersistentStateItem>,
    global_configuration: GlobalConfig,
    version_history: Vec<StateVersion>,
}

/// Item stored in persistent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentStateItem {
    pub item_id: String,
    pub item_type: StateItemType,
    pub data: Vec<u8>,
    pub metadata: StateMetadata,
    pub access_count: u32,
    pub last_accessed: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateItemType {
    Configuration,
    LearningModel,
    UserPreferences,
    SystemState,
    CachedResults,
    StrategyWeights,
}

/// Metadata for persistent state items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub version: u32,
    pub size_bytes: usize,
    pub checksum: String,
    pub retention_policy: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    Permanent,
    SessionBased,
    TimeBased(Duration),
    AccessBased(u32),
}

/// Global configuration persisted across sessions
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub user_preferences: HashMap<String, String>,
    pub system_settings: HashMap<String, String>,
    pub learned_parameters: HashMap<String, f64>,
    pub strategy_weights: HashMap<String, f64>,
}

/// Version of state for rollback capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVersion {
    pub version_id: String,
    pub created_at: SystemTime,
    pub changes_summary: String,
    pub state_snapshot: HashMap<String, Vec<u8>>,
}

/// Repository for persistent learning
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LearningRepository {
    learned_patterns: Vec<LearnedPattern>,
    strategy_performance: HashMap<String, StrategyPerformance>,
    error_knowledge: Vec<ErrorKnowledge>,
    optimization_history: Vec<OptimizationRecord>,
}

/// Pattern learned from experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub pattern_id: String,
    pub pattern_name: String,
    pub context_conditions: Vec<String>,
    pub pattern_description: String,
    pub success_rate: f64,
    pub confidence_level: f64,
    pub usage_count: u32,
    pub learned_from_sessions: Vec<String>,
}

/// Performance tracking for strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPerformance {
    pub strategy_name: String,
    pub usage_count: u32,
    pub success_rate: f64,
    pub average_efficiency: f64,
    pub context_effectiveness: HashMap<String, f64>,
    pub improvement_trend: Vec<f64>,
}

/// Knowledge about errors and prevention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorKnowledge {
    pub error_id: String,
    pub error_pattern: String,
    pub prevention_strategies: Vec<String>,
    pub recovery_methods: Vec<String>,
    pub occurrence_frequency: u32,
    pub severity_level: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Record of optimization attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecord {
    pub optimization_id: String,
    pub optimization_type: String,
    pub before_metrics: HashMap<String, f64>,
    pub after_metrics: HashMap<String, f64>,
    pub improvement_achieved: f64,
    pub applied_at: SystemTime,
}

impl CrossSessionPersistence {
    /// Create new cross-session persistence manager
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            config,
            session_manager: Arc::new(RwLock::new(SessionManager::default())),
            state_store: Arc::new(RwLock::new(StateStore::default())),
            learning_repository: Arc::new(RwLock::new(LearningRepository::default())),
        }
    }

    /// Initialize persistence system and load existing state
    pub async fn initialize(&self) -> Result<()> {
        // Create storage directory if it doesn't exist
        if !self.config.storage_path.exists() {
            tokio::fs::create_dir_all(&self.config.storage_path).await?;
        }

        // Load existing persistent state
        self.load_persistent_state().await?;

        // Load session history
        self.load_session_history().await?;

        // Load learning repository
        self.load_learning_data().await?;

        Ok(())
    }

    /// Start a new execution session
    pub async fn start_session(
        &self,
        session_type: SessionType,
        goal: Option<Goal>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        
        let session_state = SessionState {
            session_id: session_id.clone(),
            started_at: SystemTime::now(),
            last_updated: SystemTime::now(),
            session_type,
            primary_goal: goal,
            execution_context: SerializableContext {
                context_data: HashMap::new(),
                iteration_count: 0,
                goal_description: None,
                context_summary: String::new(),
                key_decisions: Vec::new(),
                performance_indicators: HashMap::new(),
            },
            completed_tasks: Vec::new(),
            failed_tasks: Vec::new(),
            session_metrics: SessionMetrics {
                total_tasks: 0,
                successful_tasks: 0,
                failed_tasks: 0,
                total_duration: Duration::from_secs(0),
                average_task_duration: Duration::from_secs(0),
                efficiency_score: 0.0,
                adaptation_count: 0,
                resource_usage: ResourceUsage {
                    peak_memory_mb: 0,
                    total_api_calls: 0,
                    files_accessed: 0,
                    network_requests: 0,
                    computation_time: Duration::from_secs(0),
                },
            },
            learned_insights: Vec::new(),
            checkpoint_data: Vec::new(),
        };

        let mut manager = self.session_manager.write().await;
        manager.current_session = Some(session_state.clone());
        manager.active_sessions.insert(session_id.clone(), session_state);

        Ok(session_id)
    }

    /// Save current session state
    pub async fn save_session_state(&self, context: &ExecutionContext) -> Result<()> {
        let mut manager = self.session_manager.write().await;
        
        if let Some(ref mut session) = manager.current_session {
            // Update session with current context
            session.execution_context = self.serialize_context(context).await?;
            session.last_updated = SystemTime::now();
            
            // Save to disk if auto-save is enabled
            if self.config.enable_automatic_save {
                drop(manager);
                self.persist_session_to_disk().await?;
            }
        }

        Ok(())
    }

    /// Create checkpoint for recovery
    pub async fn create_checkpoint(
        &self,
        checkpoint_type: CheckpointType,
        context: &ExecutionContext,
    ) -> Result<String> {
        let checkpoint_id = Uuid::new_v4().to_string();
        
        // Serialize current state
        let state_data = self.serialize_execution_state(context).await?;
        
        let checkpoint = CheckpointData {
            checkpoint_id: checkpoint_id.clone(),
            created_at: SystemTime::now(),
            checkpoint_type,
            state_snapshot: state_data,
            recovery_metadata: self.generate_recovery_metadata(context).await?,
        };

        // Add to current session
        let mut manager = self.session_manager.write().await;
        if let Some(ref mut session) = manager.current_session {
            session.checkpoint_data.push(checkpoint);
            
            // Keep only recent checkpoints
            if session.checkpoint_data.len() > 20 {
                session.checkpoint_data.drain(0..session.checkpoint_data.len() - 20);
            }
        }

        Ok(checkpoint_id)
    }

    /// Restore from checkpoint
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<ExecutionContext> {
        let manager = self.session_manager.read().await;
        
        if let Some(session) = &manager.current_session {
            if let Some(checkpoint) = session.checkpoint_data.iter().find(|c| c.checkpoint_id == checkpoint_id) {
                return self.deserialize_execution_state(&checkpoint.state_snapshot).await;
            }
        }

        Err(anyhow::anyhow!("Checkpoint not found: {}", checkpoint_id))
    }

    /// End current session and archive
    pub async fn end_session(&self, outcome: SessionOutcome) -> Result<()> {
        let mut manager = self.session_manager.write().await;
        
        if let Some(session) = manager.current_session.take() {
            // Create session record
            let record = SessionRecord {
                session_id: session.session_id.clone(),
                completed_at: SystemTime::now(),
                final_state: session,
                outcome,
                archived_location: None, // Would set actual archive path
            };

            // Add to history
            manager.session_history.push(record);
            
            // Keep only recent sessions
            if manager.session_history.len() > self.config.max_session_history as usize {
                let current_len = manager.session_history.len();
                let target_len = self.config.max_session_history as usize;
                manager.session_history.drain(0..current_len - target_len);
            }

            // Persist final state
            drop(manager);
            self.persist_session_to_disk().await?;
        }

        Ok(())
    }

    /// Store learned insight for future use
    pub async fn store_learned_insight(&self, insight: SessionInsight) -> Result<()> {
        let mut manager = self.session_manager.write().await;
        
        if let Some(ref mut session) = manager.current_session {
            session.learned_insights.push(insight.clone());
        }

        // Also add to learning repository
        let mut learning = self.learning_repository.write().await;
        
        // Convert insight to learned pattern if applicable
        if matches!(insight.insight_type, InsightType::PatternRecognition) {
            let pattern = LearnedPattern {
                pattern_id: insight.insight_id.clone(),
                pattern_name: insight.description.clone(),
                context_conditions: insight.applicability,
                pattern_description: insight.description,
                success_rate: insight.confidence,
                confidence_level: insight.confidence,
                usage_count: 0,
                learned_from_sessions: vec![self.get_current_session_id().await?],
            };
            learning.learned_patterns.push(pattern);
        }

        Ok(())
    }

    /// Retrieve learned patterns for current context
    pub async fn get_relevant_patterns(&self, context: &ExecutionContext) -> Result<Vec<LearnedPattern>> {
        let learning = self.learning_repository.read().await;
        let context_summary = self.serialize_context(context).await?.context_summary;
        
        let mut relevant_patterns = Vec::new();
        
        for pattern in &learning.learned_patterns {
            // Simple relevance check based on context conditions
            let mut relevance_score = 0.0;
            
            for condition in &pattern.context_conditions {
                if context_summary.to_lowercase().contains(&condition.to_lowercase()) {
                    relevance_score += 0.2;
                }
            }
            
            if relevance_score > 0.4 {
                relevant_patterns.push(pattern.clone());
            }
        }
        
        // Sort by confidence and success rate
        relevant_patterns.sort_by(|a, b| {
            (b.confidence_level * b.success_rate)
                .partial_cmp(&(a.confidence_level * a.success_rate))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(relevant_patterns)
    }

    // Helper methods (simplified implementations)
    
    async fn load_persistent_state(&self) -> Result<()> {
        let state_path = self.config.storage_path.join("persistent_state.json");
        if state_path.exists() {
            let content = tokio::fs::read_to_string(state_path).await?;
            if let Ok(state) = serde_json::from_str::<StateStore>(&content) {
                *self.state_store.write().await = state;
            }
        }
        Ok(())
    }

    async fn load_session_history(&self) -> Result<()> {
        let history_path = self.config.storage_path.join("session_history.json");
        if history_path.exists() {
            let content = tokio::fs::read_to_string(history_path).await?;
            if let Ok(history) = serde_json::from_str::<Vec<SessionRecord>>(&content) {
                self.session_manager.write().await.session_history = history;
            }
        }
        Ok(())
    }

    async fn load_learning_data(&self) -> Result<()> {
        let learning_path = self.config.storage_path.join("learning_repository.json");
        if learning_path.exists() {
            let content = tokio::fs::read_to_string(learning_path).await?;
            if let Ok(learning) = serde_json::from_str::<LearningRepository>(&content) {
                *self.learning_repository.write().await = learning;
            }
        }
        Ok(())
    }

    async fn persist_session_to_disk(&self) -> Result<()> {
        let manager = self.session_manager.read().await;
        
        // Save current session
        if let Some(session) = &manager.current_session {
            let session_path = self.config.storage_path.join(format!("session_{}.json", session.session_id));
            let content = serde_json::to_string_pretty(session)?;
            tokio::fs::write(session_path, content).await?;
        }
        
        // Save session history
        let history_path = self.config.storage_path.join("session_history.json");
        let history_content = serde_json::to_string_pretty(&manager.session_history)?;
        tokio::fs::write(history_path, history_content).await?;
        
        Ok(())
    }

    async fn serialize_context(&self, context: &ExecutionContext) -> Result<SerializableContext> {
        Ok(SerializableContext {
            context_data: context.context_data.clone(),
            iteration_count: context.iteration_count,
            goal_description: context.current_goal.as_ref().map(|g| g.description.clone()),
            context_summary: format!("Iteration {}, {} context items", 
                context.iteration_count, context.context_data.len()),
            key_decisions: Vec::new(), // Would extract from context
            performance_indicators: HashMap::new(), // Would calculate metrics
        })
    }

    async fn serialize_execution_state(&self, context: &ExecutionContext) -> Result<Vec<u8>> {
        let serializable = self.serialize_context(context).await?;
        Ok(serde_json::to_vec(&serializable)?)
    }

    async fn deserialize_execution_state(&self, data: &[u8]) -> Result<ExecutionContext> {
        let serializable: SerializableContext = serde_json::from_slice(data)?;
        
        let mut context = ExecutionContext::new(crate::goal::Goal::new(
            "Cross session persistence context".to_string(),
            crate::goal::GoalType::Analysis
        ));
        context.context_data = serializable.context_data;
        context.iteration_count = serializable.iteration_count;
        
        Ok(context)
    }

    async fn generate_recovery_metadata(&self, context: &ExecutionContext) -> Result<HashMap<String, String>> {
        let mut metadata = HashMap::new();
        metadata.insert("iteration".to_string(), context.iteration_count.to_string());
        metadata.insert("context_size".to_string(), context.context_data.len().to_string());
        metadata.insert("timestamp".to_string(), 
            SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs().to_string());
        Ok(metadata)
    }

    async fn get_current_session_id(&self) -> Result<String> {
        let manager = self.session_manager.read().await;
        manager.current_session
            .as_ref()
            .map(|s| s.session_id.clone())
            .ok_or_else(|| anyhow::anyhow!("No active session"))
    }
}