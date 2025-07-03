use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio_rusqlite::Connection as AsyncConnection;

use crate::context::ExecutionContext;
use crate::orchestrator::Observation;

/// Memory system for storing and retrieving agent experiences and learnings
///
/// The memory system provides both short-term and long-term memory capabilities:
/// - Short-term memory: Recent context, current session data
/// - Long-term memory: Persistent learnings, patterns, successful strategies
/// - Episodic memory: Specific experiences and their outcomes
/// - Semantic memory: General knowledge and rules learned over time
pub struct MemorySystem {
    short_term_memory: Arc<RwLock<ShortTermMemory>>,
    long_term_memory: Arc<dyn LongTermMemory>,
    episodic_memory: Arc<dyn EpisodicMemory>,
    semantic_memory: Arc<dyn SemanticMemory>,
    memory_config: MemoryConfig,
}

/// Configuration for memory system behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub short_term_capacity: usize,
    pub consolidation_threshold: f64,
    pub retention_period: Duration,
    pub relevance_decay_rate: f64,
    pub enable_forgetting: bool,
    pub compression_enabled: bool,
}

/// Short-term memory for immediate context and recent experiences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemory {
    pub current_context: Option<ExecutionContext>,
    pub recent_observations: Vec<Observation>,
    pub active_patterns: Vec<ActivePattern>,
    pub working_hypotheses: Vec<Hypothesis>,
    pub attention_focus: Vec<AttentionItem>,
    pub capacity: usize,
}

/// Active pattern being tracked in short-term memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub occurrences: u32,
    pub last_seen: SystemTime,
    pub confidence: f64,
    pub relevance: f64,
}

/// Working hypothesis about the current situation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub hypothesis_id: String,
    pub description: String,
    pub confidence: f64,
    pub supporting_evidence: Vec<String>,
    pub contradicting_evidence: Vec<String>,
    pub created_at: SystemTime,
}

/// Item in the attention focus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionItem {
    pub item_id: String,
    pub description: String,
    pub importance: f64,
    pub last_accessed: SystemTime,
    pub access_count: u32,
}

/// Types of patterns that can be tracked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    SuccessSequence,
    FailureSequence,
    PerformancePattern,
    UserBehavior,
    EnvironmentalCondition,
    ToolUsagePattern,
}

/// Trait for long-term memory storage
#[async_trait]
pub trait LongTermMemory: Send + Sync {
    /// Store a memory item for long-term retention
    async fn store(&self, memory: MemoryItem) -> Result<String>;

    /// Retrieve memories based on query
    async fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<MemoryItem>>;

    /// Update an existing memory item
    async fn update(&self, memory_id: &str, memory: MemoryItem) -> Result<()>;

    /// Delete a memory item
    async fn delete(&self, memory_id: &str) -> Result<()>;

    /// Search for similar memories
    async fn find_similar(&self, reference: &MemoryItem, threshold: f64)
        -> Result<Vec<MemoryItem>>;
}

/// Trait for episodic memory (specific experiences)
#[async_trait]
pub trait EpisodicMemory: Send + Sync {
    /// Store an episode (specific experience)
    async fn store_episode(&self, episode: Episode) -> Result<String>;

    /// Retrieve episodes matching criteria
    async fn retrieve_episodes(&self, criteria: &EpisodeCriteria) -> Result<Vec<Episode>>;

    /// Get episodes similar to current context
    async fn get_similar_episodes(
        &self,
        context: &ExecutionContext,
        limit: usize,
    ) -> Result<Vec<Episode>>;
}

/// Trait for semantic memory (general knowledge)
#[async_trait]
pub trait SemanticMemory: Send + Sync {
    /// Store semantic knowledge
    async fn store_knowledge(&self, knowledge: Knowledge) -> Result<String>;

    /// Retrieve knowledge by topic
    async fn retrieve_knowledge(&self, topic: &str) -> Result<Vec<Knowledge>>;

    /// Update knowledge based on new evidence
    async fn update_knowledge(&self, knowledge_id: &str, evidence: Evidence) -> Result<()>;
}

/// Generic memory item for long-term storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub memory_id: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub importance: f64,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

/// Types of memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Experience,
    Learning,
    Strategy,
    Pattern,
    Rule,
    Fact,
}

