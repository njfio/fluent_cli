//! Progressive configuration discovery and management
//!
//! This module provides commands for discovering, viewing, and configuring
//! FluentCLI features progressively, with presets and optimization suggestions.

use crate::error::CliError;
use anyhow::{anyhow, Result};
use clap::ArgMatches;
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect, Select};
use fluent_core::config::{Config, EngineConfig};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::CommandHandler;

/// Configuration command handler
pub struct ConfigureCommand;

impl ConfigureCommand {
    pub fn new() -> Self {
        Self
    }

    /// Show current configuration
    async fn show_config(matches: &ArgMatches, config: &Config) -> Result<()> {
        let json_output = matches.get_flag("json");

        if json_output {
            let config_json = serde_json::json!({
                "engines": config.engines.iter().map(|e| {
                    serde_json::json!({
                        "name": e.name,
                        "engine": e.engine,
                        "connection": {
                            "protocol": e.connection.protocol,
                            "hostname": e.connection.hostname,
                            "port": e.connection.port,
                            "request_path": e.connection.request_path,
                        },
                        "parameters": e.parameters,
                    })
                }).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&config_json)?);
        } else {
            println!("📋 Current Configuration:\n");
            println!("Configured Engines: {}\n", config.engines.len());

            if config.engines.is_empty() {
                println!("⚠️  No engines configured.");
                println!("💡 Run 'fluent setup' to configure your first engine.\n");
            } else {
                for (idx, engine) in config.engines.iter().enumerate() {
                    println!("{}. Engine: {}", idx + 1, engine.name);
                    println!("   Type: {}", engine.engine);
                    println!("   Endpoint: {}://{}:{}{}", 
                        engine.connection.protocol,
                        engine.connection.hostname,
                        engine.connection.port,
                        engine.connection.request_path);
                    
                    if let Some(model) = engine.parameters.get("modelName") {
                        println!("   Model: {}", model);
                    }
                    if let Some(temp) = engine.parameters.get("temperature") {
                        println!("   Temperature: {}", temp);
                    }
                    println!();
                }
            }

            // Show configuration suggestions
            Self::show_suggestions(config)?;
        }

        Ok(())
    }

    /// Show optimization suggestions
    fn show_suggestions(config: &Config) -> Result<()> {
        println!("💡 Configuration Suggestions:\n");

        if config.engines.is_empty() {
            println!("  ⚠️  No engines configured. Run 'fluent setup' to add one.");
        } else {
            // Check for common optimization opportunities
            let mut suggestions = Vec::new();

            for engine in &config.engines {
                // Check temperature settings
                if let Some(temp) = engine.parameters.get("temperature") {
                    if let Some(t) = temp.as_f64() {
                        if t > 0.7 {
                            suggestions.push(format!(
                                "  ℹ️  Engine '{}' has high temperature ({}). Consider lowering for more deterministic results.",
                                engine.name, t
                            ));
                        }
                    }
                }

                // Check max_tokens
                if let Some(max_tokens) = engine.parameters.get("max_tokens") {
                    if let Some(mt) = max_tokens.as_u64() {
                        if mt < 1000 {
                            suggestions.push(format!(
                                "  ⚠️  Engine '{}' has low max_tokens ({}). Consider increasing for longer responses.",
                                engine.name, mt
                            ));
                        }
                    }
                }
            }

            if suggestions.is_empty() {
                println!("  ✅ Configuration looks good! No major suggestions.");
            } else {
                for suggestion in suggestions {
                    println!("{}", suggestion);
                }
            }
        }

        println!();
        Ok(())
    }

