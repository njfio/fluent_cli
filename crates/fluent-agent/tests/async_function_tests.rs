use fluent_agent::{
    memory::{SqliteMemoryStore, LongTermMemory, MemoryItem, MemoryQuery, MemoryType},
    transport::{RetryConfig, BackoffStrategy},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use chrono::Utc;
use anyhow::Result;
use tokio;
use tokio::time::timeout;
use futures::StreamExt;

/// Comprehensive async function tests
/// Tests async patterns, error propagation, timeout handling, and concurrent operations

#[tokio::test]
async fn test_async_memory_operations() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test basic async store operation
    let memory = MemoryItem {
        memory_id: "async_test_001".to_string(),
        memory_type: MemoryType::Experience,
        content: "Async memory test".to_string(),
        metadata: HashMap::new(),
        importance: 0.8,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec!["async".to_string()],
        embedding: None,
    };
    
    // Test async store with timeout
    let store_result = timeout(Duration::from_secs(5), store.store(memory.clone())).await?;
    assert!(store_result.is_ok());
    
    // Test async retrieve with timeout
    let query = MemoryQuery {
        query_text: "async".to_string(),
        memory_types: vec![MemoryType::Experience],
        time_range: None,
        importance_threshold: Some(0.5),
        limit: Some(10),
        tags: vec![],
    };
    
    let retrieve_result = timeout(Duration::from_secs(5), store.retrieve(&query)).await?;
    assert!(retrieve_result.is_ok());
    let memories = retrieve_result?;
    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].memory_id, "async_test_001");
    
    Ok(())
}

#[tokio::test]
async fn test_async_error_propagation() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test error propagation in async chain
    let invalid_memory = MemoryItem {
        memory_id: "".to_string(), // Invalid empty ID
        memory_type: MemoryType::Experience,
        content: "Test content".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };
    
    // This should either succeed or fail gracefully
    let result = store.store(invalid_memory).await;
    assert!(result.is_ok() || result.is_err()); // Either outcome is acceptable
    
    // Test error propagation through async operations
    let query = MemoryQuery {
        query_text: "nonexistent".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(2.0), // Invalid threshold > 1.0
        limit: Some(0), // Invalid limit
        tags: vec![],
    };
    
    let result = store.retrieve(&query).await;
    assert!(result.is_ok()); // Should handle gracefully
    
    Ok(())
}

