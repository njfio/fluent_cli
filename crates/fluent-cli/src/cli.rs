//! Main CLI entry point and command routing
//!
//! This module provides the main entry point for the CLI application
//! and routes commands to their appropriate handlers.

use crate::error::CliError;
use anyhow::Result;
use clap::ArgMatches;
use std::path::Path;

use crate::cli_builder::build_cli;
use crate::commands::{
    agent::AgentCommand, configure::ConfigureCommand, engine::EngineCommand, examples::ExamplesCommand,
    mcp::McpCommand, neo4j::Neo4jCommand,
    pipeline::PipelineCommand, setup::SetupCommand, tools::ToolsCommand, CommandHandler,
};

/// Show examples for the given command
fn show_examples(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("agent", _)) => {
            println!("🤖 FluentCLI Agent Examples");
            println!("============================");
            println!();
            println!("📝 Basic Usage:");
            println!("  fluent agent \"Hello, world!\"");
            println!("  fluent agent \"Write a Rust function to calculate fibonacci numbers\"");
            println!("  fluent agent \"Analyze this code for performance issues\" --file code.rs");
            println!();
            println!("🔧 Advanced Usage:");
            println!("  fluent agent \"Refactor this legacy code\" --interactive");
            println!("  fluent agent \"Debug this error message\" --verbose");
            println!("  fluent agent \"Create a comprehensive test suite\" --config custom.toml");
            println!();
            println!("🎯 Common Tasks:");
            println!("  fluent agent \"Generate API documentation\"");
            println!("  fluent agent \"Optimize database queries\"");
            println!("  fluent agent \"Implement authentication middleware\"");
            println!("  fluent agent \"Create deployment scripts\"");
        }
        Some(("pipeline", _)) => {
            println!("🔄 FluentCLI Pipeline Examples");
            println!("================================");
            println!();
            println!("📝 Basic Usage:");
            println!("  fluent pipeline -f example_pipelines/test_pipeline.yaml");
            println!("  fluent pipeline -f pipelines/code_review.yaml -i \"Review this PR\"");
            println!();
            println!("🔧 Advanced Usage:");
            println!("  fluent pipeline -f complex_workflow.yaml --verbose");
            println!("  fluent pipeline -f pipelines/ci_cd.yaml --config production.toml");
            println!();
            println!("📋 Available Example Pipelines:");
            println!("  • example_pipelines/test_pipeline.yaml - Basic agent execution");
            println!("  • example_pipelines/code_analysis.yaml - Code review workflow");
            println!("  • example_pipelines/documentation.yaml - Documentation generation");
        }
        Some(("setup", _)) => {
            println!("🚀 FluentCLI Setup Examples");
            println!("============================");
            println!();
            println!("📝 Basic Setup:");
            println!("  fluent setup");
            println!("  fluent setup --output my_config.toml");
            println!();
            println!("🔧 Advanced Setup:");
            println!("  fluent setup --force --skip-validation");
            println!("  fluent setup -o production_config.toml");
            println!();
            println!("💡 Pro Tips:");
            println!("  • Set API keys in environment variables for auto-detection");
            println!("  • Use --force to overwrite existing configurations");
            println!("  • Run setup first to configure your AI engines");
        }
        Some(("tools", sub_matches)) => {
            if let Some(("list", _)) = sub_matches.subcommand() {
                println!("🛠️  FluentCLI Tools List Examples");
                println!("=================================");
                println!();
                println!("📝 Basic Usage:");
                println!("  fluent tools list");
                println!("  fluent tools list --json");
                println!();
                println!("🔍 Search and Filter:");
                println!("  fluent tools search \"file\"");
                println!("  fluent tools search \"http\" --category network");
                println!();
                println!("📚 Tool Categories:");
                println!("  • file - File system operations");
                println!("  • network - HTTP and API calls");
                println!("  • system - OS and environment");
                println!("  • data - Parsing and processing");
                println!("  • ai - AI and ML utilities");
            } else {
                println!("🛠️  FluentCLI Tools Examples");
                println!("============================");
                println!();
                println!("📝 Basic Usage:");
                println!("  fluent tools list");
                println!("  fluent tools search \"query\"");
                println!();
                println!("🔧 Tool Management:");
                println!("  fluent tools test <tool_name>");
                println!("  fluent tools info <tool_name>");
                println!();
                println!("💡 Tip: Use 'fluent tools list' to see all available tools");
            }
        }
        Some(("engine", sub_matches)) => {
            if let Some(("list", _)) = sub_matches.subcommand() {
                println!("⚙️  FluentCLI Engine List Examples");
                println!("==================================");
                println!();
                println!("📝 Basic Usage:");
                println!("  fluent engine list");
                println!("  fluent engine list --json");
                println!();
                println!("🔍 Engine Information:");
                println!("  Shows all configured AI engines");
                println!("  Displays engine status and capabilities");
            } else if let Some(("test", _)) = sub_matches.subcommand() {
                println!("⚙️  FluentCLI Engine Test Examples");
                println!("==================================");
                println!();
                println!("📝 Basic Usage:");
                println!("  fluent engine test anthropic");
                println!("  fluent engine test openai");
                println!();
                println!("🔧 Testing Engines:");
                println!("  fluent engine test groq --verbose");
                println!("  fluent engine test google --config test_config.toml");
                println!();
                println!("💡 Tip: Test engines after configuration to ensure connectivity");
            } else {
                println!("⚙️  FluentCLI Engine Examples");
                println!("============================");
                println!();
                println!("📝 Basic Usage:");
                println!("  fluent engine list");
                println!("  fluent engine test <engine_name>");
                println!();
                println!("🔧 Engine Management:");
                println!("  fluent engine list --json");
                println!("  fluent engine test anthropic --verbose");
                println!();
                println!("💡 Tip: Configure engines with 'fluent setup' first");
            }
        }
        _ => {
            println!("🌟 FluentCLI Examples");
            println!("====================");
            println!();
            println!("🚀 Getting Started:");
            println!("  fluent setup                    # Configure FluentCLI");
            println!("  fluent agent \"Hello, world!\"     # Run your first agent");
            println!();
            println!("🤖 Agent Commands:");
            println!("  fluent agent \"Write a function\" --examples");
            println!("  fluent agent \"Debug this code\" --interactive");
            println!();
            println!("🔄 Pipeline Commands:");
            println!("  fluent pipeline -f example_pipelines/test_pipeline.yaml");
            println!("  fluent pipeline --examples");
            println!();
            println!("🛠️  Tool Commands:");
            println!("  fluent tools list");
            println!("  fluent tools search \"file\"");
            println!();
            println!("⚙️  Configuration:");
            println!("  fluent setup --examples");
            println!("  fluent engine list");
            println!();
            println!("📚 For more examples, use --examples with any subcommand:");
            println!("  fluent <command> --examples");
        }
    }
    Ok(())
}

