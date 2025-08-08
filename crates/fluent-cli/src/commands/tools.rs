//! Direct tool access command handler
//!
//! This module provides comprehensive CLI commands for direct tool access,
//! including discovery, execution, configuration, and monitoring.

use anyhow::{anyhow, Result};
use crate::error::CliError;
use clap::ArgMatches;
use fluent_core::config::Config;
use fluent_agent::tools::ToolRegistry;
use fluent_agent::config::ToolConfig;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use once_cell::sync::Lazy;

use super::{CommandHandler, CommandResult};

/// Global tool registry instance for reuse across commands
static GLOBAL_TOOL_REGISTRY: Lazy<Arc<Mutex<Option<ToolRegistry>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Tool command handler for direct tool access
pub struct ToolsCommand;

impl ToolsCommand {
    /// Create a new tools command handler
    pub fn new() -> Self {
        Self
    }

    /// Get or initialize the global tool registry with configuration
    fn get_tool_registry(_config: &Config) -> Result<Arc<Mutex<Option<ToolRegistry>>>> {
        let registry_guard = GLOBAL_TOOL_REGISTRY.clone();

        // Check if registry is already initialized
        let is_initialized = {
            let registry_lock = registry_guard.lock()
                .map_err(|e| CliError::Unknown(format!("Failed to acquire registry lock: {}", e)))?;
            registry_lock.is_some()
        };

        if is_initialized {
            return Ok(registry_guard);
        }

        // Initialize registry if not already done
        let tool_config = ToolConfig {
            file_operations: true,
            shell_commands: false, // Default to false for security
            rust_compiler: true,
            git_operations: false,
            allowed_paths: Some(vec![
                "./".to_string(),
                "./src".to_string(),
                "./examples".to_string(),
                "./tests".to_string(),
            ]),
            allowed_commands: Some(vec![
                "cargo build".to_string(),
                "cargo test".to_string(),
                "cargo check".to_string(),
                "cargo clippy".to_string(),
            ]),
        };

        let new_registry = ToolRegistry::with_standard_tools(&tool_config);

        {
            let mut registry_lock = registry_guard.lock()
                .map_err(|e| CliError::Unknown(format!("Failed to acquire registry lock for initialization: {}", e)))?;
            *registry_lock = Some(new_registry);
        }

        Ok(registry_guard)
    }

    /// Execute with the tool registry, providing thread-safe access
    fn with_tool_registry<F, R>(config: &Config, f: F) -> Result<R>
    where
        F: FnOnce(&ToolRegistry) -> Result<R>,
    {
        let registry_guard = Self::get_tool_registry(config)?;
        let registry_lock = registry_guard.lock()
            .map_err(|e| CliError::Unknown(format!("Failed to acquire registry lock for execution: {}", e)))?;

        let registry = registry_lock.as_ref()
            .ok_or_else(|| anyhow!("Tool registry not initialized"))?;

        f(registry)
    }

    /// List all available tools
    async fn list_tools(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let category_filter = matches.get_one::<String>("category");
        let search_term = matches.get_one::<String>("search");
        let json_output = matches.get_flag("json");
        let detailed = matches.get_flag("detailed");
        let available_only = matches.get_flag("available");

        Self::with_tool_registry(config, |registry| {
            // Get all tools
            let all_tools = registry.get_all_available_tools();

            // Apply filters
            let mut filtered_tools = all_tools;

            if let Some(category) = category_filter {
                filtered_tools.retain(|tool| {
                    Self::get_tool_category(&tool.name).eq_ignore_ascii_case(category)
                });
            }

            if let Some(search) = search_term {
                let search_lower = search.to_lowercase();
                filtered_tools.retain(|tool| {
                    tool.name.to_lowercase().contains(&search_lower) ||
                    tool.description.to_lowercase().contains(&search_lower)
                });
            }

            if available_only {
                // Filter only enabled/available tools
                filtered_tools.retain(|tool| registry.is_tool_available(&tool.name));
            }

            if json_output {
                let json_result = json!({
                    "tools": filtered_tools,
                    "total_count": filtered_tools.len(),
                    "filters": {
                        "category": category_filter,
                        "search": search_term,
                        "available_only": available_only
                    }
                });
                println!("{}", serde_json::to_string_pretty(&json_result)?);
            } else {
                Self::print_tools_table(&filtered_tools, detailed);
            }

            Ok(CommandResult::success_with_message(format!(
                "Listed {} tools", filtered_tools.len()
            )))
        })
    }

