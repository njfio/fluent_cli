use fluent_engines::{
    optimized_openai::OptimizedOpenAIEngine,
    enhanced_cache::{EnhancedCache, CacheConfig},
};
use fluent_core::{
    config::{EngineConfig, ConnectionConfig},
};
use std::time::Duration;
use std::collections::HashMap;
use anyhow::Result;

/// Simplified async tests for engine operations
/// Tests focus on creation and basic functionality without complex API calls

#[tokio::test]
async fn test_async_engine_creation() -> Result<()> {
    // Test basic async engine creation
    let mut parameters = HashMap::new();
    parameters.insert("bearer_token".to_string(), serde_json::json!("test-token"));
    parameters.insert("model".to_string(), serde_json::json!("gpt-3.5-turbo"));

    let config = EngineConfig {
        name: "test".to_string(),
        engine: "openai".to_string(),
        connection: ConnectionConfig {
            protocol: "https".to_string(),
            hostname: "api.openai.com".to_string(),
            port: 443,
            request_path: "/v1/chat/completions".to_string(),
        },
        parameters,
        session_id: None,
        neo4j: None,
        spinner: None,
    };

    let engine = OptimizedOpenAIEngine::new(config).await?;

    // If we get here, the engine was created successfully
    println!("✅ OptimizedOpenAIEngine created successfully");

    Ok(())
}

#[tokio::test]
async fn test_async_cache_creation() -> Result<()> {
    // Test async cache creation
    let cache_config = CacheConfig {
        memory_cache_size: 100,
        ttl: Duration::from_secs(300),
        enable_disk_cache: false, // Disable for testing
        disk_cache_dir: None,
        enable_compression: true,
        max_entry_size: 1024,
        cache_errors: false,
    };

    let cache = EnhancedCache::new(cache_config)?;

    println!("✅ EnhancedCache created successfully");

    // Test cache stats
    let stats = cache.get_stats();
    assert_eq!(stats.memory_hits, 0); // Should start with 0 hits

    Ok(())
}




