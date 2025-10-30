//! Interactive setup wizard for FluentCLI configuration
//!
//! This module provides an interactive setup wizard that guides users
//! through the initial configuration of FluentCLI, including engine
//! selection, API key setup, and basic configuration generation.

use crate::commands::CommandHandler;
use anyhow::{anyhow, Context, Result};
use clap::ArgMatches;
use fluent_core::config::{Config, EngineConfig, ConnectionConfig, Neo4jConfig};
use fluent_core::spinner_configuration::SpinnerConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Serializable wrapper for configuration
#[derive(Serialize, Deserialize)]
struct SerializableConfig {
    engines: Vec<SerializableEngineConfig>,
}

#[derive(Serialize, Deserialize)]
struct SerializableEngineConfig {
    name: String,
    engine: String,
    connection: ConnectionConfig,
    parameters: HashMap<String, Value>,
    session_id: Option<String>,
    neo4j: Option<Neo4jConfig>,
    spinner: Option<SpinnerConfig>,
}

/// Setup command handler
pub struct SetupCommand;

impl SetupCommand {
    /// Create a new setup command handler
    pub fn new() -> Self {
        Self
    }

    /// Detect available API keys from environment variables
    fn detect_api_keys(&self) -> HashMap<String, String> {
        let mut detected_keys = HashMap::new();

        // Common API key patterns
        let patterns = vec![
            ("ANTHROPIC_API_KEY", "anthropic"),
            ("OPENAI_API_KEY", "openai"),
            ("GOOGLE_API_KEY", "google"),
            ("GROQ_API_KEY", "groq"),
            ("COHERE_API_KEY", "cohere"),
            ("ANTHROPIC", "anthropic"),
            ("OPENAI", "openai"),
            ("GOOGLE", "google"),
            ("GROQ", "groq"),
            ("COHERE", "cohere"),
        ];

        for (env_var, provider) in patterns {
            if let Ok(key) = env::var(env_var) {
                if !key.is_empty() && key.len() > 10 { // Basic validation
                    detected_keys.insert(provider.to_string(), key);
                }
            }
        }

        detected_keys
    }