    /// Describe a specific tool
    async fn describe_tool(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let tool_name = matches.get_one::<String>("tool")
            .ok_or_else(|| CliError::Validation("Tool name is required".to_string()))?;
        let show_schema = matches.get_flag("schema");
        let show_examples = matches.get_flag("examples");
        let json_output = matches.get_flag("json");

        Self::with_tool_registry(config, |registry| {
            // Check if tool exists
            if !registry.is_tool_available(tool_name) {
                return Err(CliError::Validation(format!("Tool '{}' not found", tool_name)).into());
            }

            // Get tool information from available tools
            let all_tools = registry.get_all_available_tools();
            let tool_info = all_tools.iter()
                .find(|tool| tool.name == *tool_name)
                .ok_or_else(|| anyhow!("Failed to get tool information"))?;

            if json_output {
                let mut result = json!({
                    "name": tool_info.name,
                    "description": tool_info.description,
                    "executor": tool_info.executor,
                    "category": Self::get_tool_category(&tool_info.name),
                    "available": registry.is_tool_available(&tool_info.name)
                });

                if show_schema {
                    result["schema"] = Self::get_tool_schema(&tool_info.name);
                }

                if show_examples {
                    result["examples"] = Self::get_tool_examples(&tool_info.name);
                }

                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                Self::print_tool_description(tool_info, show_schema, show_examples);
            }

            Ok(CommandResult::success_with_message(format!(
                "Described tool '{tool_name}'"
            )))
        })
    }

    /// Execute a tool directly
    async fn execute_tool(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let tool_name = matches.get_one::<String>("tool")
            .ok_or_else(|| CliError::Validation("Tool name is required".to_string()))?;
        let json_params = matches.get_one::<String>("json");
        let params_file = matches.get_one::<String>("params-file");
        let dry_run = matches.get_flag("dry-run");
        let _timeout = matches.get_one::<String>("timeout");
        let json_output = matches.get_flag("json-output");

        // Get registry access for tool availability check
        let registry_guard = Self::get_tool_registry(config)?;

        // Check if tool exists (sync operation)
        {
            let registry_lock = registry_guard.lock()
                .map_err(|e| CliError::Unknown(format!("Failed to acquire registry lock: {}", e)))?;
            let registry = registry_lock.as_ref()
                .ok_or_else(|| anyhow!("Tool registry not initialized"))?;

            if !registry.is_tool_available(tool_name) {
                return Err(CliError::Validation(format!("Tool '{}' not found", tool_name)).into());
            }
        }

        // Parse parameters
        let parameters = if let Some(json_str) = json_params {
            serde_json::from_str::<HashMap<String, Value>>(json_str)
                .map_err(|e| CliError::Validation(format!("Invalid JSON parameters: {}", e)))?
        } else if let Some(file_path) = params_file {
            let file_content = tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| CliError::Validation(format!("Failed to read params file: {}", e)))?;
            serde_json::from_str::<HashMap<String, Value>>(&file_content)
                .map_err(|e| CliError::Validation(format!("Invalid JSON in params file: {}", e)))?
        } else {
            // Parse individual parameters from command line
            Self::parse_cli_parameters(matches)?
        };

        if dry_run {
            println!("🔍 Dry run mode - would execute:");
            println!("Tool: {tool_name}");
            println!("Parameters: {}", serde_json::to_string_pretty(&parameters)?);
            return Ok(CommandResult::success_with_message("Dry run completed".to_string()));
        }

