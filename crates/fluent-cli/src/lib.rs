//! Fluent CLI Library
//!
//! This crate provides the main command-line interface for the Fluent CLI system,
//! including agentic capabilities, command handling, pipeline execution,
//! and various utility functions.
//!
//! # Key Modules
//!
//! - [`agentic`] - Autonomous agentic execution capabilities
//! - [`commands`] - Modular command handlers for different CLI operations
//! - [`pipeline_builder`] - Pipeline construction and execution
//! - [`memory`] - Memory management for conversations and context
//! - [`utils`] - Utility functions for text processing and validation
//! - [`cli`] - Core CLI functionality and argument parsing
//!
//! # Examples
//!
//! ```rust,no_run
//! use fluent_cli::cli::run;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Run the CLI with command line arguments
//! run().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Agentic Mode
//!
//! The CLI supports autonomous agentic execution:
//!
//! ```rust,no_run
//! use fluent_cli::agentic::{AgenticConfig, AgenticExecutor};
//! use fluent_core::config::Config;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = AgenticConfig::new(
//!     "Create a simple game".to_string(),
//!     "agent_config.json".to_string(),
//!     10,
//!     true,
//!     "config.yaml".to_string(),
//! );
//!
//! let executor = AgenticExecutor::new(config);
//! let fluent_config = Config::default();
//! executor.run(&fluent_config).await?;
//! # Ok(())
//! # }
//! ```

pub mod agentic;
pub mod commands;
pub mod neo4j_operations;
pub mod pipeline_builder;
pub mod validation;
pub mod memory;
pub mod utils;
pub mod frogger;

// New modular components
pub mod cli_builder;
pub mod engine_factory;
pub mod request_processor;
pub mod response_formatter;

use anyhow::Error;
use fluent_core::config::{EngineConfig, Neo4jConfig};
use fluent_core::traits::Engine;
use fluent_engines::create_engine;

// Re-export commonly used functions
pub use utils::{extract_cypher_query, is_valid_cypher, format_as_csv, extract_code};
pub use validation::{validate_engine_name, validate_file_path_secure, parse_key_value_pair};
pub use memory::MemoryManager;

// Re-export agentic functionality
pub use cli::run_agentic_mode;

