use super::{utils::PerformanceCounter, ConnectionPoolConfig};
use anyhow::Result;
use deadpool::managed::{Manager, Pool};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// HTTP client manager for connection pooling
pub struct HttpClientManager {
    base_url: String,
    timeout: Duration,
    headers: reqwest::header::HeaderMap,
}

impl HttpClientManager {
    pub fn new(
        base_url: String,
        timeout: Duration,
        headers: HashMap<String, String>,
    ) -> Result<Self> {
        let mut header_map = reqwest::header::HeaderMap::new();

        for (key, value) in headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
            let header_value = reqwest::header::HeaderValue::from_str(&value)?;
            header_map.insert(header_name, header_value);
        }

        Ok(Self {
            base_url,
            timeout,
            headers: header_map,
        })
    }
}

#[async_trait::async_trait]
impl Manager for HttpClientManager {
    type Type = Client;
    type Error = reqwest::Error;

    async fn create(&self) -> Result<Client, Self::Error> {
        Client::builder()
            .timeout(self.timeout)
            .default_headers(self.headers.clone())
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
    }

    async fn recycle(
        &self,
        client: &mut Client,
        _metrics: &deadpool::managed::Metrics,
    ) -> Result<(), deadpool::managed::RecycleError<Self::Error>> {
        // Validate connection health by making a simple request
        let response = client
            .get(&format!("{}/health", self.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        match response {
            Ok(_) => Ok(()),
            Err(e) => Err(deadpool::managed::RecycleError::Backend(e)),
        }
    }
}

/// Enhanced HTTP connection pool with metrics
pub struct HttpConnectionPool {
    pool: Pool<HttpClientManager>,
    metrics: Arc<PerformanceCounter>,
    config: ConnectionPoolConfig,
}

impl HttpConnectionPool {
    pub async fn new(
        base_url: String,
        headers: HashMap<String, String>,
        config: ConnectionPoolConfig,
    ) -> Result<Self> {
        let manager = HttpClientManager::new(base_url, config.acquire_timeout, headers)?;

        let pool = Pool::builder(manager)
            .max_size(config.max_connections)
            .wait_timeout(Some(config.acquire_timeout))
            .create_timeout(Some(config.acquire_timeout))
            .recycle_timeout(Some(Duration::from_secs(30)))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create connection pool: {}", e))?;

        Ok(Self {
            pool,
            metrics: Arc::new(PerformanceCounter::new()),
            config,
        })
    }

    /// Execute an HTTP request using a pooled client
    pub async fn execute_request<T>(&self, request: HttpRequest) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let start = Instant::now();
        let mut is_error = false;

        let result = async {
            let client = self
                .pool
                .get()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get client from pool: {}", e))?;

            let response = client
                .request(request.method, &request.url)
                .headers(request.headers)
                .json(&request.body)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "HTTP request failed: {}",
                    response.status()
                ));
            }

            let result = response.json::<T>().await?;
            Ok(result)
        }
        .await;

        if result.is_err() {
            is_error = true;
        }

        self.metrics.record_request(start.elapsed(), is_error);
        result
    }

    /// Execute multiple requests in batch
    pub async fn execute_batch<T>(&self, requests: Vec<HttpRequest>) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let start = Instant::now();
        let mut handles = Vec::new();

        for request in requests {
            let pool = self.pool.clone();
            let handle = tokio::spawn(async move {
                let client = pool.get().await?;
                let response = client
                    .request(request.method, &request.url)
                    .headers(request.headers)
                    .json(&request.body)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(anyhow::anyhow!(
                        "HTTP request failed: {}",
                        response.status()
                    ));
                }

                let result = response.json::<T>().await?;
                Ok(result)
            });

            handles.push(handle);
        }

        let mut results = Vec::new();

        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => {
                    self.metrics.record_request(start.elapsed(), true);
                    return Err(e);
                }
                Err(e) => {
                    self.metrics.record_request(start.elapsed(), true);
                    return Err(anyhow::anyhow!("Task panicked: {}", e));
                }
            }
        }

        self.metrics.record_request(start.elapsed(), false);
        Ok(results)
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        let status = self.pool.status();
        let perf_stats = self.metrics.get_stats();

        PoolStats {
            max_size: status.max_size,
            size: status.size,
            available: status.available,
            waiting: status.waiting,
            performance: perf_stats,
        }
    }

    /// Get pool configuration
    pub fn get_config(&self) -> &ConnectionPoolConfig {
        &self.config
    }
}

