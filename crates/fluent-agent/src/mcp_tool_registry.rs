// MCP Tool Registry for managing MCP-compatible tools
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tools::{ToolExecutor, ToolRegistry};

/// MCP Tool definition with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub title: Option<String>,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Option<Value>,
    pub category: String,
    pub tags: Vec<String>,
    pub version: String,
    pub author: Option<String>,
    pub documentation_url: Option<String>,
    pub examples: Vec<McpToolExample>,
}

/// Example usage of an MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolExample {
    pub name: String,
    pub description: String,
    pub input: Value,
    pub expected_output: Option<Value>,
}

/// MCP Tool execution result with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolExecutionResult {
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub tool_name: String,
    pub tool_version: String,
    pub metadata: HashMap<String, Value>,
}

/// MCP Tool Registry for managing and executing MCP-compatible tools
pub struct McpToolRegistry {
    tools: Arc<RwLock<HashMap<String, McpToolDefinition>>>,
    executors: Arc<RwLock<HashMap<String, Arc<dyn ToolExecutor>>>>,
    base_tool_registry: Arc<ToolRegistry>,
    execution_stats: Arc<RwLock<HashMap<String, McpToolStats>>>,
}

/// Statistics for MCP tool usage
#[derive(Debug, Clone, Default)]
pub struct McpToolStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: f64,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
}

