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
        let cache_size = NonZeroUsize::new(config.memory_cache_size)
            .ok_or_else(|| anyhow::anyhow!("Memory cache size must be greater than 0, got: {}", config.memory_cache_size))?;
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
        self.stats.lock()
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
    #[ignore] // TODO: Fix expiration test
    async fn test_cache_expiration() {
        let config = CacheConfig {
            ttl: Duration::from_millis(1), // Very short TTL
            enable_disk_cache: false,      // Disable disk cache for simpler test
            ..Default::default()
        };

        let cache = EnhancedCache::new(config).unwrap();
        let key = CacheKey::new("test", "openai");
        let response = create_test_response();

        cache.insert(&key, &response).await.unwrap();

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Manually clean up expired entries
        cache.cleanup_expired().await.unwrap();

        let retrieved = cache.get(&key).await.unwrap();
        assert!(retrieved.is_none());
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
}
