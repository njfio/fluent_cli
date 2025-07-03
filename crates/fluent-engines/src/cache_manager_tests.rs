#[cfg(test)]
mod comprehensive_cache_tests {
    use crate::cache_manager::*;
    use crate::enhanced_cache::{CacheEntry, CacheKey};
    use fluent_core::types::{Cost, Request, Response, Usage};
    use serde_json::json;
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_request() -> Request {
        Request {
            flowname: "test-flow".to_string(),
            payload: "test payload for caching".to_string(),
        }
    }

    fn create_test_response() -> Response {
        Response {
            content: "test response content".to_string(),
            usage: Usage {
                prompt_tokens: 15,
                completion_tokens: 25,
                total_tokens: 40,
            },
            model: "test-model-v1".to_string(),
            finish_reason: Some("stop".to_string()),
            cost: Cost {
                prompt_cost: 0.015,
                completion_cost: 0.025,
                total_cost: 0.040,
            },
        }
    }

    fn create_test_parameters() -> HashMap<String, serde_json::Value> {
        let mut params = HashMap::new();
        params.insert("temperature".to_string(), json!(0.7));
        params.insert("max_tokens".to_string(), json!(1000));
        params.insert("top_p".to_string(), json!(0.9));
        params
    }

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let manager = CacheManager::new();
        assert!(manager.caches.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_cache_manager_enabled_check() {
        // Test with caching enabled
        std::env::set_var("FLUENT_CACHE", "1");
        let manager = CacheManager::new();
        assert!(manager.is_caching_enabled());

        // Test with caching disabled
        std::env::set_var("FLUENT_CACHE", "0");
        let manager = CacheManager::new();
        assert!(!manager.is_caching_enabled());

        // Test with environment variable not set
        std::env::remove_var("FLUENT_CACHE");
        let manager = CacheManager::new();
        assert!(!manager.is_caching_enabled());

        // Cleanup
        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_operations_basic() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();

        // Should be cache miss initially
        let cached = manager
            .get_cached_response("test_engine", &request, Some("test-model"), None)
            .await
            .unwrap();
        assert!(cached.is_none());

        // Cache the response
        manager
            .cache_response("test_engine", &request, &response, Some("test-model"), None)
            .await
            .unwrap();

        // Should be cache hit now
        let cached = manager
            .get_cached_response("test_engine", &request, Some("test-model"), None)
            .await
            .unwrap();
        assert!(cached.is_some());
        let cached_response = cached.unwrap();
        assert_eq!(cached_response.content, "test response content");
        assert_eq!(cached_response.model, "test-model-v1");
        assert_eq!(cached_response.usage.total_tokens, 40);

        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_operations_with_parameters() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();
        let parameters = create_test_parameters();

        // Cache with parameters
        manager
            .cache_response(
                "test_engine",
                &request,
                &response,
                Some("test-model"),
                Some(&parameters),
            )
            .await
            .unwrap();

        // Should hit cache with same parameters
        let cached = manager
            .get_cached_response(
                "test_engine",
                &request,
                Some("test-model"),
                Some(&parameters),
            )
            .await
            .unwrap();
        assert!(cached.is_some());

        // Should miss cache with different parameters
        let mut different_params = parameters.clone();
        different_params.insert("temperature".to_string(), json!(0.5));
        let cached_different = manager
            .get_cached_response(
                "test_engine",
                &request,
                Some("test-model"),
                Some(&different_params),
            )
            .await
            .unwrap();
        assert!(cached_different.is_none());

        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_operations_different_engines() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();

        // Cache for engine1
        manager
            .cache_response("engine1", &request, &response, Some("model1"), None)
            .await
            .unwrap();

        // Should hit cache for engine1
        let cached_engine1 = manager
            .get_cached_response("engine1", &request, Some("model1"), None)
            .await
            .unwrap();
        assert!(cached_engine1.is_some());

        // Should miss cache for engine2 (different engine)
        let cached_engine2 = manager
            .get_cached_response("engine2", &request, Some("model1"), None)
            .await
            .unwrap();
        assert!(cached_engine2.is_none());

        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_operations_different_models() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();

        // Cache for model1
        manager
            .cache_response("test_engine", &request, &response, Some("model1"), None)
            .await
            .unwrap();

        // Should hit cache for model1
        let cached_model1 = manager
            .get_cached_response("test_engine", &request, Some("model1"), None)
            .await
            .unwrap();
        assert!(cached_model1.is_some());

        // Should miss cache for model2 (different model)
        let cached_model2 = manager
            .get_cached_response("test_engine", &request, Some("model2"), None)
            .await
            .unwrap();
        assert!(cached_model2.is_none());

        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_operations_disabled() {
        std::env::set_var("FLUENT_CACHE", "0");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();

        // Should not cache when disabled
        let result = manager
            .cache_response("test_engine", &request, &response, Some("test-model"), None)
            .await;
        assert!(result.is_ok());

        // Should always be cache miss when disabled
        let cached = manager
            .get_cached_response("test_engine", &request, Some("test-model"), None)
            .await
            .unwrap();
        assert!(cached.is_none());

        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let request = create_test_request();
        let parameters = create_test_parameters();

        // Test basic cache key
        let key1 = CacheKey::new(&request.payload, "test_engine");
        let key2 = CacheKey::new(&request.payload, "test_engine");
        assert_eq!(key1.generate(), key2.generate());

        // Test cache key with model
        let key_with_model1 = CacheKey::new(&request.payload, "test_engine").with_model("model1");
        let key_with_model2 = CacheKey::new(&request.payload, "test_engine").with_model("model2");
        assert_ne!(key_with_model1.generate(), key_with_model2.generate());

        // Test cache key with parameters
        let key_with_params1 =
            CacheKey::new(&request.payload, "test_engine").with_parameters(&parameters);
        let mut different_params = parameters.clone();
        different_params.insert("new_param".to_string(), json!("new_value"));
        let key_with_params2 =
            CacheKey::new(&request.payload, "test_engine").with_parameters(&different_params);
        assert_ne!(key_with_params1.generate(), key_with_params2.generate());
    }

    #[tokio::test]
    async fn test_global_cache_functions() {
        std::env::set_var("FLUENT_CACHE", "1");

        let request = create_test_request();
        let response = create_test_response();

        // Test global cache function
        let result = cache_response(
            "global_test_engine",
            &request,
            &response,
            Some("test-model"),
            None,
        )
        .await;
        assert!(result.is_ok());

        // Test global get function
        let cached = get_cached_response("global_test_engine", &request, Some("test-model"), None)
            .await
            .unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "test response content");

        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_entry_expiration() {
        let response = create_test_response();
        let ttl = Duration::from_millis(100); // Very short TTL for testing

        let entry = CacheEntry::new(response.clone(), ttl);
        assert!(!entry.is_expired());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(entry.is_expired());
    }

    #[tokio::test]
    async fn test_cache_multiple_engines() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();

        // Cache for multiple engines
        let engines = vec!["openai", "anthropic", "cohere"];
        for engine in &engines {
            manager
                .cache_response(engine, &request, &response, Some("test-model"), None)
                .await
                .unwrap();
        }

        // Verify all engines have cached responses
        for engine in &engines {
            let cached = manager
                .get_cached_response(engine, &request, Some("test-model"), None)
                .await
                .unwrap();
            assert!(cached.is_some());
        }

        // Verify cache manager has created separate caches for each engine
        let caches = manager.caches.read().await;
        assert_eq!(caches.len(), engines.len());

        std::env::remove_var("FLUENT_CACHE");
    }

    #[tokio::test]
    async fn test_cache_error_handling() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = Request {
            flowname: "test".to_string(),
            payload: "".to_string(), // Empty payload
        };
        let response = create_test_response();

        // Should handle empty payload gracefully
        let result = manager
            .cache_response("test_engine", &request, &response, Some("test-model"), None)
            .await;
        assert!(result.is_ok());

        let cached = manager
            .get_cached_response("test_engine", &request, Some("test-model"), None)
            .await
            .unwrap();
        assert!(cached.is_some());

        std::env::remove_var("FLUENT_CACHE");
    }

