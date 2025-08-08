use fluent_agent::{
    memory::{AsyncSqliteMemoryStore, LongTermMemory, MemoryItem, MemoryQuery, MemoryType},
    performance::utils::{PerformanceCounter, MemoryTracker},
};
use std::collections::HashMap;
use std::time::{Instant, Duration};
use chrono::Utc;
use anyhow::Result;
use tokio;

/// Comprehensive memory performance tests
/// Tests memory system performance under various load conditions

#[tokio::test]
async fn test_memory_store_throughput() -> Result<()> {
    let store = AsyncSqliteMemoryStore::new(":memory:").await?;
    let counter = PerformanceCounter::new();
    
    // Test write throughput
    let num_writes = 1000;
    let start_time = Instant::now();
    
    for i in 0..num_writes {
        let write_start = Instant::now();
        
        let memory = MemoryItem {
            memory_id: format!("throughput_test_{}", i),
            memory_type: MemoryType::Experience,
            content: format!("Throughput test memory {}", i),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("index".to_string(), serde_json::Value::Number(i.into()));
                meta.insert("batch".to_string(), serde_json::Value::String(format!("batch_{}", i / 100)));
                meta
            },
            importance: (i as f64) / (num_writes as f64),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![format!("tag_{}", i % 10)],
            embedding: None,
        };
        
        let result = store.store(memory).await;
        let write_duration = write_start.elapsed();
        
        counter.record_request(write_duration, result.is_err());
        
        if result.is_err() {
            eprintln!("Write {} failed: {:?}", i, result);
        }
    }
    
    let total_time = start_time.elapsed();
    let stats = counter.get_stats();
    
    println!("Write Throughput Results:");
    println!("  Total writes: {}", num_writes);
    println!("  Total time: {:?}", total_time);
    println!("  Writes per second: {:.2}", num_writes as f64 / total_time.as_secs_f64());
    println!("  Average write time: {:?}", stats.average_duration);
    println!("  Error rate: {:.2}%", stats.error_rate * 100.0);
    
    // Performance assertions
    assert!(total_time < Duration::from_secs(60), "Write throughput too slow");
    assert!(stats.error_rate < 0.05, "Too many write errors");
    
    Ok(())
}

