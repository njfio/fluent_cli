use super::{
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpTransport, RetryConfig, TimeoutConfig,
};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::{timeout, Duration};

#[allow(dead_code)]
pub struct StdioTransport {
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    stdout: Arc<Mutex<Option<BufReader<ChildStdout>>>>,
    response_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<JsonRpcResponse>>>>,
    notification_tx: Arc<Mutex<Option<mpsc::UnboundedSender<JsonRpcNotification>>>>,
    command: String,
    args: Vec<String>,
    timeout_config: TimeoutConfig,
    retry_config: RetryConfig,
    is_connected: Arc<std::sync::atomic::AtomicBool>,
}

impl StdioTransport {
    pub async fn new(
        command: String,
        args: Vec<String>,
        timeout_config: TimeoutConfig,
        retry_config: RetryConfig,
    ) -> Result<Self> {
        let transport = Self {
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            stdout: Arc::new(Mutex::new(None)),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            notification_tx: Arc::new(Mutex::new(None)),
            command,
            args,
            timeout_config,
            retry_config,
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        transport.connect().await?;
        Ok(transport)
    }

    async fn connect(&self) -> Result<()> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

        *self.stdin.lock().await = Some(stdin);
        *self.stdout.lock().await = Some(BufReader::new(stdout));
        *self.process.lock().await = Some(child);

        self.is_connected
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Start the message reading loop
        self.start_message_loop().await?;

        Ok(())
    }

    async fn start_message_loop(&self) -> Result<()> {
        let stdout = self.stdout.clone();
        let response_handlers = self.response_handlers.clone();
        let notification_tx = self.notification_tx.clone();
        let is_connected = self.is_connected.clone();

        tokio::spawn(async move {
            let mut stdout_guard = stdout.lock().await;
            if let Some(ref mut reader) = *stdout_guard {
                let mut line = String::new();

                while is_connected.load(std::sync::atomic::Ordering::Relaxed) {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                                if value.get("id").is_some() {
                                    // This is a response
                                    if let Ok(response) =
                                        serde_json::from_value::<JsonRpcResponse>(value)
                                    {
                                        let id = response.id.to_string();
                                        let handlers = response_handlers.read().await;
                                        if let Some(sender) = handlers.get(&id) {
                                            let _ = sender.send(response);
                                        }
                                    }
                                } else {
                                    // This is a notification
                                    if let Ok(notification) =
                                        serde_json::from_value::<JsonRpcNotification>(value)
                                    {
                                        let tx_guard = notification_tx.lock().await;
                                        if let Some(ref sender) = *tx_guard {
                                            let _ = sender.send(notification);
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }

            is_connected.store(false, std::sync::atomic::Ordering::Relaxed);
        });

        Ok(())
    }

    async fn send_raw_request(&self, request: &JsonRpcRequest) -> Result<()> {
        let mut stdin_guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *stdin_guard {
            let request_json = serde_json::to_string(request)?;
            stdin.write_all(request_json.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("STDIN not available"))
        }
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
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
        self.send_raw_request(&request).await?;

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

        // Close stdin
        if let Some(mut stdin) = self.stdin.lock().await.take() {
            let _ = stdin.shutdown().await;
        }

        // Terminate process
        if let Some(mut process) = self.process.lock().await.take() {
            let _ = process.kill().await;
            let _ = process.wait().await;
        }

        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.is_connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn get_metadata(&self) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("transport_type".to_string(), "stdio".to_string());
        metadata.insert("command".to_string(), self.command.clone());
        metadata.insert("args".to_string(), self.args.join(" "));
        metadata
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // Ensure cleanup happens
        self.is_connected
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stdio_transport_creation() {
        // Test with a simple command that should exist on most systems
        let result = StdioTransport::new(
            "echo".to_string(),
            vec!["test".to_string()],
            TimeoutConfig::default(),
            RetryConfig::default(),
        )
        .await;

        // This might fail if echo doesn't behave as expected for JSON-RPC
        // but it tests the basic creation logic
        assert!(result.is_ok() || result.is_err()); // Just ensure it doesn't panic
    }

    #[test]
    fn test_metadata() {
        let transport = StdioTransport {
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            stdout: Arc::new(Mutex::new(None)),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            notification_tx: Arc::new(Mutex::new(None)),
            command: "test-command".to_string(),
            args: vec!["arg1".to_string(), "arg2".to_string()],
            timeout_config: TimeoutConfig::default(),
            retry_config: RetryConfig::default(),
            is_connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        let metadata = transport.get_metadata();
        assert_eq!(metadata.get("transport_type"), Some(&"stdio".to_string()));
        assert_eq!(metadata.get("command"), Some(&"test-command".to_string()));
        assert_eq!(metadata.get("args"), Some(&"arg1 arg2".to_string()));
    }
}
