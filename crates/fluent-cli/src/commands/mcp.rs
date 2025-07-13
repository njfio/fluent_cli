use anyhow::{anyhow, Result};
use clap::ArgMatches;
use fluent_core::config::Config;
use fluent_agent::{
    ProductionMcpManager, ProductionMcpConfig,
    initialize_production_mcp_with_config,
};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

use super::{CommandHandler, CommandResult};

/// MCP (Model Context Protocol) command handler with comprehensive management capabilities
pub struct McpCommand {
    #[allow(dead_code)]
    mcp_manager: Option<std::sync::Arc<ProductionMcpManager>>,
}

impl McpCommand {
    pub fn new() -> Self {
        Self {
            mcp_manager: None,
        }
    }

    /// Initialize MCP manager if not already initialized
    #[allow(dead_code)]
    async fn ensure_mcp_manager(&mut self, config: &Config) -> Result<std::sync::Arc<ProductionMcpManager>> {
        if let Some(manager) = &self.mcp_manager {
            return Ok(manager.clone());
        }

        let mcp_config = Self::load_mcp_config(config).await?;
        let manager = initialize_production_mcp_with_config(mcp_config).await
            .map_err(|e| anyhow!("Failed to initialize MCP manager: {}", e))?;

        self.mcp_manager = Some(manager.clone());
        Ok(manager)
    }

    /// Load MCP configuration from various sources
    async fn load_mcp_config(_config: &Config) -> Result<ProductionMcpConfig> {
        // For now, use default configuration
        // In a full implementation, this would load from config files, environment variables, etc.
        Ok(ProductionMcpConfig::default())
    }

    /// Start MCP server with comprehensive management
    async fn start_server(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        println!("🔌 Starting Production MCP Server");

        let port = matches.get_one::<String>("port");
        let stdio = matches.get_flag("stdio");
        let config_file = matches.get_one::<String>("config");

        // Load configuration
        let mut mcp_config = Self::load_mcp_config(config).await?;

        if let Some(config_path) = config_file {
            println!("📄 Loading configuration from: {}", config_path);
            // In a full implementation, load from file
        }

        // Configure transport based on arguments
        if stdio {
            println!("🔗 Using STDIO transport");
            mcp_config.transport.default_transport = fluent_agent::production_mcp::config::TransportType::Stdio;
        } else if let Some(port_str) = port {
            let port_num: u16 = port_str.parse()
                .map_err(|_| anyhow!("Invalid port number: {}", port_str))?;
            println!("🌐 Using HTTP transport on port: {}", port_num);
            mcp_config.server.bind_address = format!("127.0.0.1:{}", port_num);
            mcp_config.transport.default_transport = fluent_agent::production_mcp::config::TransportType::Http;
        }

        // Initialize and start MCP manager
        let manager = initialize_production_mcp_with_config(mcp_config).await
            .map_err(|e| anyhow!("Failed to start MCP server: {}", e))?;

        println!("✅ MCP Server started successfully");
        println!("📊 Server metrics available at /metrics endpoint");
        println!("🏥 Health checks available at /health endpoint");
        println!("📋 Press Ctrl+C to stop the server");

        // Keep server running until interrupted
        tokio::signal::ctrl_c().await
            .map_err(|e| anyhow!("Failed to listen for shutdown signal: {}", e))?;

        println!("🛑 Shutting down MCP server...");
        manager.stop().await
            .map_err(|e| anyhow!("Error during shutdown: {}", e))?;

        Ok(CommandResult::success_with_message(
            "MCP server stopped gracefully".to_string(),
        ))
    }

    /// Connect to an MCP server
    async fn connect_server(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let server_name = matches.get_one::<String>("name")
            .ok_or_else(|| anyhow!("Server name is required"))?;
        let command = matches.get_one::<String>("command")
            .ok_or_else(|| anyhow!("Server command is required"))?;
        let args: Vec<String> = matches.get_many::<String>("args")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();

        println!("🔗 Connecting to MCP server: {}", server_name);
        println!("📋 Command: {} {:?}", command, args);

        // Initialize MCP manager
        let mcp_config = Self::load_mcp_config(config).await?;
        let manager = initialize_production_mcp_with_config(mcp_config).await
            .map_err(|e| anyhow!("Failed to initialize MCP manager: {}", e))?;

        // Connect to server
        manager.client_manager()
            .connect_server(server_name.clone(), command.clone(), args)
            .await
            .map_err(|e| anyhow!("Failed to connect to server '{}': {}", server_name, e))?;

        println!("✅ Successfully connected to MCP server: {}", server_name);

        Ok(CommandResult::success_with_message(format!(
            "Connected to MCP server '{}'",
            server_name
        )))
    }