/// Query for retrieving memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub query_text: String,
    pub memory_types: Vec<MemoryType>,
    pub tags: Vec<String>,
    pub time_range: Option<(SystemTime, SystemTime)>,
    pub importance_threshold: Option<f64>,
    pub limit: Option<usize>,
}

/// Specific episode (experience) in episodic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub episode_id: String,
    pub description: String,
    pub context: ExecutionContext,
    pub actions_taken: Vec<String>,
    pub outcomes: Vec<String>,
    pub success: bool,
    pub lessons_learned: Vec<String>,
    pub occurred_at: DateTime<Utc>,
    pub duration: Duration,
    pub importance: f64,
}

/// Criteria for retrieving episodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeCriteria {
    pub context_similarity: Option<f64>,
    pub outcome_type: Option<bool>, // Some(true) for success, Some(false) for failure, None for any
    pub time_range: Option<(SystemTime, SystemTime)>,
    pub importance_threshold: Option<f64>,
    pub tags: Vec<String>,
}

/// Semantic knowledge item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knowledge {
    pub knowledge_id: String,
    pub topic: String,
    pub content: String,
    pub confidence: f64,
    pub evidence: Vec<Evidence>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Evidence supporting knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_id: String,
    pub description: String,
    pub strength: f64,
    pub source: String,
    pub timestamp: SystemTime,
}

impl MemorySystem {
    /// Create a new memory system
    pub fn new(
        long_term_memory: Arc<dyn LongTermMemory>,
        episodic_memory: Arc<dyn EpisodicMemory>,
        semantic_memory: Arc<dyn SemanticMemory>,
        config: MemoryConfig,
    ) -> Self {
        Self {
            short_term_memory: Arc::new(RwLock::new(ShortTermMemory::new(
                config.short_term_capacity,
            ))),
            long_term_memory,
            episodic_memory,
            semantic_memory,
            memory_config: config,
        }
    }

    /// Update memory system with new context and observations
    pub async fn update(&self, context: &ExecutionContext) -> Result<()> {
        // Update short-term memory
        self.update_short_term_memory(context).await?;

        // Check for consolidation opportunities
        if self.should_consolidate().await? {
            self.consolidate_memories().await?;
        }

        // Update attention focus
        self.update_attention_focus(context).await?;

        // Detect and track patterns
        self.detect_and_track_patterns(context).await?;

        Ok(())
    }

    /// Store a significant experience in episodic memory
    pub async fn store_experience(
        &self,
        context: &ExecutionContext,
        outcome: ExperienceOutcome,
    ) -> Result<String> {
        let importance = self.calculate_experience_importance(&outcome);
        let episode = Episode {
            episode_id: uuid::Uuid::new_v4().to_string(),
            description: outcome.description,
            context: context.clone(),
            actions_taken: outcome.actions_taken,
            outcomes: outcome.outcomes,
            success: outcome.success,
            lessons_learned: outcome.lessons_learned,
            occurred_at: Utc::now(),
            duration: outcome.duration,
            importance,
        };

        self.episodic_memory.store_episode(episode).await
    }

    /// Store learned knowledge in semantic memory
    pub async fn store_learning(
        &self,
        topic: &str,
        content: &str,
        evidence: Evidence,
    ) -> Result<String> {
        let knowledge = Knowledge {
            knowledge_id: uuid::Uuid::new_v4().to_string(),
            topic: topic.to_string(),
            content: content.to_string(),
            confidence: evidence.strength,
            evidence: vec![evidence],
            created_at: Utc::now(),
            last_updated: Utc::now(),
        };

        self.semantic_memory.store_knowledge(knowledge).await
    }

    /// Retrieve relevant memories for current context
    pub async fn retrieve_relevant_memories(
        &self,
        context: &ExecutionContext,
        limit: usize,
    ) -> Result<Vec<MemoryItem>> {
        let query = MemoryQuery {
            query_text: context.get_summary(),
            memory_types: vec![
                MemoryType::Experience,
                MemoryType::Learning,
                MemoryType::Strategy,
            ],
            tags: context.get_tags(),
            time_range: None,
            importance_threshold: Some(0.5),
            limit: Some(limit),
        };

        self.long_term_memory.retrieve(&query).await
    }

    /// Get similar past experiences
    pub async fn get_similar_experiences(
        &self,
        context: &ExecutionContext,
        limit: usize,
    ) -> Result<Vec<Episode>> {
        self.episodic_memory
            .get_similar_episodes(context, limit)
            .await
    }

