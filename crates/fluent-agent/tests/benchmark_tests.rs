use fluent_agent::{
    memory::{SqliteMemoryStore, LongTermMemory, MemoryItem, MemoryQuery, MemoryType},
    mcp_tool_registry::McpToolRegistry,
    performance::utils::{PerformanceCounter, MemoryTracker},
};
use std::collections::HashMap;
use std::time::{Instant, Duration};
use chrono::Utc;
use anyhow::Result;
use tokio;

/// Comprehensive benchmark tests for critical performance paths
/// Tests system performance under realistic load conditions

#[tokio::test]
async fn benchmark_memory_operations() -> Result<()> {
    println!("=== Memory Operations Benchmark ===");
    
    let store = SqliteMemoryStore::new(":memory:")?;
    let counter = PerformanceCounter::new();
    
    // Benchmark different memory sizes
    let test_cases = vec![
        ("Small", 100, 1000),    // 100 bytes, 1000 items
        ("Medium", 10000, 500),  // 10KB, 500 items
        ("Large", 100000, 100),  // 100KB, 100 items
    ];
    
    for (size_name, content_size, num_items) in test_cases {
        println!("\nTesting {} memories ({} bytes each, {} items):", size_name, content_size, num_items);
        
        counter.reset();
        let benchmark_start = Instant::now();
        
        // Store benchmark
        for i in 0..num_items {
            let store_start = Instant::now();
            
            let memory = MemoryItem {
                memory_id: format!("benchmark_{}_{}", size_name.to_lowercase(), i),
                memory_type: MemoryType::Experience,
                content: "x".repeat(content_size),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("size".to_string(), serde_json::Value::String(size_name.to_string()));
                    meta.insert("index".to_string(), serde_json::Value::Number(i.into()));
                    meta
                },
                importance: (i as f64) / (num_items as f64),
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec![format!("benchmark_{}", size_name.to_lowercase())],
                embedding: None,
            };
            
            let result = store.store(memory).await;
            let store_duration = store_start.elapsed();
            
            counter.record_request(store_duration, result.is_err());
        }
        
        let store_time = benchmark_start.elapsed();
        let store_stats = counter.get_stats();
        
        // Retrieve benchmark
        counter.reset();
        let retrieve_start = Instant::now();
        
        let query = MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(num_items),
            tags: vec![format!("benchmark_{}", size_name.to_lowercase())],
        };
        
        let results = store.retrieve(&query).await?;
        let retrieve_time = retrieve_start.elapsed();
        
        println!("  Store: {} items in {:?} (avg: {:?}, {:.2} items/sec)", 
                 num_items, store_time, store_stats.average_duration, 
                 num_items as f64 / store_time.as_secs_f64());
        println!("  Retrieve: {} items in {:?} ({:.2} items/sec)", 
                 results.len(), retrieve_time, 
                 results.len() as f64 / retrieve_time.as_secs_f64());
        println!("  Error rate: {:.2}%", store_stats.error_rate * 100.0);
        
        // Performance assertions
        assert!(store_time < Duration::from_secs(60), "{} store benchmark too slow", size_name);
        assert!(retrieve_time < Duration::from_secs(10), "{} retrieve benchmark too slow", size_name);
        assert!(store_stats.error_rate < 0.05, "{} too many store errors", size_name);
    }
    
    Ok(())
}

#[tokio::test]
async fn benchmark_concurrent_load() -> Result<()> {
    println!("=== Concurrent Load Benchmark ===");
    
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Test different concurrency levels
    let concurrency_levels = vec![1, 5, 10, 20, 50];
    let operations_per_task = 100;
    
    for concurrency in concurrency_levels {
        println!("\nTesting concurrency level: {} tasks", concurrency);
        
        let start_time = Instant::now();
        let mut handles = Vec::new();
        
        for task_id in 0..concurrency {
            let handle = tokio::spawn(async move {
                // Create separate store for each task to avoid lifetime issues
                let task_store = SqliteMemoryStore::new(":memory:").unwrap();
                let mut task_success = 0;
                let mut task_errors = 0;
                let task_start = Instant::now();

                for op_id in 0..operations_per_task {
                    let memory = MemoryItem {
                        memory_id: format!("concurrent_{}_{}", task_id, op_id),
                        memory_type: MemoryType::Experience,
                        content: format!("Concurrent operation {} {}", task_id, op_id),
                        metadata: HashMap::new(),
                        importance: 0.5,
                        created_at: Utc::now(),
                        last_accessed: Utc::now(),
                        access_count: 1,
                        tags: vec![format!("concurrent_{}", task_id)],
                        embedding: None,
                    };

                    match task_store.store(memory).await {
                        Ok(_) => task_success += 1,
                        Err(_) => task_errors += 1,
                    }
                }

                let task_duration = task_start.elapsed();
                (task_success, task_errors, task_duration)
            });
            handles.push(handle);
        }
        
        // Collect results
        let mut total_success = 0;
        let mut total_errors = 0;
        let mut max_task_time = Duration::from_secs(0);
        
        for handle in handles {
            let (success, errors, task_time) = handle.await?;
            total_success += success;
            total_errors += errors;
            max_task_time = max_task_time.max(task_time);
        }
        
        let total_time = start_time.elapsed();
        let total_operations = concurrency * operations_per_task;
        
        println!("  Total operations: {}", total_operations);
        println!("  Successful: {}", total_success);
        println!("  Errors: {}", total_errors);
        println!("  Total time: {:?}", total_time);
        println!("  Max task time: {:?}", max_task_time);
        println!("  Throughput: {:.2} ops/sec", total_operations as f64 / total_time.as_secs_f64());
        println!("  Success rate: {:.2}%", (total_success as f64 / total_operations as f64) * 100.0);
        
        // Performance assertions
        assert!(total_time < Duration::from_secs(120), "Concurrency {} too slow", concurrency);
        assert!(total_success > total_operations * 8 / 10, "Concurrency {} too many failures", concurrency);
    }
    
    Ok(())
}