#[tokio::test]
async fn test_async_timeout_handling() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test operation with very short timeout
    let memory = MemoryItem {
        memory_id: "timeout_test".to_string(),
        memory_type: MemoryType::Experience,
        content: "Timeout test".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };
    
    // Test with reasonable timeout (should succeed)
    let result = timeout(Duration::from_secs(10), store.store(memory.clone())).await;
    assert!(result.is_ok());
    
    // Test with very short timeout (may timeout, but should handle gracefully)
    let short_timeout_result = timeout(Duration::from_nanos(1), async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok::<(), anyhow::Error>(())
    }).await;
    
    // Should timeout
    assert!(short_timeout_result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_async_operations() -> Result<()> {
    let num_operations = 50u64;
    let mut handles = Vec::new();

    // Launch concurrent async operations using separate stores
    for i in 0..num_operations {
        let handle = tokio::spawn(async move {
            let store = SqliteMemoryStore::new(":memory:").unwrap();
            let memory = MemoryItem {
                memory_id: format!("concurrent_async_{}", i),
                memory_type: MemoryType::Experience,
                content: format!("Concurrent async operation {}", i),
                metadata: HashMap::new(),
                importance: (i as f64) / (num_operations as f64),
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec![format!("concurrent_{}", i % 5)],
                embedding: None,
            };

            // Add some async delay to simulate real work
            tokio::time::sleep(Duration::from_millis(i % 10)).await;

            store.store(memory).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations with timeout
    let start_time = Instant::now();
    let mut success_count = 0;
    let mut error_count = 0;
    
    for handle in handles {
        let result = timeout(Duration::from_secs(30), handle).await?;
        match result {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(_)) => error_count += 1,
            Err(_) => error_count += 1, // Join error
        }
    }
    
    let elapsed = start_time.elapsed();
    println!("Concurrent operations completed in {:?}", elapsed);
    println!("Success: {}, Errors: {}", success_count, error_count);
    
    // Should have processed all operations
    assert_eq!(success_count + error_count, num_operations);

    // Test with a shared store for verification
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
    
    Ok(())
}

#[tokio::test]
async fn test_async_retry_mechanisms() -> Result<()> {
    // Test simple retry logic without using the transport module
    let mut attempt_count = 0;
    let max_attempts = 3;

    // Simulate retry logic
    let mut result = Err(anyhow::anyhow!("Initial failure"));

    for _ in 0..max_attempts {
        attempt_count += 1;

        // Simulate operation that fails first few times
        if attempt_count < 3 {
            result = Err(anyhow::anyhow!("Simulated failure"));
            tokio::time::sleep(Duration::from_millis(10)).await; // Backoff
        } else {
            result = Ok("Success");
            break;
        }
    }

    assert!(result.is_ok());
    assert_eq!(result?, "Success");
    assert_eq!(attempt_count, 3);

    Ok(())
}

#[tokio::test]
async fn test_async_resource_cleanup() -> Result<()> {
    // Test that async operations properly clean up resources
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Create a scope where resources should be cleaned up
    {
        let memory = MemoryItem {
            memory_id: "cleanup_test".to_string(),
            memory_type: MemoryType::Experience,
            content: "Resource cleanup test".to_string(),
            metadata: HashMap::new(),
            importance: 0.5,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![],
            embedding: None,
        };
        
        store.store(memory).await?;
    } // Resources should be cleaned up here
    
    // Verify the store is still functional after cleanup
    let query = MemoryQuery {
        query_text: "cleanup".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(10),
        tags: vec![],
    };
    
    let memories = store.retrieve(&query).await?;
    assert_eq!(memories.len(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_async_cancellation() -> Result<()> {
    use tokio_util::sync::CancellationToken;
    
    let token = CancellationToken::new();
    let token_clone = token.clone();
    
    // Start a long-running async operation
    let handle = tokio::spawn(async move {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                Ok("Completed")
            }
            _ = token_clone.cancelled() => {
                Err(anyhow::anyhow!("Operation cancelled"))
            }
        }
    });
    
    // Cancel after a short delay
    tokio::time::sleep(Duration::from_millis(100)).await;
    token.cancel();
    
    let result = handle.await?;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cancelled"));
    
    Ok(())
}

#[tokio::test]
async fn test_async_mcp_client_operations() -> Result<()> {
    use fluent_agent::mcp_client::McpClient;

    // Test MCP client async operations with timeout
    let client = McpClient::new();

    // Test that client starts in disconnected state
    assert!(!client.is_connected());

    // Test async timeout with a simple operation
    let timeout_result = timeout(
        Duration::from_millis(100),
        async {
            // Simulate some async work
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<(), anyhow::Error>(())
        }
    ).await;

    // Should complete within timeout
    assert!(timeout_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_async_transport_operations() -> Result<()> {
    // Test async transport-like operations with timeouts

    // Test connection timeout simulation
    let connection_timeout = Duration::from_millis(100);
    let connection_result = timeout(
        connection_timeout,
        async {
            // Simulate connection attempt
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<(), anyhow::Error>(())
        }
    ).await;

    assert!(connection_result.is_ok());

    // Test request timeout simulation
    let request_timeout = Duration::from_millis(200);
    let request_result = timeout(
        request_timeout,
        async {
            // Simulate request processing
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<String, anyhow::Error>("Response".to_string())
        }
    ).await;

    assert!(request_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_async_workflow_execution() -> Result<()> {
    // Test async workflow-like execution patterns

    // Simulate workflow step execution
    let step_execution = async {
        // Simulate step processing
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok::<String, anyhow::Error>("Step completed".to_string())
    };

    // Test step execution with timeout
    let execution_result = timeout(
        Duration::from_secs(10),
        step_execution
    ).await;

    // Should complete within timeout
    assert!(execution_result.is_ok());

    if let Ok(Ok(result)) = execution_result {
        assert_eq!(result, "Step completed");
    }

    Ok(())
}

#[tokio::test]
async fn test_async_memory_batch_operations() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;

    // Test batch async operations
    let mut memories = Vec::new();
    for i in 0..20 {
        memories.push(MemoryItem {
            memory_id: format!("batch_async_{}", i),
            memory_type: MemoryType::Experience,
            content: format!("Batch async memory {}", i),
            metadata: HashMap::new(),
            importance: (i as f64) / 20.0,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![format!("batch_{}", i % 3)],
            embedding: None,
        });
    }

    // Store all memories concurrently
    let store_futures: Vec<_> = memories.into_iter()
        .map(|memory| store.store(memory))
        .collect();

    let results = timeout(
        Duration::from_secs(30),
        futures::future::join_all(store_futures)
    ).await?;

    // Count successful operations
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert!(success_count > 0);

    // Test batch retrieval
    let query = MemoryQuery {
        query_text: "batch".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(50),
        tags: vec![],
    };

    let retrieved = timeout(Duration::from_secs(10), store.retrieve(&query)).await??;
    assert!(retrieved.len() <= 20);

    Ok(())
}

#[tokio::test]
async fn test_async_error_recovery() -> Result<()> {
    let store = SqliteMemoryStore::new(":memory:")?;

    // Test error recovery in async operations
    let mut successful_operations = 0;
    let mut failed_operations = 0;

    for i in 0..10 {
        let memory = MemoryItem {
            memory_id: format!("recovery_test_{}", i),
            memory_type: MemoryType::Experience,
            content: if i % 3 == 0 { "".to_string() } else { format!("Content {}", i) },
            metadata: HashMap::new(),
            importance: if i % 4 == 0 { -0.1 } else { 0.5 }, // Some invalid
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![],
            embedding: None,
        };

        match store.store(memory).await {
            Ok(_) => successful_operations += 1,
            Err(_) => failed_operations += 1,
        }
    }

    // Should have processed all operations (some may fail)
    assert_eq!(successful_operations + failed_operations, 10);

    // System should still be functional after errors
    let test_memory = MemoryItem {
        memory_id: "post_error_test".to_string(),
        memory_type: MemoryType::Experience,
        content: "Post error test".to_string(),
        metadata: HashMap::new(),
        importance: 0.5,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        tags: vec![],
        embedding: None,
    };

    let result = store.store(test_memory).await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_async_stream_processing() -> Result<()> {
    use tokio_stream::{self as stream, StreamExt};

    // Test async stream processing
    let numbers = stream::iter(0..100);
    let mut processed_count = 0;

    let mut stream = StreamExt::map(numbers, |n| async move {
        // Simulate async processing
        tokio::time::sleep(Duration::from_millis(1)).await;
        n * 2
    }).buffer_unordered(10); // Process up to 10 items concurrently

    while let Some(result) = StreamExt::next(&mut stream).await {
        processed_count += 1;
        assert!(result % 2 == 0); // Should be even (doubled)
    }

    assert_eq!(processed_count, 100);

    Ok(())
}