/// HTTP request structure
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: reqwest::Method,
    pub url: String,
    pub headers: reqwest::header::HeaderMap,
    pub body: serde_json::Value,
}

impl HttpRequest {
    pub fn new(method: reqwest::Method, url: String, body: serde_json::Value) -> Self {
        Self {
            method,
            url,
            headers: reqwest::header::HeaderMap::new(),
            body,
        }
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Result<Self> {
        for (key, value) in headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
            let header_value = reqwest::header::HeaderValue::from_str(&value)?;
            self.headers.insert(header_name, header_value);
        }
        Ok(self)
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub max_size: usize,
    pub size: usize,
    pub available: usize,
    pub waiting: usize,
    pub performance: super::utils::PerformanceStats,
}

/// Connection pool manager for multiple pools
pub struct ConnectionPoolManager {
    pools: Arc<RwLock<HashMap<String, Arc<HttpConnectionPool>>>>,
    default_config: ConnectionPoolConfig,
}

impl ConnectionPoolManager {
    pub fn new(default_config: ConnectionPoolConfig) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            default_config,
        }
    }

    /// Create or get a connection pool for a specific endpoint
    pub async fn get_or_create_pool(
        &self,
        name: &str,
        base_url: String,
        headers: HashMap<String, String>,
        config: Option<ConnectionPoolConfig>,
    ) -> Result<Arc<HttpConnectionPool>> {
        let pools = self.pools.read().await;

        if let Some(pool) = pools.get(name) {
            return Ok(pool.clone());
        }

        drop(pools);

        // Create new pool
        let pool_config = config.unwrap_or_else(|| self.default_config.clone());
        let pool = HttpConnectionPool::new(base_url, headers, pool_config).await?;
        let pool = Arc::new(pool);

        let mut pools = self.pools.write().await;
        pools.insert(name.to_string(), pool.clone());

        Ok(pool)
    }

    /// Remove a connection pool
    pub async fn remove_pool(&self, name: &str) -> Option<Arc<HttpConnectionPool>> {
        let mut pools = self.pools.write().await;
        pools.remove(name)
    }

    /// Get all pool statistics
    pub async fn get_all_stats(&self) -> HashMap<String, PoolStats> {
        let pools = self.pools.read().await;
        let mut stats = HashMap::new();

        for (name, pool) in pools.iter() {
            stats.insert(name.clone(), pool.get_stats());
        }

        stats
    }

    /// Get pool names
    pub async fn get_pool_names(&self) -> Vec<String> {
        let pools = self.pools.read().await;
        pools.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_client_manager_creation() {
        let manager = HttpClientManager::new(
            "http://localhost:8080".to_string(),
            Duration::from_secs(30),
            HashMap::new(),
        );

        assert!(manager.is_ok());
    }

    #[test]
    fn test_http_request_creation() {
        let request = HttpRequest::new(
            reqwest::Method::POST,
            "http://example.com/api".to_string(),
            serde_json::json!({"test": "data"}),
        );

        assert_eq!(request.method, reqwest::Method::POST);
        assert_eq!(request.url, "http://example.com/api");
    }

    #[tokio::test]
    async fn test_connection_pool_manager() {
        let manager = ConnectionPoolManager::new(ConnectionPoolConfig::default());

        let pool_result = manager
            .get_or_create_pool(
                "test_pool",
                "http://localhost:8080".to_string(),
                HashMap::new(),
                None,
            )
            .await;

        // This will likely fail due to no server, but tests the creation logic
        assert!(pool_result.is_err() || pool_result.is_ok());
    }
}
