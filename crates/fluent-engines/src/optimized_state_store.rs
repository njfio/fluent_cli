use crate::pipeline_executor::{PipelineState, StateStore};
use anyhow::Result;
use async_trait::async_trait;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Cached state entry with metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CachedState {
    state: PipelineState,
    last_accessed: SystemTime,
    last_modified: SystemTime,
    dirty: bool,
}

/// Configuration for the optimized state store
#[derive(Debug, Clone)]
pub struct StateStoreConfig {
    /// Maximum number of states to keep in memory cache
    pub cache_size: usize,
    /// How long to keep states in cache before evicting (seconds)
    pub cache_ttl: Duration,
    /// How often to flush dirty states to disk (seconds)
    pub flush_interval: Duration,
    /// Whether to enable compression for stored states
    pub enable_compression: bool,
    /// Whether to enable write-through caching (immediate disk writes)
    pub write_through: bool,
}

impl Default for StateStoreConfig {
    fn default() -> Self {
        Self {
            cache_size: 1000,
            cache_ttl: Duration::from_secs(3600), // 1 hour
            flush_interval: Duration::from_secs(30), // 30 seconds
            enable_compression: true,
            write_through: false,
        }
    }
}

/// Optimized state store with in-memory caching and batched writes
pub struct OptimizedStateStore {
    /// File-based storage directory
    directory: PathBuf,
    /// In-memory cache of pipeline states
    cache: Arc<RwLock<LruCache<String, CachedState>>>,
    /// Configuration
    config: StateStoreConfig,
    /// Background flush task handle
    _flush_task: tokio::task::JoinHandle<()>,
}

impl OptimizedStateStore {
    /// Create a new optimized state store
    pub fn new(directory: PathBuf, config: StateStoreConfig) -> Result<Self> {
        let cache = Arc::new(RwLock::new(LruCache::new(
            NonZeroUsize::new(config.cache_size).unwrap()
        )));

        // Start background flush task
        let flush_cache = Arc::clone(&cache);
        let flush_dir = directory.clone();
        let flush_interval = config.flush_interval;
        let enable_compression = config.enable_compression;
        
        let flush_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);
            loop {
                interval.tick().await;
                if let Err(e) = Self::flush_dirty_states(&flush_cache, &flush_dir, enable_compression).await {
                    eprintln!("Error flushing states: {}", e);
                }
            }
        });

        Ok(Self {
            directory,
            cache,
            config,
            _flush_task: flush_task,
        })
    }

    /// Create with default configuration
    pub fn with_defaults(directory: PathBuf) -> Result<Self> {
        Self::new(directory, StateStoreConfig::default())
    }

    /// Flush dirty states to disk
    async fn flush_dirty_states(
        cache: &Arc<RwLock<LruCache<String, CachedState>>>,
        directory: &PathBuf,
        enable_compression: bool,
    ) -> Result<()> {
        let mut dirty_states = Vec::new();
        
        // Collect dirty states
        {
            let mut cache_guard = cache.write().await;
            for (key, cached_state) in cache_guard.iter_mut() {
                if cached_state.dirty {
                    dirty_states.push((key.clone(), cached_state.state.clone()));
                    cached_state.dirty = false;
                }
            }
        }

        // Write dirty states to disk
        for (key, state) in dirty_states {
            Self::write_state_to_disk(directory, &key, &state, enable_compression).await?;
        }

        Ok(())
    }

    /// Write a single state to disk
    async fn write_state_to_disk(
        directory: &PathBuf,
        key: &str,
        state: &PipelineState,
        enable_compression: bool,
    ) -> Result<()> {
        let file_path = directory.join(format!("{}.json", key));
        
        if enable_compression {
            // Use compressed JSON
            let json = serde_json::to_vec(state)?;
            let compressed = lz4_flex::compress_prepend_size(&json);
            tokio::fs::write(&file_path, compressed).await?;
        } else {
            // Use regular JSON
            let json = serde_json::to_string_pretty(state)?;
            tokio::fs::write(&file_path, json).await?;
        }

        Ok(())
    }

    /// Read a state from disk
    async fn read_state_from_disk(
        directory: &PathBuf,
        key: &str,
        enable_compression: bool,
    ) -> Result<Option<PipelineState>> {
        let file_path = directory.join(format!("{}.json", key));
        
        if !file_path.exists() {
            return Ok(None);
        }

        let data = tokio::fs::read(&file_path).await?;
        
        let state = if enable_compression {
            // Try to decompress first
            match lz4_flex::decompress_size_prepended(&data) {
                Ok(decompressed) => serde_json::from_slice(&decompressed)?,
                Err(_) => {
                    // Fallback to uncompressed for backward compatibility
                    serde_json::from_slice(&data)?
                }
            }
        } else {
            serde_json::from_slice(&data)?
        };

        Ok(Some(state))
    }

    /// Clean up expired cache entries
    #[allow(dead_code)]
    async fn cleanup_expired_entries(&self) {
        let now = SystemTime::now();
        let mut cache_guard = self.cache.write().await;
        
        let mut keys_to_remove = Vec::new();
        for (key, cached_state) in cache_guard.iter() {
            if let Ok(elapsed) = now.duration_since(cached_state.last_accessed) {
                if elapsed > self.config.cache_ttl {
                    keys_to_remove.push(key.clone());
                }
            }
        }

        for key in keys_to_remove {
            cache_guard.pop(&key);
        }
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> CacheStats {
        let cache_guard = self.cache.read().await;
        CacheStats {
            size: cache_guard.len(),
            capacity: cache_guard.cap().get(),
            hit_rate: 0.0, // Would need to track hits/misses for this
        }
    }

    /// Force flush all dirty states
    pub async fn force_flush(&self) -> Result<()> {
        Self::flush_dirty_states(&self.cache, &self.directory, self.config.enable_compression).await
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache_guard = self.cache.write().await;
        cache_guard.clear();
    }
}