pub mod cli {
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
                input: payload.chars().take(100).collect::<String>() + "...",
                expected: format!("valid request payload for {}: {}", context, e),
            })),
        }
    }

    /// Validate numeric parameters with bounds checking
    #[allow(dead_code)]
    fn validate_numeric_parameter(
        value: u32,
        min: u32,
        max: u32,
        param_name: &str,
    ) -> FluentResult<u32> {
        if value < min || value > max {
            return Err(FluentError::Validation(ValidationError::InvalidFormat {
                input: value.to_string(),
                expected: format!("{} must be between {} and {}", param_name, min, max),
            }));
        }
        Ok(value)
    }

    /// Validate engine name against supported engines
    #[allow(dead_code)]
    fn validate_engine_name(engine_name: &str) -> FluentResult<String> {
        if engine_name.is_empty() {
            return Err(FluentError::Validation(ValidationError::MissingField(
                "Engine name cannot be empty".to_string(),
            )));
        }

        let supported_engines = [
            "openai",
            "anthropic",
            "google_gemini",
            "cohere",
            "mistral",
            "stability_ai",
            "replicate",
            "leonardo_ai",
            "imagine_pro",
            "webhook",
        ];

        if !supported_engines.contains(&engine_name) {
            // Use memory-efficient string concatenation
            let expected = StringUtils::concat_with_separator(&supported_engines, ", ");
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

    use crate::create_llm_engine;
    use fluent_core::output_processor::OutputProcessor;
    
    
    
    
    
    
    
    
    
    
    

    
    
    
    
    
    

    #[allow(dead_code)]
    fn parse_key_value_pair(s: &str) -> Option<(String, String)> {
        if let Some((key, value)) = s.split_once('=') {
            Some((key.to_string(), value.to_string()))
        } else {
            None
        }
    }

    pub struct CliState {
        pub command: Command,
        pub parameters: Vec<String>,
    }

    pub fn read_config_file(path: &str) -> Result<(Vec<String>, HashSet<String>)> {
        let config_str = fs::read_to_string(path)?;
        let config: Value = serde_json::from_str(&config_str)?;

        let engines = config["engines"]
            .as_array()
            .ok_or_else(|| anyhow!("No engines found in configuration"))?
            .iter()
            .filter_map(|engine| engine["name"].as_str().map(String::from))
            .collect::<Vec<String>>();

        let mut parameters = HashSet::new();
        if let Some(engines_array) = config["engines"].as_array() {
            for engine in engines_array {
                if let Some(params) = engine["parameters"].as_object() {
                    for key in params.keys() {
                        parameters.insert(key.clone());
                    }
                }
            }
        }

        Ok((engines, parameters))
    }

    pub async fn process_request_with_file(
        engine: &dyn Engine,
        request_content: &str,
        file_path: &str,
    ) -> Result<Response> {
        let file_id = Pin::from(engine.upload_file(Path::new(file_path))).await?;
        println!("File uploaded successfully. File ID: {}", file_id);

        let request = Request {
            flowname: "default".to_string(),
            payload: format!("File ID: {}. {}", file_id, request_content),
        };

        Pin::from(engine.execute(&request)).await
    }

    pub async fn process_request(engine: &dyn Engine, request_content: &str) -> Result<Response> {
        let request = Request {
            flowname: "default".to_string(),
            payload: request_content.to_string(),
        };

        Pin::from(engine.execute(&request)).await
    }

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

    pub fn build_cli() -> Command {
        Command::new("Fluent CLI")
            .version("0.1.0")
            .author("Your Name <your.email@example.com>")
            .about("A powerful CLI for interacting with various AI engines")
            .arg(
                Arg::new("config")
                    .short('c')
                    .long("config")
                    .value_name("FILE")
                    .help("Sets a custom config file")
                    .required(false),
            )
            .arg(
                Arg::new("engine")
                    .help("The engine to use (openai or anthropic)")
                    .required(true),
            )
            .arg(
                Arg::new("request")
                    .help("The request to process")
                    .required(false),
            )
            .arg(
                Arg::new("override")
                    .short('o')
                    .long("override")
                    .value_name("KEY=VALUE")
                    .help("Override configuration values")
                    .action(ArgAction::Append)
                    .num_args(1..),
            )
            .arg(
                Arg::new("additional-context-file")
                    .long("additional-context-file")
                    .short('a')
                    .help("Specifies a file from which additional request context is loaded")
                    .action(ArgAction::Set)
                    .value_hint(clap::ValueHint::FilePath)
                    .required(false),
            )
            .arg(
                Arg::new("upsert")
                    .long("upsert")
                    .help("Enables upsert mode")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("request"),
            )
            .arg(
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .value_name("FILE")
                    .help("Input file or directory to process (required for upsert)")
                    .required(false),
            )
            .arg(
                Arg::new("metadata")
                    .long("metadata")
                    .short('t')
                    .value_name("TERMS")
                    .help("Comma-separated list of metadata terms (for upsert)")
                    .required(false),
            )
            .arg(
                Arg::new("upload-image-file")
                    .short('l')
                    .long("upload_image_file")
                    .value_name("FILE")
                    .help("Upload a media file")
                    .action(ArgAction::Set)
                    .required(false),
            )
            .arg(
                Arg::new("download-media")
                    .short('d')
                    .long("download-media")
                    .value_name("DIR")
                    .help("Download media files from the output")
                    .action(ArgAction::Set)
                    .required(false),
            )
            .arg(
                Arg::new("parse-code")
                    .short('p')
                    .long("parse-code")
                    .help("Parse and display code blocks from the output")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("execute-output")
                    .short('x')
                    .long("execute-output")
                    .help("Execute code blocks from the output")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("cache")
                    .long("cache")
                    .help("Enable request caching")
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
                    .help("Generate and execute a Cypher query based on the given string")
                    .action(ArgAction::Set)
                    .required(false),
            )
            // Agentic mode arguments
            .arg(
                Arg::new("agentic")
                    .long("agentic")
                    .help("Enable agentic mode with goal-oriented execution")
                    .action(ArgAction::SetTrue)
                    .required(false),
            )
            .arg(
                Arg::new("goal")
                    .long("goal")
                    .value_name("GOAL")
                    .help("Goal for the agent to achieve")
                    .action(ArgAction::Set)
                    .required(false),
            )
            .arg(
                Arg::new("agent_config")
                    .long("agent-config")
                    .value_name("FILE")
                    .help("Agent configuration file")
                    .action(ArgAction::Set)
                    .default_value("agent_config.json")
                    .required(false),
            )
            .arg(
                Arg::new("max_iterations")
                    .long("max-iterations")
                    .value_name("NUM")
                    .help("Maximum iterations for goal achievement")
                    .action(ArgAction::Set)
                    .default_value("50")
                    .required(false),
            )
            .arg(
                Arg::new("enable_tools")
                    .long("enable-tools")
                    .help("Enable tool execution (file operations, shell commands)")
                    .action(ArgAction::SetTrue)
                    .required(false),
            )
            .subcommand(
                Command::new("pipeline")
                    .about("Execute a pipeline")
                    .arg(
                        Arg::new("file")
                            .short('f')
                            .long("file")
                            .help("The YAML file containing the pipeline definition")
                            .required(true),
                    )
                    .arg(
                        Arg::new("input")
                            .short('i')
                            .long("input")
                            .help("The input for the pipeline")
                            .required(true),
                    )
                    .arg(
                        Arg::new("force_fresh")
                            .long("force-fresh")
                            .help("Force a fresh execution of the pipeline")
                            .action(ArgAction::SetTrue),
                    )
                    .arg(
                        Arg::new("run_id")
                            .long("run-id")
                            .help("Specify a run ID for the pipeline"),
                    )
                    .arg(
                        Arg::new("json_output")
                            .long("json-output")
                            .help("Output only the JSON result, suppressing PrintOutput steps")
                            .action(ArgAction::SetTrue),
                    ),
            )
            .subcommand(
                Command::new("build-pipeline")
                    .about("Interactively build a pipeline")
            )
            .subcommand(
                Command::new("agent")
                    .about("Start interactive agent loop")
            )
            .subcommand(
                Command::new("mcp")
                    .about("Model Context Protocol (MCP) management")
                    .subcommand(
                        Command::new("server")
                            .about("Start MCP server")
                            .arg(
                                Arg::new("port")
                                    .short('p')
                                    .long("port")
                                    .value_name("PORT")
                                    .help("Port to listen on (for HTTP transport)")
                                    .required(false)
                            )
                            .arg(
                                Arg::new("stdio")
                                    .long("stdio")
                                    .help("Use STDIO transport (default)")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("config")
                                    .short('c')
                                    .long("config")
                                    .value_name("FILE")
                                    .help("Configuration file path")
                            )
                    )
                    .subcommand(
                        Command::new("connect")
                            .about("Connect to an MCP server")
                            .arg(
                                Arg::new("name")
                                    .short('n')
                                    .long("name")
                                    .value_name("NAME")
                                    .help("Server name")
                                    .required(true)
                            )
                            .arg(
                                Arg::new("command")
                                    .short('c')
                                    .long("command")
                                    .value_name("COMMAND")
                                    .help("Server command to execute")
                                    .required(true)
                            )
                            .arg(
                                Arg::new("args")
                                    .short('a')
                                    .long("args")
                                    .value_name("ARGS")
                                    .help("Command arguments")
                                    .action(ArgAction::Append)
                                    .num_args(0..)
                            )
                    )
                    .subcommand(
                        Command::new("disconnect")
                            .about("Disconnect from an MCP server")
                            .arg(
                                Arg::new("name")
                                    .short('n')
                                    .long("name")
                                    .value_name("NAME")
                                    .help("Server name")
                                    .required(true)
                            )
                    )
                    .subcommand(
                        Command::new("tools")
                            .about("List available tools")
                            .arg(
                                Arg::new("server")
                                    .short('s')
                                    .long("server")
                                    .value_name("SERVER")
                                    .help("Filter by server name")
                            )
                            .arg(
                                Arg::new("json")
                                    .long("json")
                                    .help("Output in JSON format")
                                    .action(ArgAction::SetTrue)
                            )
                    )
                    .subcommand(
                        Command::new("execute")
                            .about("Execute a tool")
                            .arg(
                                Arg::new("tool")
                                    .short('t')
                                    .long("tool")
                                    .value_name("TOOL")
                                    .help("Tool name to execute")
                                    .required(true)
                            )
                            .arg(
                                Arg::new("parameters")
                                    .short('p')
                                    .long("parameters")
                                    .value_name("JSON")
                                    .help("Tool parameters as JSON")
                                    .default_value("{}")
                            )
                            .arg(
                                Arg::new("server")
                                    .short('s')
                                    .long("server")
                                    .value_name("SERVER")
                                    .help("Preferred server name")
                            )
                            .arg(
                                Arg::new("timeout")
                                    .long("timeout")
                                    .value_name("SECONDS")
                                    .help("Execution timeout in seconds")
                                    .default_value("30")
                            )
                    )
                    .subcommand(
                        Command::new("status")
                            .about("Show MCP system status")
                            .arg(
                                Arg::new("json")
                                    .long("json")
                                    .help("Output in JSON format")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("detailed")
                                    .long("detailed")
                                    .help("Show detailed metrics")
                                    .action(ArgAction::SetTrue)
                            )
                    )
                    .subcommand(
                        Command::new("config")
                            .about("Manage MCP configuration")
                            .arg(
                                Arg::new("show")
                                    .long("show")
                                    .help("Show current configuration")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("set")
                                    .long("set")
                                    .value_name("KEY")
                                    .help("Configuration key to set")
                            )
                            .arg(
                                Arg::new("value")
                                    .long("value")
                                    .value_name("VALUE")
                                    .help("Configuration value")
                            )
                            .arg(
                                Arg::new("file")
                                    .short('f')
                                    .long("file")
                                    .value_name("FILE")
                                    .help("Save configuration to file")
                            )
                    )
                    .subcommand(
                        Command::new("agent")
                            .about("Run agent with MCP capabilities (legacy)")
                            .arg(
                                Arg::new("engine")
                                    .short('e')
                                    .long("engine")
                                    .value_name("ENGINE")
                                    .help("LLM engine to use")
                                    .required(true)
                            )
                            .arg(
                                Arg::new("task")
                                    .short('t')
                                    .long("task")
                                    .value_name("TASK")
                                    .help("Task description")
                                    .required(true)
                            )
                            .arg(
                                Arg::new("servers")
                                    .short('s')
                                    .long("servers")
                                    .value_name("SERVERS")
                                    .help("MCP servers to use")
                                    .action(ArgAction::Append)
                                    .num_args(0..)
                            )
                    )
            )
            .subcommand(
                Command::new("agent-mcp")
                    .about("Run agent with MCP (Model Context Protocol) capabilities")
                    .arg(
                        Arg::new("engine")
                            .short('e')
                            .long("engine")
                            .value_name("ENGINE")
                            .help("LLM engine to use for reasoning")
                            .required(true)
                    )
                    .arg(
                        Arg::new("task")
                            .short('t')
                            .long("task")
                            .value_name("TASK")
                            .help("Task description for the agent to execute")
                            .required(true)
                    )
                    .arg(
                        Arg::new("mcp-servers")
                            .short('s')
                            .long("servers")
                            .value_name("SERVERS")
                            .help("Comma-separated list of MCP servers (format: name:command or just command)")
                            .default_value("filesystem:mcp-server-filesystem,git:mcp-server-git")
                    )
                    .arg(
                        Arg::new("config")
                            .short('c')
                            .long("config")
                            .value_name("CONFIG")
                            .help("Configuration file path")
                    )
            )
            .subcommand(
                Command::new("tools")
                    .about("Direct tool access and management")
                    .subcommand(
                        Command::new("list")
                            .about("List available tools")
                            .arg(
                                Arg::new("category")
                                    .long("category")
                                    .value_name("CATEGORY")
                                    .help("Filter by tool category")
                            )
                            .arg(
                                Arg::new("search")
                                    .long("search")
                                    .value_name("TERM")
                                    .help("Search tools by name or description")
                            )
                            .arg(
                                Arg::new("json")
                                    .long("json")
                                    .help("Output in JSON format")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("detailed")
                                    .long("detailed")
                                    .help("Show detailed information")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("available")
                                    .long("available")
                                    .help("Show only available/enabled tools")
                                    .action(ArgAction::SetTrue)
                            )
                    )
                    .subcommand(
                        Command::new("describe")
                            .about("Describe a specific tool")
                            .arg(
                                Arg::new("tool")
                                    .help("Tool name to describe")
                                    .required(true)
                            )
                            .arg(
                                Arg::new("schema")
                                    .long("schema")
                                    .help("Show parameter schema")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("examples")
                                    .long("examples")
                                    .help("Show usage examples")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("json")
                                    .long("json")
                                    .help("Output in JSON format")
                                    .action(ArgAction::SetTrue)
                            )
                    )
                    .subcommand(
                        Command::new("exec")
                            .about("Execute a tool directly")
                            .arg(
                                Arg::new("tool")
                                    .help("Tool name to execute")
                                    .required(true)
                            )
                            .arg(
                                Arg::new("json")
                                    .long("json")
                                    .value_name("JSON")
                                    .help("Parameters as JSON string")
                            )
                            .arg(
                                Arg::new("params-file")
                                    .long("params-file")
                                    .value_name("FILE")
                                    .help("Parameters from JSON file")
                            )
                            .arg(
                                Arg::new("path")
                                    .long("path")
                                    .value_name("PATH")
                                    .help("File path parameter")
                            )
                            .arg(
                                Arg::new("content")
                                    .long("content")
                                    .value_name("CONTENT")
                                    .help("Content parameter")
                            )
                            .arg(
                                Arg::new("command")
                                    .long("command")
                                    .value_name("COMMAND")
                                    .help("Command parameter")
                            )
                            .arg(
                                Arg::new("dry-run")
                                    .long("dry-run")
                                    .help("Show what would be executed without running")
                                    .action(ArgAction::SetTrue)
                            )
                            .arg(
                                Arg::new("timeout")
                                    .long("timeout")
                                    .value_name("DURATION")
                                    .help("Execution timeout (e.g., 30s, 5m)")
                            )
                            .arg(
                                Arg::new("json-output")
                                    .long("json-output")
                                    .help("Output result in JSON format")
                                    .action(ArgAction::SetTrue)
                            )
                    )
                    .subcommand(
                        Command::new("categories")
                            .about("List tool categories")
                            .arg(
                                Arg::new("json")
                                    .long("json")
                                    .help("Output in JSON format")
                                    .action(ArgAction::SetTrue)
                            )
                    )
            )
    }

    pub async fn get_neo4j_query_llm(config: &Config) -> Option<(Box<dyn Engine>, &EngineConfig)> {
        let neo4j_config = config.engines.iter().find(|e| e.engine == "neo4j")?;
        let query_llm = neo4j_config.neo4j.as_ref()?.query_llm.as_ref()?;
        let llm_config = config.engines.iter().find(|e| e.name == *query_llm)?;
        let engine = create_llm_engine(llm_config).await.ok()?;
        Some((engine, llm_config))
    }

    pub async fn run_mcp_server(_sub_matches: &ArgMatches) -> Result<()> {
        use fluent_agent::mcp_adapter::FluentMcpServer;
        use fluent_agent::memory::SqliteMemoryStore;
        use fluent_agent::tools::ToolRegistry;
        use std::sync::Arc;

        println!("🔌 Starting Fluent CLI Model Context Protocol Server");

        // Initialize tool registry
        let tool_registry = Arc::new(ToolRegistry::new());

        // Initialize memory system
        let memory_system = Arc::new(SqliteMemoryStore::new(":memory:")?);

        // Create MCP server
        let server = FluentMcpServer::new(tool_registry, memory_system);

        // Use STDIO transport by default
        println!("📡 Using STDIO transport");
        println!("🚀 MCP Server ready - waiting for connections...");

        // Start the server
        server.start_stdio().await?;

        Ok(())
    }

    pub async fn run_agentic_mode(
        goal_description: &str,
        agent_config_path: &str,
        max_iterations: u32,
        enable_tools: bool,
        config: &Config,
        config_path: &str,
    ) -> Result<()> {
        use crate::agentic::{AgenticConfig, AgenticExecutor};

        let agentic_config = AgenticConfig::new(
            goal_description.to_string(),
            agent_config_path.to_string(),
            max_iterations,
            enable_tools,
            config_path.to_string(),
        );

        let executor = AgenticExecutor::new(agentic_config);
        executor.run(config).await

    }

    pub async fn run_agent_with_mcp(
        engine_name: &str,
        task: &str,
        mcp_servers: Vec<String>,
        config: &Config,
    ) -> Result<()> {
        use fluent_agent::agent_with_mcp::AgentWithMcp;
        use fluent_agent::memory::SqliteMemoryStore;
        use fluent_agent::reasoning::LLMReasoningEngine;

        println!("🚀 Starting Fluent CLI Agent with MCP capabilities");

        // Get the engine config
        let engine_config = config
            .engines
            .iter()
            .find(|e| e.name == engine_name)
            .ok_or_else(|| anyhow::anyhow!("Engine '{}' not found", engine_name))?;

        // Create the engine
        let engine = create_llm_engine(engine_config).await?;

        // Setup memory system
        let memory_path = format!("agent_memory_{}.db", engine_name);
        let memory = std::sync::Arc::new(SqliteMemoryStore::new(&memory_path)?);

        // Setup reasoning engine
        let reasoning_engine = Box::new(LLMReasoningEngine::new(std::sync::Arc::new(engine)));

        // Create agent
        let agent = AgentWithMcp::new(memory, reasoning_engine);

        // Connect to MCP servers
        for server_spec in mcp_servers {
            let parts: Vec<&str> = server_spec.split(':').collect();
            let (name, command) = if parts.len() >= 2 {
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

    // Autonomous execution moved to agentic module



    // extract_code function moved to utils module

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

        let config = load_config(&config_path, engine_name, &std::collections::HashMap::new())?;

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
            Some(("neo4j", sub_matches)) => {
                let handler = neo4j::Neo4jCommand::new();
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

    #[allow(dead_code)]
    async fn handle_upsert(engine_config: &EngineConfig, matches: &ArgMatches) -> Result<()> {
        crate::neo4j_operations::handle_upsert(engine_config, matches).await
    }

    pub async fn generate_cypher_query(query: &str, config: &EngineConfig) -> Result<String> {
        // Use the configured LLM to generate a Cypher query
        let llm_request = Request {
            flowname: "cypher_generation".to_string(),
            payload: format!(
                "Generate a Cypher query for Neo4j based on this request: {}",
                query
            ),
        };
        debug!("Sending request to LLM engine: {:?}", llm_request);
        let llm_engine: Box<dyn Engine> = match config.engine.as_str() {
            "openai" => Box::new(OpenAIEngine::new(config.clone()).await?),
            "anthropic" => Box::new(AnthropicEngine::new(config.clone()).await?),
            // Add other LLM engines as needed
            _ => return Err(anyhow!("Unsupported LLM engine for Cypher generation")),
        };

        let response = Pin::from(llm_engine.execute(&llm_request)).await?;

        debug!("Response from LLM engine: {:?}", response);
        Ok(response.content)
    }
}

#[allow(dead_code)]
async fn generate_and_execute_cypher(
    neo4j_config: &Neo4jConfig,
    llm_config: &EngineConfig,
    query_string: &str,
    llm_engine: &dyn Engine,
) -> Result<String, Error> {
    crate::neo4j_operations::generate_and_execute_cypher(neo4j_config, llm_config, query_string, llm_engine).await
}



async fn create_llm_engine(engine_config: &EngineConfig) -> Result<Box<dyn Engine>, Error> {
    create_engine(engine_config).await
}