#[tokio::test]
async fn benchmark_query_patterns() -> Result<()> {
    println!("=== Query Patterns Benchmark ===");
    
    let store = SqliteMemoryStore::new(":memory:")?;
    
    // Populate with diverse test data
    let num_memories = 10000;
    println!("Populating {} memories for query benchmarking...", num_memories);
    
    for i in 0..num_memories {
        let memory = MemoryItem {
            memory_id: format!("query_bench_{}", i),
            memory_type: match i % 4 {
                0 => MemoryType::Experience,
                1 => MemoryType::Fact,
                2 => MemoryType::Experience,
                _ => MemoryType::Fact,
            },
            content: format!("Query benchmark memory {} about topic {} in category {}", 
                           i, i % 50, i % 10),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("topic".to_string(), serde_json::Value::Number((i % 50).into()));
                meta.insert("category".to_string(), serde_json::Value::Number((i % 10).into()));
                meta
            },
            importance: (i as f64) / (num_memories as f64),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: (i % 20) as u32 + 1,
            tags: vec![
                format!("topic_{}", i % 50),
                format!("category_{}", i % 10),
                format!("batch_{}", i / 1000),
            ],
            embedding: None,
        };
        
        store.store(memory).await?;
    }
    
    // Benchmark different query patterns
    let query_benchmarks = vec![
        ("Full scan", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(1000),
            tags: vec![],
        }),
        ("High importance filter", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.9),
            limit: Some(1000),
            tags: vec![],
        }),
        ("Type filter", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![MemoryType::Experience],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(1000),
            tags: vec![],
        }),
        ("Tag filter", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(1000),
            tags: vec!["topic_25".to_string()],
        }),
        ("Combined filters", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![MemoryType::Experience],
            time_range: None,
            importance_threshold: Some(0.5),
            limit: Some(500),
            tags: vec!["category_5".to_string()],
        }),
        ("Small limit", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(10),
            tags: vec![],
        }),
        ("Large limit", MemoryQuery {
            query_text: "".to_string(),
            memory_types: vec![],
            time_range: None,
            importance_threshold: Some(0.0),
            limit: Some(5000),
            tags: vec![],
        }),
    ];
    
    for (query_name, query) in query_benchmarks {
        let query_start = Instant::now();
        let results = store.retrieve(&query).await?;
        let query_time = query_start.elapsed();
        
        println!("  {}: {} results in {:?} ({:.2} results/ms)", 
                 query_name, results.len(), query_time, 
                 results.len() as f64 / query_time.as_millis() as f64);
        
        // Performance assertion
        assert!(query_time < Duration::from_secs(5), "Query '{}' too slow", query_name);
    }
    
    Ok(())
}

#[tokio::test]
async fn benchmark_memory_usage_patterns() -> Result<()> {
    println!("=== Memory Usage Patterns Benchmark ===");
    
    let mut tracker = MemoryTracker::new();
    let initial_usage = tracker.get_current_usage();
    
    // Test different memory allocation patterns
    let patterns = vec![
        ("Many small stores", 10000, 100),
        ("Few large stores", 100, 10000),
        ("Mixed sizes", 1000, 1000),
    ];
    
    for (pattern_name, num_items, content_size) in patterns {
        println!("\nTesting pattern: {}", pattern_name);
        
        let store = SqliteMemoryStore::new(":memory:")?;
        let pattern_start = Instant::now();
        let start_usage = tracker.get_current_usage();
        
        for i in 0..num_items {
            let memory = MemoryItem {
                memory_id: format!("pattern_{}_{}", pattern_name.replace(" ", "_"), i),
                memory_type: MemoryType::Experience,
                content: "x".repeat(content_size),
                metadata: HashMap::new(),
                importance: 0.5,
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 1,
                tags: vec![],
                embedding: None,
            };
            
            store.store(memory).await?;
            
            if i % (num_items / 10).max(1) == 0 {
                let current_usage = tracker.get_current_usage();
                println!("    Progress: {}/{}, Memory: {} bytes", i, num_items, current_usage);
            }
        }
        
        let pattern_time = pattern_start.elapsed();
        let end_usage = tracker.get_current_usage();
        let peak_usage = tracker.get_peak_usage();
        
        println!("  Pattern completed in {:?}", pattern_time);
        println!("  Memory usage: {} -> {} bytes (peak: {})", start_usage, end_usage, peak_usage);
        println!("  Memory per item: {} bytes", (end_usage - start_usage) / num_items as u64);
        println!("  Items per second: {:.2}", num_items as f64 / pattern_time.as_secs_f64());
        
        // Performance assertions
        assert!(pattern_time < Duration::from_secs(300), "Pattern '{}' too slow", pattern_name);
    }
    
    let final_usage = tracker.get_current_usage();
    println!("\nOverall memory usage: {} -> {} bytes", initial_usage, final_usage);
    
    Ok(())
}
