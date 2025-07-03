use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::transport::{JsonRpcRequest, McpTransport, TransportConfig, TransportFactory};

/// MCP Protocol version
const MCP_VERSION: &str = "2025-06-18";

/// Enhanced MCP client with multi-transport support
pub struct EnhancedMcpClient {
    transport: Box<dyn McpTransport>,
    tools: Arc<RwLock<Vec<McpTool>>>,
    resources: Arc<RwLock<Vec<McpResource>>>,
    capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    client_info: ClientInfo,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<ToolResultContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub data: Option<String>,
    pub annotations: Option<Value>,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    pub resources: Option<ResourcesCapability>,
    pub prompts: Option<PromptsCapability>,
    pub logging: Option<LoggingCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    pub subscribe: Option<bool>,
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingCapability {
    pub level: Option<String>,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "fluent-cli".to_string(),
            version: "0.1.0".to_string(),
        }
    }
}

impl EnhancedMcpClient {
    /// Create a new enhanced MCP client with transport configuration
    pub async fn new(transport_config: TransportConfig) -> Result<Self> {
        let transport = TransportFactory::create_transport(transport_config).await?;

        let client = Self {
            transport,
            tools: Arc::new(RwLock::new(Vec::new())),
            resources: Arc::new(RwLock::new(Vec::new())),
            capabilities: Arc::new(RwLock::new(None)),
            client_info: ClientInfo::default(),
        };

        // Initialize the connection
        client.initialize().await?;

        Ok(client)
    }

    /// Initialize the MCP connection
    async fn initialize(&self) -> Result<()> {
        // Send initialize request
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(Uuid::new_v4().to_string()),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": MCP_VERSION,
                "capabilities": {
                    "roots": {
                        "listChanged": true
                    },
                    "sampling": {}
                },
                "clientInfo": self.client_info
            })),
        };

        let response = self.transport.send_request(init_request).await?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "Initialize failed: {} - {}",
                error.code,
                error.message
            ));
        }

        if let Some(result) = response.result {
            if let Ok(server_caps) = serde_json::from_value::<ServerCapabilities>(
                result.get("capabilities").unwrap_or(&json!({})).clone(),
            ) {
                *self.capabilities.write().await = Some(server_caps);
            }
        }

        // Send initialized notification
        let initialized_notification = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(null),
            method: "notifications/initialized".to_string(),
            params: None,
        };

        // For notifications, we don't wait for a response
        let _ = self.transport.send_request(initialized_notification).await;

        // Discover available tools and resources
        self.discover_tools().await?;
        self.discover_resources().await?;

        Ok(())
    }

    /// Discover available tools from the server
    async fn discover_tools(&self) -> Result<()> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(Uuid::new_v4().to_string()),
            method: "tools/list".to_string(),
            params: None,
        };

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "Tools discovery failed: {} - {}",
                error.code,
                error.message
            ));
        }

        if let Some(result) = response.result {
            if let Some(tools_array) = result.get("tools").and_then(|t| t.as_array()) {
                let mut tools = self.tools.write().await;
                tools.clear();

                for tool_value in tools_array {
                    if let Ok(tool) = serde_json::from_value::<McpTool>(tool_value.clone()) {
                        tools.push(tool);
                    }
                }
            }
        }

        Ok(())
    }

    /// Discover available resources from the server
    async fn discover_resources(&self) -> Result<()> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(Uuid::new_v4().to_string()),
            method: "resources/list".to_string(),
            params: None,
        };

        let response = self.transport.send_request(request).await?;

        if let Some(_error) = response.error {
            // Resources might not be supported, so we don't fail here
            return Ok(());
        }

        if let Some(result) = response.result {
            if let Some(resources_array) = result.get("resources").and_then(|r| r.as_array()) {
                let mut resources = self.resources.write().await;
                resources.clear();

                for resource_value in resources_array {
                    if let Ok(resource) =
                        serde_json::from_value::<McpResource>(resource_value.clone())
                    {
                        resources.push(resource);
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a tool with the given parameters
    pub async fn execute_tool(&self, tool_name: &str, parameters: Value) -> Result<McpToolResult> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(Uuid::new_v4().to_string()),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": tool_name,
                "arguments": parameters
            })),
        };

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "Tool execution failed: {} - {}",
                error.code,
                error.message
            ));
        }

        if let Some(result) = response.result {
            let tool_result = serde_json::from_value::<McpToolResult>(result)?;
            Ok(tool_result)
        } else {
            Err(anyhow!("No result returned from tool execution"))
        }
    }

    /// Read a resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(Uuid::new_v4().to_string()),
            method: "resources/read".to_string(),
            params: Some(json!({
                "uri": uri
            })),
        };

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "Resource read failed: {} - {}",
                error.code,
                error.message
            ));
        }

        response
            .result
            .ok_or_else(|| anyhow!("No result returned from resource read"))
    }

    /// Get available tools
    pub async fn get_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.clone()
    }

    /// Get available resources
    pub async fn get_resources(&self) -> Vec<McpResource> {
        self.resources.read().await.clone()
    }

    /// Get server capabilities
    pub async fn get_capabilities(&self) -> Option<ServerCapabilities> {
        self.capabilities.read().await.clone()
    }

    /// Check if the client is connected
    pub async fn is_connected(&self) -> bool {
        self.transport.is_connected().await
    }

    /// Close the connection
    pub async fn close(&self) -> Result<()> {
        self.transport.close().await
    }

    /// Get transport metadata
    pub fn get_transport_metadata(&self) -> HashMap<String, String> {
        self.transport.get_metadata()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{ConnectionConfig, RetryConfig, TimeoutConfig, TransportType};

    #[tokio::test]
    async fn test_enhanced_mcp_client_creation() {
        let config = TransportConfig {
            transport_type: TransportType::Http,
            connection_config: ConnectionConfig::Http {
                base_url: "http://localhost:8080/mcp".to_string(),
                headers: HashMap::new(),
            },
            auth_config: None,
            timeout_config: TimeoutConfig::default(),
            retry_config: RetryConfig::default(),
        };

        // This will fail due to no server, but tests the creation logic
        let result = EnhancedMcpClient::new(config).await;
        assert!(result.is_err()); // Expected to fail connection
    }

    #[test]
    fn test_client_info_default() {
        let client_info = ClientInfo::default();
        assert_eq!(client_info.name, "fluent-cli");
        assert_eq!(client_info.version, "0.1.0");
    }
}
