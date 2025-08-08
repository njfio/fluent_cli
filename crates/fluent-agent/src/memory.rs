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

    /// Retrieve a specific memory by ID
    async fn retrieve(&self, memory_id: &str) -> Result<Option<MemoryItem>>;

    /// Update an existing memory item
    async fn update(&self, memory: MemoryItem) -> Result<()>;

    /// Delete a memory item
    async fn delete(&self, memory_id: &str) -> Result<()>;

    /// Search for memories based on query
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>>;

    /// Find similar memories to a reference memory
    async fn find_similar(&self, memory: &MemoryItem, threshold: f32) -> Result<Vec<MemoryItem>>;

    /// Get recent memories
    async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryItem>>;

    /// Get memories by importance threshold
    async fn get_by_importance(&self, min_importance: f32, limit: usize) -> Result<Vec<MemoryItem>>;

    /// Clean up old memories
    async fn cleanup_old_memories(&self, days: u32) -> Result<usize>;
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

        self.long_term_memory.search(query).await
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
            .sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
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
///
/// ⚠️ **DEPRECATED**: This implementation uses blocking SQLite operations in async functions,
/// which can block the async runtime. Use `AsyncSqliteMemoryStore` instead for production code.
/// This struct is kept only for backward compatibility in tests.
#[deprecated(
    since = "0.1.0",
    note = "Use AsyncSqliteMemoryStore instead. This version blocks the async runtime."
)]
pub struct SqliteMemoryStore {
    connection: Arc<Mutex<Connection>>,
}

/// SQLite connection pool configuration
#[derive(Debug, Clone)]
pub struct SqlitePoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub acquire_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

impl Default for SqlitePoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
            max_lifetime: Some(Duration::from_secs(3600)), // 1 hour
        }
    }
}

/// SQLite connection pool for managing multiple database connections
pub struct SqliteConnectionPool {
    connections: Arc<RwLock<Vec<PooledConnection>>>,
    config: SqlitePoolConfig,
    database_path: String,
    stats: Arc<RwLock<PoolStats>>,
}

/// Pooled connection wrapper
#[derive(Debug)]
struct PooledConnection {
    connection: AsyncConnection,
    created_at: SystemTime,
    last_used: SystemTime,
    in_use: bool,
}

/// Connection pool statistics
#[derive(Debug, Default, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
    pub total_acquisitions: u64,
    pub total_releases: u64,
    pub acquisition_timeouts: u64,
    pub connection_errors: u64,
}

impl SqliteConnectionPool {
    /// Create a new SQLite connection pool
    pub async fn new(database_path: &str, config: SqlitePoolConfig) -> Result<Self> {
        let pool = Self {
            connections: Arc::new(RwLock::new(Vec::new())),
            config,
            database_path: database_path.to_string(),
            stats: Arc::new(RwLock::new(PoolStats::default())),
        };

        // Initialize minimum connections
        pool.initialize_connections().await?;

        Ok(pool)
    }

    /// Initialize minimum connections in the pool
    async fn initialize_connections(&self) -> Result<()> {
        let mut connections = self.connections.write().await;

        for _ in 0..self.config.min_connections {
            let conn = self.create_connection().await?;
            connections.push(PooledConnection {
                connection: conn,
                created_at: SystemTime::now(),
                last_used: SystemTime::now(),
                in_use: false,
            });
        }

        Ok(())
    }

    /// Create a new database connection
    async fn create_connection(&self) -> Result<AsyncConnection> {
        let conn = if self.database_path == ":memory:" {
            AsyncConnection::open_in_memory().await?
        } else {
            AsyncConnection::open(&self.database_path).await?
        };

        // Create tables for new connections
        self.create_tables_for_connection(&conn).await?;

        Ok(conn)
    }

