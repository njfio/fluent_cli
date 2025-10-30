//! Interactive setup wizard for FluentCLI configuration
//!
//! This module provides an interactive setup wizard that guides users through
//! the initial configuration process, auto-detects API keys, and generates
//! optimized configuration files.

use crate::error::CliError;
use anyhow::{anyhow, Result};
use clap::ArgMatches;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use fluent_core::config::Config;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use super::CommandHandler;

/// Setup command handler for interactive configuration
pub struct SetupCommand;

impl SetupCommand {
    pub fn new() -> Self {
        Self
    }

    /// Auto-detect available API keys from environment
    fn detect_api_keys() -> HashMap<String, String> {
        let mut detected = HashMap::new();

        // Common API key patterns
        let patterns = vec![
            ("ANTHROPIC_API_KEY", "anthropic"),
            ("OPENAI_API_KEY", "openai"),
            ("GOOGLE_API_KEY", "google"),
            ("GOOGLE_AI_API_KEY", "google"),
            ("GEMINI_API_KEY", "google"),
            ("AZURE_OPENAI_API_KEY", "azure"),
            ("AZURE_OPENAI_ENDPOINT", "azure"),
        ];

        for (env_var, provider) in patterns {
            if let Ok(value) = env::var(env_var) {
                if !value.is_empty() {
                    detected.insert(provider.to_string(), env_var.to_string());
                }
            }
        }

        detected
    }

    /// Detect and suggest engine based on available API keys
    fn suggest_engine(api_keys: &HashMap<String, String>) -> Option<String> {
        // Priority: Anthropic > OpenAI > Google
        if api_keys.contains_key("anthropic") {
            Some("anthropic".to_string())
        } else if api_keys.contains_key("openai") {
            Some("openai".to_string())
        } else if api_keys.contains_key("google") {
            Some("google".to_string())
        } else {
            None
        }
    }

    /// Get engine configuration template
    fn get_engine_config_template(engine_name: &str) -> Result<String> {
        match engine_name {
            "anthropic" => Ok(format!(
                r#"[[engines]]
name = "anthropic"
engine = "anthropic"

[engines.connection]
protocol = "https"
hostname = "api.anthropic.com"
port = 443
request_path = "/v1/messages"

[engines.parameters]
bearer_token = "${{ANTHROPIC_API_KEY}}"
modelName = "claude-3-7-sonnet-20250219"
temperature = 0.1
max_tokens = 4000
system = "You are a helpful AI assistant.""#
            )),
            "openai" => Ok(format!(
                r#"[[engines]]
name = "openai"
engine = "openai"

[engines.connection]
protocol = "https"
hostname = "api.openai.com"
port = 443
request_path = "/v1/chat/completions"

[engines.parameters]
bearer_token = "${{OPENAI_API_KEY}}"
modelName = "gpt-4o"
temperature = 0.1
max_tokens = 4000
system = "You are a helpful AI assistant.""#
            )),
            "google" => Ok(format!(
                r#"[[engines]]
name = "google"
engine = "google"

[engines.connection]
protocol = "https"
hostname = "generativelanguage.googleapis.com"
port = 443
request_path = "/v1beta/models/gemini-pro:generateContent"

[engines.parameters]
bearer_token = "${{GOOGLE_API_KEY}}"
modelName = "gemini-pro"
temperature = 0.1
max_tokens = 4000
system = "You are a helpful AI assistant.""#
            )),
            _ => Err(anyhow!("Unknown engine: {}", engine_name)),
        }
    }

    /// Validate configuration file
    fn validate_config(config_path: &str) -> Result<()> {
        if !Path::new(config_path).exists() {
            return Err(anyhow!("Configuration file does not exist: {}", config_path));
        }

        // Try to load the config to validate it
        match fluent_core::config::load_config(config_path, "", &HashMap::new()) {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Configuration validation failed: {}", e)),
        }
    }

