//! MCP (Model Context Protocol) server and client functionality
//!
//! This module provides functionality for running MCP servers and clients,
//! including agentic mode execution with MCP capabilities.

use anyhow::Result;
use clap::ArgMatches;
use fluent_core::config::Config;

/// Run MCP server
pub async fn run_mcp_server(sub_matches: &ArgMatches) -> Result<()> {
    use fluent_agent::production_mcp::{
        config::ServerConfig, 
        health::HealthMonitor, 
        metrics::MetricsCollector,
        server::ProductionMcpServerManager,
    };

    println!("🔌 Starting MCP Server...");

    // Parse configuration from command line or use defaults
    let port = sub_matches
        .get_one::<String>("port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    
    let host = sub_matches
        .get_one::<String>("host")
        .map(|h| h.to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Create server configuration
    let config = ServerConfig {
        host: host.clone(),
        port,
        max_connections: 100,
        timeout_seconds: 300,
        enable_tls: false,
        cert_path: None,
        key_path: None,
    };

    println!("📡 MCP Server configuration:");
    println!("   Host: {}", config.host);
    println!("   Port: {}", config.port);
    println!("   Max Connections: {}", config.max_connections);

    // Initialize monitoring components
    let metrics_collector = std::sync::Arc::new(MetricsCollector::new());
    let health_monitor = std::sync::Arc::new(HealthMonitor::new());

    // Create and start the server
    let server = ProductionMcpServerManager::new(
        config,
        metrics_collector.clone(),
        health_monitor.clone(),
    )
    .await?;

    println!("✅ MCP Server initialized successfully");
    println!("🚀 Starting server on {}:{}...", host, port);

    // Start the server
    server.start().await.map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}", e))?;

    println!("✨ MCP Server is running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    println!("\n🛑 Shutting down MCP Server...");
    server.stop().await.map_err(|e| anyhow::anyhow!("Error during server shutdown: {}", e))?;

    println!("✅ MCP Server stopped gracefully");
    Ok(())
}

/// Run agentic mode with goal-based execution
pub async fn run_agentic_mode(
    goal_description: &str,
    agent_config_path: &str,
    max_iterations: u32,
    enable_tools: bool,
    enable_reflection: bool,
    config_path: &str,
    model_override: Option<&str>,
    gen_retries: Option<u32>,
    min_html_size: Option<u32>,
    enable_tui: bool,
) -> Result<()> {
    println!("🎯 mcp_runner::run_agentic_mode called with TUI: {}", enable_tui);
    println!("🎯 Goal: {}", goal_description);
    use crate::agentic::{AgenticConfig, AgenticExecutor};
    // The agent builds its own engines; avoid strict global config loading
    let config = fluent_core::config::Config::new(vec![]);

    let agentic_config = AgenticConfig::new(
        goal_description.to_string(),
        agent_config_path.to_string(),
        max_iterations,
        enable_tools,
        enable_reflection,
        config_path.to_string(),
        model_override.map(|s| s.to_string()),
        gen_retries,
        min_html_size,
    );

    println!("🔧 Creating AgenticExecutor with TUI enabled: {}", enable_tui);
    let mut executor = AgenticExecutor::new(agentic_config, enable_tui);
    println!("✅ AgenticExecutor created, calling run()...");
    executor.run(&config).await?;
    println!("✅ AgenticExecutor run() completed");

    Ok(())
}

/// Run agent with MCP capabilities
pub async fn run_agent_with_mcp(
    engine_name: &str,
    task: &str,
    mcp_servers: Vec<String>,
    config: &Config,
) -> Result<()> {
    use fluent_agent::agent_with_mcp::{AgentWithMcp, MemoryQuery};
    use fluent_agent::memory::{AsyncSqliteMemoryStore, MemoryItem};
    use fluent_agent::reasoning::enhanced_multi_modal::{
        EnhancedMultiModalEngine, EnhancedReasoningConfig,
    };
    use std::sync::Arc;

    println!("🤖 Starting Agent with MCP capabilities");
    println!("   Engine: {}", engine_name);
    println!("   Task: {}", task);
    println!("   MCP Servers: {:?}", mcp_servers);

    // Initialize memory system
    let memory_db_path = std::env::var("MEMORY_DB_PATH")
        .unwrap_or_else(|_| "./agent_memory.db".to_string());
    
    println!("💾 Initializing memory system at: {}", memory_db_path);
    let memory_system = Arc::new(
        AsyncSqliteMemoryStore::new(&memory_db_path).await?
    );

    // Get the engine from config
    println!("🔧 Initializing reasoning engine: {}", engine_name);
    let base_engine = config.get_engine(engine_name)?;

    // Create reasoning engine with enhanced capabilities
    let reasoning_config = EnhancedReasoningConfig::default();
    let reasoning_engine = Box::new(EnhancedMultiModalEngine::new(
        base_engine,
        reasoning_config,
    ));

    // Create agent with MCP
    println!("🎯 Creating agent with MCP capabilities");
    let agent = AgentWithMcp::new(memory_system, reasoning_engine);

    // Connect to MCP servers
    for server_spec in mcp_servers {
        let parts: Vec<&str> = server_spec.split(':').collect();
        if parts.len() < 2 {
            println!("⚠️  Invalid MCP server specification: {}", server_spec);
            println!("   Expected format: name:command:arg1:arg2:...");
            continue;
        }

        let server_name = parts[0].to_string();
        let command = parts[1];
        let args: Vec<&str> = parts[2..].to_vec();

        println!("🔌 Connecting to MCP server: {}", server_name);
        println!("   Command: {} {}", command, args.join(" "));

        match agent.connect_to_mcp_server(server_name.clone(), command, &args).await {
            Ok(_) => println!("✅ Connected to MCP server: {}", server_name),
            Err(e) => {
                println!("❌ Failed to connect to MCP server {}: {}", server_name, e);
                println!("   Continuing with other servers...");
            }
        }
    }

    // Display available tools
    let available_tools = agent.get_available_tools().await;
    println!("\n🛠️  Available MCP Tools:");
    for (server, tools) in &available_tools {
        println!("   Server '{}':", server);
        for tool in tools {
            println!("      • {} - {}", tool.name, tool.description);
        }
    }

    if available_tools.is_empty() {
        println!("⚠️  No MCP tools available. Agent will operate in standalone mode.");
    }

    // Execute the task
    println!("\n🚀 Executing task: {}", task);
    println!("=" .repeat(60));

    match agent.execute_task(task).await {
        Ok(result) => {
            println!("\n✅ Task completed successfully!");
            println!("=" .repeat(60));
            println!("Result:");
            println!("{}", result);
            println!("=" .repeat(60));
        }
        Err(e) => {
            println!("\n❌ Task execution failed:");
            println!("   Error: {}", e);
            return Err(e);
        }
    }

    println!("\n🎉 Agent with MCP execution completed");
    Ok(())
}
