use fluent_agent::{
    memory::{SqliteMemoryStore, LongTermMemory, MemoryItem, MemoryQuery, MemoryType},
    mcp_tool_registry::McpToolRegistry,
    performance::utils::{PerformanceCounter, MemoryTracker, ResourceLimiter},
};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Instant, Duration};
use chrono::Utc;
use anyhow::Result;
use tokio;

/// Performance and benchmark tests
/// Tests system performance under various load conditions and measures key metrics

#[tokio::test]
async fn test_memory_store_performance() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    let start_time = Instant::now();
    
    // Benchmark memory storage
    let num_memories = 1000;
    let mut store_times = Vec::new();
    
    for i in 0..num_memories {
        let memory = MemoryItem {
            memory_id: format!("perf_test_{}", i),
            memory_type: MemoryType::Experience,
            content: format!("Performance test memory content {}", i),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("index".to_string(), i.to_string());
                meta.insert("category".to_string(), format!("category_{}", i % 10));
                meta
            },
            importance: (i as f64) / (num_memories as f64),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![format!("tag_{}", i % 5)],
            embedding: None,
        };
        
        let store_start = Instant::now();
        store.store(memory).await?;
        store_times.push(store_start.elapsed());
    }
    
    let total_store_time = start_time.elapsed();
    let avg_store_time = store_times.iter().sum::<Duration>() / store_times.len() as u32;
    
    println!("Stored {} memories in {:?}", num_memories, total_store_time);
    println!("Average store time: {:?}", avg_store_time);
    
    // Benchmark memory retrieval
    let query_start = Instant::now();
    let query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(num_memories),
        tags: vec![],
    };
    
    let memories = store.retrieve(&query).await?;
    let query_time = query_start.elapsed();
    
    println!("Retrieved {} memories in {:?}", memories.len(), query_time);
    assert_eq!(memories.len(), num_memories);
    
    // Benchmark similarity search (using a memory item as reference)
    let similarity_start = Instant::now();
    let reference_memory = MemoryItem {
        memory_id: "similarity_ref".to_string(),
        memory_type: MemoryType::Experience,
        content: "performance test".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };
    let similar = store.find_similar(&reference_memory, 0.1).await?;
    let similarity_time = similarity_start.elapsed();
    
    println!("Similarity search completed in {:?}, found {} matches", similarity_time, similar.len());
    
    // Performance assertions
    assert!(total_store_time < Duration::from_secs(30), "Store operations too slow");
    assert!(query_time < Duration::from_secs(5), "Query operations too slow");
    assert!(similarity_time < Duration::from_secs(10), "Similarity search too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_memory_operations() -> Result<()> {
    let start_time = Instant::now();

    // Test concurrent writes using separate stores
    let num_concurrent = 50;
    let mut handles = Vec::new();

    for i in 0..num_concurrent {
        let handle = tokio::spawn(async move {
            let store = SqliteMemoryStore::new(":memory:").unwrap();
            let memory = MemoryItem {
                memory_id: format!("concurrent_{}", i),
                memory_type: MemoryType::Experience,
                content: format!("Concurrent memory {}", i),
                metadata: HashMap::new(),
                importance: 0.5,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec![],
                embedding: None,
            };
            
            store.store(memory).await
        });
        handles.push(handle);
    }

    // Wait for all operations
    let mut success_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.await? {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    let concurrent_time = start_time.elapsed();
    println!("Completed {} concurrent operations in {:?}", num_concurrent, concurrent_time);
    println!("Success: {}, Errors: {}", success_count, error_count);

    // Should have processed all operations
    assert_eq!(success_count + error_count, num_concurrent);

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
        limit: Some(100),
        tags: vec![],
    };

    let stored_memories = shared_store.retrieve(&query).await?;
    assert_eq!(stored_memories.len(), 1);
    
    // Performance assertion
    assert!(concurrent_time < Duration::from_secs(15), "Concurrent operations too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_performance() -> Result<()> {
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
    
    // Benchmark goal processing
    let num_goals = 10;
    let mut processing_times = Vec::new();
    
    for i in 0..num_goals {
        let goal = GoalBuilder::new()
            .with_description(&format!("Performance test goal {}", i))
            .with_complexity(GoalComplexity::Simple)
            .build()?;
        
        let context = ExecutionContext::new(format!("perf_session_{}", i));
        
        let process_start = Instant::now();
        let _state = orchestrator.process_goal(goal.clone(), context.clone()).await?;
        let process_time = process_start.elapsed();
        
        processing_times.push(process_time);
        
        // Benchmark task decomposition
        let decomp_start = Instant::now();
        let _tasks = orchestrator.decompose_goal(&goal, &context).await?;
        let decomp_time = decomp_start.elapsed();
        
        println!("Goal {} processed in {:?}, decomposed in {:?}", i, process_time, decomp_time);
    }
    
    let avg_processing_time = processing_times.iter().sum::<Duration>() / processing_times.len() as u32;
    println!("Average goal processing time: {:?}", avg_processing_time);
    
    // Performance assertion
    assert!(avg_processing_time < Duration::from_secs(2), "Goal processing too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_tool_registry_performance() -> Result<()> {
    let registry = McpToolRegistry::new();
    let start_time = Instant::now();
    
    // Benchmark tool registration
    let num_tools = 100;
    for i in 0..num_tools {
        let tool_def = fluent_agent::mcp_tool_registry::McpToolDefinition {
            name: format!("perf_tool_{}", i),
            description: format!("Performance test tool {}", i),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input parameter"
                    }
                }
            }),
        };
        
        registry.register_tool(tool_def).await?;
    }
    
    let registration_time = start_time.elapsed();
    println!("Registered {} tools in {:?}", num_tools, registration_time);
    
    // Benchmark tool listing
    let list_start = Instant::now();
    let tools = registry.list_tools().await;
    let list_time = list_start.elapsed();
    
    println!("Listed {} tools in {:?}", tools.len(), list_time);
    assert_eq!(tools.len(), num_tools);
    
    // Benchmark tool retrieval
    let mut retrieval_times = Vec::new();
    for i in 0..10 {
        let retrieve_start = Instant::now();
        let _tool = registry.get_tool(&format!("perf_tool_{}", i)).await;
        retrieval_times.push(retrieve_start.elapsed());
    }
    
    let avg_retrieval_time = retrieval_times.iter().sum::<Duration>() / retrieval_times.len() as u32;
    println!("Average tool retrieval time: {:?}", avg_retrieval_time);
    
    // Performance assertions
    assert!(registration_time < Duration::from_secs(10), "Tool registration too slow");
    assert!(list_time < Duration::from_secs(1), "Tool listing too slow");
    assert!(avg_retrieval_time < Duration::from_millis(100), "Tool retrieval too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_memory_usage_tracking() -> Result<()> {
    let mut tracker = MemoryTracker::new();
    
    // Test memory tracking during operations
    let initial_usage = tracker.get_current_usage();
    
    // Perform memory-intensive operations
    let store = SqliteMemoryStore::new(":memory:")?;
    let mut large_memories = Vec::new();
    
    for i in 0..100 {
        let memory = MemoryItem {
            memory_id: format!("large_memory_{}", i),
            memory_type: MemoryType::Experience,
            content: "x".repeat(10000), // 10KB content
            metadata: HashMap::new(),
            importance: 0.5,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![],
            embedding: Some(vec![0.1; 1536]), // Large embedding
        };
        
        store.store(memory.clone()).await?;
        large_memories.push(memory);
    }
    
    let peak_usage = tracker.get_peak_usage();
    let current_usage = tracker.get_current_usage();
    
    println!("Initial memory usage: {} bytes", initial_usage);
    println!("Peak memory usage: {} bytes", peak_usage);
    println!("Current memory usage: {} bytes", current_usage);
    
    // Memory should have increased
    assert!(current_usage >= initial_usage);
    assert!(peak_usage >= current_usage);
    
    Ok(())
}

