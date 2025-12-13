//! Advanced Memory Management System
//!
//! This module provides comprehensive memory management for long-running
//! autonomous tasks including working memory, context compression, and
//! cross-session persistence.

pub mod context_compressor;
pub mod cross_session_persistence;
pub mod enhanced_memory_system;
pub mod working_memory;

pub use context_compressor::{
    CompressedContext, CompressionResult, CompressorConfig, ContextCompressor, ContextSummary,
};
pub use cross_session_persistence::{
    CrossSessionPersistence, LearnedPattern, PersistenceConfig, SessionOutcome, SessionState,
    SessionType,
};
pub use enhanced_memory_system::{
    ConceptNode, EnhancedMemoryConfig, EnhancedMemorySystem, Episode, EpisodicMemory, MetaMemory,
    ProceduralMemory, RelevantMemories, SemanticMemory, Skill,
};
pub use working_memory::{
    AttentionSystem, ConsolidationResult, MemoryContent, MemoryItem, WorkingMemory,
    WorkingMemoryConfig,
};

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
// use std::time::SystemTime;
use tokio::sync::RwLock;

/// Backward compatibility types
pub type MemorySystem = IntegratedMemorySystem;

#[derive(Debug, Clone, Default)]
pub struct MemoryConfig {
    pub working_config: WorkingMemoryConfig,
    pub compressor_config: CompressorConfig,
    pub persistence_config: PersistenceConfig,
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub items_count: usize,
    pub memory_usage_bytes: usize,
    pub compression_ratio: f64,
    pub session_count: usize,
}

use crate::agent_with_mcp::LongTermMemory;
use crate::context::ExecutionContext;
use fluent_core::traits::Engine;

/// Integrated memory management system
#[derive(Clone)]
pub struct IntegratedMemorySystem {
    working_memory: Arc<RwLock<WorkingMemory>>,
    compressor: Arc<RwLock<ContextCompressor>>,
    persistence: Arc<RwLock<CrossSessionPersistence>>,
    engine: Arc<dyn Engine>,
}

impl IntegratedMemorySystem {
    /// Create new integrated memory system from components
    pub fn from_components(
        engine: Arc<dyn Engine>,
        working_config: WorkingMemoryConfig,
        compressor_config: CompressorConfig,
        persistence_config: PersistenceConfig,
    ) -> Self {
        let working_memory = WorkingMemory::new(working_config);
        let compressor = ContextCompressor::new(engine.clone(), compressor_config);
        let persistence = CrossSessionPersistence::new(persistence_config);

        Self {
            working_memory: Arc::new(RwLock::new(working_memory)),
            compressor: Arc::new(RwLock::new(compressor)),
            persistence: Arc::new(RwLock::new(persistence)),
            engine,
        }
    }

    /// Initialize the integrated memory system
    pub async fn initialize(&self) -> Result<()> {
        self.persistence.read().await.initialize().await?;
        Ok(())
    }

    /// Update memory systems with current context
    pub async fn update_memory(&self, context: &ExecutionContext) -> Result<()> {
        // Update working memory attention
        self.working_memory
            .read()
            .await
            .update_attention(context)
            .await?;

        // Save session state
        self.persistence
            .read()
            .await
            .save_session_state(context)
            .await?;

        // Perform memory consolidation if needed
        let _consolidation_result = self
            .working_memory
            .read()
            .await
            .consolidate_memory()
            .await?;

        Ok(())
    }

    /// Get relevant learned patterns
    pub async fn get_learned_patterns(
        &self,
        context: &ExecutionContext,
    ) -> Result<Vec<LearnedPattern>> {
        self.persistence
            .read()
            .await
            .get_relevant_patterns(context)
            .await
    }

    /// Create checkpoint for recovery
    pub async fn create_checkpoint(&self, context: &ExecutionContext) -> Result<String> {
        self.persistence
            .read()
            .await
            .create_checkpoint(
                cross_session_persistence::CheckpointType::Automatic,
                context,
            )
            .await
    }

    /// Get memory statistics
    pub async fn get_stats(&self) -> Result<MemoryStats> {
        // Get actual counts from working memory store
        let working_mem = self.working_memory.read().await;
        let working_stats = working_mem.get_stats().await;

        // Get session count from persistence layer
        let persistence = self.persistence.read().await;
        let session_count = persistence.get_session_count().await.unwrap_or(1);

        Ok(MemoryStats {
            items_count: working_stats.total_items,
            memory_usage_bytes: working_stats.total_size_bytes,
            compression_ratio: working_stats.compression_ratio,
            session_count,
        })
    }
}

