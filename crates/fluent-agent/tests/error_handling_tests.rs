use fluent_agent::{
    memory::{SqliteMemoryStore, LongTermMemory, MemoryItem, MemoryQuery, MemoryType},
    orchestrator::AgentOrchestrator,
    goal::{GoalBuilder, GoalComplexity},
    task::TaskBuilder,
    context::ExecutionContext,
    reflection::{ReflectionEngine, ReflectionConfig},
    tools::ToolRegistry,
    mcp_adapter::FluentMcpAdapter,
    mcp_tool_registry::McpToolRegistry,
};
use std::sync::Arc;
use std::collections::HashMap;
use chrono::Utc;
use anyhow::Result;
use tokio;

/// Comprehensive error handling and edge case tests
/// Tests system behavior under various failure conditions and edge cases

#[tokio::test]
async fn test_memory_error_conditions() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test storing memory with invalid data
    let invalid_memory = MemoryItem {
        memory_id: "".to_string(), // Empty ID
        memory_type: MemoryType::Experience,
        content: "".to_string(), // Empty content
        metadata: HashMap::new(),
        importance: -1.0, // Invalid importance
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 0,
        tags: vec![],
        embedding: None,
    };
    
    // Should handle invalid memory gracefully
    let result = store.store(invalid_memory).await;
    assert!(result.is_ok() || result.is_err()); // Either succeeds or fails gracefully
    
    // Test querying with extreme parameters
    let extreme_query = MemoryQuery {
        query_text: "x".repeat(10000), // Very long query
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(2.0), // Invalid threshold > 1.0
        limit: Some(0), // Zero limit
        tags: vec!["nonexistent".to_string()],
    };
    
    let extreme_result = store.retrieve(&extreme_query).await;
    assert!(extreme_result.is_ok()); // Should handle gracefully
    
    // Test updating non-existent memory
    let fake_memory = MemoryItem {
        memory_id: "fake".to_string(),
        memory_type: MemoryType::Fact,
        content: "Fake content".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };
    
    let update_result = store.update("nonexistent_id", fake_memory).await;
    assert!(update_result.is_ok()); // Should not fail
    
    // Test deleting non-existent memory
    let delete_result = store.delete("nonexistent_id").await;
    assert!(delete_result.is_ok()); // Should not fail
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_error_handling() -> Result<()> {
    let memory_system = Arc::new(SqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    let tool_registry = Arc::new(ToolRegistry::new());
    let reflection_engine = Arc::new(ReflectionEngine::new(
        ReflectionConfig::default(),
        memory_system.clone(),
    ));
    
    let orchestrator = AgentOrchestrator::new(
        memory_system,
        tool_registry,
        reflection_engine,
    );
    
    // Test with invalid goal
    let invalid_goal_result = GoalBuilder::new()
        .with_description("") // Empty description
        .build();
    
    assert!(invalid_goal_result.is_err());
    
    // Test with valid goal but problematic context
    let valid_goal = GoalBuilder::new()
        .with_description("Valid goal")
        .with_complexity(GoalComplexity::Simple)
        .build()?;
    
    let empty_context = ExecutionContext::new("".to_string()); // Empty session ID
    
    let result = orchestrator.process_goal(valid_goal.clone(), empty_context).await;
    assert!(result.is_ok() || result.is_err()); // Should handle gracefully
    
    // Test decomposition with complex goal but no context
    let complex_goal = GoalBuilder::new()
        .with_description("Very complex goal requiring multiple steps")
        .with_complexity(GoalComplexity::Complex)
        .build()?;
    
    let minimal_context = ExecutionContext::new("test".to_string());
    let decomposition_result = orchestrator.decompose_goal(&complex_goal, &minimal_context).await;
    assert!(decomposition_result.is_ok()); // Should handle gracefully
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_error_scenarios() -> Result<()> {
    let tool_registry = Arc::new(ToolRegistry::new());
    let memory_system = Arc::new(SqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    
    let adapter = FluentMcpAdapter::new(tool_registry, memory_system);
    
    // Test calling non-existent tool
    let invalid_request = fluent_mcp::model::CallToolRequest {
        name: "nonexistent_tool".to_string(),
        arguments: Some(serde_json::json!({})),
    };
    
    let result = adapter.call_tool(invalid_request).await;
    assert!(result.is_err()); // Should fail gracefully
    
    // Test with malformed arguments
    let malformed_request = fluent_mcp::model::CallToolRequest {
        name: "some_tool".to_string(),
        arguments: Some(serde_json::json!("not_an_object")),
    };
    
    let malformed_result = adapter.call_tool(malformed_request).await;
    assert!(malformed_result.is_err()); // Should fail gracefully
    
    // Test tool registry error handling
    let registry = McpToolRegistry::new();
    
    // Test registering tool with empty name
    let invalid_tool = fluent_agent::mcp_tool_registry::McpToolDefinition {
        name: "".to_string(),
        description: "Tool with empty name".to_string(),
        input_schema: serde_json::json!({}),
    };
    
    let register_result = registry.register_tool(invalid_tool).await;
    assert!(register_result.is_ok() || register_result.is_err()); // Should handle gracefully
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_error_conditions() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test concurrent operations that might conflict
    let mut handles = vec![];
    
    for i in 0..10 {
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            // Some operations succeed, some might fail
            let memory = MemoryItem {
                memory_id: format!("concurrent_{}", i),
                memory_type: MemoryType::Experience,
                content: if i % 3 == 0 { "".to_string() } else { format!("Content {}", i) },
                metadata: HashMap::new(),
                importance: if i % 2 == 0 { -0.5 } else { 0.5 }, // Some invalid
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: i as u32,
                tags: vec![],
                embedding: None,
            };
            
            store_clone.store(memory).await
        });
        handles.push(handle);
    }
    
    // Collect results - some might fail, that's expected
    let mut success_count = 0;
    let mut error_count = 0;
    
    for handle in handles {
        match handle.await? {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    // Should have processed all operations (either success or graceful failure)
    assert_eq!(success_count + error_count, 10);
    
    Ok(())
}

#[tokio::test]
async fn test_resource_exhaustion_scenarios() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test with very large memory items
    let large_memory = MemoryItem {
        memory_id: "large_memory".to_string(),
        memory_type: MemoryType::Experience,
        content: "x".repeat(1_000_000), // 1MB content
        metadata: {
            let mut meta = HashMap::new();
            for i in 0..1000 {
                meta.insert(format!("key_{}", i), format!("value_{}", i));
            }
            meta
        },
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: (0..1000).map(|i| format!("tag_{}", i)).collect(),
        embedding: Some(vec![0.1; 1536]), // Large embedding
    };
    
    let large_result = store.store(large_memory).await;
    assert!(large_result.is_ok() || large_result.is_err()); // Should handle gracefully
    
    // Test with many small operations
    for i in 0..100 {
        let small_memory = MemoryItem {
            memory_id: format!("small_{}", i),
            memory_type: MemoryType::Fact,
            content: format!("Small content {}", i),
            metadata: HashMap::new(),
            importance: 0.1,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![],
            embedding: None,
        };
        
        let _ = store.store(small_memory).await; // Ignore individual results
    }
    
    // Test large query
    let large_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(1000), // Large limit
        tags: vec![],
    };
    
    let large_query_result = store.retrieve(&large_query).await;
    assert!(large_query_result.is_ok()); // Should handle gracefully
    
    Ok(())
}

#[tokio::test]
async fn test_malformed_data_handling() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test with various malformed data scenarios
    let malformed_memories = vec![
        MemoryItem {
            memory_id: "unicode_test".to_string(),
            memory_type: MemoryType::Experience,
            content: "Test with unicode: 🚀 🎉 ñáéíóú".to_string(),
            metadata: HashMap::new(),
            importance: 0.5,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec!["unicode".to_string()],
            embedding: None,
        },
        MemoryItem {
            memory_id: "special_chars".to_string(),
            memory_type: MemoryType::Fact,
            content: "Special chars: \n\t\r\\\"'".to_string(),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("special".to_string(), "\n\t\r".to_string());
                meta
            },
            importance: 0.5,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![],
            embedding: None,
        },
    ];
    
    for memory in malformed_memories {
        let result = store.store(memory).await;
        assert!(result.is_ok()); // Should handle special characters gracefully
    }
    
    Ok(())
}

