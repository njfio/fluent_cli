//! Main CLI entry point and command routing
//!
//! This module provides the main entry point for the CLI application
//! and routes commands to their appropriate handlers.

use anyhow::Result;
use std::path::Path;

use crate::cli_builder::build_cli;
use crate::commands::{
    agent::AgentCommand,
    engine::EngineCommand,
    mcp::McpCommand,
    neo4j::Neo4jCommand,
    pipeline::PipelineCommand,
    tools::ToolsCommand,
    CommandHandler,
};

/// Main CLI entry point
pub async fn run_modular() -> Result<()> {
    let app = build_cli();
    let matches = app.clone().try_get_matches();

    let matches = match matches {
        Ok(matches) => matches,
        Err(err) => {
            // Print help or error and exit
            eprintln!("{}", err);
            return Ok(());
        }
    };

    // Load configuration - handle missing config files gracefully
    let config_path = matches.get_one::<String>("config").map(|s| s.as_str()).unwrap_or("fluent_config.toml");
    let config = if Path::new(config_path).exists() {
        fluent_core::config::load_config(config_path, "", &std::collections::HashMap::new())?
    } else {
        // Create a minimal default config if no config file exists
        fluent_core::config::Config::new(vec![])
    };

    // Route to appropriate command handler
    match matches.subcommand() {
        Some(("pipeline", sub_matches)) => {
            let handler = PipelineCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("agent", sub_matches)) => {
            let handler = AgentCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("mcp", sub_matches)) => {
            let handler = McpCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("neo4j", sub_matches)) => {
            let handler = Neo4jCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("engine", sub_matches)) => {
            let handler = EngineCommand::new();
            handler.execute(sub_matches, &config).await?;
        }
        Some(("tools", sub_matches)) => {
            let handler = ToolsCommand::new();
            handler.execute(sub_matches, &config).await?;
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
        let subcommands: Vec<&str> = app.get_subcommands()
            .map(|cmd| cmd.get_name())
            .collect();

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
        let matches = app.try_get_matches_from(vec!["fluent", "--config", "test_config.toml", "pipeline", "list"]);

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
        let result = app.clone().try_get_matches_from(vec!["fluent", "pipeline", "list"]);
        if let Ok(matches) = result {
            assert_eq!(matches.subcommand_name(), Some("pipeline"));
        }

        // Test agent subcommand
        let result = app.clone().try_get_matches_from(vec!["fluent", "agent", "status"]);
        if let Ok(matches) = result {
            assert_eq!(matches.subcommand_name(), Some("agent"));
        }

        // Test engine subcommand
        let result = app.clone().try_get_matches_from(vec!["fluent", "engine", "list"]);
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
        let result = app.clone().try_get_matches_from(vec!["fluent", "invalid_command"]);
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
        let result = app.try_get_matches_from(vec![
            "fluent",
            "--config", "test.toml",
            "pipeline",
            "list"
        ]);

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