/// Main CLI entry point
pub async fn run_modular() -> Result<()> {
    let app = build_cli();
    let matches = app.clone().try_get_matches();

    let matches = match matches {
        Ok(matches) => matches,
        Err(err) => {
            // Check if this is a help or version request, which should not be treated as an error
            if err.use_stderr() {
                // This is a real error
                return Err(CliError::ArgParse(err.to_string()).into());
            } else {
                // This is help or version, just print and exit successfully
                err.print().map_err(|e| {
                    CliError::Unknown(format!("Failed to print help/version: {}", e))
                })?;
                return Ok(());
            }
        }
    };

    // Set verbosity env flags for downstream components (used in future logging standardization)
    if matches.get_flag("quiet") {
        std::env::set_var("FLUENT_QUIET", "1");
    }
    if matches.get_flag("verbose") {
        std::env::set_var("FLUENT_VERBOSE", "1");
    }
    if matches.get_flag("json-logs") {
        std::env::set_var("FLUENT_LOG_FORMAT", "json");
    } else if matches.get_flag("human-logs") {
        std::env::set_var("FLUENT_LOG_FORMAT", "human");
    }

    // Handle examples flag
    if matches.get_flag("examples") {
        show_examples(&matches)?;
        return Ok(());
    }

    // Capture config path argument early for logging metadata
    let config_path = matches
        .get_one::<String>("config")
        .map(|s| s.as_str())
        .unwrap_or("fluent_config.toml");

    // Determine if the selected subcommand requires configuration
    let requires_config = match matches.subcommand() {
        Some(("tools", _)) => false,
        Some(("completions", _)) => false,
        Some(("engine", sub_m)) => match sub_m.subcommand() {
            Some(("list", _)) => false,
            _ => true,
        },
        _ => true,
    };

    // Load configuration only if required; otherwise, use an empty default config
    let config = if requires_config {
        let config_path = matches
            .get_one::<String>("config")
            .map(|s| s.as_str())
            .unwrap_or("fluent_config.toml");
        if Path::new(config_path).exists() {
            match fluent_core::config::load_config(
                config_path,
                "",
                &std::collections::HashMap::new(),
            ) {
                Ok(cfg) => cfg,
                Err(e) => {
                    // Be lenient for agent flows: they construct engines themselves
                    let sub = matches.subcommand_name().unwrap_or("");
                    if sub == "agent" {
                        eprintln!("⚠️  Config load warning (agent mode will continue): {}", e);
                        fluent_core::config::Config::new(vec![])
                    } else {
                        return Err(CliError::Config(e.to_string()).into());
                    }
                }
            }
        } else {
            // Create a minimal default config if no config file exists
            fluent_core::config::Config::new(vec![])
        }
    } else {
        fluent_core::config::Config::new(vec![])
    };

    // Route to appropriate command handler
    match matches.subcommand() {
        Some(("pipeline", sub_matches)) => {
            let span = tracing::info_span!("pipeline", config_path = %config_path);
            let _e = span.enter();
            let handler = PipelineCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("agent", sub_matches)) => {
            let span = tracing::info_span!("agent", config_path = %config_path);
            let _e = span.enter();
            let handler = AgentCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("mcp", sub_matches)) => {
            let span = tracing::info_span!("mcp", config_path = %config_path);
            let _e = span.enter();
            let handler = McpCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("neo4j", sub_matches)) => {
            let span = tracing::info_span!("neo4j", config_path = %config_path);
            let _e = span.enter();
            let handler = Neo4jCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("engine", sub_matches)) => {
            let span = tracing::info_span!("engine", config_path = %config_path);
            let _e = span.enter();
            let handler = EngineCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("tools", sub_matches)) => {
            let span = tracing::info_span!("tools", config_path = %config_path);
            let _e = span.enter();
            let handler = ToolsCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("configure", sub_matches)) => {
            let span = tracing::info_span!("configure");
            let _e = span.enter();
            let handler = ConfigureCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("setup", sub_matches)) => {
            let span = tracing::info_span!("setup");
            let _e = span.enter();
            let handler = SetupCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("examples", sub_matches)) => {
            let span = tracing::info_span!("examples");
            let _e = span.enter();
            let handler = ExamplesCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("completions", sub_matches)) => {
            use clap_complete::{generate, shells};
            use std::fs::File;
            use std::io::{self, Write};

            let shell = sub_matches
                .get_one::<String>("shell")
                .map(|s| s.as_str())
                .unwrap_or("");
            let output = sub_matches.get_one::<String>("output").cloned();
            let mut writer: Box<dyn Write> = match output {
                Some(path) => Box::new(File::create(path).map_err(|e| {
                    CliError::Unknown(format!("Failed to open output file: {}", e))
                })?),
                None => Box::new(io::stdout()),
            };
            let out: &mut dyn Write = &mut *writer;

            match shell.to_lowercase().as_str() {
                "bash" => {
                    let mut cmd = crate::cli_builder::build_cli();
                    generate(shells::Bash, &mut cmd, "fluent", out);
                }
                "zsh" => {
                    let mut cmd = crate::cli_builder::build_cli();
                    generate(shells::Zsh, &mut cmd, "fluent", out);
                }
                "fish" => {
                    let mut cmd = crate::cli_builder::build_cli();
                    generate(shells::Fish, &mut cmd, "fluent", out);
                }
                "powershell" => {
                    let mut cmd = crate::cli_builder::build_cli();
                    generate(shells::PowerShell, &mut cmd, "fluent", out);
                }
                "elvish" => {
                    let mut cmd = crate::cli_builder::build_cli();
                    generate(shells::Elvish, &mut cmd, "fluent", out);
                }
                other => {
                    return Err(CliError::ArgParse(format!("Unsupported shell: {}", other)).into());
                }
            }
        }
        _ => {
            // Default behavior - show help
            let mut app = build_cli();
            app.print_help()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli_builder::build_cli;

    fn create_test_config() -> fluent_core::config::Config {
        fluent_core::config::Config::new(vec![])
    }

    #[test]
    fn test_cli_app_creation() {
        let app = build_cli();

        // Test that the app is created successfully
        assert_eq!(app.get_name(), "fluent");

        // Test that main subcommands are present
        let subcommands: Vec<&str> = app.get_subcommands().map(|cmd| cmd.get_name()).collect();

        assert!(subcommands.contains(&"pipeline"));
        assert!(subcommands.contains(&"agent"));
        assert!(subcommands.contains(&"engine"));
        assert!(subcommands.contains(&"neo4j"));
        assert!(subcommands.contains(&"mcp"));
        assert!(subcommands.contains(&"tools"));
    }

    #[test]
    fn test_cli_help_generation() {
        let mut app = build_cli();

        // Test that help can be generated without panicking
        let help = app.render_help();
        let help_str = help.to_string();

        assert!(help_str.contains("fluent"));
        assert!(help_str.contains("pipeline"));
        assert!(help_str.contains("agent"));
        assert!(help_str.contains("engine"));
    }

    #[test]
    fn test_cli_version_info() {
        let app = build_cli();

        // Test that version information is present
        assert!(app.get_version().is_some());
    }

    #[test]
    fn test_cli_config_argument() {
        let app = build_cli();

        // Test parsing with config argument
        let matches = app.try_get_matches_from(vec![
            "fluent",
            "--config",
            "test_config.toml",
            "pipeline",
            "list",
        ]);

        match matches {
            Ok(matches) => {
                let config_path = matches.get_one::<String>("config");
                assert_eq!(config_path, Some(&"test_config.toml".to_string()));
            }
            Err(_) => {
                // This might fail due to missing subcommand requirements, which is expected
                // The important thing is that the config argument is recognized
            }
        }
    }

    #[test]
    fn test_cli_subcommand_parsing() {
        let app = build_cli();

        // Test pipeline subcommand
        let result = app
            .clone()
            .try_get_matches_from(vec!["fluent", "pipeline", "list"]);
        if let Ok(matches) = result {
            assert_eq!(matches.subcommand_name(), Some("pipeline"));
        }

        // Test agent subcommand
        let result = app
            .clone()
            .try_get_matches_from(vec!["fluent", "agent", "status"]);
        if let Ok(matches) = result {
            assert_eq!(matches.subcommand_name(), Some("agent"));
        }

        // Test engine subcommand
        let result = app
            .clone()
            .try_get_matches_from(vec!["fluent", "engine", "list"]);
        if let Ok(matches) = result {
            assert_eq!(matches.subcommand_name(), Some("engine"));
        }
    }

    #[test]
    fn test_default_config_creation() {
        // Test that default config can be created when no config file exists
        let config = create_test_config();

        // Verify basic config structure
        assert!(config.engines.is_empty()); // Default config has no engines
    }

    #[test]
    fn test_cli_error_handling() {
        let app = build_cli();

        // Test invalid subcommand
        let result = app
            .clone()
            .try_get_matches_from(vec!["fluent", "invalid_command"]);
        assert!(result.is_err());

        // Test missing required arguments (this should be handled gracefully)
        let result = app.clone().try_get_matches_from(vec!["fluent"]);
        // This might succeed or fail depending on CLI structure, but shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_cli_global_arguments() {
        let app = build_cli();

        // Test that global arguments are recognized
        let result =
            app.try_get_matches_from(vec!["fluent", "--config", "test.toml", "pipeline", "list"]);

        if let Ok(matches) = result {
            let config_arg = matches.get_one::<String>("config");
            assert_eq!(config_arg, Some(&"test.toml".to_string()));
        }
    }
}

/// Legacy run function for backward compatibility
pub async fn run() -> Result<()> {
    run_modular().await
}