    /// Get relevant knowledge for a topic
    pub async fn get_knowledge(&self, topic: &str) -> Result<Vec<Knowledge>> {
        self.semantic_memory.retrieve_knowledge(topic).await
    }

    /// Update short-term memory with new context
    async fn update_short_term_memory(&self, context: &ExecutionContext) -> Result<()> {
        let mut stm = self.short_term_memory.write().await;
        stm.current_context = Some(context.clone());

        // Add new observations
        if let Some(latest_observation) = context.get_latest_observation() {
            stm.recent_observations.push(latest_observation);

            // Maintain capacity limit
            if stm.recent_observations.len() > stm.capacity {
                stm.recent_observations.remove(0);
            }
        }

        Ok(())
    }

    /// Check if memories should be consolidated
    async fn should_consolidate(&self) -> Result<bool> {
        let stm = self.short_term_memory.read().await;
        Ok(stm.recent_observations.len()
            >= (stm.capacity as f64 * self.memory_config.consolidation_threshold) as usize)
    }

    /// Consolidate short-term memories into long-term storage
    async fn consolidate_memories(&self) -> Result<()> {
        let mut stm = self.short_term_memory.write().await;

        // Identify important observations for consolidation
        let important_observations: Vec<_> = stm
            .recent_observations
            .iter()
            .filter(|obs| obs.relevance_score > 0.7)
            .cloned()
            .collect();

        // Create memory items from important observations
        for observation in important_observations {
            let memory_item = MemoryItem {
                memory_id: uuid::Uuid::new_v4().to_string(),
                memory_type: MemoryType::Experience,
                content: observation.content,
                metadata: HashMap::new(),
                importance: observation.relevance_score,
                created_at: Utc::now(), // Convert from SystemTime if needed
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec!["consolidated".to_string()],
                embedding: None,
            };

            self.long_term_memory.store(memory_item).await?;
        }

        // Clear consolidated observations from short-term memory
        stm.recent_observations
            .retain(|obs| obs.relevance_score <= 0.7);

        Ok(())
    }

    /// Update attention focus based on current context
    async fn update_attention_focus(&self, context: &ExecutionContext) -> Result<()> {
        let mut stm = self.short_term_memory.write().await;

        // Extract important items from context
        if let Some(goal) = context.get_current_goal() {
            let attention_item = AttentionItem {
                item_id: format!("goal_{}", goal.goal_id),
                description: goal.description.clone(),
                importance: 0.9,
                last_accessed: SystemTime::now(),
                access_count: 1,
            };

            // Update or add attention item
            if let Some(existing) = stm
                .attention_focus
                .iter_mut()
                .find(|item| item.item_id == attention_item.item_id)
            {
                existing.last_accessed = SystemTime::now();
                existing.access_count += 1;
            } else {
                stm.attention_focus.push(attention_item);
            }
        }

        // Maintain attention focus size
        stm.attention_focus
            .sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
        stm.attention_focus.truncate(10); // Keep top 10 items

        Ok(())
    }

    /// Detect and track patterns in recent observations
    async fn detect_and_track_patterns(&self, _context: &ExecutionContext) -> Result<()> {
        let mut stm = self.short_term_memory.write().await;

        // Simple pattern detection: consecutive successes or failures
        let recent_successes = stm
            .recent_observations
            .iter()
            .rev()
            .take(5)
            .filter(|obs| obs.content.contains("SUCCESS"))
            .count();

        if recent_successes >= 3 {
            let pattern = ActivePattern {
                pattern_id: "success_streak".to_string(),
                pattern_type: PatternType::SuccessSequence,
                occurrences: recent_successes as u32,
                last_seen: SystemTime::now(),
                confidence: 0.8,
                relevance: 0.9,
            };

            // Update or add pattern
            if let Some(existing) = stm
                .active_patterns
                .iter_mut()
                .find(|p| p.pattern_id == pattern.pattern_id)
            {
                existing.occurrences = pattern.occurrences;
                existing.last_seen = pattern.last_seen;
                existing.confidence = (existing.confidence + pattern.confidence) / 2.0;
            } else {
                stm.active_patterns.push(pattern);
            }
        }

        Ok(())
    }

