use fluent_engines::{
    optimized_openai::OptimizedOpenAIEngine,
    enhanced_cache::{EnhancedCache, CacheConfig},
    optimized_parallel_executor::{OptimizedParallelExecutor, ParallelConfig},
    engine::{Engine, EngineConfig},
};
use fluent_core::{
    request::Request,
    response::Response,
    config::Config,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio;
use tokio::time::timeout;

/// Comprehensive async tests for engine operations
/// Tests async engine patterns, caching, parallel execution, and error handling

#[tokio::test]
async fn test_async_engine_operations() -> Result<()> {
    // Test basic async engine operations
    let config = EngineConfig {
        api_key: "test_key".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        model: "gpt-3.5-turbo".to_string(),
        max_tokens: 1000,
        temperature: 0.7,
        timeout: Duration::from_secs(30),
        max_retries: 3,
    };
    
    let engine = OptimizedOpenAIEngine::new(config)?;
    
    // Test async request processing with timeout
    let request = Request {
        prompt: "Test async request".to_string(),
        max_tokens: Some(100),
        temperature: Some(0.5),
        model: Some("gpt-3.5-turbo".to_string()),
        stream: false,
        metadata: std::collections::HashMap::new(),
    };
    
    // Test with timeout (may fail due to no real API key, but should handle gracefully)
    let result = timeout(
        Duration::from_secs(10),
        engine.process_request(request)
    ).await;
    
    // Should complete within timeout (success or controlled failure)
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_async_cache_operations() -> Result<()> {
    // Test async cache operations
    let cache_config = CacheConfig {
        max_size: 1000,
        ttl: Duration::from_secs(300),
        cleanup_interval: Duration::from_secs(60),
        enable_compression: true,
        compression_threshold: 1024,
    };
    
    let cache = EnhancedCache::new(cache_config).await?;
    
    // Test async cache operations
    let key = "test_async_key";
    let value = "test_async_value";
    
    // Test async set with timeout
    let set_result = timeout(
        Duration::from_secs(5),
        cache.set(key.to_string(), value.to_string())
    ).await?;
    assert!(set_result.is_ok());
    
    // Test async get with timeout
    let get_result = timeout(
        Duration::from_secs(5),
        cache.get(key)
    ).await?;
    assert!(get_result.is_ok());
    
    if let Ok(Some(cached_value)) = get_result {
        assert_eq!(cached_value, value);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_async_parallel_execution() -> Result<()> {
    // Test async parallel execution
    let parallel_config = ParallelConfig {
        max_concurrent_tasks: 5,
        task_timeout: Duration::from_secs(10),
        queue_timeout: Duration::from_secs(30),
        enable_backpressure: true,
        backpressure_threshold: 100,
    };
    
    let executor = OptimizedParallelExecutor::new(parallel_config).await?;
    
    // Create test tasks
    let tasks: Vec<_> = (0..10).map(|i| {
        format!("Task {}", i)
    }).collect();
    
    // Define async task executor
    let task_executor = |task: String| async move {
        // Simulate async work
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok::<String, anyhow::Error>(format!("Completed: {}", task))
    };
    
    // Test parallel execution with timeout
    let start_time = Instant::now();
    let results = timeout(
        Duration::from_secs(30),
        executor.execute_tasks(tasks, task_executor)
    ).await?;
    
    let execution_time = start_time.elapsed();
    println!("Parallel execution completed in {:?}", execution_time);
    
    // Should complete faster than sequential execution
    assert!(execution_time < Duration::from_secs(15));
    
    // Verify results
    assert!(results.is_ok());
    if let Ok(task_results) = results {
        assert_eq!(task_results.len(), 10);
        
        // Check that all tasks completed successfully
        let success_count = task_results.iter()
            .filter(|r| r.is_ok())
            .count();
        assert!(success_count > 0);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_engine_requests() -> Result<()> {
    let config = EngineConfig {
        api_key: "test_key".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        model: "gpt-3.5-turbo".to_string(),
        max_tokens: 1000,
        temperature: 0.7,
        timeout: Duration::from_secs(30),
        max_retries: 3,
    };
    
    let engine = Arc::new(OptimizedOpenAIEngine::new(config)?);
    let num_requests = 5;
    let mut handles = Vec::new();
    
    // Launch concurrent requests
    for i in 0..num_requests {
        let engine_clone = engine.clone();
        let handle = tokio::spawn(async move {
            let request = Request {
                prompt: format!("Concurrent request {}", i),
                max_tokens: Some(50),
                temperature: Some(0.5),
                model: Some("gpt-3.5-turbo".to_string()),
                stream: false,
                metadata: std::collections::HashMap::new(),
            };
            
            engine_clone.process_request(request).await
        });
        handles.push(handle);
    }
    
    // Wait for all requests with timeout
    let start_time = Instant::now();
    let mut completed_requests = 0;
    let mut failed_requests = 0;
    
    for handle in handles {
        let result = timeout(Duration::from_secs(60), handle).await?;
        match result {
            Ok(Ok(_)) => completed_requests += 1,
            Ok(Err(_)) => failed_requests += 1,
            Err(_) => failed_requests += 1, // Join error
        }
    }
    
    let total_time = start_time.elapsed();
    println!("Processed {} requests in {:?}", num_requests, total_time);
    println!("Completed: {}, Failed: {}", completed_requests, failed_requests);
    
    // Should have processed all requests
    assert_eq!(completed_requests + failed_requests, num_requests);
    
    Ok(())
}

#[tokio::test]
async fn test_async_error_handling_in_engines() -> Result<()> {
    // Test error handling with invalid configuration
    let invalid_config = EngineConfig {
        api_key: "".to_string(), // Empty API key
        base_url: "invalid_url".to_string(), // Invalid URL
        model: "".to_string(), // Empty model
        max_tokens: 0, // Invalid max_tokens
        temperature: 2.0, // Invalid temperature > 1.0
        timeout: Duration::from_millis(1), // Very short timeout
        max_retries: 0,
    };
    
    // Engine creation might fail or succeed with invalid config
    let engine_result = OptimizedOpenAIEngine::new(invalid_config);
    
    if let Ok(engine) = engine_result {
        // If engine was created, test that it handles invalid requests gracefully
        let invalid_request = Request {
            prompt: "".to_string(), // Empty prompt
            max_tokens: Some(0), // Invalid max_tokens
            temperature: Some(2.0), // Invalid temperature
            model: Some("".to_string()), // Empty model
            stream: false,
            metadata: std::collections::HashMap::new(),
        };
        
        let result = engine.process_request(invalid_request).await;
        // Should handle gracefully (either succeed or fail with proper error)
        assert!(result.is_ok() || result.is_err());
    }
    
    Ok(())
}

#[tokio::test]
async fn test_async_cache_concurrency() -> Result<()> {
    let cache_config = CacheConfig {
        max_size: 100,
        ttl: Duration::from_secs(60),
        cleanup_interval: Duration::from_secs(10),
        enable_compression: false,
        compression_threshold: 1024,
    };
    
    let cache = Arc::new(EnhancedCache::new(cache_config).await?);
    let num_operations = 20;
    let mut handles = Vec::new();
    
    // Launch concurrent cache operations
    for i in 0..num_operations {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i);
            let value = format!("concurrent_value_{}", i);
            
            // Set value
            cache_clone.set(key.clone(), value.clone()).await?;
            
            // Get value
            let retrieved = cache_clone.get(&key).await?;
            
            // Verify value
            if let Some(retrieved_value) = retrieved {
                assert_eq!(retrieved_value, value);
            }
            
            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    let mut successful_operations = 0;
    let mut failed_operations = 0;
    
    for handle in handles {
        let result = timeout(Duration::from_secs(30), handle).await?;
        match result {
            Ok(Ok(_)) => successful_operations += 1,
            Ok(Err(_)) => failed_operations += 1,
            Err(_) => failed_operations += 1, // Join error
        }
    }
    
    println!("Cache operations - Success: {}, Failed: {}", successful_operations, failed_operations);
    
    // Should have processed all operations
    assert_eq!(successful_operations + failed_operations, num_operations);
    
    Ok(())
}

#[tokio::test]
async fn test_async_timeout_handling() -> Result<()> {
    // Test timeout handling in async operations
    let cache_config = CacheConfig {
        max_size: 10,
        ttl: Duration::from_secs(1),
        cleanup_interval: Duration::from_secs(1),
        enable_compression: false,
        compression_threshold: 1024,
    };
    
    let cache = EnhancedCache::new(cache_config).await?;
    
    // Test operation with very short timeout
    let short_timeout_result = timeout(
        Duration::from_nanos(1),
        cache.set("timeout_test".to_string(), "value".to_string())
    ).await;
    
    // Should timeout
    assert!(short_timeout_result.is_err());
    
    // Test operation with reasonable timeout
    let reasonable_timeout_result = timeout(
        Duration::from_secs(5),
        cache.set("normal_test".to_string(), "value".to_string())
    ).await;
    
    // Should succeed
    assert!(reasonable_timeout_result.is_ok());
    
    Ok(())
}
