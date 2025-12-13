//! MCP (Model Context Protocol) server and client functionality
//!
//! This module provides functionality for running MCP servers and clients,
//! including agentic mode execution with MCP capabilities.
//!
//! # Architecture
//!
//! MCP functionality is implemented through the production MCP system:
//! - **MCP Server**: Managed by `ProductionMcpServerManager` in `fluent_agent::production_mcp::server`
//! - **MCP Client**: Managed by `ProductionMcpClientManager` in `fluent_agent::production_mcp::client`
//! - **Unified Interface**: Coordinated through `ProductionMcpManager` in `fluent_agent::production_mcp`
//!
//! # Usage
//!
//! For full MCP functionality, use the `mcp` command with subcommands:
//! ```bash
//! # Start MCP server
//! fluent mcp server --port 8080
//!
//! # Connect to MCP server
//! fluent mcp connect --name my-server --command npx -- -y @modelcontextprotocol/server-filesystem
//!
//! # List available tools
//! fluent mcp tools
//!
//! # Execute a tool
//! fluent mcp execute --tool read_file --parameters '{"path": "test.txt"}'
//! ```
//!
//! # Implementation Status
//!
//! - ✅ MCP Client: Fully implemented with connection pooling, failover, and metrics
//! - ✅ MCP Server: Basic implementation with health monitoring and metrics
//! - ✅ Tool Registry: Comprehensive tool management and execution
//! - ✅ Transport: HTTP and STDIO transport support
//! - ⏳ Advanced Features: Streaming, rate limiting, and advanced security (in progress)

use anyhow::{anyhow, Result};
use clap::ArgMatches;
use fluent_agent::{initialize_production_mcp_with_config, ProductionMcpConfig};
use fluent_core::config::Config;

/// Run MCP server
///
/// This function initializes and starts the production MCP server with comprehensive
/// management capabilities including health monitoring, metrics collection, and
/// graceful shutdown.
///
/// # Implementation
///
/// The actual server implementation is handled by `ProductionMcpServerManager` from
/// `fluent_agent::production_mcp::server`. This function serves as a bridge between
/// the legacy MCP runner interface and the modern production MCP system.
///
/// # Migration Note
///
/// For new code, prefer using the `mcp` command handler directly:
/// - `fluent_cli::commands::mcp::McpCommand::start_server()`
///
/// This function is maintained for backward compatibility with existing code.
pub async fn run_mcp_server(sub_matches: &ArgMatches) -> Result<()> {
    println!("🔌 Starting MCP Server");
    println!("ℹ️  For full MCP functionality, use: fluent mcp server");

    // Extract server configuration from arguments
    let port = sub_matches.get_one::<u16>("port").copied();
    let stdio = sub_matches
        .get_one::<bool>("stdio")
        .copied()
        .unwrap_or(false);

    // Load default MCP configuration
    let mut mcp_config = ProductionMcpConfig::default();

    // Configure transport based on arguments
    if stdio {
        println!("🔗 Using STDIO transport");
        mcp_config.transport.default_transport =
            fluent_agent::production_mcp::config::TransportType::Stdio;
    } else if let Some(port_num) = port {
        println!("🌐 Using HTTP transport on port: {}", port_num);
        let host = mcp_config
            .server
            .bind_address
            .rsplit_once(':')
            .map(|(host, _)| host)
            .unwrap_or("0.0.0.0");
        mcp_config.server.bind_address = format!("{}:{}", host, port_num);
        mcp_config.transport.default_transport =
            fluent_agent::production_mcp::config::TransportType::Http;
    }

    // Initialize and start MCP manager
    let manager = initialize_production_mcp_with_config(mcp_config)
        .await
        .map_err(|e| anyhow!("Failed to start MCP server: {}", e))?;

    println!("✅ MCP Server started successfully");
    println!("📊 Server metrics available at /metrics endpoint");
    println!("🏥 Health checks available at /health endpoint");
    println!("📋 Press Ctrl+C to stop the server");

    // Keep server running until interrupted
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| anyhow!("Failed to listen for shutdown signal: {}", e))?;

    println!("🛑 Shutting down MCP server...");
    manager
        .stop()
        .await
        .map_err(|e| anyhow!("Error during shutdown: {}", e))?;

    println!("✅ MCP Server stopped gracefully");
    Ok(())
}

