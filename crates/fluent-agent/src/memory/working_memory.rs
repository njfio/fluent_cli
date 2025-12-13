//! Working Memory System with Attention Mechanisms
//!
//! This module implements an advanced working memory system that manages
//! information for long-running autonomous tasks with attention mechanisms,
//! relevance scoring, and intelligent memory management.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::context::ExecutionContext;

/// Maximum size for focus history to prevent unbounded growth
const MAX_FOCUS_HISTORY_SIZE: usize = 1000;

/// Maximum size for access log (already enforced, kept for documentation)
const MAX_ACCESS_LOG_SIZE: usize = 10000;

/// Maximum size for pressure history to prevent unbounded growth
const MAX_PRESSURE_HISTORY_SIZE: usize = 500;

/// Working memory system with attention and relevance mechanisms
pub struct WorkingMemory {
    config: WorkingMemoryConfig,
    attention_system: Arc<RwLock<AttentionSystem>>,
    memory_store: Arc<RwLock<MemoryStore>>,
    relevance_engine: Arc<RwLock<RelevanceEngine>>,
    capacity_manager: Arc<RwLock<CapacityManager>>,
}

/// Configuration for working memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryConfig {
    /// Maximum number of items in active memory
    pub max_active_items: usize,
    /// Maximum memory size in bytes
    pub max_memory_size: usize,
    /// Attention refresh interval in seconds
    pub attention_refresh_interval: u64,
    /// Relevance decay rate (0.0-1.0 per hour)
    pub relevance_decay_rate: f64,
    /// Enable automatic memory consolidation
    pub enable_consolidation: bool,
    /// Consolidation threshold (0.0-1.0)
    pub consolidation_threshold: f64,
    /// Enable predictive loading
    pub enable_predictive_loading: bool,
}

impl Default for WorkingMemoryConfig {
    fn default() -> Self {
        Self {
            max_active_items: 500,
            max_memory_size: 50 * 1024 * 1024, // 50 MB
            attention_refresh_interval: 30,
            relevance_decay_rate: 0.1,
            enable_consolidation: true,
            consolidation_threshold: 0.3,
            enable_predictive_loading: true,
        }
    }
}

/// Attention system for managing focus and prioritization
#[derive(Debug, Default)]
pub struct AttentionSystem {
    attention_weights: HashMap<String, AttentionWeight>,
    focus_history: VecDeque<FocusEvent>,
    attention_patterns: Vec<AttentionPattern>,
    current_focus: Option<String>,
    attention_capacity: f64,
}

/// Weight assigned to different memory items based on attention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionWeight {
    pub item_id: String,
    pub weight: f64,
    pub last_accessed: SystemTime,
    pub access_frequency: u32,
    pub importance_score: f64,
    pub context_relevance: f64,
    pub temporal_relevance: f64,
}

/// Event tracking attention focus changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusEvent {
    pub event_id: String,
    pub timestamp: SystemTime,
    pub focus_target: String,
    pub focus_duration: Duration,
    pub trigger_reason: String,
    pub attention_strength: f64,
}

/// Pattern in attention behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionPattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub trigger_conditions: Vec<String>,
    pub focus_targets: Vec<String>,
    pub strength: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    SequentialFocus,
    ParallelAttention,
    CyclicPattern,
    ConditionalFocus,
    EmergencyFocus,
}

/// Memory store for working memory items
#[derive(Debug, Default)]
pub struct MemoryStore {
    active_items: HashMap<String, MemoryItem>,
    archived_items: HashMap<String, ArchivedItem>,
    item_relationships: HashMap<String, Vec<String>>,
    access_log: VecDeque<AccessEvent>,
    memory_usage: MemoryUsageStats,
}

/// Item stored in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub item_id: String,
    pub content: MemoryContent,
    pub metadata: ItemMetadata,
    pub relevance_score: f64,
    pub attention_weight: f64,
    pub last_accessed: SystemTime,
    pub created_at: SystemTime,
    pub access_count: u32,
    pub consolidation_level: u32,
}

/// Content stored in memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    pub content_type: ContentType,
    pub data: Vec<u8>,
    pub text_summary: String,
    pub key_concepts: Vec<String>,
    pub relationships: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    TaskResult,
    ContextInformation,
    ReasoningStep,
    DecisionPoint,
    ErrorInfo,
    LearningItem,
    ReferenceData,
}

