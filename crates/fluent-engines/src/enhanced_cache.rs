//! # Enhanced Cache System
//!
//! This module provides a high-performance two-tier caching system for LLM responses with
//! automatic expiration, LRU eviction, and optional disk persistence.
//!
//! ## Cache Keying Strategy
//!
//! Cache keys are generated from multiple components to ensure accurate cache hits:
//!
//! - **Engine name**: The LLM provider (e.g., "openai", "anthropic", "cohere")
//! - **Request payload**: SHA-256 hash of the prompt/content
//! - **Model identifier**: Optional model name (e.g., "gpt-4", "claude-3")
//! - **File hash**: Optional file path and modification time for file-based requests
//! - **Parameters hash**: SHA-256 hash of model parameters (temperature, max_tokens, etc.)
//!
//! Keys are constructed as: `engine:payload_hash:model:model_name:params:params_hash`
//!
//! ### Example
//! ```text
//! openai:a3f2c1...:model:gpt-4:params:b7e4d2...
//! ```
//!
//! ## TTL (Time-To-Live) Behavior
//!
//! - **Default TTL**: 3600 seconds (1 hour)
//! - **Per-entry TTL**: Each cache entry stores its own TTL for flexible expiration
//! - **Expiration check**: Entries are validated on access via `is_expired()` method
//! - **Automatic removal**: Expired entries are evicted during:
//!   - Cache lookups (lazy expiration)
//!   - Background cleanup task (runs every 5 minutes)
//!   - Manual cleanup via `cleanup_expired()`
//!
//! ## Invalidation Strategy
//!
//! The cache supports multiple invalidation mechanisms:
//!
//! ### Automatic Invalidation
//! - **TTL expiration**: Entries automatically expire after their TTL period
//! - **LRU eviction**: Least recently used entries are evicted when memory limit is reached
//! - **Size-based eviction**: Entries exceeding `max_entry_size` are not cached
//!
//! ### Manual Invalidation
//! - **Clear all**: `cache.clear()` removes all entries from both memory and disk
//! - **Cleanup expired**: `cache.cleanup_expired()` removes only expired entries
//!
//! ## Size Limits and Eviction
//!
//! ### Memory Cache
//! - **Maximum entries**: Configurable via `memory_cache_size` (default: 1000)
//! - **Eviction policy**: LRU (Least Recently Used)
//! - **Entry size limit**: Individual entries cannot exceed `max_entry_size` (default: 1MB)
//!
//! ### Disk Cache
//! - **Optional persistence**: Enable/disable via `enable_disk_cache` (default: true)
//! - **Compression**: Optional LZ4 compression via `enable_compression` (default: true)
//! - **Storage location**: Configurable directory (default: "fluent_cache")
//!
//! ## Cache Statistics
//!
//! The cache tracks comprehensive metrics:
//! - Memory hits/misses
//! - Disk hits/misses
//! - Total entries count
//! - Memory and disk size usage
//! - Eviction count
//! - Error count
//! - Hit rates (overall and memory-only)
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use fluent_engines::enhanced_cache::{CacheConfig, CacheKey, EnhancedCache};
//! use std::time::Duration;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create cache with custom config
//! let config = CacheConfig {
//!     memory_cache_size: 500,
//!     ttl: Duration::from_secs(1800), // 30 minutes
//!     enable_disk_cache: true,
//!     ..Default::default()
//! };
//! let cache = EnhancedCache::new(config)?;
//!
//! // Generate cache key
//! let key = CacheKey::new("What is Rust?", "openai")
//!     .with_model("gpt-4")
//!     .with_parameters(&params);
//!
//! // Try to get from cache
//! if let Some(response) = cache.get(&key).await? {
//!     println!("Cache hit!");
//! } else {
//!     // Cache miss - make API call and cache result
//!     let response = make_llm_request().await?;
//!     cache.insert(&key, &response).await?;
//! }
//!
//! // Get statistics
//! let stats = cache.get_stats();
//! println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use fluent_core::types::Response;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Enhanced cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in memory cache
    pub memory_cache_size: usize,
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Whether to enable persistent disk cache
    pub enable_disk_cache: bool,
    /// Directory for disk cache
    pub disk_cache_dir: Option<String>,
    /// Whether to enable compression for disk cache
    pub enable_compression: bool,
    /// Maximum size of individual cache entries (in bytes)
    pub max_entry_size: usize,
    /// Whether to cache error responses
    pub cache_errors: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            memory_cache_size: 1000,
            ttl: Duration::from_secs(3600), // 1 hour
            enable_disk_cache: true,
            disk_cache_dir: None, // Will use default
            enable_compression: true,
            max_entry_size: 1024 * 1024, // 1MB
            cache_errors: false,
        }
    }
}