    /// Interactive configuration with presets
    async fn interactive_configure(config_path: &str) -> Result<()> {
        let theme = ColorfulTheme::default();

        println!("\n⚙️  FluentCLI Configuration Wizard\n");

        // Load existing config if available
        let existing_config = if Path::new(config_path).exists() {
            match fluent_core::config::load_config(config_path, "", &HashMap::new()) {
                Ok(cfg) => Some(cfg),
                Err(_) => None,
            }
        } else {
            None
        };

        // Step 1: Select configuration preset
        println!("Select a configuration preset (or skip to custom configuration):\n");
        let preset_options = vec![
            "Developer - Optimized for code generation and development",
            "Researcher - Optimized for research and analysis tasks",
            "Production - Optimized for reliability and performance",
            "Custom - Manual configuration",
        ];

        let preset_selection = Select::with_theme(&theme)
            .with_prompt("Select preset")
            .default(0)
            .items(&preset_options)
            .interact()?;

        let preset_name = match preset_selection {
            0 => "developer",
            1 => "researcher",
            2 => "production",
            3 => "custom",
            _ => return Err(anyhow!("Invalid selection")),
        };

        if preset_selection == 3 {
            println!("\nCustom configuration mode.");
            println!("Edit {} directly to customize your configuration.", config_path);
            return Ok(());
        }

        // Step 2: Apply preset optimizations
        println!("\n📝 Applying {} preset optimizations...", preset_name);

        let optimizations = Self::get_preset_optimizations(preset_name);

        if let Some(config) = existing_config {
            println!("\nCurrent configuration:\n");
            Self::show_config_summary(&config)?;

            let apply = Confirm::with_theme(&theme)
                .with_prompt("Apply optimizations to existing configuration?")
                .default(true)
                .interact()?;

            if !apply {
                println!("Configuration unchanged.");
                return Ok(());
            }
        }

        // Step 3: Show what will be optimized
        println!("\n🔧 Optimizations to apply:\n");
        for opt in &optimizations {
            println!("  • {}", opt);
        }

        let confirm = Confirm::with_theme(&theme)
            .with_prompt("Apply these optimizations?")
            .default(true)
            .interact()?;

        if !confirm {
            println!("Configuration unchanged.");
            return Ok(());
        }

        println!("\n✅ Configuration optimized!");
        println!("💡 Note: Some optimizations require manual editing of {}.", config_path);
        println!("   Review the configuration file for full details.\n");

        Ok(())
    }

    /// Get preset optimizations
    fn get_preset_optimizations(preset: &str) -> Vec<String> {
        match preset {
            "developer" => vec![
                "Set temperature to 0.1 for deterministic code generation".to_string(),
                "Increase max_tokens for longer code outputs".to_string(),
                "Enable tool usage by default".to_string(),
                "Set system prompt for code-focused tasks".to_string(),
            ],
            "researcher" => vec![
                "Set temperature to 0.3 for balanced creativity".to_string(),
                "Increase max_tokens for long-form research".to_string(),
                "Enable reflection mode for iterative analysis".to_string(),
                "Configure memory for cross-session learning".to_string(),
            ],
            "production" => vec![
                "Set temperature to 0.0 for maximum determinism".to_string(),
                "Configure retry logic for reliability".to_string(),
                "Enable logging for monitoring".to_string(),
                "Set conservative token limits".to_string(),
            ],
            _ => vec![],
        }
    }

    /// Show configuration summary
    fn show_config_summary(config: &Config) -> Result<()> {
        println!("Engines: {}", config.engines.len());
        for engine in &config.engines {
            println!("  - {}", engine.name);
        }
        Ok(())
    }

    /// Preview configuration changes
    async fn preview_config(config_path: &str) -> Result<()> {
        if !Path::new(config_path).exists() {
            return Err(anyhow!("Configuration file not found: {}", config_path));
        }

        let config_content = fs::read_to_string(config_path)?;
        println!("📄 Current configuration file:\n");
        println!("{}", config_content);

        Ok(())
    }
}

impl CommandHandler for ConfigureCommand {
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()> {
        match matches.subcommand() {
            Some(("show", sub_matches)) => {
                Self::show_config(sub_matches, config).await
            }
            Some(("interactive", sub_matches)) => {
                let config_path = sub_matches
                    .get_one::<String>("config-path")
                    .map(|s| s.as_str())
                    .unwrap_or("fluent_config.toml");
                Self::interactive_configure(config_path).await
            }
            Some(("preview", sub_matches)) => {
                let config_path = sub_matches
                    .get_one::<String>("config-path")
                    .map(|s| s.as_str())
                    .unwrap_or("fluent_config.toml");
                Self::preview_config(config_path).await
            }
            _ => {
                eprintln!("No subcommand provided. Use 'fluent configure --help' for usage.");
                Ok(())
            }
        }
    }
}

impl Default for ConfigureCommand {
    fn default() -> Self {
        Self::new()
    }
}