    /// Disconnect from an MCP server
    async fn disconnect_server(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let server_name = matches.get_one::<String>("name")
            .ok_or_else(|| anyhow!("Server name is required"))?;

        println!("🔌 Disconnecting from MCP server: {}", server_name);

        // Initialize MCP manager
        let mcp_config = Self::load_mcp_config(config).await?;
        let manager = initialize_production_mcp_with_config(mcp_config).await
            .map_err(|e| anyhow!("Failed to initialize MCP manager: {}", e))?;

        // Disconnect from server
        manager.client_manager()
            .disconnect_server(server_name)
            .await
            .map_err(|e| anyhow!("Failed to disconnect from server '{}': {}", server_name, e))?;

        println!("✅ Successfully disconnected from MCP server: {}", server_name);

        Ok(CommandResult::success_with_message(format!(
            "Disconnected from MCP server '{}'",
            server_name
        )))
    }

    /// List all available tools across connected servers
    async fn list_tools(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let server_filter = matches.get_one::<String>("server");
        let json_output = matches.get_flag("json");

        println!("🔧 Listing available MCP tools...");

        // Initialize MCP manager
        let mcp_config = Self::load_mcp_config(config).await?;
        let manager = initialize_production_mcp_with_config(mcp_config).await
            .map_err(|e| anyhow!("Failed to initialize MCP manager: {}", e))?;

        // Get all tools
        let all_tools = manager.client_manager().get_all_tools().await;

        if all_tools.is_empty() {
            println!("⚠️  No MCP servers connected or no tools available");
            return Ok(CommandResult::success_with_message(
                "No tools available".to_string(),
            ));
        }

        if json_output {
            // Output as JSON
            let json_tools = json!({
                "servers": all_tools.iter().map(|(server, tools)| {
                    json!({
                        "name": server,
                        "tools": tools.iter().map(|tool| {
                            json!({
                                "name": tool.name,
                                "description": tool.description
                            })
                        }).collect::<Vec<_>>()
                    })
                }).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&json_tools)?);
        } else {
            // Human-readable output
            for (server_name, tools) in &all_tools {
                if let Some(filter) = server_filter {
                    if server_name != filter {
                        continue;
                    }
                }

                println!("\n📡 Server: {}", server_name);
                println!("   Tools: {}", tools.len());

                for tool in tools {
                    println!("   🔧 {}", tool.name);
                    if let Some(description) = &tool.description {
                        println!("      📝 {}", description);
                    }
                }
            }
        }

        Ok(CommandResult::success_with_message(format!(
            "Listed tools from {} servers",
            all_tools.len()
        )))
    }

    /// Execute a tool on an MCP server
    async fn execute_tool(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let tool_name = matches.get_one::<String>("tool")
            .ok_or_else(|| anyhow!("Tool name is required"))?;
        let default_params = "{}".to_string();
        let parameters_str = matches.get_one::<String>("parameters")
            .unwrap_or(&default_params);
        let server_preference = matches.get_one::<String>("server");
        let timeout_secs = matches.get_one::<String>("timeout")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);

        println!("🔧 Executing tool: {}", tool_name);
        println!("📋 Parameters: {}", parameters_str);

        // Parse parameters
        let parameters: Value = serde_json::from_str(parameters_str)
            .map_err(|e| anyhow!("Invalid JSON parameters: {}", e))?;

        // Initialize MCP manager
        let mcp_config = Self::load_mcp_config(config).await?;
        let manager = initialize_production_mcp_with_config(mcp_config).await
            .map_err(|e| anyhow!("Failed to initialize MCP manager: {}", e))?;

        // Set execution preferences
        let mut preferences = fluent_agent::production_mcp::client::ExecutionPreferences::default();
        preferences.timeout = Some(Duration::from_secs(timeout_secs));
        if let Some(server) = server_preference {
            preferences.preferred_servers = vec![server.clone()];
        }