/// Cache entry with metadata
///
/// Stores a cached response along with tracking metadata for expiration,
/// access patterns, and size information.
///
/// ## Fields
/// - `response`: The cached LLM response
/// - `created_at`: Unix timestamp when entry was created
/// - `access_count`: Number of times this entry has been accessed
/// - `last_accessed`: Unix timestamp of most recent access
/// - `size_bytes`: Serialized size of the entry in bytes
/// - `ttl_seconds`: Time-to-live duration in seconds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    response: Response,
    created_at: u64, // Unix timestamp
    access_count: u64,
    last_accessed: u64,
    size_bytes: usize,
    ttl_seconds: u64, // TTL in seconds
}

impl CacheEntry {
    pub fn new(response: Response, ttl: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let size_bytes = serde_json::to_string(&response)
            .map(|s| s.len())
            .unwrap_or(0);

        Self {
            response,
            created_at: now,
            access_count: 1,
            last_accessed: now,
            size_bytes,
            ttl_seconds: ttl.as_secs(),
        }
    }

    /// Check if this cache entry has expired based on its TTL
    ///
    /// Compares the current time against the creation time plus TTL duration.
    /// Returns `true` if the entry should be evicted.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.created_at > self.ttl_seconds
    }

    fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Enhanced cache key with context
///
/// Represents a unique cache key that incorporates multiple dimensions
/// to ensure accurate cache hits and misses.
///
/// ## Key Components
/// - `engine`: The LLM provider (e.g., "openai", "anthropic")
/// - `payload_hash`: SHA-256 hash of the request payload/prompt
/// - `model`: Optional model identifier (e.g., "gpt-4")
/// - `file_hash`: Optional hash of file path + modification time
/// - `parameters_hash`: Optional SHA-256 hash of request parameters
///
/// ## Cache Key Format
/// Keys are serialized as colon-separated strings:
/// ```text
/// engine:payload_hash[:model:model_name][:file:file_hash][:params:params_hash]
/// ```
///
/// This ensures that requests with different parameters, models, or content
/// generate distinct cache keys, preventing incorrect cache hits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub payload_hash: String,
    pub model: Option<String>,
    pub engine: String,
    pub file_hash: Option<String>,
    pub parameters_hash: Option<String>,
}