#[tokio::test]
async fn test_memory_query_performance() -> Result<()> {
    let store = AsyncSqliteMemoryStore::new(":memory:").await?;
    
    // Populate with test data
    let num_memories = 5000;
    println!("Populating {} memories for query testing...", num_memories);
    
    for i in 0..num_memories {
        let memory = MemoryItem {
            memory_id: format!("query_test_{}", i),
            memory_type: if i % 3 == 0 { MemoryType::Experience } else { MemoryType::Fact },
            content: format!("Query test memory {} with content about topic {}", i, i % 20),
            metadata: HashMap::new(),
            importance: (i as f64) / (num_memories as f64),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: (i % 10) as u32 + 1,
            tags: vec![format!("topic_{}", i % 20), format!("category_{}", i % 5)],
            embedding: None,
        };
        
        store.store(memory).await?;
    }
    
    println!("Testing query performance...");
    
    // Test different query patterns
    let query_tests = vec![
        ("All memories", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(1000),
            tags: vec![],
        }),
        ("High importance", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.8),
            limit: Some(1000),
            tags: vec![],
        }),
        ("Experience type", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![MemoryType::Experience],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(1000),
            tags: vec![],
        }),
        ("Tagged memories", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(1000),
            tags: vec!["topic_5".to_string()],
        }),
    ];
    
    for (test_name, query) in query_tests {
        let query_start = Instant::now();
        let results = store.retrieve(&query).await?;
        let query_time = query_start.elapsed();
        
        println!("  {}: {} results in {:?}", test_name, results.len(), query_time);
        
        // Performance assertion
        assert!(query_time < Duration::from_secs(5), "Query {} too slow", test_name);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_memory_stress_test() -> Result<()> {
    let store = AsyncSqliteMemoryStore::new(":memory:").await?;
    let mut tracker = MemoryTracker::new();
    
    // Stress test with large memories
    let num_large_memories = 100;
    let large_content_size = 50_000; // 50KB per memory
    
    println!("Starting memory stress test with {} large memories...", num_large_memories);
    
    let start_time = Instant::now();
    let initial_usage = tracker.get_current_usage();
    
    for i in 0..num_large_memories {
        let memory = MemoryItem {
            memory_id: format!("stress_test_{}", i),
            memory_type: MemoryType::Experience,
            content: "x".repeat(large_content_size),
            metadata: {
                let mut meta = HashMap::new();
                for j in 0..50 {
                    meta.insert(format!("key_{}", j), serde_json::Value::String(format!("value_{}", j)));
                }
                meta
            },
            importance: 0.5,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: (0..20).map(|j| format!("stress_tag_{}", j)).collect(),
            embedding: Some(vec![0.1; 1536]), // Large embedding
        };
        
        store.store(memory).await?;
        
        if i % 10 == 0 {
            let current_usage = tracker.get_current_usage();
            println!("  Stored {} memories, memory usage: {} bytes", i + 1, current_usage);
        }
    }
    
    let store_time = start_time.elapsed();
    let peak_usage = tracker.get_peak_usage();
    
    // Test retrieval performance with large data
    let retrieval_start = Instant::now();
    let query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(num_large_memories),
        tags: vec![],
    };
    
    let results = store.retrieve(&query).await?;
    let retrieval_time = retrieval_start.elapsed();
    
    println!("Stress Test Results:");
    println!("  Stored {} large memories in {:?}", num_large_memories, store_time);
    println!("  Retrieved {} memories in {:?}", results.len(), retrieval_time);
    println!("  Peak memory usage: {} bytes", peak_usage);
    println!("  Memory usage increase: {} bytes", peak_usage - initial_usage);
    
    // Performance assertions
    assert!(store_time < Duration::from_secs(120), "Stress test storage too slow");
    assert!(retrieval_time < Duration::from_secs(10), "Stress test retrieval too slow");
    assert_eq!(results.len(), num_large_memories);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_read_write_performance() -> Result<()> {
    let store = AsyncSqliteMemoryStore::new(":memory:").await?;
    
    // Pre-populate with some data
    for i in 0..100 {
        let memory = MemoryItem {
            memory_id: format!("concurrent_base_{}", i),
            memory_type: MemoryType::Experience,
            content: format!("Base memory {}", i),
            metadata: HashMap::new(),
            importance: 0.5,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![],
            embedding: None,
        };
        store.store(memory).await?;
    }
    
    println!("Testing concurrent read/write performance...");
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    // Spawn concurrent writers using separate stores
    for i in 0..20 {
        let handle = tokio::spawn(async move {
            let writer_store = AsyncSqliteMemoryStore::new(":memory:").await.unwrap();
            for j in 0..50 {
                let memory = MemoryItem {
                    memory_id: format!("concurrent_write_{}_{}", i, j),
                    memory_type: MemoryType::Experience,
                    content: format!("Concurrent write {} {}", i, j),
                    metadata: HashMap::new(),
                    importance: 0.5,
                    created_at: Utc::now(),
                    last_accessed: Utc::now(),
                    access_count: 1,
                    tags: vec![],
                    embedding: None,
                };

                if let Err(e) = writer_store.store(memory).await {
                    eprintln!("Write error: {:?}", e);
                }
            }
        });
        handles.push(handle);
    }

    // Spawn concurrent readers using separate stores
    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let reader_store = AsyncSqliteMemoryStore::new(":memory:").await.unwrap();
            // Pre-populate with some data for reading
            for k in 0..10 {
                let memory = MemoryItem {
                    memory_id: format!("reader_data_{}_{}", i, k),
                    memory_type: MemoryType::Experience,
                    content: format!("Reader data {} {}", i, k),
                    metadata: HashMap::new(),
                    importance: 0.5,
                    created_at: Utc::now(),
                    last_accessed: Utc::now(),
                    access_count: 1,
                    tags: vec![],
                    embedding: None,
                };
                let _ = reader_store.store(memory).await;
            }

            for _ in 0..100 {
                let query = MemoryQuery {
                    query_text: "".to_string(),
                    memory_types: vec![],
                    time_range: None,
                    importance_threshold: Some(0.0),
                    limit: Some(50),
                    tags: vec![],
                };

                if let Err(e) = reader_store.retrieve(&query).await {
                    eprintln!("Read error {}: {:?}", i, e);
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    for handle in handles {
        handle.await?;
    }
    
    let total_time = start_time.elapsed();
    
    println!("Concurrent read/write completed in {:?}", total_time);
    
    // Verify final state
    let final_query = MemoryQuery {
        query_text: "".to_string(),
        memory_types: vec![],
        time_range: None,
        importance_threshold: Some(0.0),
        limit: Some(2000),
        tags: vec![],
    };
    
    let final_results = store.retrieve(&final_query).await?;
    println!("Final memory count: {}", final_results.len());
    
    // Performance assertion
    assert!(total_time < Duration::from_secs(60), "Concurrent operations too slow");
    
    Ok(())
}
