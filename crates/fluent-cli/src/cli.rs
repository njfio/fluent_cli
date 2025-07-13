//! Core CLI functionality and argument parsing
//!
//! This module provides the main command-line interface functionality,
//! including argument parsing, command routing, and execution logic.

use anyhow::{anyhow, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use fluent_core::config::{load_config, Config, EngineConfig};
use fluent_core::error::{FluentError, FluentResult, ValidationError};
use fluent_core::input_validator::InputValidator;
use fluent_core::memory_utils::StringUtils;
use fluent_core::traits::Engine;
use fluent_core::types::{Request, Response};
use fluent_engines::anthropic::AnthropicEngine;
use fluent_engines::openai::OpenAIEngine;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use log::debug;
use serde_json::Value;

/// Convert anyhow errors to FluentError with context
#[allow(dead_code)]
fn to_fluent_error(err: anyhow::Error, context: &str) -> FluentError {
    FluentError::Internal(format!("{}: {}", context, err))
}

/// Validate required CLI arguments
#[allow(dead_code)]
fn validate_required_string(
    matches: &ArgMatches,
    arg_name: &str,
    context: &str,
) -> FluentResult<String> {
    matches.get_one::<String>(arg_name).cloned().ok_or_else(|| {
        FluentError::Validation(ValidationError::MissingField(format!(
            "{} is required for {}",
            arg_name, context
        )))
    })
}

/// Enhanced validation for file paths with security checks
#[allow(dead_code)]
fn validate_file_path_secure(path: &str, context: &str) -> FluentResult<String> {
    if path.is_empty() {
        return Err(FluentError::Validation(ValidationError::MissingField(
            format!("File path is required for {}", context),
        )));
    }

    // Use the comprehensive InputValidator
    match InputValidator::validate_file_path(path) {
        Ok(validated_path) => Ok(validated_path.to_string_lossy().to_string()),
        Err(e) => Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: path.to_string(),
            expected: format!("secure file path for {}: {}", context, e),
        })),
    }
}

/// Validate request payload with comprehensive checks
#[allow(dead_code)]
fn validate_request_payload(payload: &str, context: &str) -> FluentResult<String> {
    match InputValidator::validate_request_payload(payload) {
        Ok(validated_payload) => Ok(validated_payload),
        Err(e) => Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: payload.to_string(),
            expected: format!("valid request payload for {}: {}", context, e),
        })),
    }
}

/// Validate engine name against supported engines
#[allow(dead_code)]
fn validate_engine_name(engine_name: &str) -> FluentResult<String> {
    let supported_engines = ["openai", "anthropic", "google", "cohere", "mistral"];
    
    if !supported_engines.contains(&engine_name) {
        let expected = supported_engines.join(", ");
        return Err(FluentError::Validation(ValidationError::InvalidFormat {
            input: engine_name.to_string(),
            expected: format!("supported engine ({})", expected),
        }));
    }

    Ok(engine_name.to_string())
}

/// Memory monitoring and cleanup utilities
#[allow(dead_code)]
struct MemoryManager;

impl MemoryManager {
    /// Force garbage collection and memory cleanup
    #[allow(dead_code)]
    fn force_cleanup() {
        // In Rust, we can't force GC, but we can drop large allocations
        // and encourage the allocator to return memory to the OS
        std::hint::black_box(Vec::<u8>::with_capacity(1024 * 1024)); // Dummy allocation to trigger cleanup
    }

    /// Log current memory usage (basic implementation)
    #[allow(dead_code)]
    fn log_memory_usage(context: &str) {
        // This is a basic implementation - in production you might use a proper memory profiler
        debug!("Memory checkpoint: {}", context);
    }