#[async_trait]
impl StateStore for OptimizedStateStore {
    async fn save_state(&self, state_key: &str, state: &PipelineState) -> Result<()> {
        let now = SystemTime::now();
        
        // Update cache
        {
            let mut cache_guard = self.cache.write().await;
            cache_guard.put(state_key.to_string(), CachedState {
                state: state.clone(),
                last_accessed: now,
                last_modified: now,
                dirty: true,
            });
        }

        // If write-through is enabled, immediately write to disk
        if self.config.write_through {
            Self::write_state_to_disk(
                &self.directory,
                state_key,
                state,
                self.config.enable_compression,
            ).await?;
        }

        Ok(())
    }

    async fn load_state(&self, state_key: &str) -> Result<Option<PipelineState>> {
        let now = SystemTime::now();

        // Check cache first
        {
            let mut cache_guard = self.cache.write().await;
            if let Some(cached_state) = cache_guard.get_mut(state_key) {
                cached_state.last_accessed = now;
                return Ok(Some(cached_state.state.clone()));
            }
        }

        // Load from disk if not in cache
        if let Some(state) = Self::read_state_from_disk(
            &self.directory,
            state_key,
            self.config.enable_compression,
        ).await? {
            // Add to cache
            {
                let mut cache_guard = self.cache.write().await;
                cache_guard.put(state_key.to_string(), CachedState {
                    state: state.clone(),
                    last_accessed: now,
                    last_modified: now,
                    dirty: false,
                });
            }
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }
}

/// Cache performance statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f64,
}

/// Batch state operations for better performance
pub struct StateBatch {
    operations: Vec<StateOperation>,
}

#[derive(Debug, Clone)]
enum StateOperation {
    Save { key: String, state: PipelineState },
    Load { key: String },
}

impl StateBatch {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }

    pub fn save(&mut self, key: String, state: PipelineState) {
        self.operations.push(StateOperation::Save { key, state });
    }

    pub fn load(&mut self, key: String) {
        self.operations.push(StateOperation::Load { key });
    }

    pub async fn execute(&self, store: &OptimizedStateStore) -> Result<HashMap<String, Option<PipelineState>>> {
        let mut results = HashMap::new();

        for operation in &self.operations {
            match operation {
                StateOperation::Save { key, state } => {
                    store.save_state(key, state).await?;
                    results.insert(key.clone(), Some(state.clone()));
                }
                StateOperation::Load { key } => {
                    let state = store.load_state(key).await?;
                    results.insert(key.clone(), state);
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::collections::HashMap;
    use std::time::UNIX_EPOCH;

    #[tokio::test]
    async fn test_optimized_state_store() {
        let temp_dir = TempDir::new().unwrap();
        let store = OptimizedStateStore::with_defaults(temp_dir.path().to_path_buf()).unwrap();

        let state = PipelineState {
            current_step: 1,
            data: HashMap::new(),
            run_id: "test-run".to_string(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        // Test save and load
        store.save_state("test-key", &state).await.unwrap();
        let loaded = store.load_state("test-key").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().run_id, "test-run");

        // Test cache stats
        let stats = store.get_cache_stats().await;
        assert_eq!(stats.size, 1);
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store = OptimizedStateStore::with_defaults(temp_dir.path().to_path_buf()).unwrap();

        let mut batch = StateBatch::new();
        
        let state1 = PipelineState {
            current_step: 1,
            data: HashMap::new(),
            run_id: "batch-test-1".to_string(),
            start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        batch.save("batch-key-1".to_string(), state1);
        batch.load("batch-key-1".to_string());

        let results = batch.execute(&store).await.unwrap();
        assert!(results.contains_key("batch-key-1"));
    }
}