    /// Run interactive setup wizard
    async fn run_wizard(config_path: &str) -> Result<()> {
        let theme = ColorfulTheme::default();

        println!("\n🚀 Welcome to FluentCLI Setup Wizard!\n");
        println!("This wizard will guide you through configuring FluentCLI.");
        println!("Press Ctrl+C at any time to cancel.\n");

        // Step 1: Check if config already exists
        if Path::new(config_path).exists() {
            let overwrite = Confirm::with_theme(&theme)
                .with_prompt(format!(
                    "Configuration file '{}' already exists. Overwrite?",
                    config_path
                ))
                .default(false)
                .interact()?;

            if !overwrite {
                println!("Setup cancelled. Your existing configuration is unchanged.");
                return Ok(());
            }
        }

        // Step 2: Detect API keys
        println!("🔍 Detecting API keys from environment...\n");
        let detected_keys = Self::detect_api_keys();

        if detected_keys.is_empty() {
            println!("⚠️  No API keys detected in environment variables.");
            println!("\nCommon API key environment variables:");
            println!("  - ANTHROPIC_API_KEY (for Anthropic Claude)");
            println!("  - OPENAI_API_KEY (for OpenAI GPT)");
            println!("  - GOOGLE_API_KEY or GEMINI_API_KEY (for Google Gemini)");
            println!("\nYou can set these before running setup, or configure them manually later.");
            println!("\n⚠️  Note: Without API keys, some features may not work.");
        } else {
            println!("✅ Detected {} API key(s):", detected_keys.len());
            for (provider, env_var) in &detected_keys {
                println!("  - {}: {}", provider, env_var);
            }
        }

        // Step 3: Select engine
        println!("\n📦 Select an AI engine to configure:\n");
        let engine_options = vec![
            "Anthropic Claude (claude-3-7-sonnet-20250219)",
            "OpenAI GPT-4 (gpt-4o)",
            "Google Gemini (gemini-pro)",
            "Skip engine configuration",
        ];

        let default_index = Self::suggest_engine(&detected_keys)
            .and_then(|e| {
                engine_options.iter().position(|opt| {
                    opt.to_lowercase().contains(&e) || (e == "anthropic" && opt.contains("Anthropic"))
                        || (e == "openai" && opt.contains("OpenAI"))
                        || (e == "google" && opt.contains("Google"))
                })
            })
            .unwrap_or(0);

        let engine_selection = Select::with_theme(&theme)
            .with_prompt("Select engine")
            .default(default_index)
            .items(&engine_options)
            .interact()?;

        if engine_selection == engine_options.len() - 1 {
            println!("Skipping engine configuration.");
            println!("Setup complete. You can configure engines manually later.");
            return Ok(());
        }

        let engine_name = match engine_selection {
            0 => "anthropic",
            1 => "openai",
            2 => "google",
            _ => return Err(anyhow!("Invalid selection")),
        };

        // Step 4: Verify API key availability
        let required_key = match engine_name {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "google" => {
                if env::var("GOOGLE_API_KEY").is_ok() {
                    "GOOGLE_API_KEY"
                } else {
                    "GEMINI_API_KEY"
                }
            }
            _ => return Err(anyhow!("Unknown engine")),
        };

        if env::var(required_key).is_err() {
            println!("\n⚠️  Warning: {} is not set in your environment.", required_key);
            let continue_anyway = Confirm::with_theme(&theme)
                .with_prompt("Continue anyway? (You'll need to set it later)")
                .default(false)
                .interact()?;

            if !continue_anyway {
                println!("Setup cancelled. Please set {} and run setup again.", required_key);
                return Ok(());
            }
        }

        // Step 5: Generate configuration
        println!("\n📝 Generating configuration file...\n");
        let config_content = Self::get_engine_config_template(engine_name)?;

        // Step 6: Preview configuration
        println!("Generated configuration:\n");
        println!("{}", config_content);
        println!();

        let confirm = Confirm::with_theme(&theme)
            .with_prompt("Save this configuration?")
            .default(true)
            .interact()?;

        if !confirm {
            println!("Setup cancelled. Configuration not saved.");
            return Ok(());
        }

        // Step 7: Write configuration file
        fs::write(config_path, config_content)?;
        println!("\n✅ Configuration saved to: {}", config_path);

        // Step 8: Validate configuration
        println!("\n🔍 Validating configuration...");
        match Self::validate_config(config_path) {
            Ok(_) => {
                println!("✅ Configuration is valid!");
            }
            Err(e) => {
                println!("⚠️  Configuration validation warning: {}", e);
                println!("The file was saved but may need manual adjustment.");
            }
        }

        // Step 9: Summary
        println!("\n🎉 Setup complete!\n");
        println!("Your configuration is ready. You can now:");
        println!("  - Run: fluent engine list");
        println!("  - Test: fluent engine test <engine-name>");
        println!("  - Start using: fluent agent --goal \"Your goal here\"");
        println!("\nFor more information, run: fluent --help\n");

        Ok(())
    }
}

impl CommandHandler for SetupCommand {
    async fn execute(&self, matches: &ArgMatches, _config: &Config) -> Result<()> {
        let config_path = matches
            .get_one::<String>("config-path")
            .map(|s| s.as_str())
            .unwrap_or("fluent_config.toml");

        Self::run_wizard(config_path).await
    }
}

impl Default for SetupCommand {
    fn default() -> Self {
        Self::new()
    }
}

