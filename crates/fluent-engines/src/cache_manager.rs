use crate::enhanced_cache::{CacheConfig, CacheKey, EnhancedCache};
use anyhow::Result;
use fluent_core::types::{Request, Response};
use log::debug;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Centralized cache manager for all engines
#[derive(Clone)]
pub struct CacheManager {
    caches: Arc<RwLock<HashMap<String, Arc<EnhancedCache>>>>,
    default_config: CacheConfig,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            caches: Arc::new(RwLock::new(HashMap::new())),
            default_config: CacheConfig::default(),
        }
    }

    /// Create a cache manager with custom config
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            caches: Arc::new(RwLock::new(HashMap::new())),
            default_config: config,
        }
    }

    /// Get or create a cache for a specific engine
    pub async fn get_cache(&self, engine_name: &str) -> Result<Arc<EnhancedCache>> {
        // Check if cache already exists
        {
            let caches = self.caches.read().await;
            if let Some(cache) = caches.get(engine_name) {
                return Ok(cache.clone());
            }
        }

        // Create new cache
        let mut config = self.default_config.clone();
        config.disk_cache_dir = Some(format!("fluent_cache_{}", engine_name));

        let cache = Arc::new(EnhancedCache::new(config)?);

        // Store in map
        {
            let mut caches = self.caches.write().await;
            caches.insert(engine_name.to_string(), cache.clone());
        }

        debug!("Created cache for engine: {}", engine_name);
        Ok(cache)
    }

    /// Check cache for a request
    pub async fn get_cached_response(
        &self,
        engine_name: &str,
        request: &Request,
        model: Option<&str>,
        parameters: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<Option<Response>> {
        if !self.is_caching_enabled() {
            return Ok(None);
        }

        let cache = self.get_cache(engine_name).await?;
        let mut cache_key = CacheKey::new(&request.payload, engine_name);

        if let Some(model) = model {
            cache_key = cache_key.with_model(model);
        }

        if let Some(params) = parameters {
            cache_key = cache_key.with_parameters(params);
        }

        match cache.get(&cache_key).await {
            Ok(Some(response)) => {
                debug!("Cache hit for {} request", engine_name);
                Ok(Some(response))
            }
            Ok(None) => {
                debug!("Cache miss for {} request", engine_name);
                Ok(None)
            }
            Err(e) => {
                debug!("Cache error for {} request: {}", engine_name, e);
                Ok(None) // Don't fail the request due to cache errors
            }
        }
    }

    /// Store a response in cache
    pub async fn cache_response(
        &self,
        engine_name: &str,
        request: &Request,
        response: &Response,
        model: Option<&str>,
        parameters: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        if !self.is_caching_enabled() {
            return Ok(());
        }

        let cache = self.get_cache(engine_name).await?;
        let mut cache_key = CacheKey::new(&request.payload, engine_name);

        if let Some(model) = model {
            cache_key = cache_key.with_model(model);
        }

        if let Some(params) = parameters {
            cache_key = cache_key.with_parameters(params);
        }

        match cache.insert(&cache_key, response).await {
            Ok(()) => {
                debug!("Cached response for {} request", engine_name);
                Ok(())
            }
            Err(e) => {
                debug!("Failed to cache response for {}: {}", engine_name, e);
                Ok(()) // Don't fail the request due to cache errors
            }
        }
    }

    /// Check if caching is enabled via environment variable
    fn is_caching_enabled(&self) -> bool {
        std::env::var("FLUENT_CACHE").ok().as_deref() == Some("1")
    }

    /// Get cache statistics for all engines
    pub async fn get_statistics(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        let caches = self.caches.read().await;

        for (engine_name, cache) in caches.iter() {
            let cache_stats = cache.get_stats();
            stats.insert(
                engine_name.clone(),
                serde_json::to_value(cache_stats).unwrap_or_default(),
            );
        }

        stats
    }

    /// Clear all caches
    pub async fn clear_all(&self) -> Result<()> {
        let caches = self.caches.read().await;

        for (engine_name, cache) in caches.iter() {
            if let Err(e) = cache.clear().await {
                debug!("Failed to clear cache for {}: {}", engine_name, e);
            }
        }

        Ok(())
    }

    /// Cleanup expired entries in all caches
    pub async fn cleanup_expired(&self) -> Result<()> {
        let caches = self.caches.read().await;

        for (engine_name, cache) in caches.iter() {
            if let Err(e) = cache.cleanup_expired().await {
                debug!(
                    "Failed to cleanup expired entries for {}: {}",
                    engine_name, e
                );
            }
        }

        Ok(())
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global cache manager instance
static CACHE_MANAGER: tokio::sync::OnceCell<Arc<CacheManager>> = tokio::sync::OnceCell::const_new();

/// Get the global cache manager instance
pub async fn global_cache_manager() -> Arc<CacheManager> {
    CACHE_MANAGER
        .get_or_init(|| async { Arc::new(CacheManager::new()) })
        .await
        .clone()
}

/// Convenience function to check cache for any engine
pub async fn get_cached_response(
    engine_name: &str,
    request: &Request,
    model: Option<&str>,
    parameters: Option<&HashMap<String, serde_json::Value>>,
) -> Result<Option<Response>> {
    let manager = global_cache_manager().await;
    manager
        .get_cached_response(engine_name, request, model, parameters)
        .await
}

/// Convenience function to cache response for any engine
pub async fn cache_response(
    engine_name: &str,
    request: &Request,
    response: &Response,
    model: Option<&str>,
    parameters: Option<&HashMap<String, serde_json::Value>>,
) -> Result<()> {
    let manager = global_cache_manager().await;
    manager
        .cache_response(engine_name, request, response, model, parameters)
        .await
}

/// Start background task for cache maintenance
pub fn start_cache_maintenance() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // Every 5 minutes

        loop {
            interval.tick().await;

            let manager = global_cache_manager().await;
            if let Err(e) = manager.cleanup_expired().await {
                debug!("Cache maintenance error: {}", e);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::types::{Cost, Usage};

    fn create_test_request() -> Request {
        Request {
            flowname: "test".to_string(),
            payload: "test payload".to_string(),
        }
    }

    fn create_test_response() -> Response {
        Response {
            content: "test response".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            model: "test-model".to_string(),
            finish_reason: Some("stop".to_string()),
            cost: Cost {
                prompt_cost: 0.01,
                completion_cost: 0.02,
                total_cost: 0.03,
            },
        }
    }

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let manager = CacheManager::new();
        assert!(manager.caches.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_cache_operations() {
        std::env::set_var("FLUENT_CACHE", "1");

        let manager = CacheManager::new();
        let request = create_test_request();
        let response = create_test_response();
        let engine_name = format!("test_engine_ops_{}", uuid::Uuid::new_v4());

        // Should be cache miss initially
        let cached = manager
            .get_cached_response(&engine_name, &request, Some("test-model"), None)
            .await
            .unwrap();
        assert!(cached.is_none());

        // Cache the response
        manager
            .cache_response(&engine_name, &request, &response, Some("test-model"), None)
            .await
            .unwrap();

        // Should be cache hit now
        let cached = manager
            .get_cached_response(&engine_name, &request, Some("test-model"), None)
            .await
            .unwrap();
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().content, "test response");

        std::env::remove_var("FLUENT_CACHE");
    }
}

// Include comprehensive test suite
#[path = "cache_manager_tests.rs"]
mod cache_manager_tests;