    /// Create tables for a specific connection
    async fn create_tables_for_connection(&self, conn: &AsyncConnection) -> Result<()> {
        conn.call(|conn| {
            conn.execute(
                r#"
                CREATE TABLE IF NOT EXISTS memory_items (
                    id TEXT PRIMARY KEY,
                    memory_type TEXT NOT NULL,
                    content TEXT NOT NULL,
                    metadata TEXT NOT NULL,
                    importance REAL NOT NULL,
                    created_at TEXT NOT NULL,
                    last_accessed TEXT NOT NULL,
                    access_count INTEGER NOT NULL DEFAULT 0,
                    tags TEXT NOT NULL DEFAULT '[]',
                    embedding BLOB
                )
                "#,
                [],
            )?;

            // Create comprehensive indexes for better query performance

            // Single column indexes
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_items(memory_type)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_importance ON memory_items(importance DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_created_at ON memory_items(created_at DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_last_accessed ON memory_items(last_accessed DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_access_count ON memory_items(access_count DESC)",
                [],
            )?;

            // Composite indexes for common query patterns
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_type_importance ON memory_items(memory_type, importance DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_importance_created_at ON memory_items(importance DESC, created_at DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_type_created_at ON memory_items(memory_type, created_at DESC)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_importance_access_count ON memory_items(importance DESC, access_count DESC)",
                [],
            )?;

            // Full-text search index for content (if SQLite supports FTS)
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_content_search ON memory_items(content)",
                [],
            )?;

            // Covering index for common SELECT patterns
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_memory_covering_main ON memory_items(importance DESC, memory_type, created_at DESC) WHERE importance >= 0.1",
                [],
            )?;

            Ok(())
        })
        .await?;

        Ok(())
    }

    /// Acquire a connection from the pool
    pub async fn acquire(&self) -> Result<PooledConnectionGuard> {
        let start_time = SystemTime::now();
        let timeout = self.config.acquire_timeout;

        loop {
            // Try to get an available connection
            if let Some(guard) = self.try_acquire().await? {
                self.update_stats(|stats| {
                    stats.total_acquisitions += 1;
                }).await;
                return Ok(guard);
            }

            // Check if we can create a new connection
            if self.can_create_connection().await {
                let conn = self.create_connection().await?;
                let guard = self.add_connection(conn).await?;
                self.update_stats(|stats| {
                    stats.total_acquisitions += 1;
                    stats.total_connections += 1;
                }).await;
                return Ok(guard);
            }

            // Check timeout
            if start_time.elapsed().unwrap_or(Duration::ZERO) >= timeout {
                self.update_stats(|stats| {
                    stats.acquisition_timeouts += 1;
                }).await;
                return Err(anyhow!("Connection acquisition timeout"));
            }

            // Wait a bit before retrying
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Try to acquire an available connection
    async fn try_acquire(&self) -> Result<Option<PooledConnectionGuard>> {
        let mut connections = self.connections.write().await;

        // Clean up expired connections first
        self.cleanup_expired_connections(&mut connections).await;

        // Find an available connection
        for (index, pooled_conn) in connections.iter_mut().enumerate() {
            if !pooled_conn.in_use {
                pooled_conn.in_use = true;
                pooled_conn.last_used = SystemTime::now();

                return Ok(Some(PooledConnectionGuard {
                    pool: self.clone(),
                    connection_index: index,
                }));
            }
        }

        Ok(None)
    }

    /// Check if we can create a new connection
    async fn can_create_connection(&self) -> bool {
        let connections = self.connections.read().await;
        connections.len() < self.config.max_connections
    }

    /// Add a new connection to the pool
    async fn add_connection(&self, conn: AsyncConnection) -> Result<PooledConnectionGuard> {
        let mut connections = self.connections.write().await;

        let index = connections.len();
        connections.push(PooledConnection {
            connection: conn,
            created_at: SystemTime::now(),
            last_used: SystemTime::now(),
            in_use: true,
        });

        Ok(PooledConnectionGuard {
            pool: self.clone(),
            connection_index: index,
        })
    }

    /// Clean up expired connections
    async fn cleanup_expired_connections(&self, connections: &mut Vec<PooledConnection>) {
        if let Some(max_lifetime) = self.config.max_lifetime {
            connections.retain(|conn| {
                !conn.in_use && conn.created_at.elapsed().unwrap_or(Duration::ZERO) < max_lifetime
            });
        }

        if let Some(idle_timeout) = self.config.idle_timeout {
            connections.retain(|conn| {
                !conn.in_use && conn.last_used.elapsed().unwrap_or(Duration::ZERO) < idle_timeout
            });
        }
    }

    /// Release a connection back to the pool
    async fn release(&self, connection_index: usize) {
        let mut connections = self.connections.write().await;

        if let Some(pooled_conn) = connections.get_mut(connection_index) {
            pooled_conn.in_use = false;
            pooled_conn.last_used = SystemTime::now();
        }

        self.update_stats(|stats| {
            stats.total_releases += 1;
        }).await;
    }

    /// Update pool statistics
    async fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut PoolStats),
    {
        let mut stats = self.stats.write().await;
        update_fn(&mut *stats);

        // Update current connection counts
        let connections = self.connections.read().await;
        stats.total_connections = connections.len();
        stats.active_connections = connections.iter().filter(|c| c.in_use).count();
        stats.idle_connections = connections.iter().filter(|c| !c.in_use).count();
    }

    /// Get current pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }
}

impl Clone for SqliteConnectionPool {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            config: self.config.clone(),
            database_path: self.database_path.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Connection guard that automatically releases the connection when dropped
pub struct PooledConnectionGuard {
    pool: SqliteConnectionPool,
    connection_index: usize,
}

impl PooledConnectionGuard {


    /// Execute a closure with the connection
    pub async fn with_connection<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&AsyncConnection) -> Result<R> + Send,
        R: Send,
    {
        let connections = self.pool.connections.read().await;
        let pooled_conn = connections.get(self.connection_index)
            .ok_or_else(|| anyhow!("Connection no longer available"))?;

        f(&pooled_conn.connection)
    }
}

impl Drop for PooledConnectionGuard {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        let index = self.connection_index;

        // Release the connection asynchronously
        tokio::spawn(async move {
            pool.release(index).await;
        });
    }
}

/// Async SQLite-based persistent memory store with connection pooling
pub struct AsyncSqliteMemoryStore {
    pool: SqliteConnectionPool,
}

impl AsyncSqliteMemoryStore {
    /// Create a new async SQLite memory store with default pool configuration
    pub async fn new(database_path: &str) -> Result<Self> {
        Self::with_pool_config(database_path, SqlitePoolConfig::default()).await
    }

