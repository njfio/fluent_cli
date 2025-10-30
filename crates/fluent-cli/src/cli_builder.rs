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
        .about("A powerful CLI for interacting with various AI engines\n\nEXAMPLES:\n    fluent setup                        # Interactive setup wizard\n    fluent agent \"Hello, world!\"          # Run AI agent\n    fluent pipeline -f pipeline.yaml     # Execute pipeline\n    fluent tools list                   # List available tools\n    fluent --examples                   # Show all examples")
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
        .arg(
            Arg::new("examples")
                .long("examples")
                .help("Show examples for the command instead of help")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommand(
            Command::new("pipeline")
                .about("Execute a pipeline from a YAML file")
                .after_help("COMMON USAGE:\n  Basic pipeline execution:\n    fluent pipeline -f example_pipelines/test_pipeline.yaml\n    fluent pipeline -f pipeline.yaml -i \"Process this data\"\n    fluent pipeline -f workflow.yaml --variables key=value\n\n  Advanced usage:\n    fluent pipeline -f complex_workflow.yaml --verbose\n    fluent pipeline -f pipelines/ci_cd.yaml --config production.toml\n    fluent pipeline -f pipeline.yaml --dry-run\n\nEXAMPLES:\n  # Execute a simple pipeline\n  fluent pipeline -f example_pipelines/test_pipeline.yaml\n\n  # Execute with input data\n  fluent pipeline -f pipelines/code_review.yaml -i \"Review this PR\"\n\n  # Execute with variables\n  fluent pipeline -f workflow.yaml --variables API_KEY=xxx OUTPUT_DIR=/tmp\n\n  # Preview execution without running\n  fluent pipeline -f pipeline.yaml --dry-run\n\nTIPS:\n  • Use --dry-run to preview pipeline execution before running\n  • Variables can be passed multiple times: --variables key1=val1 --variables key2=val2\n  • Use --force-fresh to ignore cached state\n  • Pipeline files must be valid YAML\n\nCOMMON MISTAKES:\n  ⚠️  Missing -f flag: fluent pipeline pipeline.yaml (WRONG)\n  ✅ Correct: fluent pipeline -f pipeline.yaml\n  ⚠️  File not found: Check file path and ensure it exists\n  ⚠️  Invalid YAML: Validate YAML syntax before running")
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
                .about("Run agentic workflows with AI assistance")
                .after_help("COMMON USAGE:\n  Basic agent execution:\n    fluent agent \"Hello, world!\"\n    fluent agent \"Write a Rust function\" --interactive\n    fluent agent \"Debug this error\" --verbose\n\n  With files:\n    fluent agent \"Create tests\" --file src/main.rs\n    fluent agent \"Refactor this\" --goal-file goal.toml\n\nEXAMPLES:\n  # Simple agent task\n  fluent agent \"Write a function to calculate fibonacci numbers\"\n\n  # Interactive mode with file context\n  fluent agent \"Analyze this code\" --file src/main.rs --interactive\n\n  # Enable tool usage\n  fluent agent \"Create a REST API\" --enable-tools --max-iterations 20\n\n  # Use specific model\n  fluent agent \"Write Python tests\" --model gpt-4o\n\n  # Reflection mode for complex tasks\n  fluent agent \"Design a system architecture\" --reflection --tui\n\n  # Goal file for complex tasks\n  fluent agent --goal-file project_goal.toml\n\nTIPS:\n  • Use --interactive for complex tasks requiring human input\n  • --enable-tools allows the agent to modify files and run commands\n  • --tui provides a better monitoring experience\n  • Use --max-iterations to control execution depth\n  • --reflection enables self-improvement during execution\n\nCOMMON MISTAKES:\n  ⚠️  Missing quotes: fluent agent Write a function (WRONG)\n  ✅ Correct: fluent agent \"Write a function\"\n  ⚠️  Too many iterations: Start with default (10) and increase if needed\n  ⚠️  File not found: Check file path with --file option")
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
                .after_help("COMMON USAGE:\n  Start server:\n    fluent mcp server\n    fluent mcp server --port 8080\n\n  Connect as client:\n    fluent mcp client --server http://localhost:8080\n\nEXAMPLES:\n  # Start MCP server on default port\n  fluent mcp server\n\n  # Start on custom port\n  fluent mcp server --port 9090\n\n  # Connect as client\n  fluent mcp client --server http://localhost:8080\n\nTIPS:\n  • Default port is 8080\n  • Use --port to specify custom port\n  • Server URL must include protocol (http:// or https://)\n\nCOMMON MISTAKES:\n  ⚠️  Port already in use: Use --port to specify different port\n  ⚠️  Invalid URL: Include protocol (http:// or https://)")
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
                .after_help("COMMON USAGE:\n  Query Neo4j:\n    fluent neo4j --query \"MATCH (n) RETURN n LIMIT 10\"\n    fluent neo4j --query \"Find all users\" --generate-cypher\n\n  Upsert data:\n    fluent neo4j --upsert-file data.json\n\nEXAMPLES:\n  # Execute Cypher query\n  fluent neo4j --query \"MATCH (n) RETURN n LIMIT 10\"\n\n  # Generate Cypher from natural language\n  fluent neo4j --query \"Find all users\" --generate-cypher\n\n  # Upsert data from file\n  fluent neo4j --upsert-file users.json\n\nTIPS:\n  • Use --generate-cypher for natural language queries\n  • File format should be JSON for upsert operations\n  • Neo4j connection must be configured in fluent_config.toml\n\nCOMMON MISTAKES:\n  ⚠️  Neo4j not configured: Set up Neo4j connection in configuration\n  ⚠️  Invalid query: Check Cypher syntax\n  ⚠️  File format: Ensure JSON format for --upsert-file")
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
                .after_help("COMMON USAGE:\n  Generate for shell:\n    fluent completions --shell bash\n    fluent completions --shell zsh --output fluent.bash\n\nEXAMPLES:\n  # Generate for bash\n  fluent completions --shell bash\n\n  # Generate for zsh\n  fluent completions --shell zsh --output fluent.zsh\n\n  # Generate for fish\n  fluent completions --shell fish\n\nTIPS:\n  • Supported shells: bash, zsh, fish, powershell, elvish\n  • Output to file for permanent installation\n  • Add to your shell's rc file for persistence\n\nCOMMON MISTAKES:\n  ⚠️  Unsupported shell: Check supported shells list\n  ⚠️  Not installed: Source the generated file in your shell config")
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
                .after_help("COMMON USAGE:\n  List tools:\n    fluent tools list\n    fluent tools list --category file\n    fluent tools list --available\n\n  Search and describe:\n    fluent tools search \"http\"\n    fluent tools describe read_file --examples --requirements\n\n  Test and document:\n    fluent tools test read_file --interactive\n    fluent tools docs --format markdown\n    fluent tools docs --tool read_file --output docs.md\n\n  Analytics and recommendations:\n    fluent tools analytics\n    fluent tools recommend \"read and process files\"\n\n  Execute tools:\n    fluent tools exec read_file --json '{\"path\": \"README.md\"}'\n\nEXAMPLES:\n  # List all tools\n  fluent tools list\n\n  # Search for tools\n  fluent tools search \"file operations\"\n\n  # Get detailed tool information\n  fluent tools describe read_file --schema --examples --requirements\n\n  # Test a tool interactively\n  fluent tools test read_file --interactive\n\n  # Generate documentation\n  fluent tools docs\n  fluent tools docs --format json --output tools.json\n\n  # Get tool recommendations\n  fluent tools recommend \"read and process files\"\n\n  # View tool analytics\n  fluent tools analytics\n  fluent tools analytics --tool read_file\n\nTIPS:\n  • Use --json for programmatic tool access\n  • --category filters tools by type (file, network, system, data, ai)\n  • --available shows only enabled tools\n  • --detailed provides more information\n  • Use describe with --examples and --requirements for complete info\n  • Search uses semantic matching for better results\n  • Generate docs in markdown, json, or html format\n\nCOMMON MISTAKES:\n  ⚠️  Tool not found: Use 'fluent tools list' or 'fluent tools search' to find tools\n  ⚠️  Invalid JSON: Ensure proper JSON format for exec arguments\n  ⚠️  Missing tool name: Tool name is required for describe, test, and exec")
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
                        )
                        .arg(
                            Arg::new("requirements")
                                .long("requirements")
                                .help("Show tool requirements and compatibility")
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
                )
                .subcommand(
                    Command::new("analytics")
                        .about("Show tool usage analytics and performance metrics")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("tool")
                                .help("Show analytics for specific tool")
                                .required(false),
                        ),
                )
                .subcommand(
                    Command::new("recommend")
                        .about("Get tool recommendations for a task")
                        .arg(
                            Arg::new("task")
                                .help("Task description for recommendations")
                                .required(true),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("search")
                        .about("Search tools with semantic matching")
                        .arg(
                            Arg::new("query")
                                .help("Search query (semantic search)")
                                .required(true),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("limit")
                                .short('l')
                                .long("limit")
                                .value_name("N")
                                .help("Limit number of results")
                                .value_parser(clap::value_parser!(usize))
                                .default_value("10"),
                        ),
                )
                .subcommand(
                    Command::new("test")
                        .about("Interactive tool tester")
                        .arg(
                            Arg::new("tool")
                                .help("Tool name to test")
                                .required(true),
                        )
                        .arg(
                            Arg::new("interactive")
                                .short('i')
                                .long("interactive")
                                .help("Run in interactive mode")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("docs")
                        .about("Generate tool documentation")
                        .arg(
                            Arg::new("tool")
                                .help("Tool name (omit for all tools)")
                                .required(false),
                        )
                        .arg(
                            Arg::new("output")
                                .short('o')
                                .long("output")
                                .value_name("FILE")
                                .help("Output file (default: stdout)"),
                        )
                        .arg(
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .value_name("FORMAT")
                                .help("Output format: markdown, json, html")
                                .value_parser(["markdown", "json", "html"])
                                .default_value("markdown"),
                        ),
                ),
        )
        .subcommand(
            Command::new("configure")
                .about("Advanced configuration management and optimization")
                .after_help("COMMON USAGE:\n  View configuration:\n    fluent configure show\n    fluent configure show --json\n\n  Apply presets:\n    fluent configure presets\n    fluent configure presets --apply developer\n\n  Optimize:\n    fluent configure optimize\n    fluent configure optimize --dry-run\n\n  Set values:\n    fluent configure set memory.max_tokens 8000\n\nEXAMPLES:\n  # View current configuration\n  fluent configure show\n\n  # View as JSON\n  fluent configure show --json\n\n  # List available presets\n  fluent configure presets\n\n  # Apply a preset\n  fluent configure presets --apply developer\n  fluent configure presets --apply production\n\n  # Optimize configuration\n  fluent configure optimize --dry-run\n  fluent configure optimize\n\n  # Set configuration values\n  fluent configure set memory.max_tokens 8000\n  fluent configure set agent.max_iterations 20\n\nTIPS:\n  • Use presets for common configurations (developer, researcher, production)\n  • --dry-run shows what would change without applying\n  • Configuration keys use dot notation (e.g., memory.max_tokens)\n  • Optimize analyzes usage patterns and suggests improvements\n\nCOMMON MISTAKES:\n  ⚠️  Invalid key format: Use dot notation (memory.max_tokens, not memory/max_tokens)\n  ⚠️  Unknown preset: Use 'fluent configure presets' to see available options\n  ⚠️  Not saved: Changes are applied immediately, no separate save step")
                .subcommand(
                    Command::new("show")
                        .about("Display current configuration")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("presets")
                        .about("List and apply configuration presets")
                        .arg(
                            Arg::new("apply")
                                .long("apply")
                                .value_name("PRESET")
                                .help("Apply a specific preset"),
                        ),
                )
                .subcommand(
                    Command::new("optimize")
                        .about("Analyze and optimize current configuration")
                        .arg(
                            Arg::new("dry-run")
                                .long("dry-run")
                                .help("Show optimizations without applying them")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("set")
                        .about("Set a configuration value")
                        .arg(
                            Arg::new("key")
                                .help("Configuration key (e.g., memory.max_tokens)")
                                .required(true),
                        )
                        .arg(
                            Arg::new("value")
                                .help("Configuration value")
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            Command::new("setup")
                .about("Interactive setup wizard for FluentCLI configuration")
                .after_help("COMMON USAGE:\n  First-time setup:\n    fluent setup\n    fluent setup --output my_config.toml\n\n  Reconfigure:\n    fluent setup --force\n    fluent setup --skip-validation\n\nEXAMPLES:\n  # Initial setup (interactive)\n  fluent setup\n\n  # Setup with custom output file\n  fluent setup --output production_config.toml\n\n  # Overwrite existing config\n  fluent setup --force\n\n  # Skip validation (faster for testing)\n  fluent setup --skip-validation\n\nTIPS:\n  • Set API keys in environment variables for auto-detection\n  • ANTHROPIC_API_KEY, OPENAI_API_KEY, etc. are auto-detected\n  • Use --force to overwrite existing configurations\n  • Run setup first before using other commands\n  • Configuration is saved to fluent_config.toml by default\n\nCOMMON MISTAKES:\n  ⚠️  Missing API key: Set environment variable or enter manually\n  ⚠️  Invalid configuration: Use --skip-validation only for testing\n  ⚠️  File exists: Use --force to overwrite without prompt")
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Output configuration file path")
                        .default_value("fluent_config.toml"),
                )
                .arg(
                    Arg::new("force")
                        .long("force")
                        .help("Overwrite existing configuration without prompting")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("skip-validation")
                        .long("skip-validation")
                        .help("Skip configuration validation")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("engine")
                .about("Engine management and configuration")
                .after_help("COMMON USAGE:\n  List engines:\n    fluent engine list\n    fluent engine list --json\n\n  Test engines:\n    fluent engine test anthropic\n    fluent engine test openai --verbose\n\nEXAMPLES:\n  # List all configured engines\n  fluent engine list\n\n  # List engines as JSON\n  fluent engine list --json\n\n  # Test engine connectivity\n  fluent engine test anthropic\n  fluent engine test openai\n\n  # Test with verbose output\n  fluent engine test groq --verbose\n\nTIPS:\n  • Test engines after configuration to ensure connectivity\n  • Use --json for programmatic access\n  • Engine names are case-sensitive\n  • Test engines before using them in agent mode\n\nCOMMON MISTAKES:\n  ⚠️  Engine not found: Configure engines first with 'fluent setup'\n  ⚠️  Wrong engine name: Use exact names (anthropic, openai, groq, etc.)\n  ⚠️  Connection failed: Check API keys and network connectivity")
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
            Command::new("errors")
                .about("View error recovery information and patterns")
                .after_help("COMMON USAGE:\n  View error patterns:\n    fluent errors patterns\n    fluent errors patterns --json\n\n  View recovery history:\n    fluent errors history\n    fluent errors history --limit 20\n\n  View statistics:\n    fluent errors stats\n\nEXAMPLES:\n  # View detected error patterns\n  fluent errors patterns\n\n  # View recovery history\n  fluent errors history\n\n  # View error statistics\n  fluent errors stats\n\n  # View recovery decisions\n  fluent errors recovery\n\nTIPS:\n  • Patterns show common errors and their fixes\n  • History shows past recovery attempts\n  • Stats show recovery success rates\n  • Recovery improves over time as patterns are learned\n\nCOMMON MISTAKES:\n  ⚠️  No data yet: Run agent tasks to generate error data\n  ⚠️  Patterns not shown: Errors must occur first to detect patterns")
                .subcommand(
                    Command::new("patterns")
                        .about("Show detected error patterns")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("history")
                        .about("Show error recovery history")
                        .arg(
                            Arg::new("limit")
                                .short('l')
                                .long("limit")
                                .value_name("N")
                                .help("Limit number of entries shown")
                                .value_parser(clap::value_parser!(usize))
                                .default_value("20"),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("stats")
                        .about("Show error recovery statistics")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("recovery")
                        .about("Show recovery decisions and strategies")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                ),
        )
        .subcommand(
            Command::new("memory")
                .about("View memory insights and learned patterns")
                .after_help("COMMON USAGE:\n  View insights:\n    fluent memory insights\n    fluent memory insights --json\n\n  View patterns:\n    fluent memory patterns\n    fluent memory patterns --domain programming\n\n  View statistics:\n    fluent memory stats\n\nEXAMPLES:\n  # View learned insights\n  fluent memory insights\n\n  # View insights as JSON\n  fluent memory insights --json\n\n  # View learned patterns\n  fluent memory patterns\n\n  # View patterns for specific domain\n  fluent memory patterns --domain programming\n\n  # View memory statistics\n  fluent memory stats\n\nTIPS:\n  • Insights show what the agent has learned from past executions\n  • Patterns show successful approaches that can be reused\n  • Stats show memory usage and effectiveness\n\nCOMMON MISTAKES:\n  ⚠️  No insights yet: Run some agent tasks first to generate learning data\n  ⚠️  Domain not found: Use 'fluent memory stats' to see available domains")
                .subcommand(
                    Command::new("insights")
                        .about("Show learned insights from past executions")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        )
                        .arg(
                            Arg::new("limit")
                                .short('l')
                                .long("limit")
                                .value_name("N")
                                .help("Limit number of insights shown")
                                .value_parser(clap::value_parser!(usize))
                                .default_value("10"),
                        ),
                )
                .subcommand(
                    Command::new("patterns")
                        .about("Show learned success patterns")
                        .arg(
                            Arg::new("domain")
                                .short('d')
                                .long("domain")
                                .value_name("DOMAIN")
                                .help("Filter by domain (e.g., programming, file_management)"),
                        )
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("stats")
                        .about("Show memory statistics and metrics")
                        .arg(
                            Arg::new("json")
                                .long("json")
                                .help("Output in JSON format")
                                .action(ArgAction::SetTrue),
                        ),
                ),
        )
        .subcommand(
            Command::new("examples")
                .about("Show detailed examples for commands")
                .arg(
                    Arg::new("command")
                        .help("Command name to show examples for")
                        .required(false),
                )
                .arg(
                    Arg::new("all")
                        .short('a')
                        .long("all")
                        .help("Show all examples for all commands")
                        .action(ArgAction::SetTrue),
                ),
        )
}

// Re-export the centralized parse_key_value_pair function
pub use fluent_core::config::parse_key_value_pair;