    /// Calculate importance of an experience
    fn calculate_experience_importance(&self, outcome: &ExperienceOutcome) -> f64 {
        let mut importance: f64 = 0.5; // Base importance

        // Increase importance for successes and failures
        if outcome.success {
            importance += 0.2;
        } else {
            importance += 0.3; // Failures often more important for learning
        }

        // Increase importance for experiences with lessons learned
        if !outcome.lessons_learned.is_empty() {
            importance += 0.2;
        }

        // Increase importance for longer experiences
        if outcome.duration > Duration::from_secs(60) {
            importance += 0.1;
        }

        importance.min(1.0)
    }

    /// Get current short-term memory state
    pub async fn get_short_term_memory(&self) -> ShortTermMemory {
        self.short_term_memory.read().await.clone()
    }

    /// Get memory statistics
    pub async fn get_memory_stats(&self) -> MemoryStats {
        let stm = self.short_term_memory.read().await;

        MemoryStats {
            short_term_items: stm.recent_observations.len(),
            active_patterns: stm.active_patterns.len(),
            attention_items: stm.attention_focus.len(),
            working_hypotheses: stm.working_hypotheses.len(),
        }
    }
}

/// Outcome of an experience for storage
#[derive(Debug, Clone)]
pub struct ExperienceOutcome {
    pub description: String,
    pub actions_taken: Vec<String>,
    pub outcomes: Vec<String>,
    pub success: bool,
    pub lessons_learned: Vec<String>,
    pub duration: Duration,
}

/// Memory system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub short_term_items: usize,
    pub active_patterns: usize,
    pub attention_items: usize,
    pub working_hypotheses: usize,
}

impl ShortTermMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            current_context: None,
            recent_observations: Vec::new(),
            active_patterns: Vec::new(),
            working_hypotheses: Vec::new(),
            attention_focus: Vec::new(),
            capacity,
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            short_term_capacity: 100,
            consolidation_threshold: 0.8,
            retention_period: Duration::from_secs(24 * 60 * 60), // 24 hours
            relevance_decay_rate: 0.1,
            enable_forgetting: true,
            compression_enabled: true,
        }
    }
}

/// SQLite-based persistent memory store (legacy synchronous version)
pub struct SqliteMemoryStore {
    connection: Arc<Mutex<Connection>>,
}

/// Async SQLite-based persistent memory store
pub struct AsyncSqliteMemoryStore {
    connection: AsyncConnection,
}

impl AsyncSqliteMemoryStore {
    /// Create a new async SQLite memory store
    pub async fn new(database_path: &str) -> Result<Self> {
        let conn = if database_path == ":memory:" {
            AsyncConnection::open_in_memory().await?
        } else {
            AsyncConnection::open(database_path).await?
        };

        let store = Self { connection: conn };

        // Create tables if they don't exist
        store.create_tables().await?;

        Ok(store)
    }

    /// Create the necessary tables for memory storage
    async fn create_tables(&self) -> Result<()> {
        self.connection
            .call(|conn| {
                // Memory items table
                conn.execute(
                    r#"
                CREATE TABLE IF NOT EXISTS memory_items (
                    id TEXT PRIMARY KEY,
                    memory_type TEXT NOT NULL,
                    content TEXT NOT NULL,
                    metadata TEXT,
                    importance REAL NOT NULL,
                    created_at TEXT NOT NULL,
                    last_accessed TEXT NOT NULL,
                    access_count INTEGER NOT NULL DEFAULT 0,
                    tags TEXT,
                    embedding BLOB
                )
                "#,
                    [],
                )?;

                // Create indexes for better performance
                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_items(memory_type)",
                    [],
                )?;
                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_memory_importance ON memory_items(importance)",
                    [],
                )?;
                conn.execute(
                    "CREATE INDEX IF NOT EXISTS idx_memory_created_at ON memory_items(created_at)",
                    [],
                )?;

                Ok(())
            })
            .await?;

        Ok(())
    }
}

