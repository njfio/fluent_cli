// MCP Resource Management System
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use url::Url;

use crate::memory::{LongTermMemory, MemoryQuery};

/// MCP Resource definition with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub size: Option<u64>,
    pub last_modified: Option<SystemTime>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, Value>,
    pub access_permissions: ResourcePermissions,
    pub cache_policy: CachePolicy,
}

/// Resource access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub deletable: bool,
    pub allowed_operations: Vec<String>,
}

impl Default for ResourcePermissions {
    fn default() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: false,
            deletable: false,
            allowed_operations: vec!["read".to_string()],
        }
    }
}

/// Cache policy for resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy {
    pub cacheable: bool,
    pub ttl_seconds: Option<u64>,
    pub max_size_bytes: Option<u64>,
    pub invalidate_on_change: bool,
}

impl Default for CachePolicy {
    fn default() -> Self {
        Self {
            cacheable: true,
            ttl_seconds: Some(3600), // 1 hour
            max_size_bytes: Some(10 * 1024 * 1024), // 10MB
            invalidate_on_change: true,
        }
    }
}

/// Cached resource content
#[derive(Debug, Clone)]
struct CachedResource {
    content: Value,
    cached_at: SystemTime,
    ttl_seconds: Option<u64>,
    size_bytes: u64,
    access_count: u64,
    last_accessed: SystemTime,
}

/// Resource access statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceStats {
    pub total_accesses: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_bytes_served: u64,
    pub average_response_time_ms: f64,
    pub last_accessed: Option<SystemTime>,
    pub error_count: u64,
}

/// MCP Resource Manager for handling resource lifecycle and caching
pub struct McpResourceManager {
    resources: Arc<RwLock<HashMap<String, McpResource>>>,
    cache: Arc<RwLock<HashMap<String, CachedResource>>>,
    memory_system: Arc<dyn LongTermMemory>,
    stats: Arc<RwLock<HashMap<String, ResourceStats>>>,
    config: ResourceManagerConfig,
}

/// Configuration for the resource manager
#[derive(Debug, Clone)]
pub struct ResourceManagerConfig {
    pub max_cache_size_bytes: u64,
    pub default_ttl_seconds: u64,
    pub max_cached_resources: usize,
    pub enable_compression: bool,
    pub enable_statistics: bool,
}

impl Default for ResourceManagerConfig {
    fn default() -> Self {
        Self {
            max_cache_size_bytes: 100 * 1024 * 1024, // 100MB
            default_ttl_seconds: 3600, // 1 hour
            max_cached_resources: 1000,
            enable_compression: true,
            enable_statistics: true,
        }
    }
}

impl McpResourceManager {
    /// Create a new resource manager
    pub fn new(memory_system: Arc<dyn LongTermMemory>) -> Self {
        Self::with_config(memory_system, ResourceManagerConfig::default())
    }

    /// Create a new resource manager with custom configuration
    pub fn with_config(memory_system: Arc<dyn LongTermMemory>, config: ResourceManagerConfig) -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            memory_system,
            stats: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Initialize the resource manager with standard resources
    pub async fn initialize_standard_resources(&self) -> Result<()> {
        // Register memory-based resources
        self.register_memory_resources().await?;
        
        // Register file system resources
        self.register_filesystem_resources().await?;
        
        // Register configuration resources
        self.register_config_resources().await?;

        println!("✅ Initialized {} MCP resources", self.resources.read().await.len());
        Ok(())
    }

    /// Register memory-based resources
    async fn register_memory_resources(&self) -> Result<()> {
        let memory_resource = McpResource {
            uri: "memory://memories".to_string(),
            name: Some("Long-term Memories".to_string()),
            description: Some("Access to stored long-term memories".to_string()),
            mime_type: Some("application/json".to_string()),
            size: None,
            last_modified: Some(SystemTime::now()),
            tags: vec!["memory".to_string(), "storage".to_string()],
            metadata: HashMap::new(),
            access_permissions: ResourcePermissions {
                readable: true,
                writable: true,
                executable: false,
                deletable: true,
                allowed_operations: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "query".to_string(),
                    "delete".to_string(),
                ],
            },
            cache_policy: CachePolicy {
                cacheable: false, // Memory is dynamic
                ttl_seconds: None,
                max_size_bytes: None,
                invalidate_on_change: true,
            },
        };

