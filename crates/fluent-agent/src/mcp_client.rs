use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

/// MCP Protocol version
const MCP_VERSION: &str = "2025-06-18";

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP Server capabilities
#[derive(Debug, Deserialize)]
struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompts: Option<PromptsCapability>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ToolsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    list_changed: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResourcesCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    list_changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscribe: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PromptsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    list_changed: Option<bool>,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
}

/// MCP Tool result content
#[derive(Debug, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Tool call result
#[derive(Debug, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// MCP Resource definition
#[derive(Debug, Clone, Deserialize)]
pub struct McpResource {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Client for connecting to and interacting with MCP servers
pub struct McpClient {
    server_process: Option<Child>,
    stdin: Option<Arc<Mutex<ChildStdin>>>,
    response_handlers: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<JsonRpcResponse>>>>,
    capabilities: Option<ServerCapabilities>,
    tools: Arc<RwLock<Vec<McpTool>>>,
    resources: Arc<RwLock<Vec<McpResource>>>,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new() -> Self {
        Self {
            server_process: None,
            stdin: None,
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            capabilities: None,
            tools: Arc::new(RwLock::new(Vec::new())),
            resources: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Connect to an MCP server via command execution
    pub async fn connect_to_server(&mut self, command: &str, args: &[&str]) -> Result<()> {
        // Start the server process
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = tokio::process::Command::from(cmd)
            .spawn()
            .map_err(|e| anyhow!("Failed to start MCP server: {}", e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get server stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get server stdout"))?;

        self.stdin = Some(Arc::new(Mutex::new(stdin)));
        self.server_process = Some(child);

        // Start reading responses
        self.start_response_reader(stdout).await;

        // Initialize the connection
        self.initialize().await?;

        Ok(())
    }

    /// Start reading responses from the server
    async fn start_response_reader(&self, stdout: ChildStdout) {
        let response_handlers = Arc::clone(&self.response_handlers);

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                            let id_str = response.id.to_string();
                            let handlers = response_handlers.read().await;
                            if let Some(sender) = handlers.get(&id_str) {
                                let _ = sender.send(response);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading from MCP server: {}", e);
                        break;
                    }
                }
            }
        });
    }

    /// Send a JSON-RPC request and wait for response
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(id),
            method: method.to_string(),
            params,
        };