#[async_trait]
impl LongTermMemory for AsyncSqliteMemoryStore {
    async fn store(&self, memory: MemoryItem) -> Result<String> {
        let id = memory.memory_id.clone();

        self.connection
            .call(move |conn| {
                conn.execute(
                    r#"
                INSERT OR REPLACE INTO memory_items (
                    id, memory_type, content, metadata, importance,
                    created_at, last_accessed, access_count, tags
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                    rusqlite::params![
                        &memory.memory_id,
                        format!("{:?}", memory.memory_type),
                        &memory.content,
                        serde_json::to_string(&memory.metadata)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                        memory.importance,
                        memory.created_at.to_rfc3339(),
                        memory.last_accessed.to_rfc3339(),
                        memory.access_count as i64,
                        serde_json::to_string(&memory.tags)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    ],
                )?;
                Ok(())
            })
            .await?;

        Ok(id)
    }

    async fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<MemoryItem>> {
        let importance_threshold = query.importance_threshold.unwrap_or(0.0);
        let limit = query.limit.unwrap_or(100);

        let memories = self.connection.call(move |conn| {
            let sql = "SELECT * FROM memory_items WHERE importance >= ?1 ORDER BY importance DESC LIMIT ?2";
            let mut stmt = conn.prepare(sql)?;

            let memory_iter = stmt.query_map(
                rusqlite::params![importance_threshold, limit],
                |row| {
                    let memory_type_str: String = row.get("memory_type")?;
                    let memory_type = match memory_type_str.as_str() {
                        "Experience" => MemoryType::Experience,
                        "Learning" => MemoryType::Learning,
                        "Strategy" => MemoryType::Strategy,
                        "Pattern" => MemoryType::Pattern,
                        "Rule" => MemoryType::Rule,
                        "Fact" => MemoryType::Fact,
                        _ => MemoryType::Experience, // Default fallback
                    };

                    let metadata_str: String = row.get("metadata")?;
                    let tags_str: String = row.get("tags")?;
                    let created_at_str: String = row.get("created_at")?;
                    let last_accessed_str: String = row.get("last_accessed")?;

                    Ok(MemoryItem {
                        memory_id: row.get("id")?,
                        memory_type,
                        content: row.get("content")?,
                        metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                        importance: row.get("importance")?,
                        created_at: DateTime::parse_from_rfc3339(&created_at_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        last_accessed: DateTime::parse_from_rfc3339(&last_accessed_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        access_count: row.get::<_, i64>("access_count")? as u32,
                        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                        embedding: None, // TODO: Implement embedding storage
                    })
                },
            )?;

            let mut memories = Vec::new();
            for memory in memory_iter {
                memories.push(memory?);
            }
            Ok(memories)
        }).await?;

        Ok(memories)
    }

    async fn update(&self, memory_id: &str, memory: MemoryItem) -> Result<()> {
        let memory_id = memory_id.to_string();

        self.connection
            .call(move |conn| {
                conn.execute(
                    r#"
                UPDATE memory_items
                SET memory_type = ?2, content = ?3, metadata = ?4, importance = ?5,
                    last_accessed = ?6, access_count = ?7, tags = ?8
                WHERE id = ?1
                "#,
                    rusqlite::params![
                        memory_id,
                        format!("{:?}", memory.memory_type),
                        &memory.content,
                        serde_json::to_string(&memory.metadata)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                        memory.importance,
                        memory.last_accessed.to_rfc3339(),
                        memory.access_count as i64,
                        serde_json::to_string(&memory.tags)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    ],
                )?;
                Ok(())
            })
            .await?;

        Ok(())
    }

    async fn delete(&self, memory_id: &str) -> Result<()> {
        let memory_id = memory_id.to_string();

        self.connection
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM memory_items WHERE id = ?1",
                    rusqlite::params![memory_id],
                )?;
                Ok(())
            })
            .await?;

        Ok(())
    }

    async fn find_similar(
        &self,
        reference: &MemoryItem,
        threshold: f64,
    ) -> Result<Vec<MemoryItem>> {
        // Simple similarity based on content matching and importance
        // In a real implementation, you'd use embeddings and vector similarity
        let search_term = format!(
            "%{}%",
            reference.content.split_whitespace().next().unwrap_or("")
        );

        let memories = self.connection.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM memory_items WHERE importance >= ?1 AND content LIKE ?2 ORDER BY importance DESC LIMIT 10"
            )?;

            let memory_iter = stmt.query_map(rusqlite::params![threshold, search_term], |row| {
                let memory_type_str: String = row.get("memory_type")?;
                let memory_type = match memory_type_str.as_str() {
                    "Experience" => MemoryType::Experience,
                    "Learning" => MemoryType::Learning,
                    "Strategy" => MemoryType::Strategy,
                    "Pattern" => MemoryType::Pattern,
                    "Rule" => MemoryType::Rule,
                    "Fact" => MemoryType::Fact,
                    _ => MemoryType::Experience, // Default fallback
                };

                let metadata_str: String = row.get("metadata")?;
                let tags_str: String = row.get("tags")?;
                let created_at_str: String = row.get("created_at")?;
                let last_accessed_str: String = row.get("last_accessed")?;

                Ok(MemoryItem {
                    memory_id: row.get("id")?,
                    memory_type,
                    content: row.get("content")?,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                    importance: row.get("importance")?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    last_accessed: DateTime::parse_from_rfc3339(&last_accessed_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    access_count: row.get::<_, i64>("access_count")? as u32,
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    embedding: None,
                })
            })?;

            let mut memories = Vec::new();
            for memory in memory_iter {
                memories.push(memory?);
            }
            Ok(memories)
        }).await?;

        Ok(memories)
    }
}