/// Metadata for memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub tags: Vec<String>,
    pub priority: Priority,
    pub source: String,
    pub size_bytes: usize,
    pub compression_ratio: f64,
    pub retention_policy: RetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
    Archive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetentionPolicy {
    Permanent,
    ContextBased,
    TimeBased(Duration),
    AccessBased(u32),
    ConditionalRetention(String),
}

/// Archived memory item with compressed storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivedItem {
    pub item_id: String,
    pub compressed_content: Vec<u8>,
    pub summary: String,
    pub archived_at: SystemTime,
    pub original_size: usize,
    pub access_frequency: f64,
}

/// Event tracking memory access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessEvent {
    pub event_id: String,
    pub timestamp: SystemTime,
    pub item_id: String,
    pub access_type: AccessType,
    pub context: String,
    pub relevance_boost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessType {
    Read,
    Write,
    Update,
    Delete,
    Search,
    Consolidate,
}

/// Memory usage statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MemoryUsageStats {
    pub total_items: usize,
    pub active_items: usize,
    pub archived_items: usize,
    pub total_size_bytes: usize,
    pub active_size_bytes: usize,
    pub archived_size_bytes: usize,
    pub compression_ratio: f64,
    pub fragmentation: f64,
}

/// Relevance engine for scoring memory item importance
#[derive(Debug, Default)]
pub struct RelevanceEngine {
    scoring_models: Vec<ScoringModel>,
    relevance_cache: HashMap<String, RelevanceScore>,
    context_vectors: HashMap<String, Vec<f64>>,
    temporal_weights: BTreeMap<SystemTime, f64>,
}

/// Model for scoring relevance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringModel {
    pub model_id: String,
    pub model_type: ScoringType,
    pub weight: f64,
    pub parameters: HashMap<String, f64>,
    pub performance_metrics: ModelMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoringType {
    TextualSimilarity,
    TemporalRelevance,
    ContextualFit,
    AccessFrequency,
    ConceptualDistance,
    TaskRelevance,
}

/// Performance metrics for scoring models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub latency: Duration,
}

/// Relevance score for memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScore {
    pub item_id: String,
    pub overall_score: f64,
    pub component_scores: HashMap<String, f64>,
    pub confidence: f64,
    pub calculated_at: SystemTime,
    pub context_signature: String,
}

/// Capacity manager for memory optimization
#[derive(Debug, Default)]
pub struct CapacityManager {
    current_usage: MemoryUsageStats,
    capacity_limits: CapacityLimits,
    optimization_strategies: Vec<OptimizationStrategy>,
    pressure_history: VecDeque<MemoryPressure>,
}

/// Memory capacity limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityLimits {
    pub max_items: usize,
    pub max_size_bytes: usize,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub consolidation_trigger: f64,
}

impl Default for CapacityLimits {
    fn default() -> Self {
        Self {
            max_items: 1000,
            max_size_bytes: 100 * 1024 * 1024, // 100 MB
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            consolidation_trigger: 0.7,
        }
    }
}

/// Strategy for optimizing memory usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStrategy {
    pub strategy_id: String,
    pub strategy_type: OptimizationType,
    pub trigger_conditions: Vec<String>,
    pub effectiveness: f64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    LRUEviction,
    RelevanceEviction,
    Compression,
    Consolidation,
    Archival,
    SelectiveDeletion,
}

/// Memory pressure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPressure {
    pub timestamp: SystemTime,
    pub usage_ratio: f64,
    pub item_count_ratio: f64,
    pub pressure_level: PressureLevel,
    pub contributing_factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PressureLevel {
    Low,
    Moderate,
    High,
    Critical,
}

impl WorkingMemory {
    /// Create a new working memory system
    pub fn new(config: WorkingMemoryConfig) -> Self {
        Self {
            config,
            attention_system: Arc::new(RwLock::new(AttentionSystem::default())),
            memory_store: Arc::new(RwLock::new(MemoryStore::default())),
            relevance_engine: Arc::new(RwLock::new(RelevanceEngine::default())),
            capacity_manager: Arc::new(RwLock::new(CapacityManager::default())),
        }
    }