    /// Create a new async SQLite memory store with custom pool configuration
    pub async fn with_pool_config(database_path: &str, config: SqlitePoolConfig) -> Result<Self> {
        let pool = SqliteConnectionPool::new(database_path, config).await?;
        Ok(Self { pool })
    }

    /// Get pool statistics
    pub async fn get_pool_stats(&self) -> PoolStats {
        self.pool.get_stats().await
    }

    /// Get a connection from the pool and execute a closure
    #[allow(dead_code)]
    async fn with_connection<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&AsyncConnection) -> Result<R> + Send,
        R: Send,
    {
        let guard = self.pool.acquire().await?;
        guard.with_connection(f).await
    }

    /// Execute a simple transaction (simplified implementation)
    pub async fn execute_in_transaction<F>(&self, operation: F) -> Result<()>
    where
        F: FnOnce() -> Result<()> + Send,
    {
        // For now, just execute the operation without explicit transaction handling
        // This can be enhanced later with proper transaction support
        operation()
    }

    /// Clean up old memories older than the specified number of days
    pub async fn cleanup_old_memories(&self, days: u32) -> Result<usize> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_rfc3339 = cutoff_date.to_rfc3339();

        let guard = self.pool.acquire().await?;
        let connections = guard.pool.connections.read().await;
        let pooled_conn = connections.get(guard.connection_index)
            .ok_or_else(|| anyhow!("Connection no longer available"))?;

        let count = pooled_conn.connection.call(move |conn| {
            let count = conn.execute(
                "DELETE FROM memory_items WHERE created_at < ?1",
                rusqlite::params![cutoff_rfc3339],
            )?;
            Ok(count)
        }).await?;