impl SqliteMemoryStore {
    /// Create a new SQLite memory store
    pub fn new(database_path: &str) -> Result<Self> {
        let conn = if database_path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            Connection::open(database_path)?
        };

        let store = Self {
            connection: Arc::new(Mutex::new(conn)),
        };

        // Create tables if they don't exist
        store.create_tables()?;

        Ok(store)
    }

    /// Create the necessary tables for memory storage
    fn create_tables(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        // Memory items table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS memory_items (
                id TEXT PRIMARY KEY,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                importance REAL NOT NULL,
                created_at TEXT NOT NULL,
                last_accessed TEXT NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0,
                tags TEXT,
                embedding BLOB
            )
            "#,
            [],
        )?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_items(memory_type)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_importance ON memory_items(importance)",
            [],
        )?;

        Ok(())
    }
}

#[async_trait]
impl LongTermMemory for SqliteMemoryStore {
    async fn store(&self, memory: MemoryItem) -> Result<String> {
        let id = memory.memory_id.clone();
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO memory_items (
                id, memory_type, content, metadata, importance,
                created_at, last_accessed, access_count, tags
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            rusqlite::params![
                &memory.memory_id,
                format!("{:?}", memory.memory_type),
                &memory.content,
                serde_json::to_string(&memory.metadata)?,
                memory.importance,
                memory.created_at.to_rfc3339(),
                memory.last_accessed.to_rfc3339(),
                memory.access_count as i64,
                serde_json::to_string(&memory.tags)?,
            ],
        )?;

        Ok(id)
    }

    async fn retrieve(&self, query: &MemoryQuery) -> Result<Vec<MemoryItem>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        // Use a simple query for now
        let sql =
            "SELECT * FROM memory_items WHERE importance >= ?1 ORDER BY importance DESC LIMIT ?2";
        let mut stmt = conn.prepare(sql)?;

