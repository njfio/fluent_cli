use anyhow::{anyhow, Result};
use fluent_core::auth::EngineAuth;
use fluent_core::config::EngineConfig;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for connection pooling
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of clients to keep in the pool per host
    pub max_clients_per_host: usize,
    /// Maximum idle time before a client is removed from the pool
    pub max_idle_time: Duration,
    /// Connection timeout for new clients
    pub connection_timeout: Duration,
    /// Request timeout for HTTP requests
    pub request_timeout: Duration,
    /// Maximum number of connections per client
    pub max_connections_per_client: usize,
    /// Keep-alive timeout
    pub keep_alive_timeout: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_clients_per_host: 10,
            max_idle_time: Duration::from_secs(300), // 5 minutes
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_connections_per_client: 100,
            keep_alive_timeout: Duration::from_secs(90),
        }
    }
}

/// A pooled HTTP client with metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PooledClient {
    client: Client,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
}

impl PooledClient {
    fn new(client: Client) -> Self {
        let now = Instant::now();
        Self {
            client,
            created_at: now,
            last_used: now,
            use_count: 0,
        }
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }

    fn is_expired(&self, max_idle_time: Duration) -> bool {
        self.last_used.elapsed() > max_idle_time
    }
}

/// Connection pool for HTTP clients
pub struct ConnectionPool {
    pools: Arc<RwLock<HashMap<String, Vec<PooledClient>>>>,
    config: ConnectionPoolConfig,
    stats: Arc<Mutex<PoolStats>>,
}

/// Statistics for the connection pool
#[derive(Debug, Default, Clone)]
pub struct PoolStats {
    pub total_clients_created: u64,
    pub total_clients_reused: u64,
    pub total_clients_expired: u64,
    pub current_pool_size: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(Mutex::new(PoolStats::default())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ConnectionPoolConfig::default())
    }

    /// Get a client for the specified engine configuration
    pub async fn get_client(&self, engine_config: &EngineConfig) -> Result<Client> {
        let host_key = self.create_host_key(engine_config);
        
        // Try to get an existing client from the pool
        if let Some(client) = self.get_pooled_client(&host_key).await {
            self.update_stats(|stats| {
                stats.cache_hits += 1;
                stats.total_clients_reused += 1;
            });
            return Ok(client);
        }

        // Create a new client if none available in pool
        let client = self.create_new_client(engine_config).await?;
        
        self.update_stats(|stats| {
            stats.cache_misses += 1;
            stats.total_clients_created += 1;
        });

        Ok(client)
    }

    /// Return a client to the pool for reuse
    pub async fn return_client(&self, engine_config: &EngineConfig, client: Client) {
        let host_key = self.create_host_key(engine_config);
        
        let mut pools = self.pools.write().await;
        let pool = pools.entry(host_key).or_insert_with(Vec::new);
        
        // Only add to pool if we haven't exceeded the limit
        if pool.len() < self.config.max_clients_per_host {
            pool.push(PooledClient::new(client));
            
            self.update_stats(|stats| {
                stats.current_pool_size = pools.values().map(|p| p.len()).sum();
            });
        }
    }

    /// Clean up expired clients from the pool
    pub async fn cleanup_expired(&self) {
        let mut pools = self.pools.write().await;
        let mut total_expired = 0;

        for pool in pools.values_mut() {
            let initial_len = pool.len();
            pool.retain(|client| !client.is_expired(self.config.max_idle_time));
            total_expired += initial_len - pool.len();
        }

        // Remove empty pools
        pools.retain(|_, pool| !pool.is_empty());

        self.update_stats(|stats| {
            stats.total_clients_expired += total_expired as u64;
            stats.current_pool_size = pools.values().map(|p| p.len()).sum();
        });
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        self.stats.lock().unwrap().clone()
    }

    /// Clear all pools
    pub async fn clear(&self) {
        let mut pools = self.pools.write().await;
        pools.clear();
        
        self.update_stats(|stats| {
            stats.current_pool_size = 0;
        });
    }

    /// Get the number of clients in the pool for a specific host
    pub async fn get_pool_size(&self, engine_config: &EngineConfig) -> usize {
        let host_key = self.create_host_key(engine_config);
        let pools = self.pools.read().await;
        pools.get(&host_key).map(|p| p.len()).unwrap_or(0)
    }

    // Private helper methods

    fn create_host_key(&self, config: &EngineConfig) -> String {
        format!("{}://{}:{}", 
                config.connection.protocol, 
                config.connection.hostname, 
                config.connection.port)
    }

    async fn get_pooled_client(&self, host_key: &str) -> Option<Client> {
        let mut pools = self.pools.write().await;
        
        if let Some(pool) = pools.get_mut(host_key) {
            // Find a non-expired client
            if let Some(index) = pool.iter().position(|c| !c.is_expired(self.config.max_idle_time)) {
                let mut pooled_client = pool.remove(index);
                pooled_client.mark_used();
                return Some(pooled_client.client);
            }
            
            // Remove expired clients
            pool.retain(|c| !c.is_expired(self.config.max_idle_time));
            if pool.is_empty() {
                pools.remove(host_key);
            }
        }
        
        None
    }

    async fn create_new_client(&self, engine_config: &EngineConfig) -> Result<Client> {
        // Determine the authentication method based on engine type
        let auth_manager = match engine_config.engine.as_str() {
            "openai" => EngineAuth::openai(&engine_config.parameters)?,
            "anthropic" => EngineAuth::anthropic(&engine_config.parameters)?,
            "cohere" => EngineAuth::cohere(&engine_config.parameters)?,
            "mistral" => EngineAuth::mistral(&engine_config.parameters)?,
            "google_gemini" => EngineAuth::google_gemini(&engine_config.parameters)?,
            "stability_ai" => EngineAuth::stability_ai(&engine_config.parameters)?,
            "replicate" => EngineAuth::replicate(&engine_config.parameters)?,
            _ => EngineAuth::webhook(&engine_config.parameters)?, // Default fallback
        };

        // Apply authentication
        let mut headers = reqwest::header::HeaderMap::new();
        auth_manager.add_auth_headers(&mut headers)?;

        // Create optimized client with connection pooling and authentication
        let authenticated_client = reqwest::Client::builder()
            .timeout(self.config.request_timeout)
            .connect_timeout(self.config.connection_timeout)
            .pool_max_idle_per_host(self.config.max_connections_per_client)
            .pool_idle_timeout(self.config.keep_alive_timeout)
            .tcp_keepalive(self.config.keep_alive_timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow!("Failed to create authenticated HTTP client: {}", e))?;

        Ok(authenticated_client)
    }

    fn update_stats<F>(&self, update_fn: F) 
    where 
        F: FnOnce(&mut PoolStats),
    {
        if let Ok(mut stats) = self.stats.lock() {
            update_fn(&mut *stats);
        }
    }
}