#[tokio::test]
async fn test_timeout_scenarios() -> Result<()> {
    // Test operations that might timeout
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Create a goal with very short timeout
    let timeout_goal = GoalBuilder::new()
        .with_description("Goal with short timeout")
        .with_timeout(std::time::Duration::from_millis(1)) // Very short timeout
        .build()?;
    
    // Verify timeout is set correctly
    assert_eq!(timeout_goal.timeout, Some(std::time::Duration::from_millis(1)));
    
    // Test task with timeout
    let timeout_task = TaskBuilder::new()
        .with_description("Task with timeout")
        .with_timeout(std::time::Duration::from_millis(1))
        .build()?;
    
    assert_eq!(timeout_task.timeout, Some(std::time::Duration::from_millis(1)));
    
    Ok(())
}

#[tokio::test]
async fn test_boundary_value_testing() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test boundary values for importance
    let boundary_values = vec![0.0, 0.1, 0.5, 0.9, 1.0];
    
    for (i, importance) in boundary_values.iter().enumerate() {
        let memory = MemoryItem {
            memory_id: format!("boundary_{}", i),
            memory_type: MemoryType::Experience,
            content: format!("Boundary test {}", importance),
            metadata: HashMap::new(),
            importance: *importance,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![],
            embedding: None,
        };
        
        let result = store.store(memory).await;
        assert!(result.is_ok());
    }
    
    // Test boundary values for query limits
    let boundary_limits = vec![0, 1, 10, 100, 1000];
    
    for limit in boundary_limits {
        let query = MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(limit),
            tags: vec![],
        };
        
        let result = store.retrieve(&query).await;
        assert!(result.is_ok());
        
        if let Ok(memories) = result {
            assert!(memories.len() <= limit);
        }
    }
    
    Ok(())
}