    /// Cleanup temporary files and resources
    #[allow(dead_code)]
    fn cleanup_temp_resources() -> Result<()> {
        // Clean up any temporary files that might have been created
        if let Ok(temp_dir) = std::env::temp_dir().read_dir() {
            for entry in temp_dir.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with("fluent_cli_temp_") {
                        if let Err(e) = std::fs::remove_file(&path) {
                            debug!("Failed to remove temp file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Process response output with all requested transformations
#[allow(dead_code)]
async fn process_response_output(
    response_content: &str,
    mut output: String,
    matches: &ArgMatches,
) -> Result<String> {
    use crate::create_engine;
    use fluent_core::output_processor::OutputProcessor;

    // Download media files if requested
    if let Some(download_dir) = matches.get_one::<String>("download-media") {
        let download_path = PathBuf::from(download_dir);
        OutputProcessor::download_media_files(response_content, &download_path).await?;
    }

    // Parse code blocks if requested
    if matches.get_flag("parse-code") {
        debug!("Parsing code blocks");
        let code_blocks = OutputProcessor::parse_code(&output);
        debug!("Code blocks: {:?}", code_blocks);
        output = code_blocks.join("\n\n");
    }

    // Execute output code if requested
    if matches.get_flag("execute-output") {
        debug!("Executing output code");
        debug!("Attempting to execute: {}", output);
        output = OutputProcessor::execute_code(&output).await?;
    }

    // Format as markdown if requested (currently commented out)
    if matches.get_flag("markdown") {
        debug!("Formatting output as markdown");
        // output = format_markdown(&output);
    }

    Ok(output)
}

/// Read configuration file and extract engine names and parameters
pub fn read_config_file(path: &str) -> Result<(Vec<String>, HashSet<String>)> {
    let config_str = fs::read_to_string(path)?;
    let config: Value = serde_json::from_str(&config_str)?;

    let mut engine_names = Vec::new();
    let mut parameters = HashSet::new();

    if let Some(engines) = config.get("engines").and_then(|e| e.as_array()) {
        for engine in engines {
            if let Some(name) = engine.get("name").and_then(|n| n.as_str()) {
                engine_names.push(name.to_string());
            }
            if let Some(params) = engine.get("parameters").and_then(|p| p.as_object()) {
                for key in params.keys() {
                    parameters.insert(key.clone());
                }
            }
        }
    }

    Ok((engine_names, parameters))
}

/// Process request with file upload
pub async fn process_request_with_file(
    engine: &dyn Engine,
    request_content: &str,
    file_path: &str,
) -> Result<Response> {
    let file_id = Pin::from(engine.upload_file(Path::new(file_path))).await?;
    log::info!("File uploaded successfully. File ID: {}", file_id);

    let request = Request {
        flowname: "default".to_string(),
        payload: format!("File ID: {}. {}", file_id, request_content),
    };

    Pin::from(engine.execute(&request)).await
}

/// Process a standard request
pub async fn process_request(engine: &dyn Engine, request_content: &str) -> Result<Response> {
    let request = Request {
        flowname: "default".to_string(),
        payload: request_content.to_string(),
    };

    Pin::from(engine.execute(&request)).await
}

/// Print response information (legacy function)
pub fn print_response(response: &Response, response_time: f64) {
    println!("Response: {}", response.content);
    println!("Model: {}", response.model);
    println!("Usage:");
    println!("  Prompt tokens: {}", response.usage.prompt_tokens);
    println!("  Completion tokens: {}", response.usage.completion_tokens);
    println!("  Total tokens: {}", response.usage.total_tokens);
    println!("Cost:");
    println!("  Prompt cost: ${:.6}", response.cost.prompt_cost);
    println!("  Completion cost: ${:.6}", response.cost.completion_cost);
    println!("  Total cost: ${:.6}", response.cost.total_cost);
    println!("  Response time: {:.2} seconds", response_time);
    if let Some(reason) = &response.finish_reason {
        println!("Finish reason: {}", reason);
    }
}

/// Build the main CLI command structure
pub fn build_cli() -> Command {
    Command::new("Fluent CLI")
        .version("0.1.0")
        .author("Your Name <your.email@example.com>")
        .about("A flexible CLI for interacting with various LLM engines")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("override")
                .short('o')
                .long("override")
                .value_name("KEY=VALUE")
                .help("Override configuration values")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Input file path")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("metadata")
                .short('t')
                .long("metadata")
                .value_name("METADATA")
                .help("Additional metadata")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("upload-image-file")
                .short('l')
                .long("upload-image-file")
                .value_name("FILE")
                .help("Upload an image file")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("download-media")
                .short('d')
                .long("download-media")
                .value_name("DIR")
                .help("Download media files to directory")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("parse-code")
                .short('p')
                .long("parse-code")
                .help("Parse code blocks from response")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("execute-output")
                .short('x')
                .long("execute-output")
                .help("Execute the output code")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("markdown")
                .short('m')
                .long("markdown")
                .help("Format output as markdown")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("generate-cypher")
                .long("generate-cypher")
                .value_name("QUERY")
                .help("Generate Cypher query from natural language")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("upsert")
                .long("upsert")
                .help("Upsert data to Neo4j")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("additional-context-file")
                .short('a')
                .long("additional-context-file")
                .value_name("FILE")
                .help("Additional context file")
                .action(ArgAction::Set),
        )
        // Add subcommands here as needed
}

/// New modular run function using command handlers
pub async fn run_modular() -> Result<()> {
    use crate::commands::*;

    let matches = build_cli().get_matches();

    // Extract engine name from command line
    let engine_name = matches.get_one::<String>("engine")
        .ok_or_else(|| anyhow!("Engine name is required"))?;

    // Load configuration
    let config_path = matches
        .get_one::<String>("config")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "config.yaml".to_string());

    let config = fluent_core::config::load_config(&config_path, engine_name, &std::collections::HashMap::new())?;

    // Route to appropriate command handler
    match matches.subcommand() {
        Some(("pipeline", sub_matches)) => {
            let handler = pipeline::PipelineCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("agent", sub_matches)) => {
            let handler = agent::AgentCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("mcp", sub_matches)) => {
            let handler = mcp::McpCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("tools", sub_matches)) => {
            let handler = crate::commands::tools::ToolsCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some((_engine_name, sub_matches)) => {
            // Handle engine commands
            let handler = engine::EngineCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        None => {
            // Check if there's a request to process
            if matches.get_one::<String>("request").is_some() {
                // Handle direct engine query
                let handler = engine::EngineCommand::new();
                handler.execute(&matches, &config).await?;
            } else {
                // Default behavior - show help
                build_cli().print_help()?;
            }
        }
    }

    Ok(())
}

/// Legacy run function - now delegates to run_modular for consistency
pub async fn run() -> Result<()> {
    run_modular().await
}