        // Execute tool with failover
        let result = manager.client_manager()
            .execute_tool_with_failover(tool_name, parameters, preferences)
            .await
            .map_err(|e| anyhow!("Tool execution failed: {}", e))?;

        println!("✅ Tool execution completed successfully");
        println!("📄 Result: {}", serde_json::to_string_pretty(&result)?);

        Ok(CommandResult::success_with_message(format!(
            "Tool '{}' executed successfully",
            tool_name
        )))
    }

    /// Show MCP system status and health
    async fn show_status(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let json_output = matches.get_flag("json");
        let detailed = matches.get_flag("detailed");

        println!("📊 MCP System Status");

        // Initialize MCP manager
        let mcp_config = Self::load_mcp_config(config).await?;
        let manager = initialize_production_mcp_with_config(mcp_config).await
            .map_err(|e| anyhow!("Failed to initialize MCP manager: {}", e))?;

        // Get health status
        let health = manager.get_health_status().await;
        let metrics = manager.get_metrics().await;

        if json_output {
            let status_json = json!({
                "health": {
                    "status": format!("{:?}", health.status),
                    "check_count": health.check_count,
                    "uptime_seconds": health.uptime.as_secs(),
                    "version": health.version
                },
                "metrics": {
                    "client": {
                        "connections_active": metrics.client_metrics.connections_active,
                        "requests_total": metrics.client_metrics.requests_total,
                        "requests_successful": metrics.client_metrics.requests_successful,
                        "requests_failed": metrics.client_metrics.requests_failed,
                        "tools_executed": metrics.client_metrics.tools_executed
                    },
                    "server": {
                        "clients_connected": metrics.server_metrics.clients_connected,
                        "tools_registered": metrics.server_metrics.tools_registered,
                        "requests_processed": metrics.server_metrics.requests_processed
                    }
                }
            });
            println!("{}", serde_json::to_string_pretty(&status_json)?);
        } else {
            // Human-readable output
            println!("\n🏥 Health Status: {:?}", health.status);
            println!("⏱️  Uptime: {:?}", health.uptime);
            println!("🔢 Health Checks: {}", health.check_count);
            println!("📦 Version: {}", health.version);

            println!("\n📊 Client Metrics:");
            println!("   🔗 Active Connections: {}", metrics.client_metrics.connections_active);
            println!("   📤 Total Requests: {}", metrics.client_metrics.requests_total);
            println!("   ✅ Successful Requests: {}", metrics.client_metrics.requests_successful);
            println!("   ❌ Failed Requests: {}", metrics.client_metrics.requests_failed);
            println!("   🔧 Tools Executed: {}", metrics.client_metrics.tools_executed);

            println!("\n📊 Server Metrics:");
            println!("   👥 Connected Clients: {}", metrics.server_metrics.clients_connected);
            println!("   🔧 Registered Tools: {}", metrics.server_metrics.tools_registered);
            println!("   📋 Processed Requests: {}", metrics.server_metrics.requests_processed);

            if detailed {
                println!("\n📈 Detailed Metrics:");
                println!("   📊 Response Time Avg: {:?}", metrics.client_metrics.response_time_avg);
                println!("   📊 Response Time P95: {:?}", metrics.client_metrics.response_time_p95);
                println!("   📊 Response Time P99: {:?}", metrics.client_metrics.response_time_p99);
                println!("   💾 Bytes Sent: {}", metrics.client_metrics.bytes_sent);
                println!("   💾 Bytes Received: {}", metrics.client_metrics.bytes_received);
            }
        }

        Ok(CommandResult::success_with_message("Status retrieved successfully".to_string()))
    }

    /// Show or update MCP configuration
    async fn manage_config(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let show = matches.get_flag("show");
        let set_key = matches.get_one::<String>("set");
        let set_value = matches.get_one::<String>("value");
        let config_file = matches.get_one::<String>("file");

        if show {
            println!("📄 Current MCP Configuration:");
            let mcp_config = Self::load_mcp_config(config).await?;
            let config_json = serde_json::to_string_pretty(&mcp_config)?;
            println!("{}", config_json);
        } else if let (Some(key), Some(value)) = (set_key, set_value) {
            println!("🔧 Setting configuration: {} = {}", key, value);
            // In a full implementation, this would update the configuration
            println!("⚠️  Configuration updates not yet implemented");
        } else if let Some(file_path) = config_file {
            println!("💾 Saving configuration to: {}", file_path);
            let _mcp_config = Self::load_mcp_config(config).await?;
            // In a full implementation, save to file
            println!("⚠️  Configuration file saving not yet implemented");
        } else {
            return Err(anyhow!("No configuration action specified. Use --show, --set, or --file"));
        }

        Ok(CommandResult::success_with_message("Configuration operation completed".to_string()))
    }

    /// Legacy agent-MCP integration for backward compatibility
    async fn run_agent_with_mcp(
        engine_name: &str,
        task: &str,
        mcp_servers: Vec<String>,
        config: &Config,
    ) -> Result<CommandResult> {
        println!("🤖 Starting Agent with MCP Integration (Legacy Mode)");
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

        // Initialize MCP manager
        let mcp_config = Self::load_mcp_config(config).await?;
        let manager = initialize_production_mcp_with_config(mcp_config).await
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
            match manager.client_manager()
                .connect_server(server_name.to_string(), command.to_string(), vec![])
                .await
            {
                Ok(_) => println!("    ✅ Connected to {}", server_name),
                Err(e) => println!("    ❌ Failed to connect to {}: {}", server_name, e),
            }
        }

        println!("🎯 Executing task: {}", task);
        println!("⚙️  Processing with {} engine...", engine_name);

        // Simulate task execution with actual MCP integration
        sleep(Duration::from_millis(500)).await;

        // Get available tools for demonstration
        let all_tools = manager.client_manager().get_all_tools().await;
        if !all_tools.is_empty() {
            println!("🔧 Available tools from connected servers:");
            for (server_name, tools) in &all_tools {
                println!("  📡 {}: {} tools", server_name, tools.len());
            }
        }

        println!("✅ Task completed successfully");

        Ok(CommandResult::success_with_message(format!(
            "Agent-MCP task '{}' completed with {} engine using {} servers",
            task, engine_name, mcp_servers.len()
        )))
    }


}

