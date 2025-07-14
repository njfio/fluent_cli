//! MCP (Model Context Protocol) server and client functionality
//!
//! This module provides functionality for running MCP servers and clients,
//! including agentic mode execution with MCP capabilities.

use anyhow::Result;
use clap::ArgMatches;
use fluent_core::config::Config;

/// Run MCP server
pub async fn run_mcp_server(_sub_matches: &ArgMatches) -> Result<()> {
    use fluent_agent::mcp_adapter::FluentMcpServer;
    use fluent_agent::memory::AsyncSqliteMemoryStore;
    use fluent_agent::tools::ToolRegistry;
    use std::sync::Arc;

    println!("🔌 Starting Fluent CLI Model Context Protocol Server");

    // Initialize tool registry
    let tool_registry = Arc::new(ToolRegistry::new());

    // Initialize memory system
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await?);

    // Create MCP server
    let server = FluentMcpServer::new(tool_registry, memory_system);

    // Use STDIO transport by default
    println!("📡 Using STDIO transport");
    println!("🚀 MCP Server ready - waiting for connections...");

    // Start the server
    server.start_stdio().await?;

    Ok(())
}

/// Run agentic mode with goal-based execution
pub async fn run_agentic_mode(
    goal_description: &str,
    agent_config_path: &str,
    max_iterations: u32,
    enable_reflection: bool,
    config_path: &str,
) -> Result<()> {
    use crate::agentic::{AgenticConfig, AgenticExecutor};
    // Config loading handled by fluent_core::config::load_config

    let config = fluent_core::config::load_config(config_path, "", &std::collections::HashMap::new())?;
    
    let agentic_config = AgenticConfig::new(
        goal_description.to_string(),
        agent_config_path.to_string(),
        max_iterations,
        enable_reflection,
        config_path.to_string(),
    );

    let executor = AgenticExecutor::new(agentic_config);
    executor.run(&config).await?;

    Ok(())
}

/// Run agent with MCP capabilities
pub async fn run_agent_with_mcp(
    engine_name: &str,
    task: &str,
    mcp_servers: Vec<String>,
    config: &Config,
) -> Result<()> {
    use fluent_agent::agent_with_mcp::AgentWithMcp;
    use fluent_agent::memory::AsyncSqliteMemoryStore;
    use fluent_agent::reasoning::LLMReasoningEngine;

    println!("🚀 Starting Fluent CLI Agent with MCP capabilities");

    // Get the engine config
    let engine_config = config
        .engines
        .iter()
        .find(|e| e.name == engine_name)
        .ok_or_else(|| anyhow::anyhow!("Engine '{}' not found in configuration", engine_name))?;

    // Create reasoning engine
    let engine = crate::create_engine(engine_config).await?;
    let reasoning_engine = LLMReasoningEngine::new(std::sync::Arc::new(engine));

    // Create memory system
    let memory_path = format!("agent_memory_{}.db", engine_name);
    let memory = std::sync::Arc::new(AsyncSqliteMemoryStore::new(&memory_path).await?);

    // Create agent
    let agent = AgentWithMcp::new(
        memory,
        Box::new(reasoning_engine),
    );

    // Connect to MCP servers
    for server_spec in mcp_servers {
        let (name, command) = if server_spec.contains(':') {
            let parts: Vec<&str> = server_spec.splitn(2, ':').collect();
            (parts[0], parts[1])
        } else {
            (server_spec.as_str(), server_spec.as_str())
        };

        println!("🔌 Connecting to MCP server: {}", name);
        match agent
            .connect_to_mcp_server(name.to_string(), command, &["--stdio"])
            .await
        {
            Ok(_) => println!("✅ Connected to {}", name),
            Err(e) => println!("⚠️ Failed to connect to {}: {}", name, e),
        }
    }

    // Show available tools
    let tools = agent.get_available_tools().await;
    if !tools.is_empty() {
        println!("\n🔧 Available MCP tools:");
        for (server, server_tools) in &tools {
            println!("  📦 {} ({} tools)", server, server_tools.len());
            for tool in server_tools.iter().take(3) {
                println!("    • {} - {}", tool.name, tool.description);
            }
            if server_tools.len() > 3 {
                println!("    ... and {} more", server_tools.len() - 3);
            }
        }
    }

    // Execute the task
    println!("\n🤖 Executing task: {}", task);
    match agent.execute_task_with_mcp(task).await {
        Ok(result) => {
            println!("\n✅ Task completed successfully!");
            println!("📋 Result:\n{}", result);
        }
        Err(e) => {
            println!("\n❌ Task failed: {}", e);

            // Show learning insights
            println!("\n🧠 Learning from this experience...");
            if let Ok(insights) = agent.learn_from_mcp_usage("task execution").await {
                for insight in insights.iter().take(3) {
                    println!("💡 {}", insight);
                }
            }
        }
    }

    Ok(())
}
