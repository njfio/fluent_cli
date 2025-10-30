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
                .long_about(
                    "Execute a pipeline defined in a YAML file\n\n\
                    Examples:\n\
                      # Basic pipeline execution\n\
                      fluent pipeline -f my_pipeline.yaml -i \"Hello world\"\n\n\
                      # With variables\n\
                      fluent pipeline -f pipeline.yaml -i \"input\" --variables name=value key=value2\n\n\
                      # Dry run to preview\n\
                      fluent pipeline -f pipeline.yaml -i \"test\" --dry-run\n\n\
                      # JSON output\n\
                      fluent pipeline -f pipeline.yaml -i \"input\" --json\n\n\
                    Tips:\n\
                      - Pipeline files must be valid YAML\n\
                      - Use --dry-run to preview without executing\n\
                      - Variables override pipeline defaults\n\
                      - Use --force-fresh to ignore cached state"
                )
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
                .long_about(
                    "Run autonomous agentic workflows with AI agents\n\n\
                    Examples:\n\
                      # Simple goal\n\
                      fluent agent --goal \"Create a simple web game\" --enable-tools\n\n\
                      # With specific model\n\
                      fluent agent --goal \"Analyze code\" --model claude-3-5-sonnet-20241022\n\n\
                      # With reflection enabled\n\
                      fluent agent --goal \"Research topic\" --reflection --enable-tools\n\n\
                      # Using goal file\n\
                      fluent agent --goal-file goal.toml --enable-tools --tui\n\n\
                      # Dry run to preview\n\
                      fluent agent --goal \"Task\" --dry-run\n\n\
                    Tips:\n\
                      - Use --enable-tools for file operations and shell commands\n\
                      - --reflection enables self-improvement mode\n\
                      - --tui provides real-time monitoring\n\
                      - --max-iterations controls how many steps the agent takes\n\
                      - Always test with --dry-run first in production"
                )
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
            Command::new("completions")
                .about("Generate shell completion scripts")
                .long_about(
                    "Generate shell completion scripts for bash, zsh, fish, powershell, or elvish\n\n\
                    Examples:\n\
                      # Generate for bash\n\
                      fluent completions --shell bash > fluent.bash\n\
                      source fluent.bash\n\n\
                      # Generate for zsh\n\
                      fluent completions --shell zsh > ~/.zsh/completions/_fluent\n\n\
                      # Generate for fish\n\
                      fluent completions --shell fish > ~/.config/fish/completions/fluent.fish\n\n\
                      # Save to custom location\n\
                      fluent completions --shell bash --output /usr/local/share/bash-completion/completions/fluent\n\n\
                    Tips:\n\
                      - Install completions in your shell's completion directory\n\
                      - Restart your shell after installing\n\
                      - Fish completions go in ~/.config/fish/completions/\n\
                      - Bash completions go in /etc/bash_completion.d/ or ~/.bash_completion.d/"
                )
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
                .long_about(
                    "Manage and execute tools directly\n\n\
                    Examples:\n\
                      # List all tools\n\
                      fluent tools list\n\n\
                      # Search for tools\n\
                      fluent tools list --search file\n\n\
                      # Describe a tool with examples\n\
                      fluent tools describe read_file --examples\n\n\
                      # List tools by category\n\
                      fluent tools list --category filesystem\n\n\
                      # Execute a tool\n\
                      fluent tools exec read_file --args '{\"path\": \"file.txt\"}'\n\n\
                    Tips:\n\
                      - Use --detailed for more information\n\
                      - --examples shows usage examples\n\
                      - --schema shows parameter schemas\n\
                      - Tools must be available (not disabled)"
                )
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
                        .long_about(
                            "Execute a tool directly with provided arguments\n\n\
                            Examples:\n\
                              # Read a file\n\
                              fluent tools exec read_file --args '{\"path\": \"README.md\"}'\n\n\
                              # Write a file\n\
                              fluent tools exec write_file --args '{\"path\": \"test.txt\", \"content\": \"Hello\"}'\n\n\
                              # Get JSON output\n\
                              fluent tools exec read_file --args '{\"path\": \"file.txt\"}' --json-output\n\n\
                            Tips:\n\
                              - Arguments must be valid JSON\n\
                              - Use --json-output for structured results\n\
                              - Test tools before using them in agents"
                        )
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
                .long_about(
                    "Manage and test AI engine configurations\n\n\
                    Examples:\n\
                      # List configured engines\n\
                      fluent engine list\n\n\
                      # List engines as JSON\n\
                      fluent engine list --json\n\n\
                      # Test engine connectivity\n\
                      fluent engine test anthropic\n\n\
                    Tips:\n\
                      - Engines must be configured in fluent_config.toml\n\
                      - Use test to verify API keys and connectivity\n\
                      - JSON output is useful for scripting\n\
                      - Engine names are case-sensitive"
                )
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
        .subcommand(
            Command::new("setup")
                .about("Interactive setup wizard for initial configuration")
                .long_about(
                    "Guides you through setting up FluentCLI with an interactive wizard.\n\
                    Auto-detects API keys from environment variables and generates\n\
                    optimized configuration files.\n\n\
                    Examples:\n\
                      fluent setup                  # Setup with default config path\n\
                      fluent setup --config-path custom.toml  # Custom config path"
                )
                .arg(
                    Arg::new("config-path")
                        .long("config-path")
                        .value_name("FILE")
                        .help("Path where configuration file will be saved")
                        .default_value("fluent_config.toml"),
                ),
        )
        .subcommand(
            Command::new("configure")
                .about("Manage and optimize configuration")
                .long_about(
                    "View, manage, and optimize your FluentCLI configuration\n\n\
                    Examples:\n\
                      # Show current configuration\n\
                      fluent configure show\n\n\
                      # Show as JSON\n\
                      fluent configure show --json\n\n\
                      # Interactive configuration wizard\n\
                      fluent configure interactive\n\n\
                      # Preview configuration file\n\
                      fluent configure preview\n\n\
                    Tips:\n\
                      - Use 'show' to view current settings\n\
                      - Use 'interactive' for guided optimization\n\
                      - Presets optimize for common use cases"
                )
                .subcommand(
                    Command::new("show")
                        .about("Show current configuration")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("interactive")
                        .about("Interactive configuration wizard with presets")
                        .arg(
                            Arg::new("config-path")
                                .long("config-path")
                                .value_name("FILE")
                                .help("Path to configuration file")
                                .default_value("fluent_config.toml"),
                        ),
                )
                .subcommand(
                    Command::new("preview")
                        .about("Preview configuration file contents")
                        .arg(
                            Arg::new("config-path")
                                .long("config-path")
                                .value_name("FILE")
                                .help("Path to configuration file")
                                .default_value("fluent_config.toml"),
                        ),
                ),
        )
}

// Re-export the centralized parse_key_value_pair function
pub use fluent_core::config::parse_key_value_pair;