    /// Get user input with optional default
    fn get_input(&self, prompt: &str, default: Option<&str>) -> Result<String> {
        use std::io::{self, Write};

        print!("{}", prompt);
        if let Some(default) = default {
            print!(" [{}]", default);
        }
        print!(": ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            if let Some(default) = default {
                Ok(default.to_string())
            } else {
                self.get_input(prompt, default) // Retry if no default
            }
        } else {
            Ok(input.to_string())
        }
    }

    /// Get yes/no input
    fn get_yes_no(&self, prompt: &str, default: bool) -> Result<bool> {
        let default_str = if default { "Y/n" } else { "y/N" };
        let prompt = format!("{} ({})", prompt, default_str);

        loop {
            let input = self.get_input(&prompt, None)?.to_lowercase();
            match input.as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                "" => return Ok(default),
                _ => println!("Please enter 'y' for yes or 'n' for no."),
            }
        }
    }

    /// Select engine from available options
    fn select_engine(&self, detected_keys: &HashMap<String, String>) -> Result<String> {
        println!("\n🤖 Available AI Engines:");
        println!("  1. Anthropic (Claude) - Recommended for coding tasks");
        println!("  2. OpenAI (GPT) - General purpose AI");
        println!("  3. Google (Gemini) - Google's AI models");
        println!("  4. Groq - Fast inference with Llama models");
        println!("  5. Cohere - Command-focused AI");

        // Show detected keys
        if !detected_keys.is_empty() {
            println!("\n✅ Detected API keys for:");
            for (provider, _) in detected_keys {
                println!("  • {}", provider);
            }
        }

        loop {
            let choice = self.get_input("Select engine (1-5)", Some("1"))?;
            match choice.as_str() {
                "1" => return Ok("anthropic".to_string()),
                "2" => return Ok("openai".to_string()),
                "3" => return Ok("google".to_string()),
                "4" => return Ok("groq".to_string()),
                "5" => return Ok("cohere".to_string()),
                _ => println!("Invalid choice. Please select 1-5."),
            }
        }
    }

    /// Configure engine parameters
    fn configure_engine(&self, engine_name: &str, detected_keys: &HashMap<String, String>) -> Result<EngineConfig> {
        let (hostname, request_path, model_name, system_prompt) = match engine_name {
            "anthropic" => (
                "api.anthropic.com",
                "/v1/messages",
                "claude-3-7-sonnet-20250219",
                "You are an expert Rust programmer and game developer. You create complete, working code with proper error handling."
            ),
            "openai" => (
                "api.openai.com",
                "/v1/chat/completions",
                "gpt-4",
                "You are a helpful AI assistant with expertise in programming and software development."
            ),
            "google" => (
                "generativelanguage.googleapis.com",
                "/v1beta/models/{model}:generateContent",
                "gemini-pro",
                "You are a helpful AI assistant with expertise in programming and software development."
            ),
            "groq" => (
                "api.groq.com",
                "/openai/v1/chat/completions",
                "llama3-70b-8192",
                "You are a helpful AI assistant with expertise in programming and software development."
            ),
            "cohere" => (
                "api.cohere.ai",
                "/v1/generate",
                "command",
                "You are a helpful AI assistant with expertise in programming and software development."
            ),
            _ => return Err(anyhow!("Unknown engine: {}", engine_name)),
        };

        // Get API key
        let api_key = if let Some(key) = detected_keys.get(engine_name) {
            println!("✅ Using detected {} API key", engine_name);
            if self.get_yes_no("Use this detected key?", true)? {
                key.clone()
            } else {
                self.get_input("Enter your API key", None)?
            }
        } else {
            println!("🔑 No {} API key detected in environment.", engine_name);
            self.get_input("Enter your API key", None)?
        };

        // Get custom model if desired
        let use_custom_model = self.get_yes_no("Use custom model name?", false)?;
        let final_model = if use_custom_model {
            self.get_input("Model name", Some(model_name))?
        } else {
            model_name.to_string()
        };

        // Get temperature
        let temperature = self.get_input("Temperature (0.0-1.0)", Some("0.1"))?;
        let temperature: f64 = temperature.parse().context("Invalid temperature value")?;

        Ok(EngineConfig {
            name: engine_name.to_string(),
            engine: engine_name.to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: hostname.to_string(),
                port: 443,
                request_path: request_path.to_string(),
            },
            parameters: {
                let mut params = HashMap::new();
                params.insert("bearer_token".to_string(), Value::String(api_key));
                params.insert("modelName".to_string(), Value::String(final_model));
                params.insert("temperature".to_string(), Value::Number(serde_json::Number::from_f64(temperature).unwrap()));
                params.insert("max_tokens".to_string(), Value::Number(4000.into()));
                params.insert("system".to_string(), Value::String(system_prompt.to_string()));
                params
            },
            session_id: None,
            neo4j: None,
            spinner: None,
        })
    }

    /// Generate configuration file
    fn generate_config(&self, engine_config: EngineConfig, output_path: &str, force: bool) -> Result<()> {
        // Check if file exists
        if Path::new(output_path).exists() && !force {
            if !self.get_yes_no(&format!("Configuration file '{}' already exists. Overwrite?", output_path), false)? {
                println!("Setup cancelled.");
                return Ok(());
            }
        }

        // Convert to serializable format
        let serializable_engine = SerializableEngineConfig {
            name: engine_config.name,
            engine: engine_config.engine,
            connection: engine_config.connection,
            parameters: engine_config.parameters,
            session_id: engine_config.session_id,
            neo4j: engine_config.neo4j,
            spinner: engine_config.spinner,
        };

        let serializable_config = SerializableConfig {
            engines: vec![serializable_engine],
        };

        // Convert to YAML
        let yaml = serde_yaml::to_string(&serializable_config).context("Failed to serialize configuration")?;

        // Write file
        fs::write(output_path, yaml).context("Failed to write configuration file")?;

        println!("✅ Configuration saved to: {}", output_path);
        Ok(())
    }

    /// Validate configuration
    fn validate_config(&self, config_path: &str) -> Result<()> {
        println!("🔍 Validating configuration...");

        match fluent_core::config::load_config(config_path, "", &HashMap::new()) {
            Ok(config) => {
                if config.engines.is_empty() {
                    return Err(anyhow!("No engines configured"));
                }

                println!("✅ Configuration is valid!");
                println!("   • {} engine(s) configured", config.engines.len());
                for engine in &config.engines {
                    println!("   • {} ({})", engine.name, engine.engine);
                }
                Ok(())
            }
            Err(e) => {
                println!("❌ Configuration validation failed: {}", e);
                Err(e)
            }
        }
    }
}

impl CommandHandler for SetupCommand {
    async fn execute(&self, matches: &ArgMatches, _config: &Config) -> Result<()> {
        println!("🚀 Welcome to FluentCLI Setup Wizard!");
        println!("This wizard will help you configure FluentCLI for the first time.\n");

        let output_path = matches.get_one::<String>("output").map(|s| s.as_str()).unwrap_or("fluent_config.toml");
        let force = matches.get_flag("force");
        let skip_validation = matches.get_flag("skip-validation");

        // Detect API keys
        let detected_keys = self.detect_api_keys();

        // Select engine
        let engine_name = self.select_engine(&detected_keys)?;

        // Configure engine
        let engine_config = self.configure_engine(&engine_name, &detected_keys)?;

        // Generate configuration
        self.generate_config(engine_config, output_path, force)?;

        // Validate if requested
        if !skip_validation {
            if let Err(e) = self.validate_config(output_path) {
                println!("⚠️  Configuration generated but validation failed: {}", e);
                println!("   You may need to check your API key and try again.");
            }
        }

        println!("\n🎉 Setup complete! You can now use FluentCLI.");
        println!("   Try: fluent agent \"Hello, world!\"");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_api_keys() {
        let cmd = SetupCommand::new();
        let keys = cmd.detect_api_keys();
        // This will be empty in test environment unless env vars are set
        assert!(keys.is_empty() || !keys.is_empty()); // Just check it's a valid HashMap
    }
}