impl CacheKey {
    pub fn new(payload: &str, engine: &str) -> Self {
        Self {
            payload_hash: Self::hash_string(payload),
            model: None,
            engine: engine.to_string(),
            file_hash: None,
            parameters_hash: None,
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    pub fn with_file(mut self, file_path: &Path) -> Result<Self> {
        // For files, we hash the file path and modification time
        let metadata = std::fs::metadata(file_path)?;
        let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();

        let file_key = format!("{}:{}", file_path.display(), modified);
        self.file_hash = Some(Self::hash_string(&file_key));
        Ok(self)
    }

    pub fn with_parameters(mut self, params: &HashMap<String, serde_json::Value>) -> Self {
        let params_str = serde_json::to_string(params).unwrap_or_default();
        self.parameters_hash = Some(Self::hash_string(&params_str));
        self
    }

    fn hash_string(input: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn to_string(&self) -> String {
        let mut parts = vec![self.engine.clone(), self.payload_hash.clone()];

        if let Some(model) = &self.model {
            parts.push(format!("model:{}", model));
        }

        if let Some(file_hash) = &self.file_hash {
            parts.push(format!("file:{}", file_hash));
        }

        if let Some(params_hash) = &self.parameters_hash {
            parts.push(format!("params:{}", params_hash));
        }

        parts.join(":")
    }

    /// Generate a unique string representation of this cache key
    pub fn generate(&self) -> String {
        self.to_string()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub memory_hits: u64,
    pub memory_misses: u64,
    pub disk_hits: u64,
    pub disk_misses: u64,
    pub total_entries: usize,
    pub memory_size_bytes: usize,
    pub disk_size_bytes: usize,
    pub evictions: u64,
    pub errors: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total_hits = self.memory_hits + self.disk_hits;
        let total_requests = total_hits + self.memory_misses + self.disk_misses;

        if total_requests == 0 {
            0.0
        } else {
            total_hits as f64 / total_requests as f64
        }
    }

    pub fn memory_hit_rate(&self) -> f64 {
        let total_requests = self.memory_hits + self.memory_misses;

        if total_requests == 0 {
            0.0
        } else {
            self.memory_hits as f64 / total_requests as f64
        }
    }
}

/// Enhanced response cache with memory and disk tiers
pub struct EnhancedCache {
    config: CacheConfig,
    memory_cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    disk_cache: Option<sled::Db>,
    stats: Arc<Mutex<CacheStats>>,
}

impl EnhancedCache {
    /// Create a new enhanced cache
    pub fn new(config: CacheConfig) -> Result<Self> {
        let cache_size = NonZeroUsize::new(config.memory_cache_size).ok_or_else(|| {
            anyhow::anyhow!(
                "Memory cache size must be greater than 0, got: {}",
                config.memory_cache_size
            )
        })?;
        let memory_cache = Arc::new(RwLock::new(LruCache::new(cache_size)));

        let disk_cache = if config.enable_disk_cache {
            let cache_dir = config.disk_cache_dir.as_deref().unwrap_or("fluent_cache");
            Some(sled::open(cache_dir)?)
        } else {
            None
        };

        Ok(Self {
            config,
            memory_cache,
            disk_cache,
            stats: Arc::new(Mutex::new(CacheStats::default())),
        })
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(CacheConfig::default())
    }

    /// Get a response from cache
    pub async fn get(&self, key: &CacheKey) -> Result<Option<Response>> {
        let key_str = key.to_string();

        // Check memory cache first
        {
            let mut memory_cache = self.memory_cache.write().await;
            if let Some(entry) = memory_cache.peek(&key_str) {
                if !entry.is_expired() {
                    // Entry is valid, get it and mark as accessed
                    if let Some(entry) = memory_cache.get_mut(&key_str) {
                        entry.mark_accessed();
                        self.update_stats(|stats| stats.memory_hits += 1);
                        return Ok(Some(entry.response.clone()));
                    }
                } else {
                    // Remove expired entry
                    memory_cache.pop(&key_str);
                    self.update_stats(|stats| stats.evictions += 1);
                }
            }
        }

        self.update_stats(|stats| stats.memory_misses += 1);

        // Check disk cache if enabled
        if let Some(disk_cache) = &self.disk_cache {
            if let Some(data) = disk_cache.get(&key_str)? {
                match serde_json::from_slice::<CacheEntry>(&data) {
                    Ok(mut entry) => {
                        if !entry.is_expired() {
                            entry.mark_accessed();

                            // Promote to memory cache
                            {
                                let mut memory_cache = self.memory_cache.write().await;
                                memory_cache.put(key_str, entry.clone());
                            }

                            self.update_stats(|stats| stats.disk_hits += 1);
                            return Ok(Some(entry.response));
                        } else {
                            // Remove expired entry from disk
                            disk_cache.remove(&key_str)?;
                            self.update_stats(|stats| stats.evictions += 1);
                        }
                    }
                    Err(_) => {
                        self.update_stats(|stats| stats.errors += 1);
                    }
                }
            }
        }

        self.update_stats(|stats| stats.disk_misses += 1);
        Ok(None)
    }

    /// Insert a response into cache
    pub async fn insert(&self, key: &CacheKey, response: &Response) -> Result<()> {
        // Check if response should be cached
        if !self.should_cache_response(response) {
            return Ok(());
        }

        let key_str = key.to_string();
        let entry = CacheEntry::new(response.clone(), self.config.ttl);

        // Check size limit
        if entry.size_bytes > self.config.max_entry_size {
            return Ok(()); // Skip caching large entries
        }

        // Insert into memory cache
        {
            let mut memory_cache = self.memory_cache.write().await;
            memory_cache.put(key_str.clone(), entry.clone());
        }

        // Insert into disk cache if enabled
        if let Some(disk_cache) = &self.disk_cache {
            let data = if self.config.enable_compression {
                let json = serde_json::to_vec(&entry)?;
                lz4_flex::compress_prepend_size(&json)
            } else {
                serde_json::to_vec(&entry)?
            };

            disk_cache.insert(&key_str, data)?;
        }

        self.update_stats(|stats| {
            stats.total_entries += 1;
            stats.memory_size_bytes += entry.size_bytes;
        });

        Ok(())
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> Result<()> {
        {
            let mut memory_cache = self.memory_cache.write().await;
            memory_cache.clear();
        }

        if let Some(disk_cache) = &self.disk_cache {
            disk_cache.clear()?;
        }

        self.update_stats(|stats| {
            *stats = CacheStats::default();
        });

        Ok(())
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) -> Result<()> {
        let mut expired_count = 0;

        // Clean memory cache
        {
            let mut memory_cache = self.memory_cache.write().await;
            let mut keys_to_remove = Vec::new();

            for (key, entry) in memory_cache.iter() {
                if entry.is_expired() {
                    keys_to_remove.push(key.clone());
                }
            }

            for key in keys_to_remove {
                memory_cache.pop(&key);
                expired_count += 1;
            }
        }

        // Clean disk cache
        if let Some(disk_cache) = &self.disk_cache {
            let mut keys_to_remove = Vec::new();

            for item in disk_cache.iter() {
                if let Ok((key, data)) = item {
                    if let Ok(entry) = serde_json::from_slice::<CacheEntry>(&data) {
                        if entry.is_expired() {
                            keys_to_remove.push(key);
                        }
                    }
                }
            }

            for key in keys_to_remove {
                disk_cache.remove(&key)?;
                expired_count += 1;
            }
        }

        self.update_stats(|stats| {
            stats.evictions += expired_count;
        });

        Ok(())
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }

    /// Get cache size information
    pub async fn get_size_info(&self) -> (usize, usize) {
        let memory_size = {
            let memory_cache = self.memory_cache.read().await;
            memory_cache.len()
        };

        let disk_size = if let Some(disk_cache) = &self.disk_cache {
            disk_cache.len()
        } else {
            0
        };

        (memory_size, disk_size)
    }

    // Private helper methods

    fn should_cache_response(&self, response: &Response) -> bool {
        // Don't cache error responses unless configured to do so
        if response.content.contains("error") && !self.config.cache_errors {
            return false;
        }

        // Don't cache very large responses
        let response_size = serde_json::to_string(response)
            .map(|s| s.len())
            .unwrap_or(0);

        response_size <= self.config.max_entry_size
    }

    fn update_stats<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut CacheStats),
    {
        if let Ok(mut stats) = self.stats.lock() {
            update_fn(&mut *stats);
        }
    }
}

/// Start a background task to clean up expired cache entries
///
/// Spawns a tokio task that runs every 5 minutes to remove expired entries
/// from both memory and disk caches. This prevents unbounded growth and
/// ensures stale entries are eventually removed even if not accessed.
///
/// Returns a `JoinHandle` that can be used to cancel the cleanup task if needed.
pub fn start_cache_cleanup_task(cache: Arc<EnhancedCache>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // Clean up every 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = cache.cleanup_expired().await {
                eprintln!("Error cleaning up cache: {}", e);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::types::{Cost, Usage};

    fn create_test_response() -> Response {
        Response {
            content: "Test response".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            cost: Cost {
                prompt_cost: 0.001,
                completion_cost: 0.001,
                total_cost: 0.002,
            },
            model: "test-model".to_string(),
            finish_reason: Some("stop".to_string()),
        }
    }

    #[tokio::test]
    async fn test_enhanced_cache_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();
        let key = CacheKey::new("test payload", "openai");
        let response = create_test_response();

        // Insert and retrieve
        cache.insert(&key, &response).await.unwrap();
        let retrieved = cache.get(&key).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test response");
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let key1 = CacheKey::new("test", "openai").with_model("gpt-4");

        let key2 = CacheKey::new("test", "openai").with_model("gpt-3.5-turbo");

        assert_ne!(key1.to_string(), key2.to_string());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = CacheConfig {
            ttl: Duration::from_secs(1), // 1 second TTL
            enable_disk_cache: false,    // Disable disk cache for simpler test
            ..Default::default()
        };

        let cache = EnhancedCache::new(config).unwrap();
        let key = CacheKey::new("test", "openai");
        let response = create_test_response();

        // Insert entry
        cache.insert(&key, &response).await.unwrap();

        // Verify entry exists and is not expired
        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_some());

        // Wait for expiration (2 seconds to be well past the 1s TTL)
        tokio::time::sleep(Duration::from_secs(2)).await;

        // The get() method should automatically remove expired entries
        let retrieved_after_expiry = cache.get(&key).await.unwrap();
        assert!(retrieved_after_expiry.is_none());

        // Verify cache stats show eviction
        let stats = cache.get_stats();
        assert!(stats.evictions > 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();
        let key = CacheKey::new("test", "openai");
        let response = create_test_response();

        // Miss
        let _ = cache.get(&key).await.unwrap();

        // Insert
        cache.insert(&key, &response).await.unwrap();

        // Hit
        let _ = cache.get(&key).await.unwrap();

        let stats = cache.get_stats();
        assert_eq!(stats.memory_hits, 1);
        assert_eq!(stats.memory_misses, 1);
    }

    #[tokio::test]
    async fn test_cache_hit_rate_calculation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            enable_disk_cache: false, // Disable disk for simpler calculation
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();
        let response = create_test_response();

        // Create 10 misses, then cache, then 5 hits
        for i in 0..10 {
            let key = CacheKey::new(&format!("test_{}", i), "openai");
            let _ = cache.get(&key).await.unwrap(); // Miss
            cache.insert(&key, &response).await.unwrap();
        }

        // Now get 5 hits
        for i in 0..5 {
            let key = CacheKey::new(&format!("test_{}", i), "openai");
            let _ = cache.get(&key).await.unwrap(); // Hit
        }

        let stats = cache.get_stats();
        assert_eq!(stats.memory_hits, 5);
        assert_eq!(stats.memory_misses, 10);

        // Hit rate should be 5 / (5 + 10) = 0.333...
        let hit_rate = stats.memory_hit_rate();
        assert!((hit_rate - 0.333).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_cache_size_limit_enforcement() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            max_entry_size: 100, // Very small limit
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();

        // Create a large response that exceeds size limit
        let large_response = Response {
            content: "x".repeat(1000), // Much larger than 100 bytes
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            cost: Cost {
                prompt_cost: 0.001,
                completion_cost: 0.001,
                total_cost: 0.002,
            },
            model: "test-model".to_string(),
            finish_reason: Some("stop".to_string()),
        };

        let key = CacheKey::new("large_test", "openai");

        // Insert should succeed but not actually cache due to size
        cache.insert(&key, &large_response).await.unwrap();

        // Should be cache miss since entry was too large
        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            memory_cache_size: 5, // Small cache to trigger eviction
            enable_disk_cache: false,
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();
        let response = create_test_response();

        // Fill cache beyond capacity
        for i in 0..10 {
            let key = CacheKey::new(&format!("test_{}", i), "openai");
            cache.insert(&key, &response).await.unwrap();
        }

        // First entries should be evicted due to LRU
        let first_key = CacheKey::new("test_0", "openai");
        let first_entry = cache.get(&first_key).await.unwrap();
        assert!(first_entry.is_none()); // Should be evicted

        // Recent entries should still be present
        let recent_key = CacheKey::new("test_9", "openai");
        let recent_entry = cache.get(&recent_key).await.unwrap();
        assert!(recent_entry.is_some()); // Should still be cached
    }

    #[tokio::test]
    async fn test_ttl_with_different_durations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let short_ttl_config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            ttl: Duration::from_secs(1), // 1 second TTL
            enable_disk_cache: false,
            ..Default::default()
        };

        let cache = EnhancedCache::new(short_ttl_config).unwrap();
        let key = CacheKey::new("ttl_test", "openai");
        let response = create_test_response();

        // Insert entry
        cache.insert(&key, &response).await.unwrap();

        // Should be cached immediately
        assert!(cache.get(&key).await.unwrap().is_some());

        // Wait for TTL to expire (2 seconds to be well past the 1s TTL)
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired
        assert!(cache.get(&key).await.unwrap().is_none());

        // Verify eviction was counted
        let stats = cache.get_stats();
        assert!(stats.evictions > 0);
    }