/// MemorySystem implementation for backward compatibility
impl MemorySystem {
    pub async fn new(config: MemoryConfig) -> Result<Self> {
        // Create a mock engine for the memory system
        let mock_engine: Arc<dyn Engine> = Arc::new(MockMemoryEngine);

        let system = IntegratedMemorySystem::from_components(
            mock_engine,
            config.working_config,
            config.compressor_config,
            config.persistence_config,
        );

        system.initialize().await?;
        Ok(system)
    }

    /// Update context - backward compatibility method
    pub async fn update_context(&self, context: &ExecutionContext) -> Result<()> {
        self.update_memory(context).await
    }
}

/// Mock engine for memory system (placeholder)
struct MockMemoryEngine;

#[async_trait::async_trait]
impl Engine for MockMemoryEngine {
    fn execute<'a>(
        &'a self,
        _request: &'a fluent_core::types::Request,
    ) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::Response>> + Send + 'a>
    {
        Box::new(async move {
            Ok(fluent_core::types::Response {
                content: "Mock memory response".to_string(),
                usage: fluent_core::types::Usage {
                    prompt_tokens: 5,
                    completion_tokens: 10,
                    total_tokens: 15,
                },
                model: "mock-memory".to_string(),
                finish_reason: Some("stop".to_string()),
                cost: fluent_core::types::Cost {
                    prompt_cost: 0.0005,
                    completion_cost: 0.001,
                    total_cost: 0.0015,
                },
            })
        })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a fluent_core::types::UpsertRequest,
    ) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::UpsertResponse>> + Send + 'a>
    {
        Box::new(async move {
            Ok(fluent_core::types::UpsertResponse {
                processed_files: vec![],
                errors: vec![],
            })
        })
    }

    fn get_neo4j_client(&self) -> Option<&std::sync::Arc<fluent_core::neo4j_client::Neo4jClient>> {
        None
    }

    fn get_session_id(&self) -> Option<String> {
        None
    }

    fn extract_content(
        &self,
        _content: &serde_json::Value,
    ) -> Option<fluent_core::types::ExtractedContent> {
        None
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a std::path::Path,
    ) -> Box<dyn std::future::Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move { Ok("mock_file_id".to_string()) })
    }

    fn process_request_with_file<'a>(
        &'a self,
        _request: &'a fluent_core::types::Request,
        _file_path: &'a std::path::Path,
    ) -> Box<dyn std::future::Future<Output = Result<fluent_core::types::Response>> + Send + 'a>
    {
        Box::new(async move {
            Ok(fluent_core::types::Response {
                content: "Mock file processing response".to_string(),
                usage: fluent_core::types::Usage {
                    prompt_tokens: 5,
                    completion_tokens: 10,
                    total_tokens: 15,
                },
                model: "mock-file-processing".to_string(),
                finish_reason: Some("stop".to_string()),
                cost: fluent_core::types::Cost {
                    prompt_cost: 0.0005,
                    completion_cost: 0.001,
                    total_cost: 0.0015,
                },
            })
        })
    }
}

/// Mock AsyncSqliteMemoryStore for examples and tests
#[derive(Debug)]
pub struct AsyncSqliteMemoryStore {
    memories: Arc<RwLock<HashMap<String, MemoryItem>>>,
}

impl AsyncSqliteMemoryStore {
    /// Create a new AsyncSqliteMemoryStore
    pub async fn new(_db_path: &str) -> Result<Self> {
        Ok(Self {
            memories: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

#[async_trait::async_trait]
impl LongTermMemory for AsyncSqliteMemoryStore {
    async fn store(&self, item: MemoryItem) -> Result<String> {
        let id = item.item_id.clone();
        let mut memories = self.memories.write().await;
        memories.insert(id.clone(), item);
        Ok(id)
    }

    async fn query(&self, query: &crate::agent_with_mcp::MemoryQuery) -> Result<Vec<MemoryItem>> {
        let memories = self.memories.read().await;
        let mut results = Vec::new();

        for (_, memory) in memories.iter() {
            // Apply filters
            let matches = true;

            // For simplicity in this mock, we're not implementing complex filtering
            // In a real implementation, we would apply the query filters

            if matches {
                results.push(memory.clone());
            }
        }

        // Apply limit if specified
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }
}