        // Execute tool (async operation)
        let start_time = Instant::now();
        println!("🔧 Executing tool: {tool_name}");

        let result = {
            let registry_lock = registry_guard.lock()
                .map_err(|e| CliError::Unknown(format!("Failed to acquire registry lock for execution: {}", e)))?;
            let registry = registry_lock.as_ref()
                .ok_or_else(|| anyhow!("Tool registry not initialized"))?;

            registry.execute_tool(tool_name, &parameters).await
        };
        let execution_time = start_time.elapsed();

        match result {
            Ok(output) => {
                if json_output {
                    let json_result = json!({
                        "success": true,
                        "tool": tool_name,
                        "parameters": parameters,
                        "result": output,
                        "execution_time_ms": execution_time.as_millis(),
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_result)?);
                } else {
                    println!("✅ Tool executed successfully");
                    println!("⏱️  Execution time: {}ms", execution_time.as_millis());
                    println!("📋 Result:\n{output}");
                }

                Ok(CommandResult::success_with_message(format!(
                    "Tool '{tool_name}' executed successfully"
                )))
            }
            Err(e) => {
                if json_output {
                    let json_result = json!({
                        "success": false,
                        "tool": tool_name,
                        "parameters": parameters,
                        "error": e.to_string(),
                        "execution_time_ms": execution_time.as_millis(),
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    println!("{}", serde_json::to_string_pretty(&json_result)?);
                } else {
                    println!("❌ Tool execution failed");
                    println!("⏱️  Execution time: {}ms", execution_time.as_millis());
                    println!("💥 Error: {e}");
                }

                Err(CliError::Engine(format!("Tool execution failed: {}", e)).into())
            }
        }
    }

    /// List tool categories
    async fn list_categories(_matches: &ArgMatches, _config: &Config) -> Result<CommandResult> {
        let categories = vec![
            ("file", "File system operations (read, write, list, etc.)"),
            ("shell", "Shell command execution and scripting"),
            ("compiler", "Rust compilation and project management"),
            ("editor", "Text editing and string manipulation"),
            ("mcp", "Model Context Protocol tools"),
        ];

        println!("📂 Available tool categories:\n");
        for (name, description) in &categories {
            println!("  {name} - {description}");
        }

        Ok(CommandResult::success_with_message(format!(
            "Listed {} categories", categories.len()
        )))
    }

    /// Get tool category based on tool name
    fn get_tool_category(tool_name: &str) -> &'static str {
        match tool_name {
            name if name.starts_with("read_") || name.starts_with("write_") || 
                    name.starts_with("list_") || name.starts_with("file_") ||
                    name.starts_with("create_directory") => "file",
            name if name.starts_with("run_") || name.starts_with("check_command") ||
                    name.starts_with("get_working") => "shell",
            name if name.starts_with("cargo_") || name.starts_with("rustc") ||
                    name.starts_with("validate_cargo") => "compiler",
            name if name.starts_with("replace_") || name.contains("editor") => "editor",
            _ => "other",
        }
    }

    /// Print tools in table format
    fn print_tools_table(tools: &[fluent_agent::tools::ToolInfo], detailed: bool) {
        if tools.is_empty() {
            println!("No tools found matching the criteria.");
            return;
        }

        println!("🔧 Available tools:\n");
        
        if detailed {
            for tool in tools {
                println!("📦 {}", tool.name);
                println!("   Category: {}", Self::get_tool_category(&tool.name));
                println!("   Executor: {}", tool.executor);
                println!("   Description: {}", tool.description);
                println!();
            }
        } else {
            println!("{:<20} {:<12} DESCRIPTION", "TOOL", "CATEGORY");
            println!("{}", "-".repeat(80));
            
            for tool in tools {
                let category = Self::get_tool_category(&tool.name);
                println!("{:<20} {:<12} {}", 
                    tool.name, 
                    category, 
                    if tool.description.len() > 45 {
                        format!("{}...", &tool.description[..42])
                    } else {
                        tool.description.clone()
                    }
                );
            }
        }
    }