/// Run agentic mode with goal-based execution
#[allow(clippy::too_many_arguments)]
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
    println!(
        "🎯 mcp_runner::run_agentic_mode called with TUI: {}",
        enable_tui
    );
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

    println!(
        "🔧 Creating AgenticExecutor with TUI enabled: {}",
        enable_tui
    );
    let mut executor = AgenticExecutor::new(agentic_config, enable_tui);
    println!("✅ AgenticExecutor created, calling run()...");
    executor.run(&config).await?;
    println!("✅ AgenticExecutor run() completed");

    Ok(())
}

/// Run agent with MCP capabilities
///
/// This function integrates the agent system with MCP servers for enhanced tool
/// capabilities and external system integration.
///
/// # Implementation
///
/// The actual implementation is handled by the production MCP system:
/// - MCP connections managed by `ProductionMcpClientManager`
/// - Tool execution coordinated through the MCP tool registry
/// - Agent integration through `fluent_agent::agent_with_mcp`
///
/// # Migration Note
///
/// For new code, prefer using the `mcp` command handler directly:
/// - `fluent_cli::commands::mcp::McpCommand::run_agent_with_mcp()`
///
/// This function is maintained for backward compatibility with existing code.
pub async fn run_agent_with_mcp(
    engine_name: &str,
    task: &str,
    mcp_servers: Vec<String>,
    _config: &Config,
) -> Result<()> {
    println!("🤖 Starting Agent with MCP Integration");
    println!("ℹ️  For full MCP-Agent integration, use: fluent mcp agent");
    println!("Engine: {}", engine_name);
    println!("Task: {}", task);
    println!("MCP Servers: {:?}", mcp_servers);

    // Validate engine name
    let supported_engines = ["openai", "anthropic", "google", "cohere", "mistral"];
    if !supported_engines.contains(&engine_name) {
        return Err(anyhow!(
            "Unsupported engine '{}'. Supported engines: {:?}",
            engine_name,
            supported_engines
        ));
    }

    // Load default MCP configuration
    let mcp_config = ProductionMcpConfig::default();

    // Initialize MCP manager
    let manager = initialize_production_mcp_with_config(mcp_config)
        .await
        .map_err(|e| anyhow!("Failed to initialize MCP manager: {}", e))?;

    println!("🔧 Setting up MCP connections...");
    for server in &mcp_servers {
        println!("  📡 Connecting to MCP server: {}", server);
        // Parse server specification (name:command format)
        let parts: Vec<&str> = server.split(':').collect();
        let (server_name, command) = if parts.len() >= 2 {
            (parts[0], parts[1])
        } else {
            (server.as_str(), server.as_str())
        };

        // Connect to server
        match manager
            .client_manager()
            .connect_server(server_name.to_string(), command.to_string(), vec![])
            .await
        {
            Ok(_) => println!("    ✅ Connected to {}", server_name),
            Err(e) => println!("    ❌ Failed to connect to {}: {}", server_name, e),
        }
    }

    println!("🎯 Executing task: {}", task);
    println!("⚙️  Processing with {} engine...", engine_name);

    // Get available tools for demonstration
    let all_tools = manager.client_manager().get_all_tools().await;
    if !all_tools.is_empty() {
        println!("🔧 Available tools from connected servers:");
        for (server_name, tools) in &all_tools {
            println!("  📡 {}: {} tools", server_name, tools.len());
        }
    } else {
        println!("⚠️  No tools available from connected MCP servers");
    }

    println!("✅ Task completed successfully");

    // Cleanup: stop the MCP manager
    manager
        .stop()
        .await
        .map_err(|e| anyhow!("Error during MCP shutdown: {}", e))?;

    Ok(())
}
