//! CLI argument parsing and command building
//!
//! This module handles the construction of the command-line interface,
//! including argument definitions, validation, and parsing.

use clap::{Arg, ArgAction, Command};

/// Build the main CLI command structure
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
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("File to upload and process")
                .required(false),
        )
        .arg(
            Arg::new("json")
                .short('j')
                .long("json")
                .help("Output response in JSON format")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .help("Disable colored output")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("parse-code")
                .long("parse-code")
                .help("Parse and extract code blocks from response")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("execute-output")
                .long("execute-output")
                .help("Execute the output code (use with caution)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("markdown")
                .long("markdown")
                .help("Format output as markdown")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("download-media")
                .long("download-media")
                .value_name("DIR")
                .help("Download media files to specified directory")
                .required(false),
        )
        .subcommand(
            Command::new("pipeline")
                .about("Execute a pipeline from a YAML file")
                .arg(
                    Arg::new("file")
                        .short('f')
                        .long("file")
                        .value_name("FILE")
                        .help("Pipeline YAML file")
                        .required(true),
                )
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("INPUT")
                        .help("Pipeline input")
                        .required(true),
                )
                .arg(
                    Arg::new("force_fresh")
                        .long("force-fresh")
                        .help("Force fresh execution, ignoring cache")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("run_id")
                        .long("run-id")
                        .value_name("ID")
                        .help("Unique run identifier")
                        .required(false),
                )
                .arg(
                    Arg::new("json_output")
                        .long("json")
                        .help("Output in JSON format")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("agent")
                .about("Run in agentic mode")
                .arg(
                    Arg::new("agentic")
                        .long("agentic")
                        .help("Enable agentic mode")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("goal")
                        .short('g')
                        .long("goal")
                        .value_name("GOAL")
                        .help("Goal for the agent to achieve")
                        .required(false),
                )
                .arg(
                    Arg::new("agent_config")
                        .short('c')
                        .long("config")
                        .value_name("FILE")
                        .help("Agent configuration file")
                        .required(false),
                )
                .arg(
                    Arg::new("max_iterations")
                        .long("max-iterations")
                        .value_name("NUM")
                        .help("Maximum number of iterations")
                        .value_parser(clap::value_parser!(u32))
                        .required(false),
                )
                .arg(
                    Arg::new("enable_tools")
                        .long("enable-tools")
                        .help("Enable tool usage")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("task")
                        .short('t')
                        .long("task")
                        .value_name("TASK")
                        .help("Specific task for the agent")
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("mcp")
                .about("MCP server operations")
                .subcommand(
                    Command::new("server")
                        .about("Start MCP server")
                        .arg(
                            Arg::new("port")
                                .short('p')
                                .long("port")
                                .value_name("PORT")
                                .help("Port to listen on")
                                .value_parser(clap::value_parser!(u16))
                                .required(false),
                        ),
                ),
        )
        .subcommand(
            Command::new("neo4j")
                .about("Neo4j operations")
                .arg(
                    Arg::new("generate-cypher")
                        .long("generate-cypher")
                        .value_name("QUERY")
                        .help("Generate Cypher query from natural language")
                        .required(false),
                )
                .arg(
                    Arg::new("upsert")
                        .long("upsert")
                        .help("Perform upsert operation")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("FILE")
                        .help("Input file for upsert operation")
                        .required(false),
                ),
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

/// Parse key-value pairs from command line arguments
pub fn parse_key_value_pair(s: &str) -> Option<(String, String)> {
    if let Some((key, value)) = s.split_once('=') {
        Some((key.to_string(), value.to_string()))
    } else {
        None
    }
}
