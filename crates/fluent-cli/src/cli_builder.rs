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
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Increase output verbosity (overrides --quiet)")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress non-error output")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            Arg::new("json-logs")
                .long("json-logs")
                .help("Emit JSON logs (same as FLUENT_LOG_FORMAT=json)")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            Arg::new("human-logs")
                .long("human-logs")
                .help("Emit human-readable logs (default if not set)")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(
            Command::new("pipeline")
                .about("Execute a pipeline from a YAML file")
                .after_help(r#"EXAMPLES:
    # Execute a pipeline with input
    fluent pipeline -f example_pipelines/test_pipeline.yaml -i "Hello world"

    # Execute with a custom run ID
    fluent pipeline -f pipeline.yaml --run-id my-test-run

    # Get JSON output
    fluent pipeline -f pipeline.yaml -i "test" --json

    # Dry run to validate pipeline
    fluent pipeline -f pipeline.yaml --dry-run

    # Force fresh execution, ignoring cached state
    fluent pipeline -f pipeline.yaml --force-fresh -i "test"
"#)
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
                .after_help(r#"EXAMPLES:
    # Interactive agent mode with TUI
    fluent agent --tui

    # Run agent with a specific goal
    fluent agent -g "Create a Tetris game in HTML"

    # Use a goal file with success criteria
    fluent agent --goal-file examples/goals/tetris.toml

    # Enable tools and set max iterations
    fluent agent -g "Analyze code" --enable-tools --max-iterations 20

    # Run with reflection mode
    fluent agent -g "Refactor code" --reflection --enable-tools

    # Dry run to preview agent configuration
    fluent agent -g "Test task" --dry-run
"#)
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
                    Arg::new("tui")
                        .long("tui")
                        .help("Enable terminal user interface for better monitoring")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("tui-mode")
                        .long("tui-mode")
                        .value_name("MODE")
                        .help("TUI mode: collab | simple | full | ascii")
                        .required(false),
                )
                .arg(
                    Arg::new("ascii")
                        .long("ascii")
                        .help("Force ASCII TUI (prints to stdout, no alternate screen)")
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
                .after_help(r#"EXAMPLES:
    # Start MCP server on default port 8080
    fluent mcp server

    # Start MCP server on custom port
    fluent mcp server -p 9000

    # Connect as MCP client to a server
    fluent mcp client -s http://localhost:8080
"#)
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
                .after_help(r#"EXAMPLES:
    # Generate Cypher query from natural language
    fluent neo4j --generate-cypher -q "Find all users who purchased in the last month"

    # Execute a direct Cypher query
    fluent neo4j -q "MATCH (n:User) RETURN n LIMIT 10"

    # Upsert data from a file
    fluent neo4j --upsert-file data.json
"#)
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
            Command::new("completions")
                .about("Generate shell completion scripts")
                .after_help(r#"EXAMPLES:
    # Generate Zsh completions and save to file
    fluent completions -s zsh -o _fluent

    # Generate Bash completions to stdout
    fluent completions -s bash

    # Generate Fish completions
    fluent completions -s fish -o ~/.config/fish/completions/fluent.fish

    # Generate PowerShell completions
    fluent completions -s powershell -o fluent.ps1
"#)
                .arg(
                    Arg::new("shell")
                        .short('s')
                        .long("shell")
                        .value_name("SHELL")
                        .help("Shell type: bash, zsh, fish, powershell, elvish")
                        .required(true),
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Write completions to file (default: stdout)"),
                ),
        )
        .subcommand(
            Command::new("tools")
                .about("Direct tool access and management")
                .after_help(r#"EXAMPLES:
    # List all available tools
    fluent tools list

    # List tools in JSON format
    fluent tools list --json

    # Search for file-related tools
    fluent tools list --search file

    # Describe a specific tool
    fluent tools describe read_file

    # Get tool schema
    fluent tools describe read_file --schema

    # Execute a tool
    fluent tools exec read_file --json '{"path": "README.md"}'

    # List tool categories
    fluent tools categories
"#)
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
                .after_help(r#"EXAMPLES:
    # List all configured engines
    fluent engine list

    # List engines in JSON format
    fluent engine list --json

    # Test engine connectivity
    fluent engine test openai

    # Test engine with JSON output
    fluent engine test anthropic --json
"#)
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
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output test results in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                ),
        )
}

// Re-export the centralized parse_key_value_pair function
pub use fluent_core::config::parse_key_value_pair;