    /// Store a new item in working memory
    pub async fn store_item(
        &self,
        content: MemoryContent,
        metadata: ItemMetadata,
    ) -> Result<String> {
        let item_id = Uuid::new_v4().to_string();

        // Calculate initial relevance score
        let relevance_score = self.calculate_relevance(&content, &metadata).await?;

        // Check capacity and optimize if needed
        self.ensure_capacity().await?;

        let item = MemoryItem {
            item_id: item_id.clone(),
            content,
            metadata,
            relevance_score,
            attention_weight: 1.0,
            last_accessed: SystemTime::now(),
            created_at: SystemTime::now(),
            access_count: 0,
            consolidation_level: 0,
        };

        // Store item
        let mut store = self.memory_store.write().await;
        store.active_items.insert(item_id.clone(), item);

        // Update attention system
        self.update_attention_for_new_item(&item_id).await?;

        // Log access
        self.log_access(&item_id, AccessType::Write, "Initial storage")
            .await?;

        Ok(item_id)
    }

    /// Retrieve an item from working memory
    pub async fn retrieve_item(&self, item_id: &str) -> Result<Option<MemoryItem>> {
        let mut store = self.memory_store.write().await;

        if let Some(mut item) = store.active_items.get(item_id).cloned() {
            // Update access statistics
            item.last_accessed = SystemTime::now();
            item.access_count += 1;

            // Update relevance based on access
            item.relevance_score = self.update_relevance_on_access(&item).await?;

            // Store updated item
            store.active_items.insert(item_id.to_string(), item.clone());

            // Update attention
            self.update_attention_on_access(item_id).await?;

            // Log access
            drop(store);
            self.log_access(item_id, AccessType::Read, "Item retrieval")
                .await?;

            Ok(Some(item))
        } else {
            // Check if item is archived
            self.retrieve_from_archive(item_id).await
        }
    }

    /// Search for relevant items based on query
    pub async fn search_relevant(
        &self,
        query: &str,
        max_results: usize,
    ) -> Result<Vec<MemoryItem>> {
        let store = self.memory_store.read().await;
        let candidates: Vec<(&String, &MemoryItem)> = store.active_items.iter().collect();

        // Score items based on query relevance
        let mut scored_items = Vec::new();
        for (_id, item) in candidates {
            let score = self.calculate_query_relevance(item, query).await?;
            scored_items.push((score, item.clone()));
        }

        // Sort by relevance score
        scored_items.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Update attention for accessed items
        for (_, item) in scored_items.iter().take(max_results) {
            self.update_attention_on_access(&item.item_id).await?;
        }

        Ok(scored_items
            .into_iter()
            .take(max_results)
            .map(|(_, item)| item)
            .collect())
    }

    /// Update attention based on current context
    pub async fn update_attention(&self, context: &ExecutionContext) -> Result<()> {
        let mut attention = self.attention_system.write().await;

        // Extract current focus from context
        let current_focus = context
            .current_goal
            .as_ref()
            .map(|g| g.description.clone())
            .unwrap_or_else(|| "general".to_string());

        // Update attention weights based on context relevance
        let store = self.memory_store.read().await;
        for (item_id, item) in &store.active_items {
            let context_relevance = self.calculate_context_relevance(item, context).await?;

            let attention_weight = AttentionWeight {
                item_id: item_id.clone(),
                weight: context_relevance,
                last_accessed: item.last_accessed,
                access_frequency: item.access_count,
                importance_score: item.relevance_score,
                context_relevance,
                temporal_relevance: self.calculate_temporal_relevance(item.created_at).await?,
            };

            attention
                .attention_weights
                .insert(item_id.clone(), attention_weight);
        }

        // Update current focus
        attention.current_focus = Some(current_focus);

        Ok(())
    }

    /// Perform memory consolidation
    pub async fn consolidate_memory(&self) -> Result<ConsolidationResult> {
        if !self.config.enable_consolidation {
            return Ok(ConsolidationResult::default());
        }

        let mut consolidated_items = 0;
        let mut archived_items = 0;
        let mut deleted_items = 0;

        // Identify items for consolidation
        let consolidation_candidates = self.identify_consolidation_candidates().await?;

        for item_id in consolidation_candidates {
            let consolidation_action = self.determine_consolidation_action(&item_id).await?;

            match consolidation_action {
                ConsolidationAction::Compress => {
                    self.compress_item(&item_id).await?;
                    consolidated_items += 1;
                }
                ConsolidationAction::Archive => {
                    self.archive_item(&item_id).await?;
                    archived_items += 1;
                }
                ConsolidationAction::Delete => {
                    self.delete_item(&item_id).await?;
                    deleted_items += 1;
                }
                ConsolidationAction::Keep => {
                    // No action needed
                }
            }
        }

        Ok(ConsolidationResult {
            consolidated_items,
            archived_items,
            deleted_items,
            memory_freed: 0, // Would calculate actual memory freed
        })
    }