    #[test]
    fn test_cache_key_consistency() {
        let payload = "test payload";
        let engine = "test_engine";
        let model = "test_model";
        let mut params = HashMap::new();
        params.insert("temp".to_string(), json!(0.7));

        // Generate multiple keys with same inputs
        let key1 = CacheKey::new(payload, engine)
            .with_model(model)
            .with_parameters(&params)
            .generate();
        let key2 = CacheKey::new(payload, engine)
            .with_model(model)
            .with_parameters(&params)
            .generate();
        let key3 = CacheKey::new(payload, engine)
            .with_model(model)
            .with_parameters(&params)
            .generate();

        // All keys should be identical
        assert_eq!(key1, key2);
        assert_eq!(key2, key3);
        assert_eq!(key1, key3);
    }

    #[test]
    fn test_cache_key_sensitivity() {
        let payload = "test payload";
        let engine = "test_engine";

        // Test case sensitivity
        let key_lower = CacheKey::new(payload, engine)
            .with_model("model")
            .generate();
        let key_upper = CacheKey::new(payload, engine)
            .with_model("MODEL")
            .generate();
        assert_ne!(key_lower, key_upper);

        // Test whitespace sensitivity
        let key_normal = CacheKey::new(payload, engine).generate();
        let key_spaces = CacheKey::new("test  payload", engine).generate();
        assert_ne!(key_normal, key_spaces);
    }

    #[tokio::test]
    async fn test_concurrent_cache_operations() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();

        // Perform concurrent cache operations
        let mut handles = vec![];
        for i in 0..10 {
            let manager_clone = manager.clone();
            let request_clone = request.clone();
            let response_clone = response.clone();
            let engine_name = format!("engine_{}", i);

            let handle = tokio::spawn(async move {
                manager_clone
                    .cache_response(
                        &engine_name,
                        &request_clone,
                        &response_clone,
                        Some("test-model"),
                        None,
                    )
                    .await
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Verify all caches were created
        let caches = manager.caches.read().await;
        assert_eq!(caches.len(), 10);

        std::env::remove_var("FLUENT_CACHE");
    }
}