        // Create response channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.insert(id.clone(), tx);
        }

        // Send request
        let request_json = serde_json::to_string(&request)?;
        if let Some(stdin) = &self.stdin {
            let mut stdin_guard = stdin.lock().await;
            stdin_guard.write_all(request_json.as_bytes()).await?;
            stdin_guard.write_all(b"\n").await?;
            stdin_guard.flush().await?;
        } else {
            return Err(anyhow!("Not connected to server"));
        }

        // Wait for response
        let response = rx
            .recv()
            .await
            .ok_or_else(|| anyhow!("No response received"))?;

        // Clean up handler
        {
            let mut handlers = self.response_handlers.write().await;
            handlers.remove(&id);
        }

        if let Some(error) = response.error {
            return Err(anyhow!("MCP Error {}: {}", error.code, error.message));
        }

        response
            .result
            .ok_or_else(|| anyhow!("No result in response"))
    }

    /// Initialize the MCP connection
    async fn initialize(&mut self) -> Result<()> {
        let params = json!({
            "protocolVersion": MCP_VERSION,
            "capabilities": {
                "roots": {
                    "listChanged": true
                },
                "sampling": {}
            },
            "clientInfo": {
                "name": "fluent-cli-agent",
                "version": "0.1.0"
            }
        });

        let result = self.send_request("initialize", Some(params)).await?;

        // Parse server capabilities
        if let Some(capabilities) = result.get("capabilities") {
            self.capabilities = serde_json::from_value(capabilities.clone()).ok();
        }

        // Send initialized notification
        let initialized_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(null),
            method: "notifications/initialized".to_string(),
            params: None,
        };

        if let Some(stdin) = &self.stdin {
            let request_json = serde_json::to_string(&initialized_request)?;
            let mut stdin_guard = stdin.lock().await;
            stdin_guard.write_all(request_json.as_bytes()).await?;
            stdin_guard.write_all(b"\n").await?;
            stdin_guard.flush().await?;
        }

        // Load available tools and resources
        self.refresh_tools().await?;
        self.refresh_resources().await?;

        Ok(())
    }

    /// Refresh the list of available tools from the server
    async fn refresh_tools(&self) -> Result<()> {
        if self
            .capabilities
            .as_ref()
            .and_then(|c| c.tools.as_ref())
            .is_some()
        {
            let result = self.send_request("tools/list", None).await?;
            if let Some(tools_array) = result.get("tools") {
                if let Ok(tools) = serde_json::from_value::<Vec<McpTool>>(tools_array.clone()) {
                    let mut tools_guard = self.tools.write().await;
                    *tools_guard = tools;
                }
            }
        }
        Ok(())
    }

    /// Refresh the list of available resources from the server
    async fn refresh_resources(&self) -> Result<()> {
        if self
            .capabilities
            .as_ref()
            .and_then(|c| c.resources.as_ref())
            .is_some()
        {
            let result = self.send_request("resources/list", None).await?;
            if let Some(resources_array) = result.get("resources") {
                if let Ok(resources) =
                    serde_json::from_value::<Vec<McpResource>>(resources_array.clone())
                {
                    let mut resources_guard = self.resources.write().await;
                    *resources_guard = resources;
                }
            }
        }
        Ok(())
    }

    /// Get the list of available tools
    pub async fn get_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.clone()
    }

    /// Get the list of available resources
    pub async fn get_resources(&self) -> Vec<McpResource> {
        self.resources.read().await.clone()
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.send_request("tools/call", Some(params)).await?;
        serde_json::from_value(result).map_err(|e| anyhow!("Failed to parse tool result: {}", e))
    }

    /// Read a resource from the MCP server
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        let params = json!({
            "uri": uri
        });

        self.send_request("resources/read", Some(params)).await
    }

    /// Check if the server supports tools
    pub fn supports_tools(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.tools.as_ref())
            .is_some()
    }

    /// Check if the server supports resources
    pub fn supports_resources(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.resources.as_ref())
            .is_some()
    }

    /// Check if the server supports prompts
    pub fn supports_prompts(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.prompts.as_ref())
            .is_some()
    }

    /// Disconnect from the server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut process) = self.server_process.take() {
            process.kill().await?;
        }
        self.stdin = None;
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Some(mut process) = self.server_process.take() {
            let _ = futures::executor::block_on(process.kill());
        }
    }
}

/// MCP Client Manager for handling multiple server connections
pub struct McpClientManager {
    clients: HashMap<String, McpClient>,
}

impl McpClientManager {
    /// Create a new MCP client manager
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Add a new MCP server connection
    pub async fn add_server(&mut self, name: String, command: &str, args: &[&str]) -> Result<()> {
        let mut client = McpClient::new();
        client.connect_to_server(command, args).await?;
        self.clients.insert(name, client);
        Ok(())
    }

    /// Get a client by name
    pub fn get_client(&self, name: &str) -> Option<&McpClient> {
        self.clients.get(name)
    }

    /// Get all available tools from all connected servers
    pub async fn get_all_tools(&self) -> HashMap<String, Vec<McpTool>> {
        let mut all_tools = HashMap::new();

        for (server_name, client) in &self.clients {
            let tools = client.get_tools().await;
            if !tools.is_empty() {
                all_tools.insert(server_name.clone(), tools);
            }
        }

        all_tools
    }

    /// Call a tool on a specific server
    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Value,
    ) -> Result<McpToolResult> {
        let client = self
            .clients
            .get(server_name)
            .ok_or_else(|| anyhow!("Server '{}' not found", server_name))?;

        client.call_tool(tool_name, arguments).await
    }

    /// Find and call a tool by name across all servers
    pub async fn find_and_call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<McpToolResult> {
        for (_server_name, client) in &self.clients {
            let tools = client.get_tools().await;
            if tools.iter().any(|t| t.name == tool_name) {
                return client.call_tool(tool_name, arguments).await;
            }
        }

        Err(anyhow!(
            "Tool '{}' not found on any connected server",
            tool_name
        ))
    }

    /// Disconnect all servers
    pub async fn disconnect_all(&mut self) -> Result<()> {
        for (_, mut client) in self.clients.drain() {
            client.disconnect().await?;
        }
        Ok(())
    }
}