impl McpToolRegistry {
    /// Create a new MCP tool registry
    pub fn new(base_tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            executors: Arc::new(RwLock::new(HashMap::new())),
            base_tool_registry,
            execution_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the registry with standard MCP tools
    pub async fn initialize_standard_tools(&self) -> Result<()> {
        // Register file system tools
        self.register_file_system_tools().await?;
        
        // Register memory tools
        self.register_memory_tools().await?;
        
        // Register system tools
        self.register_system_tools().await?;
        
        // Register code tools
        self.register_code_tools().await?;

        println!("✅ Initialized {} MCP tools", self.tools.read().await.len());
        Ok(())
    }

    /// Register file system related MCP tools
    async fn register_file_system_tools(&self) -> Result<()> {
        // Read file tool
        let read_file_tool = McpToolDefinition {
            name: "read_file".to_string(),
            title: Some("Read File".to_string()),
            description: "Read the contents of a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    }
                },
                "required": ["path"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "File contents"
                    },
                    "size": {
                        "type": "number",
                        "description": "File size in bytes"
                    }
                }
            })),
            category: "filesystem".to_string(),
            tags: vec!["file".to_string(), "read".to_string(), "io".to_string()],
            version: "1.0.0".to_string(),
            author: Some("Fluent CLI".to_string()),
            documentation_url: None,
            examples: vec![
                McpToolExample {
                    name: "Read README".to_string(),
                    description: "Read the README.md file".to_string(),
                    input: json!({"path": "README.md"}),
                    expected_output: Some(json!({"content": "# Project Title\n...", "size": 1024})),
                }
            ],
        };

        // Write file tool
        let write_file_tool = McpToolDefinition {
            name: "write_file".to_string(),
            title: Some("Write File".to_string()),
            description: "Write content to a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "bytes_written": {
                        "type": "number",
                        "description": "Number of bytes written"
                    }
                }
            })),
            category: "filesystem".to_string(),
            tags: vec!["file".to_string(), "write".to_string(), "io".to_string()],
            version: "1.0.0".to_string(),
            author: Some("Fluent CLI".to_string()),
            documentation_url: None,
            examples: vec![
                McpToolExample {
                    name: "Write config".to_string(),
                    description: "Write configuration to a file".to_string(),
                    input: json!({"path": "config.json", "content": "{\"key\": \"value\"}"}),
                    expected_output: Some(json!({"bytes_written": 16})),
                }
            ],
        };

        // List directory tool
        let list_dir_tool = McpToolDefinition {
            name: "list_directory".to_string(),
            title: Some("List Directory".to_string()),
            description: "List contents of a directory".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the directory to list"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Whether to list recursively",
                        "default": false
                    }
                },
                "required": ["path"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "entries": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "type": {"type": "string", "enum": ["file", "directory"]},
                                "size": {"type": "number"}
                            }
                        }
                    }
                }
            })),
            category: "filesystem".to_string(),
            tags: vec!["directory".to_string(), "list".to_string(), "io".to_string()],
            version: "1.0.0".to_string(),
            author: Some("Fluent CLI".to_string()),
            documentation_url: None,
            examples: vec![
                McpToolExample {
                    name: "List current directory".to_string(),
                    description: "List files in current directory".to_string(),
                    input: json!({"path": "."}),
                    expected_output: Some(json!({"entries": [{"name": "README.md", "type": "file", "size": 1024}]})),
                }
            ],
        };

        self.register_tool(read_file_tool).await?;
        self.register_tool(write_file_tool).await?;
        self.register_tool(list_dir_tool).await?;

        Ok(())
    }

    /// Register memory-related MCP tools
    async fn register_memory_tools(&self) -> Result<()> {
        let store_memory_tool = McpToolDefinition {
            name: "store_memory".to_string(),
            title: Some("Store Memory".to_string()),
            description: "Store information in long-term memory".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Content to store in memory"
                    },
                    "importance": {
                        "type": "number",
                        "description": "Importance score (0.0 to 1.0)",
                        "minimum": 0.0,
                        "maximum": 1.0
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags for categorization"
                    }
                },
                "required": ["content"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "memory_id": {
                        "type": "string",
                        "description": "Unique identifier for the stored memory"
                    }
                }
            })),
            category: "memory".to_string(),
            tags: vec!["memory".to_string(), "storage".to_string(), "persistence".to_string()],
            version: "1.0.0".to_string(),
            author: Some("Fluent CLI".to_string()),
            documentation_url: None,
            examples: vec![
                McpToolExample {
                    name: "Store important fact".to_string(),
                    description: "Store an important piece of information".to_string(),
                    input: json!({"content": "User prefers JSON format for configuration", "importance": 0.8, "tags": ["preference", "config"]}),
                    expected_output: Some(json!({"memory_id": "mem_12345"})),
                }
            ],
        };

        self.register_tool(store_memory_tool).await?;
        Ok(())
    }

    /// Register system-related MCP tools
    async fn register_system_tools(&self) -> Result<()> {
        let execute_command_tool = McpToolDefinition {
            name: "execute_command".to_string(),
            title: Some("Execute Command".to_string()),
            description: "Execute a system command safely".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Command arguments"
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Working directory for command execution"
                    }
                },
                "required": ["command"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "stdout": {"type": "string"},
                    "stderr": {"type": "string"},
                    "exit_code": {"type": "number"},
                    "execution_time_ms": {"type": "number"}
                }
            })),
            category: "system".to_string(),
            tags: vec!["command".to_string(), "execution".to_string(), "system".to_string()],
            version: "1.0.0".to_string(),
            author: Some("Fluent CLI".to_string()),
            documentation_url: None,
            examples: vec![
                McpToolExample {
                    name: "List files".to_string(),
                    description: "List files using ls command".to_string(),
                    input: json!({"command": "ls", "args": ["-la"]}),
                    expected_output: Some(json!({"stdout": "total 8\ndrwxr-xr-x...", "stderr": "", "exit_code": 0})),
                }
            ],
        };

        self.register_tool(execute_command_tool).await?;
        Ok(())
    }

    /// Register code-related MCP tools
    async fn register_code_tools(&self) -> Result<()> {
        let compile_rust_tool = McpToolDefinition {
            name: "compile_rust".to_string(),
            title: Some("Compile Rust Code".to_string()),
            description: "Compile Rust code and return compilation results".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source_path": {
                        "type": "string",
                        "description": "Path to Rust source file or project"
                    },
                    "release": {
                        "type": "boolean",
                        "description": "Whether to compile in release mode",
                        "default": false
                    }
                },
                "required": ["source_path"]
            }),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "output": {"type": "string"},
                    "errors": {"type": "array", "items": {"type": "string"}},
                    "warnings": {"type": "array", "items": {"type": "string"}}
                }
            })),
            category: "code".to_string(),
            tags: vec!["rust".to_string(), "compilation".to_string(), "development".to_string()],
            version: "1.0.0".to_string(),
            author: Some("Fluent CLI".to_string()),
            documentation_url: None,
            examples: vec![
                McpToolExample {
                    name: "Compile main.rs".to_string(),
                    description: "Compile a simple Rust file".to_string(),
                    input: json!({"source_path": "src/main.rs"}),
                    expected_output: Some(json!({"success": true, "output": "Compiled successfully", "errors": [], "warnings": []})),
                }
            ],
        };

        self.register_tool(compile_rust_tool).await?;
        Ok(())
    }

    /// Register a new MCP tool
    pub async fn register_tool(&self, tool: McpToolDefinition) -> Result<()> {
        let mut tools = self.tools.write().await;

        if tools.contains_key(&tool.name) {
            return Err(anyhow!("Tool '{}' already registered", tool.name));
        }

        // Validate tool definition
        self.validate_tool_definition(&tool)?;

        tools.insert(tool.name.clone(), tool.clone());

        // Initialize stats
        let mut stats = self.execution_stats.write().await;
        stats.insert(tool.name.clone(), McpToolStats::default());

        println!("✅ Registered MCP tool: {} ({})", tool.name, tool.description);
        Ok(())
    }

    /// Validate tool definition
    fn validate_tool_definition(&self, tool: &McpToolDefinition) -> Result<()> {
        if tool.name.is_empty() {
            return Err(anyhow!("Tool name cannot be empty"));
        }

        if tool.description.is_empty() {
            return Err(anyhow!("Tool description cannot be empty"));
        }

        // Validate input schema is valid JSON Schema
        if !tool.input_schema.is_object() {
            return Err(anyhow!("Tool input schema must be a JSON object"));
        }

        // Check for required schema properties
        if let Some(schema_obj) = tool.input_schema.as_object() {
            if !schema_obj.contains_key("type") {
                return Err(anyhow!("Tool input schema must have a 'type' property"));
            }
        }

        Ok(())
    }

    /// Get all registered tools
    pub async fn list_tools(&self) -> Vec<McpToolDefinition> {
        self.tools.read().await.values().cloned().collect()
    }

    /// Get a specific tool by name
    pub async fn get_tool(&self, name: &str) -> Option<McpToolDefinition> {
        self.tools.read().await.get(name).cloned()
    }

    /// Check if a tool exists
    pub async fn has_tool(&self, name: &str) -> bool {
        self.tools.read().await.contains_key(name)
    }

    /// Execute an MCP tool
    pub async fn execute_tool(&self, name: &str, input: Value) -> Result<McpToolExecutionResult> {
        let start_time = std::time::Instant::now();

        // Get tool definition
        let tool = self.get_tool(name).await
            .ok_or_else(|| anyhow!("Tool '{}' not found", name))?;

        // Validate input against schema
        self.validate_input(&tool, &input)?;

        // Convert input to tool registry format
        let tool_args = self.convert_input_to_args(&input)?;

        // Convert string args to Value args for tool registry
        let value_args: HashMap<String, Value> = tool_args.into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();

        // Execute using base tool registry
        let result = match self.base_tool_registry.execute_tool(name, &value_args).await {
            Ok(output) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                // Update stats
                self.update_tool_stats(name, true, execution_time).await;

                McpToolExecutionResult {
                    success: true,
                    output: json!(output),
                    error: None,
                    execution_time_ms: execution_time,
                    tool_name: tool.name.clone(),
                    tool_version: tool.version.clone(),
                    metadata: HashMap::new(),
                }
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;

                // Update stats
                self.update_tool_stats(name, false, execution_time).await;

                McpToolExecutionResult {
                    success: false,
                    output: json!(null),
                    error: Some(e.to_string()),
                    execution_time_ms: execution_time,
                    tool_name: tool.name.clone(),
                    tool_version: tool.version.clone(),
                    metadata: HashMap::new(),
                }
            }
        };

        Ok(result)
    }

    /// Validate input against tool schema
    fn validate_input(&self, tool: &McpToolDefinition, input: &Value) -> Result<()> {
        // Basic validation - in a full implementation, use a JSON Schema validator
        if let Some(schema_obj) = tool.input_schema.as_object() {
            if let Some(required) = schema_obj.get("required") {
                if let Some(required_array) = required.as_array() {
                    for required_field in required_array {
                        if let Some(field_name) = required_field.as_str() {
                            if !input.get(field_name).is_some() {
                                return Err(anyhow!(
                                    "Required field '{}' missing in input for tool '{}'",
                                    field_name,
                                    tool.name
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Convert MCP input to tool registry arguments
    fn convert_input_to_args(&self, input: &Value) -> Result<HashMap<String, String>> {
        let mut args = HashMap::new();

        if let Some(obj) = input.as_object() {
            for (key, value) in obj {
                let string_value = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Array(_) | Value::Object(_) => serde_json::to_string(value)?,
                    Value::Null => "null".to_string(),
                };
                args.insert(key.clone(), string_value);
            }
        }

        Ok(args)
    }

    /// Update tool execution statistics
    async fn update_tool_stats(&self, tool_name: &str, success: bool, execution_time_ms: u64) {
        let mut stats = self.execution_stats.write().await;
        if let Some(tool_stats) = stats.get_mut(tool_name) {
            tool_stats.total_executions += 1;
            tool_stats.total_execution_time_ms += execution_time_ms;

            if success {
                tool_stats.successful_executions += 1;
            } else {
                tool_stats.failed_executions += 1;
            }

            tool_stats.average_execution_time_ms =
                tool_stats.total_execution_time_ms as f64 / tool_stats.total_executions as f64;
            tool_stats.last_execution = Some(chrono::Utc::now());
        }
    }

    /// Get tool execution statistics
    pub async fn get_tool_stats(&self, tool_name: &str) -> Option<McpToolStats> {
        self.execution_stats.read().await.get(tool_name).cloned()
    }

    /// Get all tool statistics
    pub async fn get_all_stats(&self) -> HashMap<String, McpToolStats> {
        self.execution_stats.read().await.clone()
    }

    /// Get tools by category
    pub async fn get_tools_by_category(&self, category: &str) -> Vec<McpToolDefinition> {
        self.tools.read().await
            .values()
            .filter(|tool| tool.category == category)
            .cloned()
            .collect()
    }

    /// Search tools by tag
    pub async fn search_tools_by_tag(&self, tag: &str) -> Vec<McpToolDefinition> {
        self.tools.read().await
            .values()
            .filter(|tool| tool.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Get tool categories
    pub async fn get_categories(&self) -> Vec<String> {
        let tools = self.tools.read().await;
        let mut categories: Vec<String> = tools.values()
            .map(|tool| tool.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }
}