    #[tokio::test]
    async fn test_cache_error_response_handling() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            cache_errors: false, // Don't cache errors
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();

        let error_response = Response {
            content: "error: something went wrong".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            cost: Cost {
                prompt_cost: 0.001,
                completion_cost: 0.001,
                total_cost: 0.002,
            },
            model: "test-model".to_string(),
            finish_reason: Some("error".to_string()),
        };

        let key = CacheKey::new("error_test", "openai");

        // Insert error response - should not be cached
        cache.insert(&key, &error_response).await.unwrap();

        // Should be cache miss
        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_cache_with_error_caching_enabled() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            cache_errors: true, // Cache errors
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();

        let error_response = Response {
            content: "error: something went wrong".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
            cost: Cost {
                prompt_cost: 0.001,
                completion_cost: 0.001,
                total_cost: 0.002,
            },
            model: "test-model".to_string(),
            finish_reason: Some("error".to_string()),
        };

        let key = CacheKey::new("error_test_enabled", "openai");

        // Insert error response - should be cached
        cache.insert(&key, &error_response).await.unwrap();

        // Should be cache hit
        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().content.contains("error"));
    }

    #[tokio::test]
    async fn test_cleanup_expired_entries() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            ttl: Duration::from_secs(1), // 1 second TTL
            enable_disk_cache: false,
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();
        let response = create_test_response();

        // Insert multiple entries
        for i in 0..5 {
            let key = CacheKey::new(&format!("cleanup_test_{}", i), "openai");
            cache.insert(&key, &response).await.unwrap();
        }

        // Verify cache has entries
        let (memory_size, _) = cache.get_size_info().await;
        assert_eq!(memory_size, 5);

        // Wait for expiration (2 seconds to be well past the 1s TTL)
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Run cleanup
        cache.cleanup_expired().await.unwrap();

        // Cache should be empty after cleanup
        let (memory_size_after, _) = cache.get_size_info().await;
        assert_eq!(memory_size_after, 0);

        // Verify eviction count
        let stats = cache.get_stats();
        assert_eq!(stats.evictions, 5);
    }

    #[tokio::test]
    async fn test_cache_clear_all() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();
        let response = create_test_response();

        // Insert entries
        for i in 0..5 {
            let key = CacheKey::new(&format!("clear_test_{}", i), "openai");
            cache.insert(&key, &response).await.unwrap();
        }

        // Verify entries exist
        let key = CacheKey::new("clear_test_0", "openai");
        assert!(cache.get(&key).await.unwrap().is_some());

        // Clear cache
        cache.clear().await.unwrap();

        // All entries should be gone
        for i in 0..5 {
            let key = CacheKey::new(&format!("clear_test_{}", i), "openai");
            assert!(cache.get(&key).await.unwrap().is_none());
        }

        // Stats should be reset
        let stats = cache.get_stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[tokio::test]
    async fn test_access_count_tracking() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = CacheConfig {
            disk_cache_dir: Some(temp_dir.path().to_string_lossy().to_string()),
            enable_disk_cache: false,
            ..Default::default()
        };
        let cache = EnhancedCache::new(config).unwrap();
        let key = CacheKey::new("access_count_test", "openai");
        let response = create_test_response();

        // Insert entry
        cache.insert(&key, &response).await.unwrap();

        // Access multiple times
        for _ in 0..5 {
            let _ = cache.get(&key).await.unwrap();
        }

        let stats = cache.get_stats();
        assert_eq!(stats.memory_hits, 5);
    }
}