/// Global connection pool instance
static GLOBAL_POOL: once_cell::sync::Lazy<ConnectionPool> = 
    once_cell::sync::Lazy::new(|| ConnectionPool::with_defaults());

/// Get the global connection pool instance
pub fn global_pool() -> &'static ConnectionPool {
    &GLOBAL_POOL
}

/// Convenience function to get a client from the global pool
pub async fn get_pooled_client(engine_config: &EngineConfig) -> Result<Client> {
    global_pool().get_client(engine_config).await
}

/// Convenience function to return a client to the global pool
pub async fn return_pooled_client(engine_config: &EngineConfig, client: Client) {
    global_pool().return_client(engine_config, client).await;
}

/// Start a background task to clean up expired connections
pub fn start_cleanup_task() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(60)); // Clean up every minute
        loop {
            interval.tick().await;
            global_pool().cleanup_expired().await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> EngineConfig {
        let mut parameters = HashMap::new();
        parameters.insert("bearer_token".to_string(), serde_json::json!("test-token"));

        EngineConfig {
            name: "test".to_string(),
            engine: "openai".to_string(),
            connection: fluent_core::config::ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.openai.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters,
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let pool = ConnectionPool::with_defaults();
        let stats = pool.get_stats();
        assert_eq!(stats.current_pool_size, 0);
    }

    #[tokio::test]
    async fn test_client_creation_and_reuse() {
        let pool = ConnectionPool::with_defaults();
        let config = create_test_config();

        // Get first client
        let client1 = pool.get_client(&config).await.unwrap();
        
        // Return it to pool
        pool.return_client(&config, client1).await;
        
        // Get another client (should be reused)
        let _client2 = pool.get_client(&config).await.unwrap();
        
        let stats = pool.get_stats();
        assert!(stats.total_clients_created > 0);
    }

    #[tokio::test]
    async fn test_pool_cleanup() {
        let config = ConnectionPoolConfig {
            max_idle_time: Duration::from_millis(1), // Very short for testing
            ..Default::default()
        };
        
        let pool = ConnectionPool::new(config);
        let engine_config = create_test_config();

        // Add a client to pool
        let client = pool.get_client(&engine_config).await.unwrap();
        pool.return_client(&engine_config, client).await;
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Cleanup should remove expired clients
        pool.cleanup_expired().await;
        
        let stats = pool.get_stats();
        assert_eq!(stats.current_pool_size, 0);
    }

    #[tokio::test]
    async fn test_global_pool() {
        let config = create_test_config();
        let client = get_pooled_client(&config).await.unwrap();
        return_pooled_client(&config, client).await;
        
        let stats = global_pool().get_stats();
        assert!(stats.total_clients_created > 0);
    }
}
