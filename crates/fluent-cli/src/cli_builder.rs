//! CLI argument parsing and command building
//!
//! This module handles the construction of the command-line interface,
//! including argument definitions, validation, and parsing.

use clap::{Arg, ArgAction, Command};

/// Build the main CLI command structure
pub fn build_cli() -> Command {
    Command::new("fluent")
        .version("0.1.0")
        .author("Fluent CLI Team")
        .about("A powerful CLI for interacting with various AI engines")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .default_value("fluent_config.toml")
                .global(true),
        )
        .subcommand(
            Command::new("pipeline")
                .about("Execute a pipeline from a YAML file")
                .arg(
                    Arg::new("file")
                        .short('f')
                        .long("file")
                        .value_name("FILE")
                        .help("Pipeline YAML file to execute")
                        .required(true),
                )
                .arg(
                    Arg::new("input")
                        .short('i')
                        .long("input")
                        .value_name("INPUT")
                        .help("Input string to feed into the pipeline")
                        .required(false),
                )
                .arg(
                    Arg::new("variables")
                        .short('v')
                        .long("variables")
                        .value_name("KEY=VALUE")
                        .help("Pipeline variables")
                        .action(ArgAction::Append)
                        .num_args(1..),
                )
                .arg(
                    Arg::new("force_fresh")
                        .long("force-fresh")
                        .help("Force fresh execution, ignoring any saved state")
                        .action(clap::ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("run_id")
                        .long("run-id")
                        .value_name("ID")
                        .help("Optional run identifier to tag this execution")
                        .required(false),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .help("Show what would be executed without running")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Output in JSON format")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("agent")
                .about("Run agentic workflows")
                .arg(
                    Arg::new("agentic")
                        .long("agentic")
                        .help("Enable agentic mode")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("preview")
                        .long("preview")
                        .help("Open the generated artifact in the default viewer")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("preview-path")
                        .long("preview-path")
                        .value_name("FILE")
                        .help("Path to preview (defaults to examples/web_tetris.html)"),
                )
                .arg(
                    Arg::new("goal")
                        .short('g')
                        .long("goal")
                        .value_name("GOAL")
                        .help("Goal description for the agent")
                        .required(false),
                )
                .arg(
                    Arg::new("goal-file")
                        .long("goal-file")
                        .value_name("FILE")
                        .help("Path to a TOML goal file (goal_description, max_iterations, success_criteria)")
                        .required(false),
                )
                .arg(
                    Arg::new("model")
                        .long("model")
                        .value_name("MODEL")
                        .help("Override model for default engines (e.g. gpt-4o, claude-3-5-sonnet-20241022)")
                        .required(false),
                )
                .arg(
                    Arg::new("max-iterations")
                        .long("max-iterations")
                        .value_name("COUNT")
                        .help("Maximum number of iterations")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("10"),
                )
                .arg(
                    Arg::new("reflection")
                        .long("reflection")
                        .help("Enable reflection mode")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("enable-tools")
                        .long("enable-tools")
                        .help("Enable tool usage (filesystem, compiler, shell)")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("agent-config")
                        .long("agent-config")
                        .value_name("FILE")
                        .help("Path to agent configuration JSON")
                        .default_value("agent_config.json"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .help("Preview planned actions without executing side effects")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("gen-retries")
                        .long("gen-retries")
                        .value_name("N")
                        .help("Max retries for LLM code generation")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("3"),
                )
                .arg(
                    Arg::new("min-html-size")
                        .long("min-html-size")
                        .value_name("BYTES")
                        .help("Minimum HTML size to accept as valid output")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("2000"),
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
                                .help("Port to run the server on")
                                .value_parser(clap::value_parser!(u16))
                                .default_value("8080"),
                        ),
                )
                .subcommand(
                    Command::new("client")
                        .about("Connect as MCP client")
                        .arg(
                            Arg::new("server")
                                .short('s')
                                .long("server")
                                .value_name("URL")
                                .help("MCP server URL to connect to")
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            Command::new("neo4j")
                .about("Neo4j database operations")
                .arg(
                    Arg::new("generate-cypher")
                        .long("generate-cypher")
                        .help("Generate Cypher query from natural language")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("query")
                        .short('q')
                        .long("query")
                        .value_name("QUERY")
                        .help("Natural language query or Cypher query")
                        .required(false),
                )
                .arg(
                    Arg::new("upsert-file")
                        .long("upsert-file")
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
                                .required(false),
                        )
                        .arg(
                            Arg::new("search")
                                .long("search")
                                .value_name("TERM")
                                .help("Search tools by name or description")
                                .required(false),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("available")
                                .long("available")
                                .help("Show only available/enabled tools")
                                .action(ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("detailed")
                                .long("detailed")
                                .help("Show detailed information for each tool")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("describe")
                        .about("Describe a specific tool")
                        .arg(
                            Arg::new("tool")
                                .help("Tool name to describe")
                                .required(true),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("schema")
                                .long("schema")
                                .help("Show tool schema/parameters")
                                .action(ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("examples")
                                .long("examples")
                                .help("Show usage examples")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("exec")
                        .about("Execute a tool directly")
                        .arg(
                            Arg::new("tool")
                                .help("Tool name to execute")
                                .required(true),
                        )
                        .arg(
                            Arg::new("args")
                                .help("Tool arguments (JSON format)")
                                .required(false),
                        )
                        .arg(
                            Arg::new("json-output")
                                .long("json-output")
                                .help("Output result in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("categories")
                        .about("List tool categories")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                ),
        )
        .subcommand(
            Command::new("engine")
                .about("Engine management and configuration")
                .subcommand(
                    Command::new("list")
                        .about("List available engines")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("test")
                        .about("Test engine connectivity")
                        .arg(
                            Arg::new("engine")
                                .help("Engine name to test")
                                .required(true),
                        ),
                ),
        )
}

// Re-export the centralized parse_key_value_pair function
pub use fluent_core::config::parse_key_value_pair;