    // Helper methods (simplified implementations)

    async fn calculate_relevance(
        &self,
        content: &MemoryContent,
        _metadata: &ItemMetadata,
    ) -> Result<f64> {
        // Simplified relevance calculation based on content type
        let base_score = match content.content_type {
            ContentType::TaskResult => 0.8,
            ContentType::ContextInformation => 0.6,
            ContentType::ReasoningStep => 0.7,
            ContentType::DecisionPoint => 0.9,
            ContentType::ErrorInfo => 0.5,
            ContentType::LearningItem => 0.8,
            ContentType::ReferenceData => 0.4,
        };

        Ok(base_score)
    }

    async fn ensure_capacity(&self) -> Result<()> {
        let manager = self.capacity_manager.read().await;
        let usage = &manager.current_usage;

        let usage_ratio =
            usage.active_size_bytes as f64 / manager.capacity_limits.max_size_bytes as f64;

        if usage_ratio > manager.capacity_limits.warning_threshold {
            drop(manager);
            self.optimize_memory().await?;
        }

        Ok(())
    }

    async fn optimize_memory(&self) -> Result<()> {
        // Simple LRU eviction for now
        let store = self.memory_store.read().await;
        let mut items_by_access: Vec<_> = store
            .active_items
            .iter()
            .map(|(id, item)| (item.last_accessed, id.clone()))
            .collect();

        items_by_access.sort_by_key(|(time, _)| *time);

        // Remove oldest 10% of items
        let items_to_remove = items_by_access.len() / 10;
        drop(store);

        for (_, item_id) in items_by_access.into_iter().take(items_to_remove) {
            self.archive_item(&item_id).await?;
        }

        Ok(())
    }

    async fn update_attention_for_new_item(&self, item_id: &str) -> Result<()> {
        let mut attention = self.attention_system.write().await;

        attention.attention_weights.insert(
            item_id.to_string(),
            AttentionWeight {
                item_id: item_id.to_string(),
                weight: 1.0, // New items get full attention initially
                last_accessed: SystemTime::now(),
                access_frequency: 1,
                importance_score: 0.8,
                context_relevance: 0.8,
                temporal_relevance: 1.0,
            },
        );

        Ok(())
    }

    async fn update_attention_on_access(&self, item_id: &str) -> Result<()> {
        let mut attention = self.attention_system.write().await;

        if let Some(weight) = attention.attention_weights.get_mut(item_id) {
            weight.access_frequency += 1;
            weight.last_accessed = SystemTime::now();
            weight.weight = (weight.weight * 0.9 + 0.1).min(1.0); // Boost attention
        }

        Ok(())
    }

    async fn log_access(
        &self,
        item_id: &str,
        access_type: AccessType,
        context: &str,
    ) -> Result<()> {
        let mut store = self.memory_store.write().await;

        store.access_log.push_back(AccessEvent {
            event_id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            item_id: item_id.to_string(),
            access_type,
            context: context.to_string(),
            relevance_boost: 0.1,
        });

        // Keep only recent access events (enforce memory bounds)
        while store.access_log.len() > MAX_ACCESS_LOG_SIZE {
            store.access_log.pop_front();
        }

        Ok(())
    }

    async fn update_relevance_on_access(&self, item: &MemoryItem) -> Result<f64> {
        // Simple relevance boost on access
        Ok((item.relevance_score * 0.95 + 0.05).min(1.0))
    }

    async fn retrieve_from_archive(&self, item_id: &str) -> Result<Option<MemoryItem>> {
        let store = self.memory_store.read().await;

        if let Some(archived) = store.archived_items.get(item_id) {
            // Would decompress and restore item
            // For now, return a placeholder
            Ok(Some(MemoryItem {
                item_id: archived.item_id.clone(),
                content: MemoryContent {
                    content_type: ContentType::ReferenceData,
                    data: Vec::new(),
                    text_summary: archived.summary.clone(),
                    key_concepts: Vec::new(),
                    relationships: Vec::new(),
                },
                metadata: ItemMetadata {
                    tags: Vec::new(),
                    priority: Priority::Archive,
                    source: "archive".to_string(),
                    size_bytes: archived.original_size,
                    compression_ratio: 0.5,
                    retention_policy: RetentionPolicy::Permanent,
                },
                relevance_score: 0.3,
                attention_weight: 0.1,
                last_accessed: SystemTime::now(),
                created_at: archived.archived_at,
                access_count: 0,
                consolidation_level: 1,
            }))
        } else {
            Ok(None)
        }
    }