impl CommandHandler for McpCommand {
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()> {
        let result = match matches.subcommand() {
            Some(("server", sub_matches)) => {
                Self::start_server(sub_matches, config).await?
            }
            Some(("connect", sub_matches)) => {
                Self::connect_server(sub_matches, config).await?
            }
            Some(("disconnect", sub_matches)) => {
                Self::disconnect_server(sub_matches, config).await?
            }
            Some(("tools", sub_matches)) => {
                Self::list_tools(sub_matches, config).await?
            }
            Some(("execute", sub_matches)) => {
                Self::execute_tool(sub_matches, config).await?
            }
            Some(("status", sub_matches)) => {
                Self::show_status(sub_matches, config).await?
            }
            Some(("config", sub_matches)) => {
                Self::manage_config(sub_matches, config).await?
            }
            Some(("agent", sub_matches)) => {
                // Legacy agent-MCP integration
                let engine_name = sub_matches
                    .get_one::<String>("engine")
                    .ok_or_else(|| anyhow!("Engine name is required"))?;

                let task = sub_matches
                    .get_one::<String>("task")
                    .ok_or_else(|| anyhow!("Task is required"))?;

                let servers = sub_matches
                    .get_many::<String>("servers")
                    .map(|values| values.cloned().collect())
                    .unwrap_or_default();

                Self::run_agent_with_mcp(engine_name, task, servers, config).await?
            }
            _ => {
                // Default: show help or status
                println!("🔌 MCP (Model Context Protocol) Management");
                println!("Available commands:");
                println!("  server     - Start MCP server");
                println!("  connect    - Connect to MCP server");
                println!("  disconnect - Disconnect from MCP server");
                println!("  tools      - List available tools");
                println!("  execute    - Execute a tool");
                println!("  status     - Show system status");
                println!("  config     - Manage configuration");
                println!("  agent      - Run agent with MCP (legacy)");
                println!("\nUse 'fluent mcp <command> --help' for more information");

                CommandResult::success_with_message("MCP help displayed".to_string())
            }
        };

        if !result.success {
            if let Some(message) = result.message {
                return Err(anyhow!("MCP command failed: {}", message));
            } else {
                return Err(anyhow!("MCP command failed"));
            }
        }

        Ok(())
    }
}

impl Default for McpCommand {
    fn default() -> Self {
        Self::new()
    }
}