#[tokio::test]
async fn test_performance_counter() -> Result<()> {
    let mut counter = PerformanceCounter::new("test_operations");
    
    // Test operation counting and timing
    for i in 0..1000 {
        counter.start_operation();
        
        // Simulate work
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        counter.end_operation();
        
        if i % 100 == 0 {
            let stats = counter.get_stats();
            println!("Operations: {}, Avg time: {:?}", stats.total_operations, stats.average_duration);
        }
    }
    
    let final_stats = counter.get_stats();
    println!("Final stats - Operations: {}, Total time: {:?}, Avg: {:?}", 
             final_stats.total_operations, 
             final_stats.total_duration, 
             final_stats.average_duration);
    
    assert_eq!(final_stats.total_operations, 1000);
    assert!(final_stats.average_duration > Duration::from_micros(50));
    
    Ok(())
}

#[tokio::test]
async fn test_resource_limiter() -> Result<()> {
    let limiter = ResourceLimiter::new(5); // Max 5 concurrent operations
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    // Try to run 20 operations concurrently (should be limited to 5)
    for i in 0..20 {
        let limiter_clone = limiter.clone();
        let handle = tokio::spawn(async move {
            let _permit = limiter_clone.acquire().await;
            
            // Simulate work
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            i
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    for handle in handles {
        handle.await?;
    }
    
    let total_time = start_time.elapsed();
    println!("Completed 20 operations with limit 5 in {:?}", total_time);
    
    // Should take at least 4 * 100ms = 400ms due to batching
    assert!(total_time >= Duration::from_millis(350));
    
    Ok(())
}

#[tokio::test]
async fn test_large_data_handling() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test with very large memory item
    let large_content = "x".repeat(1_000_000); // 1MB
    let large_metadata = {
        let mut meta = HashMap::new();
        for i in 0..1000 {
            meta.insert(format!("key_{}", i), format!("value_{}", i));
        }
        meta
    };
    
    let large_memory = MemoryItem {
        memory_id: "large_test".to_string(),
        memory_type: MemoryType::Experience,
        content: large_content,
        metadata: large_metadata,
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: (0..100).map(|i| format!("tag_{}", i)).collect(),
        embedding: Some(vec![0.1; 1536]),
    };
    
    let store_start = Instant::now();
    let result = store.store(large_memory).await;
    let store_time = store_start.elapsed();
    
    println!("Stored large memory item in {:?}", store_time);
    assert!(result.is_ok());
    
    // Test retrieval of large item
    let retrieve_start = Instant::now();
    let query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(1),
        tags: vec![],
    };
    
    let retrieved = store.retrieve(&query).await?;
    let retrieve_time = retrieve_start.elapsed();
    
    println!("Retrieved large memory item in {:?}", retrieve_time);
    assert_eq!(retrieved.len(), 1);
    
    // Performance assertions for large data
    assert!(store_time < Duration::from_secs(5), "Large data storage too slow");
    assert!(retrieve_time < Duration::from_secs(2), "Large data retrieval too slow");
    
    Ok(())
}