        let memory_iter = stmt.query_map(
            rusqlite::params![
                query.importance_threshold.unwrap_or(0.0),
                query.limit.unwrap_or(100)
            ],
            |row| {
                let memory_type_str: String = row.get("memory_type")?;
                let memory_type = match memory_type_str.as_str() {
                    "Experience" => MemoryType::Experience,
                    "Learning" => MemoryType::Learning,
                    "Strategy" => MemoryType::Strategy,
                    "Pattern" => MemoryType::Pattern,
                    "Rule" => MemoryType::Rule,
                    "Fact" => MemoryType::Fact,
                    _ => MemoryType::Experience, // Default fallback
                };

                let metadata_str: String = row.get("metadata")?;
                let tags_str: String = row.get("tags")?;
                let created_at_str: String = row.get("created_at")?;
                let last_accessed_str: String = row.get("last_accessed")?;

                Ok(MemoryItem {
                    memory_id: row.get("id")?,
                    memory_type,
                    content: row.get("content")?,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                    importance: row.get("importance")?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    last_accessed: DateTime::parse_from_rfc3339(&last_accessed_str)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc),
                    access_count: row.get::<_, i64>("access_count")? as u32,
                    tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                    embedding: None,
                })
            },
        )?;

        let mut memories = Vec::new();
        for memory in memory_iter {
            memories.push(memory?);
        }

        Ok(memories)
    }

    async fn update(&self, memory_id: &str, memory: MemoryItem) -> Result<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        conn.execute(
            r#"
            UPDATE memory_items
            SET memory_type = ?2, content = ?3, metadata = ?4, importance = ?5,
                last_accessed = ?6, access_count = ?7, tags = ?8
            WHERE id = ?1
            "#,
            rusqlite::params![
                memory_id,
                format!("{:?}", memory.memory_type),
                &memory.content,
                serde_json::to_string(&memory.metadata)?,
                memory.importance,
                memory.last_accessed.to_rfc3339(),
                memory.access_count as i64,
                serde_json::to_string(&memory.tags)?,
            ],
        )?;

        Ok(())
    }

    async fn delete(&self, memory_id: &str) -> Result<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        conn.execute(
            "DELETE FROM memory_items WHERE id = ?1",
            rusqlite::params![memory_id],
        )?;

        Ok(())
    }

    async fn find_similar(
        &self,
        reference: &MemoryItem,
        threshold: f64,
    ) -> Result<Vec<MemoryItem>> {
        // Simple similarity based on content matching and importance
        // In a real implementation, you'd use embeddings and vector similarity
        let conn = self.connection.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT * FROM memory_items WHERE importance >= ?1 AND content LIKE ?2 ORDER BY importance DESC LIMIT 10"
        )?;

        let search_term = format!(
            "%{}%",
            reference.content.split_whitespace().next().unwrap_or("")
        );

        let memory_iter = stmt.query_map(rusqlite::params![threshold, search_term], |row| {
            let memory_type_str: String = row.get("memory_type")?;
            let memory_type = match memory_type_str.as_str() {
                "Experience" => MemoryType::Experience,
                "Learning" => MemoryType::Learning,
                "Strategy" => MemoryType::Strategy,
                "Pattern" => MemoryType::Pattern,
                "Rule" => MemoryType::Rule,
                "Fact" => MemoryType::Fact,
                _ => MemoryType::Experience, // Default fallback
            };

            let metadata_str: String = row.get("metadata")?;
            let tags_str: String = row.get("tags")?;
            let created_at_str: String = row.get("created_at")?;
            let last_accessed_str: String = row.get("last_accessed")?;

            Ok(MemoryItem {
                memory_id: row.get("id")?,
                memory_type,
                content: row.get("content")?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
                importance: row.get("importance")?,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                last_accessed: DateTime::parse_from_rfc3339(&last_accessed_str)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc),
                access_count: row.get::<_, i64>("access_count")? as u32,
                tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                embedding: None,
            })
        })?;

        let mut memories = Vec::new();
        for memory in memory_iter {
            memories.push(memory?);
        }

        Ok(memories)
    }
}

/// Query parameters for episodes
#[derive(Debug, Clone)]
pub struct EpisodeQuery {
    pub success_filter: Option<bool>,
    pub importance_threshold: Option<f64>,
    pub limit: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert_eq!(config.short_term_capacity, 100);
        assert_eq!(config.consolidation_threshold, 0.8);
        assert!(config.enable_forgetting);
    }

    #[test]
    fn test_short_term_memory_creation() {
        let stm = ShortTermMemory::new(50);
        assert_eq!(stm.capacity, 50);
        assert!(stm.recent_observations.is_empty());
        assert!(stm.active_patterns.is_empty());
    }

    #[test]
    fn test_memory_item_creation() {
        let memory_item = MemoryItem {
            memory_id: "test-memory".to_string(),
            memory_type: MemoryType::Experience,
            content: "Test content".to_string(),
            metadata: HashMap::new(),
            importance: 0.8,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec!["test".to_string()],
            embedding: None,
        };

        assert_eq!(memory_item.memory_id, "test-memory");
        assert_eq!(memory_item.importance, 0.8);
        assert!(matches!(memory_item.memory_type, MemoryType::Experience));
    }

    #[tokio::test]
    async fn test_sqlite_memory_store() {
        let store = SqliteMemoryStore::new(":memory:").expect("Failed to create memory store");

        let memory = MemoryItem {
            memory_id: "test-memory".to_string(),
            memory_type: MemoryType::Experience,
            content: "Test memory content".to_string(),
            metadata: HashMap::new(),
            importance: 0.8,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec!["test".to_string()],
            embedding: None,
        };

        // Test storing memory
        let stored_id = store
            .store(memory.clone())
            .await
            .expect("Failed to store memory");
        assert_eq!(stored_id, "test-memory");

        // Test retrieving memory
        let query = MemoryQuery {
            memory_types: vec![MemoryType::Experience],
            importance_threshold: Some(0.5),
            limit: Some(10),
            query_text: "test".to_string(),
            tags: vec![],
            time_range: None,
        };

        let retrieved = store.retrieve(&query).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].memory_id, "test-memory");
        assert_eq!(retrieved[0].content, "Test memory content");
    }

    #[test]
    fn test_sqlite_memory_store_creation() {
        let store = SqliteMemoryStore::new(":memory:");
        assert!(store.is_ok());
    }
}
