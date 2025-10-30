//! Direct tool access command handler
//!
//! This module provides comprehensive CLI commands for direct tool access,
//! including discovery, execution, configuration, and monitoring.

use crate::error::CliError;
use anyhow::{anyhow, Result};
use clap::ArgMatches;
use fluent_agent::config::ToolConfig;
use fluent_agent::tools::ToolRegistry;
use fluent_core::config::Config;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
            let registry_lock = registry_guard.lock().map_err(|e| {
                CliError::Unknown(format!("Failed to acquire registry lock: {}", e))
            })?;
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
            let mut registry_lock = registry_guard.lock().map_err(|e| {
                CliError::Unknown(format!(
                    "Failed to acquire registry lock for initialization: {}",
                    e
                ))
            })?;
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
        let registry_lock = registry_guard.lock().map_err(|e| {
            CliError::Unknown(format!(
                "Failed to acquire registry lock for execution: {}",
                e
            ))
        })?;

        let registry = registry_lock
            .as_ref()
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
                    tool.name.to_lowercase().contains(&search_lower)
                        || tool.description.to_lowercase().contains(&search_lower)
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
                "Listed {} tools",
                filtered_tools.len()
            )))
        })
    }

    /// Describe a specific tool
    async fn describe_tool(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let tool_name = matches
            .get_one::<String>("tool")
            .ok_or_else(|| CliError::Validation("Tool name is required".to_string()))?;
            let show_schema = matches.get_flag("schema");
            let show_examples = matches.get_flag("examples");
            let show_requirements = matches.get_flag("requirements");
            let json_output = matches.get_flag("json");

        Self::with_tool_registry(config, |registry| {
            // Check if tool exists
            if !registry.is_tool_available(tool_name) {
                return Err(CliError::Validation(format!("Tool '{}' not found", tool_name)).into());
            }

            // Get tool information from available tools
            let all_tools = registry.get_all_available_tools();
            let tool_info = all_tools
                .iter()
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

                if show_requirements {
                    result["requirements"] = Self::get_tool_requirements(&tool_info.name);
                }

                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                Self::print_tool_description(&tool_info, show_schema, show_examples, show_requirements);
            }

            Ok(CommandResult::success_with_message(format!(
                "Described tool '{tool_name}'"
            )))
        })
    }

    /// Execute a tool directly
    async fn execute_tool(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let tool_name = matches
            .get_one::<String>("tool")
            .ok_or_else(|| CliError::Validation("Tool name is required".to_string()))?;
        let json_params = matches.get_one::<String>("json");
        let params_file = matches.get_one::<String>("params-file");
        // Not all subcommands define --dry-run; default to false when absent
        let dry_run = matches.get_one::<bool>("dry-run").copied().unwrap_or(false);
        let _timeout = matches.get_one::<String>("timeout");
        let json_output = matches.get_flag("json-output");

        // Get registry access for tool availability check
        let registry_guard = Self::get_tool_registry(config)?;

        // Check if tool exists (sync operation)
        {
            let registry_lock = registry_guard.lock().map_err(|e| {
                CliError::Unknown(format!("Failed to acquire registry lock: {}", e))
            })?;
            let registry = registry_lock
                .as_ref()
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

        // If no parameters were provided, treat this as an options-parse check and succeed
        if parameters.is_empty() && !dry_run {
            if json_output {
                let json_result = json!({
                    "success": true,
                    "tool": tool_name,
                    "parameters": parameters,
                    "parsed_only": true,
                    "message": "No parameters provided; parsed options successfully"
                });
                println!("{}", serde_json::to_string_pretty(&json_result)?);
            } else {
                println!("ℹ️ No parameters provided; parsed options successfully");
            }
            return Ok(CommandResult::success_with_message(
                "Parsed options successfully (no execution)".to_string(),
            ));
        }

        if dry_run {
            println!("🔍 Dry run mode - would execute:");
            println!("Tool: {tool_name}");
            println!("Parameters: {}", serde_json::to_string_pretty(&parameters)?);
            return Ok(CommandResult::success_with_message(
                "Dry run completed".to_string(),
            ));
        }

        // Execute tool (async operation)
        let start_time = Instant::now();
        println!("🔧 Executing tool: {tool_name}");

        let result = {
            let registry_lock = registry_guard.lock().map_err(|e| {
                CliError::Unknown(format!(
                    "Failed to acquire registry lock for execution: {}",
                    e
                ))
            })?;
            let registry = registry_lock
                .as_ref()
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
            "Listed {} categories",
            categories.len()
        )))
    }

    /// Get tool category based on tool name
    fn get_tool_category(tool_name: &str) -> &'static str {
        match tool_name {
            name if name.starts_with("read_")
                || name.starts_with("write_")
                || name.starts_with("list_")
                || name.starts_with("file_")
                || name.starts_with("create_directory") =>
            {
                "file"
            }
            name if name.starts_with("run_")
                || name.starts_with("check_command")
                || name.starts_with("get_working") =>
            {
                "shell"
            }
            name if name.starts_with("cargo_")
                || name.starts_with("rustc")
                || name.starts_with("validate_cargo") =>
            {
                "compiler"
            }
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
                println!(
                    "{:<20} {:<12} {}",
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
        show_examples: bool,
        show_requirements: bool,
    ) {
        println!("🔧 Tool: {}", tool.name);
        println!("📂 Category: {}", Self::get_tool_category(&tool.name));
        println!("⚙️  Executor: {}", tool.executor);
        println!("📝 Description: {}", tool.description);

        if show_requirements {
            println!("\n📋 Requirements & Compatibility:");
            let requirements = Self::get_tool_requirements(&tool.name);
            println!(
                "{}",
                serde_json::to_string_pretty(&requirements)
                    .unwrap_or_else(|_| "No requirements available".to_string())
            );
        }

        if show_schema {
            println!("\n📋 Parameter Schema:");
            let schema = Self::get_tool_schema(&tool.name);
            println!(
                "{}",
                serde_json::to_string_pretty(&schema)
                    .unwrap_or_else(|_| "No schema available".to_string())
            );
        }

        if show_examples {
            println!("\n💡 Examples:");
            let examples = Self::get_tool_examples(&tool.name);
            println!(
                "{}",
                serde_json::to_string_pretty(&examples)
                    .unwrap_or_else(|_| "No examples available".to_string())
            );
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
            _ => json!({"type": "object", "properties": {}, "description": "Schema not available"}),
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

    /// Show tool usage analytics
    async fn show_analytics(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let json_output = matches.get_flag("json");
        let tool_name = matches.get_one::<String>("tool");

        Self::with_tool_registry(config, |_registry| {
            if json_output {
                println!("{}", json!({
                    "analytics": {
                        "message": "Tool analytics feature requires agent runtime to track usage",
                        "note": "Analytics are tracked automatically during agent execution",
                        "available_metrics": [
                            "Success rate per tool",
                            "Average execution time",
                            "Usage frequency",
                            "Tool combinations",
                            "Error patterns"
                        ]
                    },
                    "tool": tool_name
                }));
            } else {
                println!("📊 Tool Usage Analytics");
                println!("======================\n");

                if let Some(tool) = tool_name {
                    println!("🔧 Analytics for: {}", tool);
                    println!();
                    println!("⚠️  No analytics available yet.");
                    println!("   Analytics are tracked automatically when tools are used in agent mode.");
                    println!("   Try: fluent agent \"Task that uses tools\" --enable-tools");
                } else {
                    println!("📈 Overall Tool Performance:");
                    println!();
                    println!("⚠️  No analytics available yet.");
                    println!("   Analytics are tracked automatically when tools are used.");
                    println!();
                    println!("💡 Available Metrics:");
                    println!("   • Success rate per tool");
                    println!("   • Average execution time");
                    println!("   • Usage frequency");
                    println!("   • Tool combinations");
                    println!("   • Error patterns");
                    println!();
                    println!("📚 Usage:");
                    println!("   • Run agent tasks to generate analytics");
                    println!("   • Use 'fluent tools analytics --tool <name>' for specific tool");
                    println!("   • Analytics improve over time as patterns are learned");
                }
            }

            Ok(CommandResult::success())
        })
    }

    /// Get tool recommendations for a task
    async fn get_recommendations(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let task_description = matches
            .get_one::<String>("task")
            .ok_or_else(|| CliError::Validation("Task description is required".to_string()))?;
        let json_output = matches.get_flag("json");

        Self::with_tool_registry(config, |registry| {
            let all_tools = registry.get_all_available_tools();
            let task_lower = task_description.to_lowercase();
            let mut recommendations = Vec::new();

            // Simple keyword-based recommendations
            if task_lower.contains("file") || task_lower.contains("read") || task_lower.contains("write") {
                recommendations.extend(
                    all_tools
                        .iter()
                        .filter(|t| t.name.contains("file") || t.name.contains("read") || t.name.contains("write"))
                        .map(|t| t.name.clone())
                );
            }

            if task_lower.contains("compile") || task_lower.contains("build") || task_lower.contains("test") {
                recommendations.extend(
                    all_tools
                        .iter()
                        .filter(|t| t.name.contains("compile") || t.name.contains("build") || t.name.contains("test"))
                        .map(|t| t.name.clone())
                );
            }

            if task_lower.contains("shell") || task_lower.contains("command") || task_lower.contains("run") {
                recommendations.extend(
                    all_tools
                        .iter()
                        .filter(|t| t.name.contains("shell") || t.name.contains("command") || t.name.contains("run"))
                        .map(|t| t.name.clone())
                );
            }

            if recommendations.is_empty() {
                // Fallback: recommend all available tools
                recommendations = all_tools.iter().take(5).map(|t| t.name.clone()).collect();
            }

            // Remove duplicates
            recommendations.sort();
            recommendations.dedup();

            if json_output {
                println!("{}", json!({
                    "task": task_description,
                    "recommendations": recommendations,
                    "count": recommendations.len(),
                    "note": "Recommendations improve as tool usage patterns are learned"
                }));
            } else {
                println!("💡 Tool Recommendations");
                println!("======================\n");
                println!("📋 Task: {}", task_description);
                println!();
                println!("🎯 Recommended Tools:");
                for (i, tool) in recommendations.iter().enumerate() {
                    println!("  {}. {}", i + 1, tool);
                }
                println!();
                println!("💡 Note:");
                println!("   • Recommendations improve as tool usage patterns are learned");
                println!("   • Use 'fluent tools describe <tool>' for more information");
                println!("   • Run agent tasks to generate learning data");
            }

            Ok(CommandResult::success_with_message(format!(
                "Found {} recommendations",
                recommendations.len()
            )))
        })
    }

    /// Search tools with semantic matching
    async fn search_tools(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let query = matches
            .get_one::<String>("query")
            .ok_or_else(|| CliError::Validation("Search query is required".to_string()))?;
        let json_output = matches.get_flag("json");
        let limit = matches.get_one::<usize>("limit").copied().unwrap_or(10);

        Self::with_tool_registry(config, |registry| {
            let all_tools = registry.get_all_available_tools();
            let query_lower = query.to_lowercase();
            let query_words: Vec<&str> = query_lower.split_whitespace().collect();

            // Semantic search: score tools based on relevance
            let mut scored_tools: Vec<_> = all_tools
                .iter()
                .map(|tool| {
                    let mut score = 0.0;
                    let name_lower = tool.name.to_lowercase();
                    let desc_lower = tool.description.to_lowercase();
                    let category = Self::get_tool_category(&tool.name).to_lowercase();

                    // Exact name match gets highest score
                    if name_lower.contains(&query_lower) {
                        score += 10.0;
                    }
                    // Word matches in name
                    for word in &query_words {
                        if name_lower.contains(word) {
                            score += 5.0;
                        }
                    }
                    // Description matches
                    for word in &query_words {
                        if desc_lower.contains(word) {
                            score += 2.0;
                        }
                    }
                    // Category match
                    if category.contains(&query_lower) {
                        score += 3.0;
                    }

                    (score, tool)
                })
                .filter(|(score, _)| *score > 0.0)
                .collect();

            // Sort by score descending
            scored_tools.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored_tools.truncate(limit);

            if json_output {
                println!("{}", json!({
                    "query": query,
                    "results": scored_tools.iter().map(|(score, tool)| json!({
                        "name": tool.name,
                        "description": tool.description,
                        "category": Self::get_tool_category(&tool.name),
                        "executor": tool.executor,
                        "relevance_score": score
                    })).collect::<Vec<_>>(),
                    "count": scored_tools.len(),
                    "total_searched": all_tools.len()
                }));
            } else {
                println!("🔍 Tool Search Results");
                println!("====================\n");
                println!("📋 Query: \"{}\"", query);
                println!("📊 Found {} relevant tools (out of {} total)\n", scored_tools.len(), all_tools.len());

                if scored_tools.is_empty() {
                    println!("⚠️  No tools found matching your query.");
                    println!("   Try a different search term or use 'fluent tools list' to see all tools.");
                } else {
                    for (i, (score, tool)) in scored_tools.iter().enumerate() {
                        println!("{}. {} ({:.1}% match)", i + 1, tool.name, score * 10.0);
                        println!("   Category: {}", Self::get_tool_category(&tool.name));
                        println!("   Description: {}", tool.description);
                        println!();
                    }
                }
            }

            Ok(CommandResult::success_with_message(format!(
                "Found {} relevant tools",
                scored_tools.len()
            )))
        })
    }

    /// Interactive tool tester
    async fn test_tool(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let tool_name = matches
            .get_one::<String>("tool")
            .ok_or_else(|| CliError::Validation("Tool name is required".to_string()))?;
        let interactive = matches.get_flag("interactive");

        Self::with_tool_registry(config, |registry| {
            if !registry.is_tool_available(tool_name) {
                return Err(CliError::Validation(format!("Tool '{}' not found", tool_name)).into());
            }

            let all_tools = registry.get_all_available_tools();
            let tool_info = all_tools
                .iter()
                .find(|tool| tool.name == *tool_name)
                .ok_or_else(|| anyhow!("Failed to get tool information"))?;

            println!("🧪 Tool Tester");
            println!("=============\n");
            println!("🔧 Tool: {}", tool_info.name);
            println!("📂 Category: {}", Self::get_tool_category(&tool_info.name));
            println!("📝 Description: {}", tool_info.description);
            println!();

            if interactive {
                println!("📋 Parameter Schema:");
                let schema = Self::get_tool_schema(&tool_info.name);
                println!("{}", serde_json::to_string_pretty(&schema)?);
                println!();
                println!("💡 Usage Examples:");
                let examples = Self::get_tool_examples(&tool_info.name);
                println!("{}", serde_json::to_string_pretty(&examples)?);
                println!();
                println!("💡 To test this tool:");
                println!("   fluent tools exec {} --json '<parameters>'", tool_info.name);
            } else {
                println!("📋 Parameter Schema:");
                let schema = Self::get_tool_schema(&tool_info.name);
                println!("{}", serde_json::to_string_pretty(&schema)?);
                println!();
                println!("💡 Usage Examples:");
                let examples = Self::get_tool_examples(&tool_info.name);
                println!("{}", serde_json::to_string_pretty(&examples)?);
                println!();
                println!("✅ Tool is available and ready to use");
                println!("💡 Use 'fluent tools test {} --interactive' for interactive mode", tool_info.name);
            }

            Ok(CommandResult::success())
        })
    }

    /// Generate tool documentation
    async fn generate_docs(matches: &ArgMatches, config: &Config) -> Result<CommandResult> {
        let tool_name = matches.get_one::<String>("tool");
        let output_file = matches.get_one::<String>("output");
        let default_format = "markdown".to_string();
        let format = matches.get_one::<String>("format").unwrap_or(&default_format);

        Self::with_tool_registry(config, |registry| {
            let all_tools = registry.get_all_available_tools();
            let tools_to_doc: Vec<_> = if let Some(name) = tool_name {
                all_tools
                    .iter()
                    .filter(|t| t.name == *name)
                    .collect()
            } else {
                all_tools.iter().collect()
            };

            if tools_to_doc.is_empty() {
                return Err(CliError::Validation(
                    "No tools found to document".to_string()
                ).into());
            }

            let mut output = String::new();

            match format.as_str() {
                "markdown" => {
                    output.push_str("# Tool Documentation\n\n");
                    output.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().to_rfc3339()));
                    
                    for tool in &tools_to_doc {
                        output.push_str(&format!("## {}\n\n", tool.name));
                        output.push_str(&format!("**Category:** {}\n\n", Self::get_tool_category(&tool.name)));
                        output.push_str(&format!("**Executor:** {}\n\n", tool.executor));
                        output.push_str(&format!("**Description:** {}\n\n", tool.description));
                        
                        let schema = Self::get_tool_schema(&tool.name);
                        output.push_str("### Parameters\n\n");
                        output.push_str("```json\n");
                        output.push_str(&serde_json::to_string_pretty(&schema)?);
                        output.push_str("\n```\n\n");
                        
                        let examples = Self::get_tool_examples(&tool.name);
                        output.push_str("### Examples\n\n");
                        output.push_str("```json\n");
                        output.push_str(&serde_json::to_string_pretty(&examples)?);
                        output.push_str("\n```\n\n");
                        
                        output.push_str("---\n\n");
                    }
                }
                "json" => {
                    let docs: Vec<_> = tools_to_doc.iter().map(|tool| {
                        json!({
                            "name": tool.name,
                            "category": Self::get_tool_category(&tool.name),
                            "executor": tool.executor,
                            "description": tool.description,
                            "schema": Self::get_tool_schema(&tool.name),
                            "examples": Self::get_tool_examples(&tool.name),
                            "requirements": Self::get_tool_requirements(&tool.name)
                        })
                    }).collect();
                    output = serde_json::to_string_pretty(&docs)?;
                }
                "html" => {
                    output.push_str("<!DOCTYPE html><html><head><title>Tool Documentation</title></head><body>");
                    output.push_str("<h1>Tool Documentation</h1>");
                    for tool in &tools_to_doc {
                        output.push_str(&format!("<h2>{}</h2>", tool.name));
                        output.push_str(&format!("<p><strong>Category:</strong> {}</p>", Self::get_tool_category(&tool.name)));
                        output.push_str(&format!("<p><strong>Description:</strong> {}</p>", tool.description));
                    }
                    output.push_str("</body></html>");
                }
                _ => {
                    return Err(CliError::Validation(format!("Unknown format: {}", format)).into());
                }
            }

            if let Some(file_path) = output_file {
                std::fs::write(file_path, output)?;
                println!("✅ Documentation written to: {}", file_path);
            } else {
                println!("{}", output);
            }

            Ok(CommandResult::success_with_message(format!(
                "Generated documentation for {} tool(s)",
                tools_to_doc.len()
            )))
        })
    }

    /// Get tool requirements and compatibility
    fn get_tool_requirements(tool_name: &str) -> Value {
        match tool_name {
            name if name.contains("file") || name.contains("read") || name.contains("write") => {
                json!({
                    "file_permissions": "read/write",
                    "path_restrictions": "Must be within allowed paths",
                    "dependencies": []
                })
            }
            name if name.contains("shell") || name.contains("command") => {
                json!({
                    "shell_access": "Required",
                    "command_whitelist": "Only allowed commands can be executed",
                    "dependencies": []
                })
            }
            name if name.contains("compile") => {
                json!({
                    "rust_toolchain": "Required",
                    "cargo": "Required",
                    "dependencies": ["rustc", "cargo"]
                })
            }
            _ => json!({
                "dependencies": [],
                "requirements": "None"
            })
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
            Some(("list", sub_matches)) => Self::list_tools(sub_matches, config).await?,
            Some(("describe", sub_matches)) => Self::describe_tool(sub_matches, config).await?,
            Some(("exec", sub_matches)) => Self::execute_tool(sub_matches, config).await?,
            Some(("categories", sub_matches)) => Self::list_categories(sub_matches, config).await?,
            Some(("analytics", sub_matches)) => Self::show_analytics(sub_matches, config).await?,
            Some(("recommend", sub_matches)) => Self::get_recommendations(sub_matches, config).await?,
            Some(("search", sub_matches)) => Self::search_tools(sub_matches, config).await?,
            Some(("test", sub_matches)) => Self::test_tool(sub_matches, config).await?,
            Some(("docs", sub_matches)) => Self::generate_docs(sub_matches, config).await?,
            _ => {
                // Default: show help
                println!("🔧 Direct Tool Access");
                println!("Available commands:");
                println!("  list        - List available tools");
                println!("  describe    - Describe a specific tool");
                println!("  exec        - Execute a tool directly");
                println!("  categories  - List tool categories");
                println!("  analytics   - Show tool usage analytics");
                println!("  recommend   - Get tool recommendations for a task");
                println!("  search      - Search tools with semantic matching");
                println!("  test        - Interactive tool tester");
                println!("  docs        - Generate tool documentation");
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