    /// Print detailed tool description
    fn print_tool_description(
        tool: &fluent_agent::tools::ToolInfo, 
        show_schema: bool, 
        show_examples: bool
    ) {
        println!("🔧 Tool: {}", tool.name);
        println!("📂 Category: {}", Self::get_tool_category(&tool.name));
        println!("⚙️  Executor: {}", tool.executor);
        println!("📝 Description: {}", tool.description);

        if show_schema {
            println!("\n📋 Parameter Schema:");
            let schema = Self::get_tool_schema(&tool.name);
            println!("{}", serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "No schema available".to_string()));
        }

        if show_examples {
            println!("\n💡 Examples:");
            let examples = Self::get_tool_examples(&tool.name);
            println!("{}", serde_json::to_string_pretty(&examples).unwrap_or_else(|_| "No examples available".to_string()));
        }
    }

    /// Get tool parameter schema (placeholder implementation)
    fn get_tool_schema(tool_name: &str) -> Value {
        match tool_name {
            "read_file" => json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to read"},
                    "encoding": {"type": "string", "description": "File encoding", "default": "utf-8"}
                },
                "required": ["path"]
            }),
            "write_file" => json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file to write"},
                    "content": {"type": "string", "description": "Content to write to the file"},
                    "encoding": {"type": "string", "description": "File encoding", "default": "utf-8"}
                },
                "required": ["path", "content"]
            }),
            _ => json!({"type": "object", "properties": {}, "description": "Schema not available"})
        }
    }

    /// Get tool usage examples (placeholder implementation)
    fn get_tool_examples(tool_name: &str) -> Value {
        match tool_name {
            "read_file" => json!([
                "fluent tools exec read_file --path README.md",
                "fluent tools exec read_file --json '{\"path\": \"src/main.rs\"}'"
            ]),
            "write_file" => json!([
                "fluent tools exec write_file --path output.txt --content 'Hello World'",
                "fluent tools exec write_file --json '{\"path\": \"test.txt\", \"content\": \"Test content\"}'"
            ]),
            _ => json!(["No examples available"])
        }
    }

    /// Parse CLI parameters into HashMap
    fn parse_cli_parameters(matches: &ArgMatches) -> Result<HashMap<String, Value>> {
        let mut parameters = HashMap::new();

        // Extract common parameters
        if let Some(path) = matches.get_one::<String>("path") {
            parameters.insert("path".to_string(), Value::String(path.clone()));
        }
        if let Some(content) = matches.get_one::<String>("content") {
            parameters.insert("content".to_string(), Value::String(content.clone()));
        }
        if let Some(command) = matches.get_one::<String>("command") {
            parameters.insert("command".to_string(), Value::String(command.clone()));
        }

        Ok(parameters)
    }
}

impl CommandHandler for ToolsCommand {
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()> {
        let result = match matches.subcommand() {
            Some(("list", sub_matches)) => {
                Self::list_tools(sub_matches, config).await?
            }
            Some(("describe", sub_matches)) => {
                Self::describe_tool(sub_matches, config).await?
            }
            Some(("exec", sub_matches)) => {
                Self::execute_tool(sub_matches, config).await?
            }
            Some(("categories", sub_matches)) => {
                Self::list_categories(sub_matches, config).await?
            }
            _ => {
                // Default: show help
                println!("🔧 Direct Tool Access");
                println!("Available commands:");
                println!("  list        - List available tools");
                println!("  describe    - Describe a specific tool");
                println!("  exec        - Execute a tool directly");
                println!("  categories  - List tool categories");
                println!("\nUse 'fluent tools <command> --help' for more information");

                CommandResult::success_with_message("Tools help displayed".to_string())
            }
        };

        if !result.success {
            if let Some(message) = result.message {
                return Err(anyhow!("Tools command failed: {}", message));
            }
            return Err(anyhow!("Tools command failed"));
        }

        Ok(())
    }
}

impl Default for ToolsCommand {
    fn default() -> Self {
        Self::new()
    }
}