    async fn calculate_query_relevance(&self, item: &MemoryItem, query: &str) -> Result<f64> {
        // Simple text matching for now
        let summary = &item.content.text_summary.to_lowercase();
        let query_lower = query.to_lowercase();

        let mut score = 0.0;

        // Check for exact matches
        if summary.contains(&query_lower) {
            score += 0.5;
        }

        // Check for word matches
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        for word in query_words {
            if summary.contains(word) {
                score += 0.1;
            }
        }

        // Factor in existing relevance
        score = (score + item.relevance_score) / 2.0;

        Ok(score.min(1.0))
    }

    async fn calculate_context_relevance(
        &self,
        item: &MemoryItem,
        context: &ExecutionContext,
    ) -> Result<f64> {
        // Simple context relevance based on current goal
        if let Some(goal) = &context.current_goal {
            let goal_description_lower = goal.description.to_lowercase();
            let goal_words: Vec<&str> = goal_description_lower.split_whitespace().collect();
            let summary = item.content.text_summary.to_lowercase();

            let mut matches = 0;
            for word in goal_words {
                if summary.contains(word) {
                    matches += 1;
                }
            }

            Ok((matches as f64 * 0.2).min(1.0))
        } else {
            Ok(0.5) // Neutral relevance
        }
    }

    async fn calculate_temporal_relevance(&self, created_at: SystemTime) -> Result<f64> {
        let age = SystemTime::now()
            .duration_since(created_at)
            .unwrap_or_default();
        let age_hours = age.as_secs() as f64 / 3600.0;

        // Exponential decay based on age
        let relevance = (-age_hours * self.config.relevance_decay_rate).exp();
        Ok(relevance.max(0.1)) // Minimum relevance threshold
    }

    async fn identify_consolidation_candidates(&self) -> Result<Vec<String>> {
        let store = self.memory_store.read().await;
        let attention = self.attention_system.read().await;

        let mut candidates = Vec::new();

        for (item_id, item) in &store.active_items {
            let attention_weight = attention
                .attention_weights
                .get(item_id)
                .map(|w| w.weight)
                .unwrap_or(0.5);

            if item.relevance_score < self.config.consolidation_threshold
                && attention_weight < self.config.consolidation_threshold
            {
                candidates.push(item_id.clone());
            }
        }

        Ok(candidates)
    }

    async fn determine_consolidation_action(&self, _item_id: &str) -> Result<ConsolidationAction> {
        // Simple heuristic for now
        Ok(ConsolidationAction::Archive)
    }

    async fn compress_item(&self, _item_id: &str) -> Result<()> {
        // Would implement compression algorithm
        Ok(())
    }

    async fn archive_item(&self, item_id: &str) -> Result<()> {
        let mut store = self.memory_store.write().await;

        if let Some(item) = store.active_items.remove(item_id) {
            let archived = ArchivedItem {
                item_id: item.item_id,
                compressed_content: item.content.data, // Would compress
                summary: item.content.text_summary,
                archived_at: SystemTime::now(),
                original_size: item.metadata.size_bytes,
                access_frequency: item.access_count as f64,
            };

            store.archived_items.insert(item_id.to_string(), archived);
        }

        Ok(())
    }

    async fn delete_item(&self, item_id: &str) -> Result<()> {
        let mut store = self.memory_store.write().await;
        store.active_items.remove(item_id);
        store.archived_items.remove(item_id);
        Ok(())
    }

    /// Get memory usage statistics
    pub async fn get_stats(&self) -> MemoryUsageStats {
        let store = self.memory_store.read().await;
        store.memory_usage.clone()
    }
}

/// Action to take during consolidation
#[derive(Debug, Clone)]
pub enum ConsolidationAction {
    Compress,
    Archive,
    Delete,
    Keep,
}

