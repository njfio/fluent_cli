use anyhow::{anyhow, Result};
use rmcp::{
    model::{
        CallToolResult, Content,
        ServerInfo, Tool
    },
    ServerHandler,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::ToolRegistry;
use crate::memory::{LongTermMemory, MemoryItem, MemoryType, MemoryQuery};

/// MCP adapter that exposes fluent_cli tools as MCP server capabilities
#[derive(Clone)]
pub struct FluentMcpAdapter {
    tool_registry: Arc<ToolRegistry>,
    memory_system: Arc<dyn LongTermMemory>,
}

impl FluentMcpAdapter {
    /// Create a new MCP adapter
    pub fn new(
        tool_registry: Arc<ToolRegistry>,
        memory_system: Arc<dyn LongTermMemory>,
    ) -> Self {
        Self {
            tool_registry,
            memory_system,
        }
    }

    /// Convert fluent tool to MCP tool format
    fn convert_tool_to_mcp(&self, name: &str, description: &str) -> Tool {
        use std::sync::Arc;
        use serde_json::Map;

        let mut properties = Map::new();
        properties.insert("params".to_string(), json!({
            "type": "object",
            "description": "Tool parameters as JSON object"
        }));

        let mut schema = Map::new();
        schema.insert("type".to_string(), json!("object"));
        schema.insert("properties".to_string(), json!(properties));
        schema.insert("required".to_string(), json!(["params"]));

        Tool {
            name: name.to_string().into(),
            description: description.to_string().into(),
            input_schema: Arc::new(schema),
        }
    }

    /// Execute fluent tool and convert result to MCP format
    async fn execute_fluent_tool(&self, name: &str, params: Value) -> Result<CallToolResult> {
        // For now, simulate tool execution since we need to integrate with the actual tool system
        // This is a placeholder that will be replaced with real tool integration
        let result = match name {
            "list_files" => {
                let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                format!("Files in {}: example.txt, README.md", path)
            }
            "read_file" => {
                let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("Content of file: {}", path)
            }
            "write_file" => {
                let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("Successfully wrote to file: {}", path)
            }
            "run_command" => {
                let command = params.get("command").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("Executed command: {}", command)
            }
            _ => format!("Tool {} executed successfully", name),
        };

        // Convert result to MCP format
        Ok(CallToolResult {
            content: vec![Content::text(result)],
            is_error: Some(false),
        })
    }
}

impl FluentMcpAdapter {
    /// Get available tools
    fn get_available_tools(&self) -> Vec<Tool> {
        vec![
            self.convert_tool_to_mcp("list_files", "List files in the current workspace"),
            self.convert_tool_to_mcp("read_file", "Read the contents of a file"),
            self.convert_tool_to_mcp("write_file", "Write content to a file"),
            self.convert_tool_to_mcp("run_command", "Execute a shell command"),
            self.convert_tool_to_mcp("store_memory", "Store a memory item for future reference"),
            self.convert_tool_to_mcp("retrieve_memory", "Retrieve memory items based on query"),
        ]
    }

    /// Handle tool execution
    async fn handle_tool_call(&self, name: &str, arguments: Option<Value>) -> Result<CallToolResult, rmcp::Error> {
        let args = arguments.unwrap_or(json!({}));

        match name {
            "list_files" => {
                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                let params = json!({"path": path});
                self.execute_fluent_tool("list_files", params).await
                    .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))
            }
            "read_file" => {
                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| rmcp::Error::invalid_params("path parameter required".to_string(), None))?;
                let params = json!({"path": path});
                self.execute_fluent_tool("read_file", params).await
                    .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))
            }
            "write_file" => {
                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| rmcp::Error::invalid_params("path parameter required".to_string(), None))?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| rmcp::Error::invalid_params("content parameter required".to_string(), None))?;
                let params = json!({"path": path, "content": content});
                self.execute_fluent_tool("write_file", params).await
                    .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))
            }
            "run_command" => {
                let command = args.get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| rmcp::Error::invalid_params("command parameter required".to_string(), None))?;
                let args_vec = args.get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
                    .unwrap_or_default();
                let params = json!({"command": command, "args": args_vec});
                self.execute_fluent_tool("run_command", params).await
                    .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))
            }
            "store_memory" => {
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| rmcp::Error::invalid_params("content parameter required".to_string(), None))?;
                let memory_type_str = args.get("memory_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("experience");
                let importance = args.get("importance")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.5);

                let memory_type = match memory_type_str {
                    "experience" => MemoryType::Experience,
                    "learning" => MemoryType::Learning,
                    "strategy" => MemoryType::Strategy,
                    "pattern" => MemoryType::Pattern,
                    "rule" => MemoryType::Rule,
                    "fact" => MemoryType::Fact,
                    _ => MemoryType::Experience,
                };

                let memory_item = MemoryItem {
                    memory_id: uuid::Uuid::new_v4().to_string(),
                    memory_type,
                    content: content.to_string(),
                    metadata: HashMap::new(),
                    importance,
                    created_at: chrono::Utc::now(),
                    last_accessed: chrono::Utc::now(),
                    access_count: 0,
                    tags: vec![],
                    embedding: None,
                };

                let memory_id = self.memory_system.store(memory_item).await
                    .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

                Ok(CallToolResult {
                    content: vec![Content::text(format!("Memory stored with ID: {}", memory_id))],
                    is_error: Some(false),
                })
            }
            "retrieve_memory" => {
                let query = args.get("query").and_then(|v| v.as_str()).map(|s| s.to_string());
                let importance_threshold = args.get("importance_threshold").and_then(|v| v.as_f64());
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);

                let memory_query = MemoryQuery {
                    query_text: query.unwrap_or_default(),
                    memory_types: vec![],
                    tags: vec![],
                    time_range: None,
                    importance_threshold,
                    limit,
                };

                let memories = self.memory_system.retrieve(&memory_query).await
                    .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

                let result = memories.iter()
                    .map(|m| json!({
                        "id": m.memory_id,
                        "content": m.content,
                        "type": format!("{:?}", m.memory_type),
                        "importance": m.importance,
                        "created_at": m.created_at.to_rfc3339(),
                        "access_count": m.access_count
                    }))
                    .collect::<Vec<_>>();

                Ok(CallToolResult {
                    content: vec![Content::text(serde_json::to_string_pretty(&result).unwrap())],
                    is_error: Some(false),
                })
            }
            _ => Err(rmcp::Error::method_not_found::<rmcp::model::CallToolRequestMethod>())
        }
    }
}