        self.register_resource(memory_resource).await?;
        Ok(())
    }

    /// Register file system resources
    async fn register_filesystem_resources(&self) -> Result<()> {
        let fs_resource = McpResource {
            uri: "file://workspace".to_string(),
            name: Some("Workspace Files".to_string()),
            description: Some("Access to workspace files and directories".to_string()),
            mime_type: Some("application/octet-stream".to_string()),
            size: None,
            last_modified: Some(SystemTime::now()),
            tags: vec!["filesystem".to_string(), "workspace".to_string()],
            metadata: HashMap::new(),
            access_permissions: ResourcePermissions {
                readable: true,
                writable: true,
                executable: false,
                deletable: false,
                allowed_operations: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "list".to_string(),
                ],
            },
            cache_policy: CachePolicy::default(),
        };

        self.register_resource(fs_resource).await?;
        Ok(())
    }

    /// Register configuration resources
    async fn register_config_resources(&self) -> Result<()> {
        let config_resource = McpResource {
            uri: "config://agent".to_string(),
            name: Some("Agent Configuration".to_string()),
            description: Some("Agent configuration and settings".to_string()),
            mime_type: Some("application/json".to_string()),
            size: None,
            last_modified: Some(SystemTime::now()),
            tags: vec!["config".to_string(), "settings".to_string()],
            metadata: HashMap::new(),
            access_permissions: ResourcePermissions {
                readable: true,
                writable: false,
                executable: false,
                deletable: false,
                allowed_operations: vec!["read".to_string()],
            },
            cache_policy: CachePolicy {
                cacheable: true,
                ttl_seconds: Some(300), // 5 minutes
                max_size_bytes: Some(1024 * 1024), // 1MB
                invalidate_on_change: true,
            },
        };

        self.register_resource(config_resource).await?;
        Ok(())
    }

    /// Register a new resource
    pub async fn register_resource(&self, resource: McpResource) -> Result<()> {
        // Validate URI
        self.validate_resource_uri(&resource.uri)?;
        
        let mut resources = self.resources.write().await;
        
        if resources.contains_key(&resource.uri) {
            return Err(anyhow!("Resource '{}' already registered", resource.uri));
        }
        
        resources.insert(resource.uri.clone(), resource.clone());
        
        // Initialize stats
        if self.config.enable_statistics {
            let mut stats = self.stats.write().await;
            stats.insert(resource.uri.clone(), ResourceStats::default());
        }
        
        println!("✅ Registered MCP resource: {} ({})", 
                 resource.uri, 
                 resource.name.as_deref().unwrap_or("unnamed"));
        Ok(())
    }

    /// Validate resource URI
    fn validate_resource_uri(&self, uri: &str) -> Result<()> {
        let parsed_uri = Url::parse(uri)
            .map_err(|e| anyhow!("Invalid resource URI '{}': {}", uri, e))?;
        
        // Check supported schemes
        match parsed_uri.scheme() {
            "memory" | "file" | "config" | "http" | "https" => Ok(()),
            scheme => Err(anyhow!("Unsupported URI scheme: {}", scheme)),
        }
    }

    /// List all registered resources
    pub async fn list_resources(&self) -> Vec<McpResource> {
        self.resources.read().await.values().cloned().collect()
    }

    /// Get a specific resource by URI
    pub async fn get_resource(&self, uri: &str) -> Option<McpResource> {
        self.resources.read().await.get(uri).cloned()
    }

    /// Read resource content with caching
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        let start_time = std::time::Instant::now();
        
        // Get resource definition
        let resource = self.get_resource(uri).await
            .ok_or_else(|| anyhow!("Resource '{}' not found", uri))?;
        
        // Check permissions
        if !resource.access_permissions.readable {
            return Err(anyhow!("Resource '{}' is not readable", uri));
        }
        
        // Try cache first
        if resource.cache_policy.cacheable {
            if let Some(cached_content) = self.get_from_cache(uri).await? {
                self.update_stats(uri, true, start_time.elapsed().as_millis() as u64, 0).await;
                return Ok(cached_content);
            }
        }
        
        // Read from source
        let content = self.read_resource_from_source(&resource).await?;
        let content_size = self.estimate_content_size(&content);
        
        // Cache if policy allows
        if resource.cache_policy.cacheable {
            self.store_in_cache(uri, &content, &resource.cache_policy).await?;
        }
        
        // Update stats
        self.update_stats(uri, false, start_time.elapsed().as_millis() as u64, content_size).await;
        
        Ok(content)
    }

    /// Read resource content from its source
    async fn read_resource_from_source(&self, resource: &McpResource) -> Result<Value> {
        let uri = Url::parse(&resource.uri)?;

        match uri.scheme() {
            "memory" => self.read_memory_resource(&uri).await,
            "file" => self.read_file_resource(&uri).await,
            "config" => self.read_config_resource(&uri).await,
            "http" | "https" => self.read_http_resource(&uri).await,
            scheme => Err(anyhow!("Unsupported URI scheme: {}", scheme)),
        }
    }

    /// Read from memory system
    async fn read_memory_resource(&self, uri: &Url) -> Result<Value> {
        match uri.host_str() {
            Some("memories") => {
                // Query all memories (simplified for now)
                let query = MemoryQuery {
                    query_text: "".to_string(),
                    memory_types: vec![],
                    time_range: None,
                    importance_threshold: Some(0.0),
                    limit: Some(100),
                    tags: vec![],
                };

                // For now, return empty memories since we need to fix the memory system integration
                let memories: Vec<serde_json::Value> = vec![];
                Ok(json!({
                    "memories": memories,
                    "count": memories.len(),
                    "timestamp": chrono::Utc::now()
                }))
            }
            _ => Err(anyhow!("Unknown memory resource path")),
        }
    }

    /// Read from file system
    async fn read_file_resource(&self, uri: &Url) -> Result<Value> {
        let path = uri.path();

        // Security check - ensure path is within allowed workspace
        if !self.is_path_allowed(path) {
            return Err(anyhow!("Access to path '{}' is not allowed", path));
        }

        match tokio::fs::read_to_string(path).await {
            Ok(content) => Ok(json!({
                "content": content,
                "path": path,
                "size": content.len(),
                "timestamp": chrono::Utc::now()
            })),
            Err(e) => Err(anyhow!("Failed to read file '{}': {}", path, e)),
        }
    }

    /// Read configuration resource
    async fn read_config_resource(&self, uri: &Url) -> Result<Value> {
        match uri.host_str() {
            Some("agent") => {
                // Return agent configuration
                Ok(json!({
                    "agent_id": "fluent-cli-agent",
                    "version": "0.1.0",
                    "capabilities": {
                        "tools": true,
                        "memory": true,
                        "resources": true,
                        "reasoning": true
                    },
                    "limits": {
                        "max_memory_items": 10000,
                        "max_tool_execution_time_ms": 30000,
                        "max_resource_size_bytes": 10485760
                    },
                    "timestamp": chrono::Utc::now()
                }))
            }
            _ => Err(anyhow!("Unknown config resource")),
        }
    }

    /// Read from HTTP resource
    async fn read_http_resource(&self, uri: &Url) -> Result<Value> {
        let client = reqwest::Client::new();
        let response = client.get(uri.as_str()).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP request failed with status: {}", response.status()));
        }

        let content = response.text().await?;
        Ok(json!({
            "content": content,
            "url": uri.as_str(),
            "size": content.len(),
            "timestamp": chrono::Utc::now()
        }))
    }

    /// Check if file path is allowed
    fn is_path_allowed(&self, path: &str) -> bool {
        // Basic security check - prevent path traversal
        !path.contains("..") && (
            path.starts_with("./") ||
            path.starts_with("src/") ||
            path.starts_with("crates/") ||
            path.starts_with("examples/") ||
            path == "README.md" ||
            path == "Cargo.toml"
        )
    }

    /// Get content from cache
    async fn get_from_cache(&self, uri: &str) -> Result<Option<Value>> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.get(uri) {
            // Check if cache is still valid
            if let Some(ttl) = cached.ttl_seconds {
                let age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if age.as_secs() > ttl {
                    // Cache expired
                    return Ok(None);
                }
            }

            let content = cached.content.clone();

            // Release the read lock before updating access statistics
            drop(cache);
            self.update_cache_access(uri).await;

            return Ok(Some(content));
        }

        Ok(None)
    }

    /// Store content in cache
    async fn store_in_cache(&self, uri: &str, content: &Value, policy: &CachePolicy) -> Result<()> {
        let content_size = self.estimate_content_size(content);

        // Check size limits
        if let Some(max_size) = policy.max_size_bytes {
            if content_size > max_size {
                return Ok(()); // Don't cache oversized content
            }
        }

        let mut cache = self.cache.write().await;

        // Check total cache size
        let total_size: u64 = cache.values().map(|c| c.size_bytes).sum();
        if total_size + content_size > self.config.max_cache_size_bytes {
            // Evict oldest entries
            self.evict_cache_entries(&mut cache, content_size).await;
        }

        let cached_resource = CachedResource {
            content: content.clone(),
            cached_at: SystemTime::now(),
            ttl_seconds: policy.ttl_seconds,
            size_bytes: content_size,
            access_count: 0,
            last_accessed: SystemTime::now(),
        };

        cache.insert(uri.to_string(), cached_resource);
        Ok(())
    }

    /// Evict cache entries to make space
    async fn evict_cache_entries(&self, cache: &mut HashMap<String, CachedResource>, needed_space: u64) {
        let mut entries: Vec<_> = cache.iter().collect();
        entries.sort_by_key(|(_, cached)| cached.last_accessed);

        let mut freed_space = 0u64;
        let mut to_remove = Vec::new();

        for (uri, cached) in entries {
            to_remove.push(uri.clone());
            freed_space += cached.size_bytes;

            if freed_space >= needed_space {
                break;
            }
        }

        for uri in to_remove {
            cache.remove(&uri);
        }
    }

    /// Update cache access statistics
    async fn update_cache_access(&self, uri: &str) {
        let mut cache = self.cache.write().await;
        if let Some(cached) = cache.get_mut(uri) {
            cached.access_count += 1;
            cached.last_accessed = SystemTime::now();
        }
    }

    /// Estimate content size in bytes
    fn estimate_content_size(&self, content: &Value) -> u64 {
        serde_json::to_string(content)
            .map(|s| s.len() as u64)
            .unwrap_or(0)
    }

    /// Update resource statistics
    async fn update_stats(&self, uri: &str, cache_hit: bool, response_time_ms: u64, bytes_served: u64) {
        if !self.config.enable_statistics {
            return;
        }

        let mut stats = self.stats.write().await;
        if let Some(resource_stats) = stats.get_mut(uri) {
            resource_stats.total_accesses += 1;
            resource_stats.total_bytes_served += bytes_served;
            resource_stats.last_accessed = Some(SystemTime::now());

            if cache_hit {
                resource_stats.cache_hits += 1;
            } else {
                resource_stats.cache_misses += 1;
            }

            // Update average response time
            let total_time = resource_stats.average_response_time_ms * (resource_stats.total_accesses - 1) as f64 + response_time_ms as f64;
            resource_stats.average_response_time_ms = total_time / resource_stats.total_accesses as f64;
        }
    }

    /// Get resource statistics
    pub async fn get_resource_stats(&self, uri: &str) -> Option<ResourceStats> {
        self.stats.read().await.get(uri).cloned()
    }

    /// Get all resource statistics
    pub async fn get_all_stats(&self) -> HashMap<String, ResourceStats> {
        self.stats.read().await.clone()
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        println!("✅ Cleared resource cache");
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> HashMap<String, Value> {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let total_size: u64 = cache.values().map(|c| c.size_bytes).sum();
        let total_accesses: u64 = cache.values().map(|c| c.access_count).sum();

        let mut stats = HashMap::new();
        stats.insert("total_entries".to_string(), json!(total_entries));
        stats.insert("total_size_bytes".to_string(), json!(total_size));
        stats.insert("total_accesses".to_string(), json!(total_accesses));
        stats.insert("max_size_bytes".to_string(), json!(self.config.max_cache_size_bytes));
        stats.insert("utilization_percent".to_string(),
                     json!((total_size as f64 / self.config.max_cache_size_bytes as f64) * 100.0));

        stats
    }

    /// Remove a resource
    pub async fn remove_resource(&self, uri: &str) -> Result<()> {
        let mut resources = self.resources.write().await;

        if resources.remove(uri).is_none() {
            return Err(anyhow!("Resource '{}' not found", uri));
        }

        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(uri);

        // Remove stats
        let mut stats = self.stats.write().await;
        stats.remove(uri);

        println!("✅ Removed MCP resource: {}", uri);
        Ok(())
    }
}
