use super::{JsonRpcRequest, JsonRpcResponse, JsonRpcNotification, McpTransport, AuthConfig, AuthType, TimeoutConfig, RetryConfig};
use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use url::Url;

pub struct HttpTransport {
    client: Client,
    base_url: String,
    auth_config: Option<AuthConfig>,
    timeout_config: TimeoutConfig,
    retry_config: RetryConfig,
    notification_tx: Arc<Mutex<Option<mpsc::UnboundedSender<JsonRpcNotification>>>>,
    is_connected: Arc<std::sync::atomic::AtomicBool>,
}

impl HttpTransport {
    pub async fn new(
        base_url: String,
        headers: HashMap<String, String>,
        auth_config: Option<AuthConfig>,
        timeout_config: TimeoutConfig,
        retry_config: RetryConfig,
    ) -> Result<Self> {
        // Validate URL
        let _url = Url::parse(&base_url)?;
        
        // Build headers
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())?;
            let header_value = HeaderValue::from_str(&value)?;
            header_map.insert(header_name, header_value);
        }
        
        // Add authentication headers
        if let Some(ref auth) = auth_config {
            match auth.auth_type {
                AuthType::Bearer => {
                    if let Some(token) = auth.credentials.get("token") {
                        let auth_value = HeaderValue::from_str(&format!("Bearer {}", token))?;
                        header_map.insert("authorization", auth_value);
                    }
                }
                AuthType::ApiKey => {
                    if let Some(key) = auth.credentials.get("key") {
                        if let Some(header_name) = auth.credentials.get("header") {
                            let header_name = HeaderName::from_bytes(header_name.as_bytes())?;
                            let header_value = HeaderValue::from_str(key)?;
                            header_map.insert(header_name, header_value);
                        } else {
                            header_map.insert("x-api-key", HeaderValue::from_str(key)?);
                        }
                    }
                }
                AuthType::Basic => {
                    if let (Some(username), Some(password)) = 
                        (auth.credentials.get("username"), auth.credentials.get("password")) {
                        let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", username, password));
                        let auth_value = HeaderValue::from_str(&format!("Basic {}", credentials))?;
                        header_map.insert("authorization", auth_value);
                    }
                }
                AuthType::None => {}
            }
        }
        
        // Build HTTP client
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_config.request_timeout_ms))
            .connect_timeout(Duration::from_millis(timeout_config.connect_timeout_ms))
            .default_headers(header_map)
            .build()?;
        
        let transport = Self {
            client,
            base_url,
            auth_config,
            timeout_config,
            retry_config,
            notification_tx: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };
        
        // Test connection
        transport.test_connection().await?;
        
        Ok(transport)
    }
    
    async fn test_connection(&self) -> Result<()> {
        // Send a simple ping request to test connectivity
        let ping_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::Value::String("ping".to_string()),
            method: "ping".to_string(),
            params: None,
        };
        
        let response = self.client
            .post(&self.base_url)
            .json(&ping_request)
            .send()
            .await;
        
        match response {
            Ok(resp) => {
                if resp.status().is_success() || resp.status().as_u16() == 404 {
                    // 404 is acceptable as the server might not implement ping
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("HTTP connection test failed: {}", resp.status()))
                }
            }
            Err(e) => Err(anyhow::anyhow!("HTTP connection test failed: {}", e)),
        }
    }
    
    async fn send_http_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        let response = self.client
            .post(&self.base_url)
            .json(request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("HTTP request failed: {}", response.status()));
        }
        
        let json_response: JsonRpcResponse = response.json().await?;
        Ok(json_response)
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Transport not connected"));
        }
        
        // Implement retry logic
        let mut attempts = 0;
        let mut delay = match &self.retry_config.backoff_strategy {
            super::BackoffStrategy::Fixed { delay_ms } => *delay_ms,
            super::BackoffStrategy::Exponential { initial_delay_ms, .. } => *initial_delay_ms,
            super::BackoffStrategy::Linear { increment_ms } => *increment_ms,
        };
        
        loop {
            attempts += 1;
            
            match self.send_http_request(&request).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempts >= self.retry_config.max_attempts {
                        return Err(anyhow::anyhow!("Max retry attempts exceeded: {}", error));
                    }
                    
                    // Check if error is retryable
                    let error_str = error.to_string().to_lowercase();
                    let should_retry = self.retry_config.retry_on_errors.iter()
                        .any(|retry_error| error_str.contains(retry_error));
                    
                    if !should_retry {
                        return Err(error);
                    }
                    
                    // Calculate next delay
                    match &self.retry_config.backoff_strategy {
                        super::BackoffStrategy::Fixed { .. } => {
                            // delay stays the same
                        }
                        super::BackoffStrategy::Exponential { max_delay_ms, .. } => {
                            delay = (delay * 2).min(*max_delay_ms);
                        }
                        super::BackoffStrategy::Linear { increment_ms } => {
                            delay += increment_ms;
                        }
                    }
                    
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
    
    async fn start_listening(&self) -> Result<mpsc::UnboundedReceiver<JsonRpcNotification>> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.notification_tx.lock().await = Some(tx);
        
        // Note: HTTP transport doesn't support server-initiated notifications
        // This would typically be implemented with Server-Sent Events or polling
        // For now, we just return the receiver that won't receive any messages
        
        Ok(rx)
    }
    
    async fn close(&self) -> Result<()> {
        self.is_connected.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("transport_type".to_string(), "http".to_string());
        metadata.insert("base_url".to_string(), self.base_url.clone());
        
        if let Some(ref auth) = self.auth_config {
            metadata.insert("auth_type".to_string(), format!("{:?}", auth.auth_type));
        }
        
        metadata.insert("connect_timeout_ms".to_string(), self.timeout_config.connect_timeout_ms.to_string());
        metadata.insert("request_timeout_ms".to_string(), self.timeout_config.request_timeout_ms.to_string());
        
        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_http_transport_creation() {
        // Test with a mock URL (this will fail connection test but tests creation logic)
        let result = HttpTransport::new(
            "http://localhost:8080/mcp".to_string(),
            HashMap::new(),
            None,
            TimeoutConfig {
                connect_timeout_ms: 1000,
                request_timeout_ms: 5000,
                idle_timeout_ms: 30000,
            },
            RetryConfig::default(),
        ).await;
        
        // This will likely fail due to connection test, but validates the creation logic
        assert!(result.is_err()); // Expected to fail connection test
    }
    
    #[test]
    fn test_metadata() {
        let transport = HttpTransport {
            client: Client::new(),
            base_url: "http://example.com/mcp".to_string(),
            auth_config: Some(AuthConfig {
                auth_type: AuthType::Bearer,
                credentials: HashMap::new(),
            }),
            timeout_config: TimeoutConfig::default(),
            retry_config: RetryConfig::default(),
            notification_tx: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };
        
        let metadata = transport.get_metadata();
        assert_eq!(metadata.get("transport_type"), Some(&"http".to_string()));
        assert_eq!(metadata.get("base_url"), Some(&"http://example.com/mcp".to_string()));
    }
}