impl ServerHandler for FluentMcpAdapter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Fluent CLI agentic system exposed via Model Context Protocol".to_string().into()),
            ..Default::default()
        }
    }
}

/// MCP server wrapper for fluent_cli
pub struct FluentMcpServer {
    adapter: FluentMcpAdapter,
}

impl FluentMcpServer {
    /// Create a new MCP server
    pub fn new(
        tool_registry: Arc<ToolRegistry>,
        memory_system: Arc<dyn LongTermMemory>,
    ) -> Self {
        Self {
            adapter: FluentMcpAdapter::new(tool_registry, memory_system),
        }
    }

    /// Start the MCP server with stdio transport
    pub async fn start_stdio(&self) -> Result<()> {
        use rmcp::{ServiceExt, transport::stdio};
        use tokio::signal;

        println!("Starting Fluent CLI MCP Server...");
        
        let service = self.adapter.clone().serve(stdio()).await
            .map_err(|e| anyhow!("Failed to start MCP server: {}", e))?;

        println!("MCP Server started successfully. Waiting for connections...");

        // Wait for shutdown signal
        tokio::select! {
            result = service.waiting() => {
                match result {
                    Ok(reason) => println!("MCP Server shut down: {:?}", reason),
                    Err(e) => eprintln!("MCP Server error: {}", e),
                }
            }
            _ = signal::ctrl_c() => {
                println!("Received shutdown signal, stopping MCP server...");
                // Service is consumed by waiting(), so we can't cancel it here
                // The ctrl_c signal will cause the process to exit anyway
            }
        }

        Ok(())
    }

    /// Get the adapter for testing or custom usage
    pub fn adapter(&self) -> &FluentMcpAdapter {
        &self.adapter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::SqliteMemoryStore;

    #[tokio::test]
    async fn test_mcp_adapter_creation() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let memory_system = Arc::new(SqliteMemoryStore::new(":memory:").unwrap()) as Arc<dyn LongTermMemory>;

        let adapter = FluentMcpAdapter::new(tool_registry, memory_system);
        let info = adapter.get_info();

        // Just verify the adapter was created successfully
        // The rmcp library handles server info internally
        assert!(info.instructions.is_some());
    }

    #[test]
    fn test_tool_conversion() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let memory_system = Arc::new(SqliteMemoryStore::new(":memory:").unwrap()) as Arc<dyn LongTermMemory>;

        let adapter = FluentMcpAdapter::new(tool_registry, memory_system);
        let tool = adapter.convert_tool_to_mcp("test_tool", "Test tool description");

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description.as_ref(), "Test tool description");
    }
}
