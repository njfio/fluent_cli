use super::{
    AuthConfig, AuthType, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpTransport,
    RetryConfig, TimeoutConfig,
};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use anyhow::Result;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

#[allow(dead_code)]
pub struct WebSocketTransport {
    ws_url: String,
    auth_config: Option<AuthConfig>,
    timeout_config: TimeoutConfig,
    retry_config: RetryConfig,
    connection: Arc<tokio::sync::Mutex<Option<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>>>,
    response_handlers: Arc<tokio::sync::RwLock<HashMap<String, mpsc::UnboundedSender<JsonRpcResponse>>>>,
    notification_tx: Arc<tokio::sync::Mutex<Option<mpsc::UnboundedSender<JsonRpcNotification>>>>,
    is_connected: Arc<std::sync::atomic::AtomicBool>,
    message_tx: Arc<tokio::sync::Mutex<Option<mpsc::UnboundedSender<Message>>>>,
}

impl WebSocketTransport {
    pub async fn new(
        ws_url: String,
        _headers: HashMap<String, String>,
        auth_config: Option<AuthConfig>,
        timeout_config: TimeoutConfig,
        retry_config: RetryConfig,
    ) -> Result<Self> {
        // Validate WebSocket URL
        let mut url = Url::parse(&ws_url)?;

        // Add authentication to URL or headers if needed
        if let Some(ref auth) = auth_config {
            match auth.auth_type {
                AuthType::Basic => {
                    if let (Some(username), Some(password)) = (
                        auth.credentials.get("username"),
                        auth.credentials.get("password"),
                    ) {
                        url.set_username(username)
                            .map_err(|_| anyhow::anyhow!("Invalid username"))?;
                        url.set_password(Some(password))
                            .map_err(|_| anyhow::anyhow!("Invalid password"))?;
                    }
                }
                _ => {
                    // Other auth types would be handled via headers in a real implementation
                    // For now, we'll handle them in the connection process
                }
            }
        }

        let transport = Self {
            ws_url: url.to_string(),
            auth_config,
            timeout_config,
            retry_config,
            connection: Arc::new(tokio::sync::Mutex::new(None)),
            response_handlers: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            notification_tx: Arc::new(tokio::sync::Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            message_tx: Arc::new(tokio::sync::Mutex::new(None)),
        };

        transport.connect().await?;
        Ok(transport)
    }

    /// Extract authentication token from URL query parameters or headers
    fn extract_auth_from_url(&self, url: &Url) -> Result<Option<String>> {
        // Check for token in query parameters
        for (key, value) in url.query_pairs() {
            if key == "token" || key == "auth" || key == "access_token" {
                return Ok(Some(value.to_string()));
            }
        }

        // Check auth config for bearer token
        if let Some(auth) = &self.auth_config {
            if let AuthType::Bearer = auth.auth_type {
                if let Some(token) = auth.credentials.get("token") {
                    return Ok(Some(token.clone()));
                }
            }
        }

        Ok(None)
    }

    async fn connect(&self) -> Result<()> {
        let url = Url::parse(&self.ws_url)?;

        // Extract authentication before converting URL to request
        let auth_token = self.extract_auth_from_url(&url)?;

        // Create request with authentication headers if needed
        let mut request = url.into_client_request()?;

        // Add authentication headers if available
        if let Some(auth) = auth_token {
            request.headers_mut().insert(
                "Authorization",
                format!("Bearer {}", auth).parse()
                    .map_err(|e| anyhow::anyhow!("Invalid auth header: {}", e))?
            );
        }

        let (ws_stream, _) = timeout(
            Duration::from_millis(self.timeout_config.connect_timeout_ms),
            connect_async(request),
        )
        .await??;

        *self.connection.lock().await = Some(ws_stream);
        self.is_connected
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Start message handling loops
        self.start_message_loops().await?;

        Ok(())
    }

    async fn start_message_loops(&self) -> Result<()> {
        let connection = self.connection.clone();
        let response_handlers = self.response_handlers.clone();
        let notification_tx = self.notification_tx.clone();
        let is_connected = self.is_connected.clone();
        let message_tx = self.message_tx.clone();

        // Create message sending channel
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        *message_tx.lock().await = Some(tx);

        // Spawn message reading task
        let connection_read = connection.clone();
        let response_handlers_read = response_handlers.clone();
        let notification_tx_read = notification_tx.clone();
        let is_connected_read = is_connected.clone();

        tokio::spawn(async move {
            let mut conn_guard = connection_read.lock().await;
            if let Some(ref mut ws_stream) = *conn_guard {
                while is_connected_read.load(std::sync::atomic::Ordering::Relaxed) {
                    match ws_stream.next().await {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                if value.get("id").is_some() {
                                    // This is a response
                                    if let Ok(response) =
                                        serde_json::from_value::<JsonRpcResponse>(value)
                                    {
                                        let id = response.id.to_string();
                                        let handlers = response_handlers_read.read().await;
                                        if let Some(sender) = handlers.get(&id) {
                                            let _ = sender.send(response);
                                        }
                                    }
                                } else {
                                    // This is a notification
                                    if let Ok(notification) =
                                        serde_json::from_value::<JsonRpcNotification>(value)
                                    {
                                        let tx_guard = notification_tx_read.lock().await;
                                        if let Some(ref sender) = *tx_guard {
                                            let _ = sender.send(notification);
                                        }
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => break,
                        Some(Err(_)) => break,
                        None => break,
                        _ => {} // Ignore other message types
                    }
                }
            }

            is_connected_read.store(false, std::sync::atomic::Ordering::Relaxed);
        });

        // Spawn message writing task
        let connection_write = connection.clone();
        let is_connected_write = is_connected.clone();

        tokio::spawn(async move {
            while is_connected_write.load(std::sync::atomic::Ordering::Relaxed) {
                if let Some(message) = rx.recv().await {
                    let mut conn_guard = connection_write.lock().await;
                    if let Some(ref mut ws_stream) = *conn_guard {
                        if ws_stream.send(message).await.is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            is_connected_write.store(false, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(())
    }

    async fn send_message(&self, message: Message) -> Result<()> {
        let tx_guard = self.message_tx.lock().await;
        if let Some(ref sender) = *tx_guard {
            sender.send(message)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Message sender not available"))
        }
    }
}

#[async_trait]
impl McpTransport for WebSocketTransport {
    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        if !self.is_connected().await {
            return Err(anyhow::anyhow!("Transport not connected"));
        }

        let id = request.id.to_string();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Register response handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.insert(id.clone(), tx);
        }

        // Send request
        let request_json = serde_json::to_string(&request)?;
        self.send_message(Message::Text(request_json)).await?;

        // Wait for response with timeout
        let response_option = timeout(
            Duration::from_millis(self.timeout_config.request_timeout_ms),
            rx.recv(),
        )
        .await?;

        // Clean up handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.remove(&id);
        }

        let response = response_option.ok_or_else(|| anyhow::anyhow!("No response received"))?;
        Ok(response)
    }

    async fn start_listening(&self) -> Result<mpsc::UnboundedReceiver<JsonRpcNotification>> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.notification_tx.lock().await = Some(tx);
        Ok(rx)
    }

    async fn close(&self) -> Result<()> {
        self.is_connected
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // Send close message
        let _ = self.send_message(Message::Close(None)).await;

        // Clear connection
        *self.connection.lock().await = None;

        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("transport_type".to_string(), "websocket".to_string());
        metadata.insert("ws_url".to_string(), self.ws_url.clone());

        if let Some(ref auth) = self.auth_config {
            metadata.insert("auth_type".to_string(), format!("{:?}", auth.auth_type));
        }

        metadata.insert(
            "connect_timeout_ms".to_string(),
            self.timeout_config.connect_timeout_ms.to_string(),
        );
        metadata.insert(
            "request_timeout_ms".to_string(),
            self.timeout_config.request_timeout_ms.to_string(),
        );

        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_transport_creation() {
        // Test with a mock WebSocket URL (this will fail connection but tests creation logic)
        let result = WebSocketTransport::new(
            "ws://localhost:8080/mcp".to_string(),
            HashMap::new(),
            None,
            TimeoutConfig {
                connect_timeout_ms: 1000,
                request_timeout_ms: 5000,
                idle_timeout_ms: 30000,
            },
            RetryConfig::default(),
        )
        .await;

        // This will likely fail due to connection test, but validates the creation logic
        assert!(result.is_err()); // Expected to fail connection test
    }

    #[test]
    fn test_metadata() {
        let transport = WebSocketTransport {
            ws_url: "ws://example.com/mcp".to_string(),
            auth_config: Some(AuthConfig {
                auth_type: AuthType::Bearer,
                credentials: HashMap::new(),
            }),
            timeout_config: TimeoutConfig::default(),
            retry_config: RetryConfig::default(),
            connection: Arc::new(Mutex::new(None)),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            notification_tx: Arc::new(Mutex::new(None)),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            message_tx: Arc::new(Mutex::new(None)),
        };

        let metadata = transport.get_metadata();
        assert_eq!(
            metadata.get("transport_type"),
            Some(&"websocket".to_string())
        );
        assert_eq!(
            metadata.get("ws_url"),
            Some(&"ws://example.com/mcp".to_string())
        );
    }
}