/// Result of memory consolidation
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConsolidationResult {
    pub consolidated_items: u32,
    pub archived_items: u32,
    pub deleted_items: u32,
    pub memory_freed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{Goal, GoalPriority, GoalType};
    use std::collections::HashMap;

    fn create_test_context() -> ExecutionContext {
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal for memory operations".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::High,
            success_criteria: vec!["Test success".to_string()],
            max_iterations: Some(10),
            timeout: None,
            metadata: HashMap::new(),
        };
        ExecutionContext::new(goal)
    }

    fn create_test_memory_content(summary: &str) -> MemoryContent {
        MemoryContent {
            content_type: ContentType::TaskResult,
            data: summary.as_bytes().to_vec(),
            text_summary: summary.to_string(),
            key_concepts: vec!["test".to_string(), "memory".to_string()],
            relationships: Vec::new(),
        }
    }

    fn create_test_metadata(priority: Priority) -> ItemMetadata {
        ItemMetadata {
            tags: vec!["test".to_string()],
            priority,
            source: "test_source".to_string(),
            size_bytes: 100,
            compression_ratio: 1.0,
            retention_policy: RetentionPolicy::ContextBased,
        }
    }

    #[tokio::test]
    async fn test_working_memory_creation() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        // Verify memory was created successfully
        let store = memory.memory_store.read().await;
        assert_eq!(store.active_items.len(), 0);
        assert_eq!(store.archived_items.len(), 0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_item() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test content for memory storage");
        let metadata = create_test_metadata(Priority::High);

        // Store item
        let item_id = memory.store_item(content.clone(), metadata).await.unwrap();
        assert!(!item_id.is_empty());

        // Retrieve item
        let retrieved = memory.retrieve_item(&item_id).await.unwrap();
        assert!(retrieved.is_some());

        let item = retrieved.unwrap();
        assert_eq!(item.item_id, item_id);
        assert_eq!(item.content.text_summary, "Test content for memory storage");
        assert_eq!(item.access_count, 1); // Access count should be incremented
    }

    #[tokio::test]
    async fn test_retrieve_nonexistent_item() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let result = memory.retrieve_item("nonexistent-id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_multiple_item_storage() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let mut item_ids = Vec::new();

        // Store multiple items
        for i in 0..5 {
            let content = create_test_memory_content(&format!("Test content {}", i));
            let metadata = create_test_metadata(Priority::Medium);
            let item_id = memory.store_item(content, metadata).await.unwrap();
            item_ids.push(item_id);
        }

        // Verify all items are stored
        let store = memory.memory_store.read().await;
        assert_eq!(store.active_items.len(), 5);

        // Retrieve each item
        drop(store);
        for (i, item_id) in item_ids.iter().enumerate() {
            let retrieved = memory.retrieve_item(item_id).await.unwrap();
            assert!(retrieved.is_some());
            let item = retrieved.unwrap();
            assert_eq!(item.content.text_summary, format!("Test content {}", i));
        }
    }

    #[tokio::test]
    async fn test_access_count_increments() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test content");
        let metadata = create_test_metadata(Priority::High);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        // Access the item multiple times
        for _ in 0..3 {
            memory.retrieve_item(&item_id).await.unwrap();
        }

        // Check access count
        let retrieved = memory.retrieve_item(&item_id).await.unwrap();
        let item = retrieved.unwrap();
        assert_eq!(item.access_count, 4); // 3 accesses + 1 final retrieval
    }

    #[tokio::test]
    async fn test_relevance_scoring() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        // Store items with different content types
        let content_types = vec![
            ContentType::TaskResult,
            ContentType::ContextInformation,
            ContentType::DecisionPoint,
            ContentType::ErrorInfo,
        ];

        let mut item_ids = Vec::new();
        for content_type in content_types {
            let content = MemoryContent {
                content_type: content_type.clone(),
                data: vec![1, 2, 3],
                text_summary: "Test".to_string(),
                key_concepts: Vec::new(),
                relationships: Vec::new(),
            };
            let metadata = create_test_metadata(Priority::Medium);
            let item_id = memory.store_item(content, metadata).await.unwrap();
            item_ids.push(item_id);
        }

        // Verify items have different relevance scores based on content type
        let store = memory.memory_store.read().await;
        for item_id in &item_ids {
            let item = store.active_items.get(item_id).unwrap();
            assert!(item.relevance_score > 0.0);
            assert!(item.relevance_score <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_search_relevant_items() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        // Store items with different summaries
        let summaries = vec![
            "This is about Rust programming",
            "This is about Python programming",
            "This is about memory management",
            "This is about database queries",
        ];

        for summary in summaries {
            let content = create_test_memory_content(summary);
            let metadata = create_test_metadata(Priority::Medium);
            memory.store_item(content, metadata).await.unwrap();
        }

        // Search for items related to "programming"
        let results = memory.search_relevant("programming", 10).await.unwrap();

        assert!(results.len() >= 2); // Should find at least Rust and Python items
        for item in &results {
            assert!(item
                .content
                .text_summary
                .to_lowercase()
                .contains("programming"));
        }
    }

    #[tokio::test]
    async fn test_search_with_max_results() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        // Store 10 items
        for i in 0..10 {
            let content = create_test_memory_content(&format!("Test item {}", i));
            let metadata = create_test_metadata(Priority::Medium);
            memory.store_item(content, metadata).await.unwrap();
        }

        // Search with max_results = 3
        let results = memory.search_relevant("Test", 3).await.unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_attention_update() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test content");
        let metadata = create_test_metadata(Priority::High);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        let context = create_test_context();

        // Update attention based on context
        memory.update_attention(&context).await.unwrap();

        // Verify attention weights were updated
        let attention = memory.attention_system.read().await;
        assert!(attention.attention_weights.contains_key(&item_id));
        assert!(attention.current_focus.is_some());
    }

    #[tokio::test]
    async fn test_memory_consolidation() {
        let config = WorkingMemoryConfig {
            enable_consolidation: true,
            consolidation_threshold: 0.3,
            ..Default::default()
        };
        let memory = WorkingMemory::new(config);

        // Store items with low relevance
        for i in 0..5 {
            let content = create_test_memory_content(&format!("Low priority item {}", i));
            let metadata = ItemMetadata {
                tags: vec!["test".to_string()],
                priority: Priority::Low,
                source: "test".to_string(),
                size_bytes: 100,
                compression_ratio: 1.0,
                retention_policy: RetentionPolicy::ContextBased,
            };
            memory.store_item(content, metadata).await.unwrap();
        }

        // Perform consolidation
        let result = memory.consolidate_memory().await.unwrap();

        // Some items should be consolidated or archived
        assert!(
            result.consolidated_items > 0 || result.archived_items > 0 || result.deleted_items > 0
        );
    }

    #[tokio::test]
    async fn test_consolidation_disabled() {
        let config = WorkingMemoryConfig {
            enable_consolidation: false,
            ..Default::default()
        };
        let memory = WorkingMemory::new(config);

        // Store some items
        for i in 0..3 {
            let content = create_test_memory_content(&format!("Item {}", i));
            let metadata = create_test_metadata(Priority::Low);
            memory.store_item(content, metadata).await.unwrap();
        }

        // Perform consolidation (should do nothing)
        let result = memory.consolidate_memory().await.unwrap();

        assert_eq!(result.consolidated_items, 0);
        assert_eq!(result.archived_items, 0);
        assert_eq!(result.deleted_items, 0);
    }

    #[tokio::test]
    async fn test_item_archival() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test content");
        let metadata = create_test_metadata(Priority::Low);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        // Archive the item
        memory.archive_item(&item_id).await.unwrap();

        // Verify item is no longer in active memory
        let store = memory.memory_store.read().await;
        assert!(!store.active_items.contains_key(&item_id));
        assert!(store.archived_items.contains_key(&item_id));
    }

    #[tokio::test]
    async fn test_retrieve_from_archive() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Archived content");
        let metadata = create_test_metadata(Priority::Low);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        // Archive the item
        memory.archive_item(&item_id).await.unwrap();

        // Retrieve from archive
        let retrieved = memory.retrieve_item(&item_id).await.unwrap();
        assert!(retrieved.is_some());

        let item = retrieved.unwrap();
        assert_eq!(item.item_id, item_id);
        assert_eq!(item.metadata.priority, Priority::Archive);
    }

    #[tokio::test]
    async fn test_delete_item() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test content");
        let metadata = create_test_metadata(Priority::Low);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        // Delete the item
        memory.delete_item(&item_id).await.unwrap();

        // Verify item is deleted
        let store = memory.memory_store.read().await;
        assert!(!store.active_items.contains_key(&item_id));
        assert!(!store.archived_items.contains_key(&item_id));

        // Trying to retrieve should return None
        drop(store);
        let retrieved = memory.retrieve_item(&item_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_attention_weight_updates() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test content");
        let metadata = create_test_metadata(Priority::High);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        // Access the item multiple times
        for _ in 0..5 {
            memory.update_attention_on_access(&item_id).await.unwrap();
        }

        // Check attention weight
        let attention = memory.attention_system.read().await;
        let weight = attention.attention_weights.get(&item_id).unwrap();
        assert!(weight.access_frequency >= 5);
        assert!(weight.weight > 0.0);
    }

    #[tokio::test]
    async fn test_temporal_relevance_decay() {
        let config = WorkingMemoryConfig {
            relevance_decay_rate: 0.1,
            ..Default::default()
        };
        let memory = WorkingMemory::new(config);

        // Calculate temporal relevance for different ages
        let now = SystemTime::now();
        let recent = now;
        let old = now - Duration::from_secs(3600 * 24); // 24 hours ago

        let recent_relevance = memory.calculate_temporal_relevance(recent).await.unwrap();
        let old_relevance = memory.calculate_temporal_relevance(old).await.unwrap();

        // Recent items should have higher temporal relevance
        assert!(recent_relevance > old_relevance);
        assert!(recent_relevance <= 1.0);
        assert!(old_relevance >= 0.1); // Minimum threshold
    }

    #[tokio::test]
    async fn test_context_relevance_calculation() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test goal for memory operations analysis");
        let metadata = create_test_metadata(Priority::High);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        let context = create_test_context();

        // Retrieve item to get updated relevance
        let item = memory.retrieve_item(&item_id).await.unwrap().unwrap();

        let relevance = memory
            .calculate_context_relevance(&item, &context)
            .await
            .unwrap();

        // Should have some relevance due to matching words
        assert!(relevance > 0.0);
        assert!(relevance <= 1.0);
    }

    #[tokio::test]
    async fn test_empty_memory_search() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        // Search in empty memory
        let results = memory.search_relevant("anything", 10).await.unwrap();

        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_capacity_management() {
        let config = WorkingMemoryConfig {
            max_active_items: 10,
            ..Default::default()
        };
        let memory = WorkingMemory::new(config);

        // Store many items to trigger capacity management
        for i in 0..15 {
            let content = create_test_memory_content(&format!("Item {}", i));
            let metadata = create_test_metadata(Priority::Medium);
            memory.store_item(content, metadata).await.unwrap();
        }

        // Memory should handle capacity limits gracefully
        let store = memory.memory_store.read().await;
        // Some items might be archived due to capacity management
        assert!(store.active_items.len() > 0);
    }

    #[tokio::test]
    async fn test_different_content_types() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content_types = vec![
            ContentType::TaskResult,
            ContentType::ContextInformation,
            ContentType::ReasoningStep,
            ContentType::DecisionPoint,
            ContentType::ErrorInfo,
            ContentType::LearningItem,
            ContentType::ReferenceData,
        ];

        for content_type in content_types {
            let content = MemoryContent {
                content_type: content_type.clone(),
                data: b"test data".to_vec(),
                text_summary: format!("Content of type {:?}", content_type),
                key_concepts: Vec::new(),
                relationships: Vec::new(),
            };
            let metadata = create_test_metadata(Priority::Medium);
            let item_id = memory.store_item(content, metadata).await.unwrap();

            // Verify item was stored
            let retrieved = memory.retrieve_item(&item_id).await.unwrap();
            assert!(retrieved.is_some());
        }
    }

    #[tokio::test]
    async fn test_different_priority_levels() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let priorities = vec![
            Priority::Critical,
            Priority::High,
            Priority::Medium,
            Priority::Low,
            Priority::Archive,
        ];

        for priority in priorities {
            let content =
                create_test_memory_content(&format!("Content with {:?} priority", priority));
            let metadata = create_test_metadata(priority.clone());
            let item_id = memory.store_item(content, metadata).await.unwrap();

            // Verify item was stored with correct priority
            let store = memory.memory_store.read().await;
            let item = store.active_items.get(&item_id).unwrap();
            assert_eq!(item.metadata.priority, priority);
            drop(store);
        }
    }

    #[tokio::test]
    async fn test_access_log_management() {
        let config = WorkingMemoryConfig::default();
        let memory = WorkingMemory::new(config);

        let content = create_test_memory_content("Test content");
        let metadata = create_test_metadata(Priority::High);
        let item_id = memory.store_item(content, metadata).await.unwrap();

        // Perform multiple accesses
        for _ in 0..5 {
            memory.retrieve_item(&item_id).await.unwrap();
        }

        // Verify access log contains events
        let store = memory.memory_store.read().await;
        assert!(store.access_log.len() > 0);
    }
}
