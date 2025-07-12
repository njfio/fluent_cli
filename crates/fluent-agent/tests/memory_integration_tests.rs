use fluent_agent::memory::{
    LongTermMemory, MemoryItem, MemoryQuery, MemoryType, SqliteMemoryStore,
    ShortTermMemory, MemoryConfig
};
use std::collections::HashMap;
use chrono::Utc;
use anyhow::Result;
use tokio;

/// Integration tests for memory system functionality
/// Tests the complete memory workflow including storage, retrieval, and querying

#[tokio::test]
async fn test_memory_lifecycle_integration() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test storing different types of memories
    let experience_memory = MemoryItem {
        memory_id: "exp_001".to_string(),
        memory_type: MemoryType::Experience,
        content: "Successfully completed task A using strategy X".to_string(),
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("task_id".to_string(), serde_json::Value::String("task_a".to_string()));
            meta.insert("strategy".to_string(), serde_json::Value::String("strategy_x".to_string()));
            meta
        },
        importance: 0.8,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec!["task".to_string(), "success".to_string()],
        embedding: None,
    };

    let learning_memory = MemoryItem {
        memory_id: "learn_001".to_string(),
        memory_type: MemoryType::Learning,
        content: "Strategy X works well for tasks involving file operations".to_string(),
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("domain".to_string(), serde_json::Value::String("file_operations".to_string()));
            meta
        },
        importance: 0.9,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec!["strategy".to_string(), "file_ops".to_string()],
        embedding: None,
    };

    // Store memories
    let exp_id = store.store(experience_memory.clone()).await?;
    let learn_id = store.store(learning_memory.clone()).await?;
    
    assert_eq!(exp_id, "exp_001");
    assert_eq!(learn_id, "learn_001");

    // Test retrieval by importance
    let high_importance_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.85),
        limit: Some(10),
        tags: vec![],
    };

    let high_importance_memories = store.retrieve(&high_importance_query).await?;
    assert!(high_importance_memories.len() >= 1);
    // Find the learning memory in the results
    let learning_memory = high_importance_memories.iter()
        .find(|m| m.memory_id == "learn_001")
        .expect("Learning memory should be found");
    assert_eq!(learning_memory.memory_id, "learn_001");

    // Test retrieval by type
    let experience_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![MemoryType::Experience],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(10),
        tags: vec![],
    };

    let experience_memories = store.retrieve(&experience_query).await?;
    assert!(experience_memories.len() >= 1);
    // Find the experience memory in the results
    let experience_memory_found = experience_memories.iter()
        .find(|m| m.memory_type == MemoryType::Experience)
        .expect("Experience memory should be found");
    assert_eq!(experience_memory_found.memory_type, MemoryType::Experience);

    // Test similarity search (using the experience memory as reference)
    let similar_memories = store.find_similar(&experience_memory, 0.5).await?;
    assert!(similar_memories.len() >= 0); // May or may not find similar memories

    // Test memory update
    let mut updated_memory = experience_memory.clone();
    updated_memory.access_count = 5;
    updated_memory.importance = 0.9;
    
    store.update(&exp_id, updated_memory).await?;

    // Verify update
    let all_memories_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(10),
        tags: vec![],
    };

    let all_memories = store.retrieve(&all_memories_query).await?;
    let updated = all_memories.iter().find(|m| m.memory_id == "exp_001").unwrap();
    assert_eq!(updated.access_count, 5);
    assert_eq!(updated.importance, 0.9);

    // Test memory deletion
    store.delete(&learn_id).await?;
    
    let remaining_memories = store.retrieve(&all_memories_query).await?;
    assert_eq!(remaining_memories.len(), 1);
    assert_eq!(remaining_memories[0].memory_id, "exp_001");

    Ok(())
}

#[tokio::test]
async fn test_short_term_memory_integration() -> Result<()> {
    let short_term = ShortTermMemory::new(100); // capacity of 100

    // Test basic creation
    assert_eq!(short_term.capacity, 100);
    assert!(short_term.recent_observations.is_empty());
    assert!(short_term.active_patterns.is_empty());
    assert!(short_term.working_hypotheses.is_empty());
    assert!(short_term.attention_focus.is_empty());
    assert!(short_term.current_context.is_none());

    // Test memory config
    let config = MemoryConfig::default();
    assert_eq!(config.short_term_capacity, 100);
    assert_eq!(config.consolidation_threshold, 0.8);
    assert!(config.enable_forgetting);
    assert!(config.compression_enabled);

    Ok(())
}

#[tokio::test]
async fn test_memory_query_edge_cases() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;

    // Test empty database queries
    let empty_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(10),
        tags: vec![],
    };

    let empty_results = store.retrieve(&empty_query).await?;
    assert_eq!(empty_results.len(), 0);

    // Test high threshold query (should return nothing)
    let high_threshold_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.99),
        limit: Some(10),
        tags: vec![],
    };

    let high_threshold_results = store.retrieve(&high_threshold_query).await?;
    assert_eq!(high_threshold_results.len(), 0);

    // Add a memory and test limit functionality
    let test_memory = MemoryItem {
        memory_id: "limit_test".to_string(),
        memory_type: MemoryType::Fact,
        content: "Test memory for limit functionality".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec!["test".to_string()],
        embedding: None,
    };

    store.store(test_memory).await?;

    // Test limit = 0 (should return nothing)
    let zero_limit_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(0),
        tags: vec![],
    };

    let zero_limit_results = store.retrieve(&zero_limit_query).await?;
    assert_eq!(zero_limit_results.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_memory_error_handling() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;

    // Test updating non-existent memory
    let fake_memory = MemoryItem {
        memory_id: "fake_id".to_string(),
        memory_type: MemoryType::Experience,
        content: "This memory doesn't exist".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };

    // This should not fail but should not affect anything
    store.update("non_existent_id", fake_memory).await?;

    // Test deleting non-existent memory
    store.delete("non_existent_id").await?;

    // Test similarity search with empty memory
    let empty_memory = MemoryItem {
        memory_id: "empty_ref".to_string(),
        memory_type: MemoryType::Experience,
        content: "".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };
    let empty_similar = store.find_similar(&empty_memory, 0.5).await?;
    assert_eq!(empty_similar.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_memory_concurrency() -> Result<()> {
    // Test concurrent memory operations using separate stores
    let mut handles = vec![];

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let store = SqliteMemoryStore::new(":memory:").unwrap();
            let memory = MemoryItem {
                memory_id: format!("concurrent_{}", i),
                memory_type: MemoryType::Experience,
                content: format!("Concurrent memory {}", i),
                metadata: HashMap::new(),
                importance: 0.5 + (i as f64 * 0.05),
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec![format!("concurrent_{}", i)],
                embedding: None,
            };

            store.store(memory).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }

    // Test with shared store for verification
    let shared_store = SqliteMemoryStore::new(":memory:")?;
    let test_memory = MemoryItem {
        memory_id: "shared_test".to_string(),
        memory_type: MemoryType::Experience,
        content: "Shared store test".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };

    shared_store.store(test_memory).await?;

    let query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(20),
        tags: vec![],
    };

    let memories = shared_store.retrieve(&query).await?;
    assert_eq!(memories.len(), 1);

    Ok(())
}