        Ok(count)
    }

    /// Store a memory item
    pub async fn store(&self, memory: MemoryItem) -> Result<String> {
        let id = memory.memory_id.clone();
        let guard = self.pool.acquire().await?;
        let connections = guard.pool.connections.read().await;
        let pooled_conn = connections.get(guard.connection_index)
            .ok_or_else(|| anyhow!("Connection no longer available"))?;

        pooled_conn.connection.call(move |conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO memory_items (
                    id, memory_type, content, metadata, importance,
                    created_at, last_accessed, access_count, tags, embedding
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                rusqlite::params![
                    &memory.memory_id,
                    format!("{:?}", memory.memory_type),
                    &memory.content,
                    serde_json::to_string(&memory.metadata).unwrap_or_default(),
                    memory.importance,
                    memory.created_at.to_rfc3339(),
                    memory.last_accessed.to_rfc3339(),
                    memory.access_count as i64,
                    serde_json::to_string(&memory.tags).unwrap_or_default(),
                    memory.embedding.as_ref().and_then(|emb| {
                        bincode::serialize(emb).ok()
                    }),
                ],
            )?;
            Ok(())
        }).await?;

        Ok(id)
    }

    /// Retrieve a memory item by ID
    pub async fn retrieve(&self, memory_id: &str) -> Result<Option<MemoryItem>> {
        let guard = self.pool.acquire().await?;
        let connections = guard.pool.connections.read().await;
        let pooled_conn = connections.get(guard.connection_index)
            .ok_or_else(|| anyhow!("Connection no longer available"))?;

        let memory_id = memory_id.to_string();
        pooled_conn.connection.call(move |conn| {
            let mut stmt = conn.prepare("SELECT * FROM memory_items WHERE id = ?1")?;

            let memory_iter = stmt.query_map(
                rusqlite::params![memory_id],
                |row| {
                    let memory_type_str: String = row.get("memory_type")?;
                    let memory_type = match memory_type_str.as_str() {
                        "Experience" => MemoryType::Experience,
                        "Learning" => MemoryType::Learning,
                        "Strategy" => MemoryType::Strategy,
                        "Pattern" => MemoryType::Pattern,
                        "Rule" => MemoryType::Rule,
                        "Fact" => MemoryType::Fact,
                        _ => MemoryType::Experience,
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

            let mut memories: Vec<MemoryItem> = memory_iter.collect::<Result<Vec<_>, _>>()?;
            Ok(memories.pop())
        }).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
    }

    /// Update a memory item
    pub async fn update(&self, memory: MemoryItem) -> Result<()> {
        let guard = self.pool.acquire().await?;
        let connections = guard.pool.connections.read().await;
        let pooled_conn = connections.get(guard.connection_index)
            .ok_or_else(|| anyhow!("Connection no longer available"))?;

        pooled_conn.connection.call(move |conn| {
            conn.execute(
                r#"
                UPDATE memory_items
                SET memory_type = ?2, content = ?3, metadata = ?4, importance = ?5,
                    last_accessed = ?6, access_count = ?7, tags = ?8
                WHERE id = ?1
                "#,
                rusqlite::params![
                    &memory.memory_id,
                    format!("{:?}", memory.memory_type),
                    &memory.content,
                    serde_json::to_string(&memory.metadata).unwrap_or_default(),
                    memory.importance,
                    memory.last_accessed.to_rfc3339(),
                    memory.access_count as i64,
                    serde_json::to_string(&memory.tags).unwrap_or_default(),
                ],
            )?;
            Ok(())
        }).await?;

        Ok(())
    }

    /// Delete a memory item by ID
    pub async fn delete(&self, memory_id: &str) -> Result<()> {
        let guard = self.pool.acquire().await?;
        let connections = guard.pool.connections.read().await;
        let pooled_conn = connections.get(guard.connection_index)
            .ok_or_else(|| anyhow!("Connection no longer available"))?;

        let memory_id = memory_id.to_string();
        pooled_conn.connection.call(move |conn| {
            conn.execute(
                "DELETE FROM memory_items WHERE id = ?1",
                rusqlite::params![memory_id],
            )?;
            Ok(())
        }).await?;

        Ok(())
    }

    /// Search for memory items based on query parameters
    pub async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>> {
        let guard = self.pool.acquire().await?;
        let connections = guard.pool.connections.read().await;
        let pooled_conn = connections.get(guard.connection_index)
            .ok_or_else(|| anyhow!("Connection no longer available"))?;

        pooled_conn.connection.call(move |conn| {
            // Build dynamic query based on MemoryQuery parameters
            let mut sql = "SELECT * FROM memory_items WHERE 1=1".to_string();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            // Filter by memory types
            if !query.memory_types.is_empty() {
                let type_placeholders = query.memory_types.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                sql.push_str(&format!(" AND memory_type IN ({})", type_placeholders));
                for memory_type in &query.memory_types {
                    params.push(Box::new(format!("{:?}", memory_type)));
                }
            }

            // Filter by content (query_text)
            if !query.query_text.is_empty() {
                sql.push_str(" AND content LIKE ?");
                params.push(Box::new(format!("%{}%", query.query_text)));
            }

            // Filter by importance threshold
            if let Some(min_importance) = query.importance_threshold {
                sql.push_str(" AND importance >= ?");
                params.push(Box::new(min_importance));
            }

            // Filter by tags
            if !query.tags.is_empty() {
                for tag in &query.tags {
                    sql.push_str(" AND tags LIKE ?");
                    params.push(Box::new(format!("%{}%", tag)));
                }
            }

            sql.push_str(" ORDER BY importance DESC");

            if let Some(limit) = query.limit {
                sql.push_str(" LIMIT ?");
                params.push(Box::new(limit as i64));
            }

            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            let memory_iter = stmt.query_map(&param_refs[..], |row| {
                let memory_type_str: String = row.get("memory_type")?;
                let memory_type = match memory_type_str.as_str() {
                    "Experience" => MemoryType::Experience,
                    "Learning" => MemoryType::Learning,
                    "Strategy" => MemoryType::Strategy,
                    "Pattern" => MemoryType::Pattern,
                    "Rule" => MemoryType::Rule,
                    "Fact" => MemoryType::Fact,
                    _ => MemoryType::Experience,
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

            let memories: Vec<MemoryItem> = memory_iter.collect::<Result<Vec<_>, _>>()?;
            Ok(memories)
        }).await.map_err(|e| anyhow::anyhow!("Database error: {}", e))
    }
}

#[async_trait]
impl LongTermMemory for AsyncSqliteMemoryStore {
    async fn store(&self, memory: MemoryItem) -> Result<String> {
        let id = memory.memory_id.clone();
        self.store(memory).await?;
        Ok(id)
    }

    async fn retrieve(&self, memory_id: &str) -> Result<Option<MemoryItem>> {
        self.retrieve(memory_id).await
    }

    async fn update(&self, memory: MemoryItem) -> Result<()> {
        self.update(memory).await
    }

    async fn delete(&self, memory_id: &str) -> Result<()> {
        self.delete(memory_id).await
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>> {
        self.search(query).await
    }

    async fn cleanup_old_memories(&self, days: u32) -> Result<usize> {
        self.cleanup_old_memories(days).await
    }

    async fn find_similar(&self, memory: &MemoryItem, threshold: f32) -> Result<Vec<MemoryItem>> {
        // For now, implement a simple content-based similarity search
        // In a full implementation, this would use vector embeddings
        let query = MemoryQuery {
            query_text: String::new(),
            memory_types: vec![],
            tags: vec![],
            time_range: None,
            importance_threshold: None,
            limit: None,
        };
        let all_memories = self.search(query).await?;
        let mut similar = Vec::new();

        for candidate in all_memories {
            if candidate.memory_id != memory.memory_id {
                // Simple similarity based on content overlap
                let similarity = calculate_simple_similarity(&memory.content, &candidate.content);
                if similarity >= threshold {
                    similar.push(candidate);
                }
            }
        }

        Ok(similar)
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryItem>> {
        let query = MemoryQuery {
            query_text: String::new(),
            memory_types: vec![],
            tags: vec![],
            time_range: None,
            importance_threshold: None,
            limit: Some(limit),
        };
        self.search(query).await
    }

    async fn get_by_importance(&self, min_importance: f32, limit: usize) -> Result<Vec<MemoryItem>> {
        let query = MemoryQuery {
            query_text: String::new(),
            memory_types: vec![],
            tags: vec![],
            time_range: None,
            importance_threshold: Some(min_importance as f64),
            limit: Some(limit),
        };
        self.search(query).await
    }


}

/// Calculate simple text similarity (placeholder for vector similarity)
fn calculate_simple_similarity(text1: &str, text2: &str) -> f32 {
    let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();

    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
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

    /// Create the necessary tables
    fn create_tables(&self) -> Result<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS memory_items (
                id TEXT PRIMARY KEY,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT NOT NULL,
                importance REAL NOT NULL,
                created_at TEXT NOT NULL,
                last_accessed TEXT NOT NULL,
                access_count INTEGER NOT NULL,
                tags TEXT NOT NULL,
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

// Removed duplicate LongTermMemory implementation
// Keeping only the second implementation below


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
                created_at, last_accessed, access_count, tags, embedding
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
                memory.embedding.as_ref().and_then(|emb| {
                    bincode::serialize(emb).ok()
                }),
            ],
        )?;

        Ok(id)
    }

    async fn retrieve(&self, memory_id: &str) -> Result<Option<MemoryItem>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT * FROM memory_items WHERE id = ?1"
        )?;

        let memory_iter = stmt.query_map(
            rusqlite::params![memory_id],
            |row| {
                let memory_type_str: String = row.get("memory_type")?;
                let memory_type = match memory_type_str.as_str() {
                    "Experience" => MemoryType::Experience,
                    "Learning" => MemoryType::Learning,
                    "Strategy" => MemoryType::Strategy,
                    "Pattern" => MemoryType::Pattern,
                    "Rule" => MemoryType::Rule,
                    "Fact" => MemoryType::Fact,
                    _ => MemoryType::Experience,
                };

                let metadata_str: String = row.get("metadata")?;
                let tags_str: String = row.get("tags")?;
                let created_at_str: String = row.get("created_at")?;
                let last_accessed_str: String = row.get("last_accessed")?;

                Ok(MemoryItem {
                    memory_id: row.get("id")?,
                    memory_type,
                    content: row.get("content")?,
                    metadata: serde_json::from_str(&metadata_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0, rusqlite::types::Type::Text, Box::new(e)
                        )
                    })?,
                    importance: row.get("importance")?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    last_accessed: chrono::DateTime::parse_from_rfc3339(&last_accessed_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    access_count: row.get::<_, i64>("access_count")? as u32,
                    tags: serde_json::from_str(&tags_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0, rusqlite::types::Type::Text, Box::new(e)
                        )
                    })?,
                    embedding: {
                        let embedding_blob: Option<Vec<u8>> = row.get("embedding")?;
                        embedding_blob.and_then(|blob| {
                            bincode::deserialize(&blob).ok()
                        })
                    },
                })
            }
        )?;

        let mut memories: Vec<MemoryItem> = memory_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(memories.pop())
    }

    async fn update(&self, memory: MemoryItem) -> Result<()> {
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
                &memory.memory_id,
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

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryItem>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        // Build dynamic query based on MemoryQuery parameters
        let mut sql = "SELECT * FROM memory_items WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Filter by memory types
        if !query.memory_types.is_empty() {
            let type_placeholders = query.memory_types.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            sql.push_str(&format!(" AND memory_type IN ({})", type_placeholders));
            for memory_type in &query.memory_types {
                params.push(Box::new(format!("{:?}", memory_type)));
            }
        }

        // Filter by content (query_text)
        if !query.query_text.is_empty() {
            sql.push_str(" AND content LIKE ?");
            params.push(Box::new(format!("%{}%", query.query_text)));
        }

        // Filter by importance threshold
        if let Some(min_importance) = query.importance_threshold {
            sql.push_str(" AND importance >= ?");
            params.push(Box::new(min_importance));
        }

        // Filter by tags
        if !query.tags.is_empty() {
            for tag in &query.tags {
                sql.push_str(" AND tags LIKE ?");
                params.push(Box::new(format!("%{}%", tag)));
            }
        }

        sql.push_str(" ORDER BY importance DESC");

        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit as i64));
        }

        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let memory_iter = stmt.query_map(&param_refs[..], |row| {
            let memory_type_str: String = row.get("memory_type")?;
            let memory_type = match memory_type_str.as_str() {
                "Experience" => MemoryType::Experience,
                "Learning" => MemoryType::Learning,
                "Strategy" => MemoryType::Strategy,
                "Pattern" => MemoryType::Pattern,
                "Rule" => MemoryType::Rule,
                "Fact" => MemoryType::Fact,
                _ => MemoryType::Experience,
            };

            let metadata_str: String = row.get("metadata")?;
            let tags_str: String = row.get("tags")?;
            let created_at_str: String = row.get("created_at")?;
            let last_accessed_str: String = row.get("last_accessed")?;

            Ok(MemoryItem {
                memory_id: row.get("id")?,
                memory_type,
                content: row.get("content")?,
                metadata: serde_json::from_str(&metadata_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0, rusqlite::types::Type::Text, Box::new(e)
                    )
                })?,
                importance: row.get("importance")?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                last_accessed: chrono::DateTime::parse_from_rfc3339(&last_accessed_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                access_count: row.get::<_, i64>("access_count")? as u32,
                tags: serde_json::from_str(&tags_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0, rusqlite::types::Type::Text, Box::new(e)
                    )
                })?,
                embedding: {
                    let embedding_blob: Option<Vec<u8>> = row.get("embedding")?;
                    embedding_blob.and_then(|blob| {
                        bincode::deserialize(&blob).ok()
                    })
                },
            })
        })?;

        let memories: Vec<MemoryItem> = memory_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(memories)
    }

    async fn find_similar(
        &self,
        memory: &MemoryItem,
        threshold: f32,
    ) -> Result<Vec<MemoryItem>> {
        // Simple similarity based on content matching
        // In a real implementation, you'd use embeddings and vector similarity
        let query = MemoryQuery {
            query_text: String::new(),
            memory_types: vec![],
            tags: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(50),
        };

        let all_memories = self.search(query).await?;
        let mut similar = Vec::new();

        for candidate in all_memories {
            if candidate.memory_id != memory.memory_id {
                let similarity = calculate_simple_similarity(&memory.content, &candidate.content);
                if similarity >= threshold {
                    similar.push(candidate);
                }
            }
        }

        Ok(similar)
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryItem>> {
        let query = MemoryQuery {
            query_text: String::new(),
            memory_types: vec![],
            tags: vec![],
            time_range: None,
            importance_threshold: None,
            limit: Some(limit),
        };
        self.search(query).await
    }

    async fn get_by_importance(&self, min_importance: f32, limit: usize) -> Result<Vec<MemoryItem>> {
        let query = MemoryQuery {
            query_text: String::new(),
            memory_types: vec![],
            tags: vec![],
            time_range: None,
            importance_threshold: Some(min_importance as f64),
            limit: Some(limit),
        };
        self.search(query).await
    }

    async fn cleanup_old_memories(&self, days: u32) -> Result<usize> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow!("Failed to acquire database lock: {}", e))?;

        let count = conn.execute(
            "DELETE FROM memory_items WHERE created_at < ?1",
            rusqlite::params![cutoff_date.to_rfc3339()],
        )?;

        Ok(count)
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
        let store = AsyncSqliteMemoryStore::new(":memory:").await.expect("Failed to create memory store");

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

        // Test retrieving memory by ID
        let retrieved = store.retrieve("test-memory").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_item = retrieved.unwrap();
        assert_eq!(retrieved_item.memory_id, "test-memory");
        assert_eq!(retrieved_item.content, "Test memory content");
    }

    #[tokio::test]
    async fn test_sqlite_memory_store_creation() {
        let store = AsyncSqliteMemoryStore::new(":memory:").await;
        assert!(store.is_ok());
    }

    #[tokio::test]
    async fn test_async_sqlite_memory_store() {
        let store = AsyncSqliteMemoryStore::new(":memory:").await.expect("Failed to create memory store");

        let memory = MemoryItem {
            memory_id: "test-async-memory".to_string(),
            memory_type: MemoryType::Experience,
            content: "Test async memory content".to_string(),
            metadata: HashMap::new(),
            importance: 0.8,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec!["test".to_string(), "async".to_string()],
            embedding: None,
        };

        // Test storing memory
        let stored_id = store
            .store(memory.clone())
            .await
            .expect("Failed to store memory");
        assert_eq!(stored_id, "test-async-memory");

        // Test retrieving memory by ID
        let retrieved = store
            .retrieve("test-async-memory")
            .await
            .expect("Failed to retrieve memory");
        assert!(retrieved.is_some());
        let retrieved_item = retrieved.unwrap();
        assert_eq!(retrieved_item.memory_id, "test-async-memory");
        assert_eq!(retrieved_item.content, "Test async memory content");

        // Test searching memories
        let query = MemoryQuery {
            query_text: "test".to_string(),
            memory_types: vec![MemoryType::Experience],
            time_range: None,
            importance_threshold: Some(0.5),
            limit: Some(10),
            tags: vec!["async".to_string()],
        };

        let search_results = store
            .search(query)
            .await
            .expect("Failed to search memories");
        assert!(!search_results.is_empty());
        assert_eq!(search_results[0].memory_id, "test-async-memory");
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let store = AsyncSqliteMemoryStore::new(":memory:").await.expect("Failed to create memory store");

        // Create test memories
        let memories = vec![
            MemoryItem {
                memory_id: "batch-1".to_string(),
                memory_type: MemoryType::Experience,
                content: "Batch memory 1".to_string(),
                metadata: HashMap::new(),
                importance: 0.7,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec!["batch".to_string()],
                embedding: None,
            },
            MemoryItem {
                memory_id: "batch-2".to_string(),
                memory_type: MemoryType::Learning,
                content: "Batch memory 2".to_string(),
                metadata: HashMap::new(),
                importance: 0.8,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec!["batch".to_string()],
                embedding: None,
            },
        ];

        // Test individual store operations
        let id1 = store.store(memories[0].clone()).await.expect("Failed to store memory 1");
        let id2 = store.store(memories[1].clone()).await.expect("Failed to store memory 2");
        assert_eq!(id1, "batch-1");
        assert_eq!(id2, "batch-2");

        // Test individual delete operations
        store.delete(&id1).await.expect("Failed to delete memory 1");
        store.delete(&id2).await.expect("Failed to delete memory 2");

        // Verify deletion
        let query = MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(10),
            tags: vec!["batch".to_string()],
        };

        let retrieved = store
            .search(query)
            .await
            .expect("Failed to search memories");
        assert_eq!(retrieved.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_store_functionality() {
        let store = AsyncSqliteMemoryStore::new(":memory:").await.expect("Failed to create memory store");

        // Test basic functionality
        let memory = MemoryItem {
            memory_id: "functionality-test".to_string(),
            memory_type: MemoryType::Experience,
            content: "Test functionality content".to_string(),
            metadata: HashMap::new(),
            importance: 0.8,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec!["functionality".to_string()],
            embedding: None,
        };

        let stored_id = store.store(memory).await.expect("Failed to store memory");
        assert_eq!(stored_id, "functionality-test");
    }

    #[tokio::test]
    async fn test_memory_store_queries() {
        let store = AsyncSqliteMemoryStore::new(":memory:").await.expect("Failed to create memory store");

        // Test that we can store and retrieve memories
        let memory = MemoryItem {
            memory_id: "query-test".to_string(),
            memory_type: MemoryType::Experience,
            content: "Query test content".to_string(),
            metadata: HashMap::new(),
            importance: 0.7,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec!["query".to_string()],
            embedding: None,
        };

        let stored_id = store.store(memory).await.expect("Failed to store memory");
        assert_eq!(stored_id, "query-test");

        // Test searching memories
        let query = MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![MemoryType::Experience],
            time_range: None,
            importance_threshold: Some(0.5),
            limit: Some(10),
            tags: vec![],
        };

        let search_results = store.search(query).await.expect("Failed to search memories");
        assert!(!search_results.is_empty());
        assert_eq!(search_results[0].memory_id, "query-test");
    }
}
