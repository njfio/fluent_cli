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

/// Legacy run function for backward compatibility
pub async fn run() -> Result<()> {
    run_modular().await
}
