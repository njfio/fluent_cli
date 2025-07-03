use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::context::ExecutionContext;
use crate::goal::{Goal, GoalType};
use crate::mcp_client::{McpClientManager, McpTool, McpToolResult};
use crate::memory::{LongTermMemory, MemoryItem, MemoryQuery, MemoryType};
use crate::orchestrator::{Observation, ObservationType};
use crate::reasoning::ReasoningEngine;

/// Enhanced agent that can use MCP tools and resources
pub struct AgentWithMcp {
    mcp_manager: Arc<RwLock<McpClientManager>>,
    memory_system: Arc<dyn LongTermMemory>,
    reasoning_engine: Box<dyn ReasoningEngine>,
    available_tools: Arc<RwLock<HashMap<String, Vec<McpTool>>>>,
}

impl AgentWithMcp {
    /// Create a new agent with MCP capabilities
    pub fn new(
        memory_system: Arc<dyn LongTermMemory>,
        reasoning_engine: Box<dyn ReasoningEngine>,
    ) -> Self {
        Self {
            mcp_manager: Arc::new(RwLock::new(McpClientManager::new())),
            memory_system,
            reasoning_engine,
            available_tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Connect to an MCP server
    pub async fn connect_to_mcp_server(
        &self,
        name: String,
        command: &str,
        args: &[&str],
    ) -> Result<()> {
        let mut manager = self.mcp_manager.write().await;
        manager.add_server(name.clone(), command, args).await?;

        // Refresh available tools
        let all_tools = manager.get_all_tools().await;
        let mut tools_guard = self.available_tools.write().await;
        *tools_guard = all_tools;

        // Store connection info in memory
        let memory_item = MemoryItem {
            memory_id: uuid::Uuid::new_v4().to_string(),
            memory_type: MemoryType::Experience,
            content: format!(
                "Connected to MCP server '{}' with command: {} {}",
                name,
                command,
                args.join(" ")
            ),
            metadata: HashMap::new(),
            importance: 0.8,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            tags: vec!["mcp".to_string(), "connection".to_string()],
            embedding: None,
        };

        self.memory_system.store(memory_item).await?;

        println!("✅ Connected to MCP server: {}", name);
        Ok(())
    }

    /// Get all available MCP tools
    pub async fn get_available_tools(&self) -> HashMap<String, Vec<McpTool>> {
        self.available_tools.read().await.clone()
    }

    /// Use reasoning to determine which tool to use for a given task
    pub async fn reason_about_tool_usage(
        &self,
        task: &str,
    ) -> Result<Option<(String, String, Value)>> {
        let tools = self.get_available_tools().await;

        if tools.is_empty() {
            return Ok(None);
        }

        // Create a prompt for the reasoning engine
        let tools_description = tools
            .iter()
            .flat_map(|(server, server_tools)| {
                server_tools.iter().map(move |tool| {
                    format!(
                        "Server '{}': Tool '{}' - {}",
                        server, tool.name, tool.description
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("\n");

        let reasoning_prompt = format!(
            "Given the task: '{}'\n\nAvailable MCP tools:\n{}\n\nWhich tool should be used and with what parameters? Respond with JSON in format: {{\"server\": \"server_name\", \"tool\": \"tool_name\", \"arguments\": {{...}}}} or {{\"no_tool\": true}} if no tool is suitable.",
            task, tools_description
        );

        // Create execution context for reasoning
        let goal = Goal::new(task.to_string(), GoalType::Analysis);
        let mut context = ExecutionContext::new(goal);

        let tools_obs = Observation {
            observation_id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            observation_type: ObservationType::SystemEvent,
            content: format!("Available MCP tools: {}", tools_description),
            source: "mcp_client".to_string(),
            relevance_score: 1.0,
            impact_assessment: Some("High - affects tool selection".to_string()),
        };
        context.add_observation(tools_obs);

        let prompt_obs = Observation {
            observation_id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            observation_type: ObservationType::SystemEvent,
            content: format!("Reasoning prompt: {}", reasoning_prompt),
            source: "reasoning_engine".to_string(),
            relevance_score: 0.8,
            impact_assessment: Some("Medium - provides context for reasoning".to_string()),
        };
        context.add_observation(prompt_obs);

        let reasoning_result = self.reasoning_engine.reason(&context).await?;

        // Parse the reasoning result
        if let Ok(parsed) = serde_json::from_str::<Value>(&reasoning_result.reasoning_output) {
            if parsed.get("no_tool").is_some() {
                return Ok(None);
            }

            if let (Some(server), Some(tool), Some(args)) = (
                parsed.get("server").and_then(|v| v.as_str()),
                parsed.get("tool").and_then(|v| v.as_str()),
                parsed.get("arguments"),
            ) {
                return Ok(Some((server.to_string(), tool.to_string(), args.clone())));
            }
        }

        Ok(None)
    }

    /// Execute a task using MCP tools
    pub async fn execute_task_with_mcp(&self, task: &str) -> Result<String> {
        println!("🤖 Executing task: {}", task);

        // First, reason about which tool to use
        if let Some((server, tool_name, arguments)) = self.reason_about_tool_usage(task).await? {
            println!("🔧 Using tool '{}' from server '{}'", tool_name, server);

            // Execute the tool
            let manager = self.mcp_manager.read().await;
            let result = manager
                .call_tool(&server, &tool_name, arguments.clone())
                .await?;

            // Process the result
            let result_text = self.process_tool_result(&result).await?;

            // Store the execution in memory
            let memory_item = MemoryItem {
                memory_id: uuid::Uuid::new_v4().to_string(),
                memory_type: MemoryType::Experience,
                content: format!(
                    "Executed task '{}' using tool '{}' from server '{}'. Result: {}",
                    task, tool_name, server, result_text
                ),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("task".to_string(), json!(task));
                    meta.insert("server".to_string(), json!(server));
                    meta.insert("tool".to_string(), json!(tool_name));
                    meta.insert("arguments".to_string(), arguments);
                    meta
                },
                importance: 0.7,
                created_at: chrono::Utc::now(),
                last_accessed: chrono::Utc::now(),
                access_count: 0,
                tags: vec![
                    "mcp".to_string(),
                    "execution".to_string(),
                    tool_name.clone(),
                ],
                embedding: None,
            };

            self.memory_system.store(memory_item).await?;

            Ok(result_text)
        } else {
            // No suitable MCP tool found, try to handle with built-in capabilities
            println!("⚠️ No suitable MCP tool found for task: {}", task);

            // Store this as a learning experience
            let memory_item = MemoryItem {
                memory_id: uuid::Uuid::new_v4().to_string(),
                memory_type: MemoryType::Learning,
                content: format!("Could not find suitable MCP tool for task: {}", task),
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("task".to_string(), json!(task));
                    meta.insert(
                        "available_tools".to_string(),
                        json!(self.get_available_tools().await),
                    );
                    meta
                },
                importance: 0.6,
                created_at: chrono::Utc::now(),
                last_accessed: chrono::Utc::now(),
                access_count: 0,
                tags: vec!["mcp".to_string(), "no_tool".to_string()],
                embedding: None,
            };

            self.memory_system.store(memory_item).await?;

            Err(anyhow!("No suitable MCP tool available for task: {}", task))
        }
    }

    /// Process MCP tool result into a readable format
    async fn process_tool_result(&self, result: &McpToolResult) -> Result<String> {
        if result.is_error.unwrap_or(false) {
            return Err(anyhow!("Tool execution failed"));
        }

        let mut output = String::new();
        for content in &result.content {
            match content.content_type.as_str() {
                "text" => {
                    if let Some(text) = &content.text {
                        output.push_str(text);
                        output.push('\n');
                    }
                }
                "image" => {
                    output.push_str(&format!(
                        "[Image: {}]\n",
                        content.mime_type.as_deref().unwrap_or("unknown")
                    ));
                }
                "audio" => {
                    output.push_str(&format!(
                        "[Audio: {}]\n",
                        content.mime_type.as_deref().unwrap_or("unknown")
                    ));
                }
                _ => {
                    output.push_str(&format!("[Content: {}]\n", content.content_type));
                }
            }
        }

        Ok(output.trim().to_string())
    }

    /// Learn from past MCP tool usage
    pub async fn learn_from_mcp_usage(&self, task_pattern: &str) -> Result<Vec<String>> {
        let query = MemoryQuery {
            query_text: format!("mcp execution {}", task_pattern),
            memory_types: vec![MemoryType::Experience],
            tags: vec!["mcp".to_string(), "execution".to_string()],
            time_range: None,
            importance_threshold: Some(0.5),
            limit: Some(10),
        };

        let memories = self.memory_system.retrieve(&query).await?;

        let insights = memories
            .iter()
            .map(|memory| {
                format!(
                    "Previous execution: {} (importance: {:.2})",
                    memory.content, memory.importance
                )
            })
            .collect();

        Ok(insights)
    }

    /// Get recommendations for MCP servers to connect to
    pub async fn get_mcp_server_recommendations(&self, domain: &str) -> Vec<String> {
        // This would typically query a registry or use AI to recommend servers
        // For now, return some common MCP servers based on domain
        match domain.to_lowercase().as_str() {
            "filesystem" => vec![
                "mcp-server-filesystem".to_string(),
                "file-operations-server".to_string(),
            ],
            "git" => vec![
                "mcp-server-git".to_string(),
                "github-mcp-server".to_string(),
            ],
            "database" => vec![
                "mcp-server-sqlite".to_string(),
                "postgres-mcp-server".to_string(),
            ],
            "web" => vec![
                "mcp-server-fetch".to_string(),
                "browser-automation-server".to_string(),
            ],
            _ => vec!["general-purpose-mcp-server".to_string()],
        }
    }

    /// Disconnect from all MCP servers
    pub async fn disconnect_all_mcp_servers(&self) -> Result<()> {
        let mut manager = self.mcp_manager.write().await;
        manager.disconnect_all().await?;

        let mut tools_guard = self.available_tools.write().await;
        tools_guard.clear();

        println!("🔌 Disconnected from all MCP servers");
        Ok(())
    }
}
