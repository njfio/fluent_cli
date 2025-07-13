use crate::enhanced_config::{ConfigManager, EnhancedEngineConfig, EnvironmentOverrides};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

/// CLI tool for managing Fluent engine configurations
#[derive(Parser)]
#[command(name = "fluent-config")]
#[command(about = "A CLI tool for managing Fluent engine configurations")]
pub struct ConfigCli {
    /// Configuration directory
    #[arg(short, long, default_value = "./configs")]
    config_dir: PathBuf,

    /// Environment to use
    #[arg(short, long, default_value = "development")]
    environment: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new configuration
    Create {
        /// Engine type (openai, anthropic, google_gemini, etc.)
        engine_type: String,
        /// Configuration name
        name: String,
        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all configurations
    List,
    /// Show configuration details
    Show {
        /// Configuration name
        name: String,
    },
    /// Validate a configuration
    Validate {
        /// Configuration name
        name: String,
    },
    /// Update configuration parameters
    Update {
        /// Configuration name
        name: String,
        /// Parameter updates in KEY=VALUE format
        #[arg(short, long)]
        set: Vec<String>,
    },
    /// Add environment-specific overrides
    Environment {
        /// Configuration name
        name: String,
        /// Environment name
        env: String,
        /// Parameter overrides in KEY=VALUE format
        #[arg(short, long)]
        set: Vec<String>,
    },
    /// Export configuration to JSON
    Export {
        /// Configuration name
        name: String,
        /// Output file (optional, defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import configuration from JSON
    Import {
        /// Input file
        input: PathBuf,
        /// Configuration name (optional, uses name from file)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Copy configuration
    Copy {
        /// Source configuration name
        from: String,
        /// Target configuration name
        to: String,
    },
    /// Delete configuration
    Delete {
        /// Configuration name
        name: String,
        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

impl ConfigCli {
    /// Run the CLI application
    pub async fn run() -> Result<()> {
        let cli = ConfigCli::parse();

        // Set environment variable for the config manager
        std::env::set_var("FLUENT_ENV", &cli.environment);

        let manager = ConfigManager::new(cli.config_dir.clone());

        // Ensure config directory exists
        tokio::fs::create_dir_all(&cli.config_dir).await?;

        match cli.command {
            Commands::Create {
                engine_type,
                name,
                description,
            } => Self::create_config(&manager, &engine_type, &name, description.as_deref()).await,
            Commands::List => Self::list_configs(&manager).await,
            Commands::Show { name } => Self::show_config(&manager, &name).await,
            Commands::Validate { name } => Self::validate_config(&manager, &name).await,
            Commands::Update { name, set } => Self::update_config(&manager, &name, set).await,
            Commands::Environment { name, env, set } => {
                Self::add_environment(&manager, &name, &env, set).await
            }
            Commands::Export { name, output } => {
                Self::export_config(&manager, &name, output.as_deref()).await
            }
            Commands::Import { input, name } => {
                Self::import_config(&manager, &input, name.as_deref()).await
            }
            Commands::Copy { from, to } => Self::copy_config(&manager, &from, &to).await,
            Commands::Delete { name, force } => Self::delete_config(&manager, &name, force).await,
        }
    }

    async fn create_config(
        manager: &ConfigManager,
        engine_type: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<()> {
        let mut config = ConfigManager::create_default_config(engine_type, name);

        if let Some(desc) = description {
            config.metadata.description = Some(desc.to_string());
        }

        manager.save_config(name, &config).await?;
        println!(
            "✅ Created configuration '{}' for engine type '{}'",
            name, engine_type
        );

        // Show next steps
        println!("\n📝 Next steps:");
        println!(
            "  1. Add required parameters: fluent-config update {} --set bearer_token=YOUR_TOKEN",
            name
        );
        println!(
            "  2. Validate configuration: fluent-config validate {}",
            name
        );
        println!("  3. View configuration: fluent-config show {}", name);

        Ok(())
    }

    async fn list_configs(manager: &ConfigManager) -> Result<()> {
        let configs = manager.list_configs().await?;

        if configs.is_empty() {
            println!("No configurations found.");
            return Ok(());
        }

        println!("📋 Available configurations:");
        for config_name in configs {
            match manager.get_metadata(&config_name).await {
                Ok(metadata) => {
                    println!(
                        "  • {} ({})",
                        config_name,
                        metadata
                            .description
                            .unwrap_or_else(|| "No description".to_string())
                    );
                    println!(
                        "    Version: {}, Updated: {}",
                        metadata.version, metadata.updated_at
                    );
                }
                Err(_) => {
                    println!("  • {} (metadata unavailable)", config_name);
                }
            }
        }

        Ok(())
    }

    async fn show_config(manager: &ConfigManager, name: &str) -> Result<()> {
        let config = manager.load_config(name).await?;
        let metadata = manager.get_metadata(name).await?;

        println!("🔧 Configuration: {}", name);
        println!("Engine: {}", config.engine);
        println!(
            "Description: {}",
            metadata
                .description
                .unwrap_or_else(|| "No description".to_string())
        );
        println!("Version: {}", metadata.version);
        println!("Updated: {}", metadata.updated_at);

        println!("\n🔗 Connection:");
        println!("  Protocol: {}", config.connection.protocol);
        println!("  Hostname: {}", config.connection.hostname);
        println!("  Port: {}", config.connection.port);
        println!("  Path: {}", config.connection.request_path);

        println!("\n⚙️  Parameters:");
        for (key, value) in &config.parameters {
            if key.to_lowercase().contains("token") || key.to_lowercase().contains("key") {
                println!("  {}: [REDACTED]", key);
            } else {
                println!("  {}: {}", key, value);
            }
        }

        if let Some(neo4j) = &config.neo4j {
            println!("\n🗄️  Neo4j:");
            println!("  URI: {}", neo4j.uri);
            println!("  Database: {}", neo4j.database);
            println!("  User: {}", neo4j.user);
        }

        Ok(())
    }

    async fn validate_config(manager: &ConfigManager, name: &str) -> Result<()> {
        println!("🔍 Validating configuration '{}'...", name);

        // Load the enhanced config for validation
        let config_path = manager.config_dir.join(format!("{}.json", name));
        let content = tokio::fs::read_to_string(&config_path).await?;
        let enhanced_config: EnhancedEngineConfig = serde_json::from_str(&content)?;

        match manager.validate_config(&enhanced_config).await {
            Ok(()) => {
                println!("✅ Configuration '{}' is valid!", name);
            }
            Err(e) => {
                println!("❌ Configuration '{}' is invalid: {}", name, e);
                return Err(e);
            }
        }

        Ok(())
    }

    async fn update_config(
        manager: &ConfigManager,
        name: &str,
        updates: Vec<String>,
    ) -> Result<()> {
        let mut parameter_updates = HashMap::new();

        for update in updates {
            if let Some((key, value_str)) = fluent_core::config::parse_key_value_pair(&update) {
                let value = Self::parse_value(&value_str)?;
                parameter_updates.insert(key, value);
            } else {
                return Err(anyhow!(
                    "Invalid update format: '{}'. Use KEY=VALUE",
                    update
                ));
            }
        }

        manager.update_parameters(name, parameter_updates).await?;
        println!("✅ Updated configuration '{}'", name);

        Ok(())
    }

    async fn add_environment(
        manager: &ConfigManager,
        name: &str,
        env: &str,
        overrides: Vec<String>,
    ) -> Result<()> {
        // Load existing config
        let config_path = manager.config_dir.join(format!("{}.json", name));
        let content = tokio::fs::read_to_string(&config_path).await?;
        let mut enhanced_config: EnhancedEngineConfig = serde_json::from_str(&content)?;

        // Parse overrides
        let mut parameters = HashMap::new();
        for override_str in overrides {
            let parts: Vec<&str> = override_str.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid override format: '{}'. Use KEY=VALUE",
                    override_str
                ));
            }

            let key = parts[0].to_string();
            let value = Self::parse_value(parts[1])?;
            parameters.insert(key, value);
        }

        // Add environment override
        let env_override = EnvironmentOverrides {
            parameters,
            connection: None,
            neo4j: None,
        };

        enhanced_config
            .environments
            .insert(env.to_string(), env_override);
        manager.save_config(name, &enhanced_config).await?;

        println!(
            "✅ Added environment '{}' overrides to configuration '{}'",
            env, name
        );

        Ok(())
    }

    async fn export_config(
        manager: &ConfigManager,
        name: &str,
        output: Option<&std::path::Path>,
    ) -> Result<()> {
        let config_path = manager.config_dir.join(format!("{}.json", name));
        let content = tokio::fs::read_to_string(&config_path).await?;

        match output {
            Some(path) => {
                tokio::fs::write(path, &content).await?;
                println!("✅ Exported configuration '{}' to {}", name, path.display());
            }
            None => {
                println!("{}", content);
            }
        }

        Ok(())
    }

    async fn import_config(
        manager: &ConfigManager,
        input: &std::path::Path,
        name: Option<&str>,
    ) -> Result<()> {
        let content = tokio::fs::read_to_string(input).await?;
        let config: EnhancedEngineConfig = serde_json::from_str(&content)?;

        let config_name = name.unwrap_or(&config.base.name);
        manager.save_config(config_name, &config).await?;

        println!(
            "✅ Imported configuration '{}' from {}",
            config_name,
            input.display()
        );

        Ok(())
    }

    async fn copy_config(manager: &ConfigManager, from: &str, to: &str) -> Result<()> {
        let config_path = manager.config_dir.join(format!("{}.json", from));
        let content = tokio::fs::read_to_string(&config_path).await?;
        let mut config: EnhancedEngineConfig = serde_json::from_str(&content)?;

        // Update name and metadata
        config.base.name = to.to_string();
        config.metadata.created_at = chrono::Utc::now().to_rfc3339();
        config.metadata.updated_at = chrono::Utc::now().to_rfc3339();
        config.metadata.description = Some(format!("Copy of {}", from));

        manager.save_config(to, &config).await?;
        println!("✅ Copied configuration '{}' to '{}'", from, to);

        Ok(())
    }

    async fn delete_config(manager: &ConfigManager, name: &str, force: bool) -> Result<()> {
        if !force {
            println!("⚠️  This will permanently delete configuration '{}'", name);
            println!("Use --force to confirm deletion");
            return Ok(());
        }

        let config_path = manager.config_dir.join(format!("{}.json", name));
        tokio::fs::remove_file(&config_path).await?;

        println!("✅ Deleted configuration '{}'", name);

        Ok(())
    }

    fn parse_value(value_str: &str) -> Result<Value> {
        // Try to parse as different types
        if let Ok(bool_val) = value_str.parse::<bool>() {
            return Ok(Value::Bool(bool_val));
        }

        if let Ok(int_val) = value_str.parse::<i64>() {
            return Ok(Value::Number(serde_json::Number::from(int_val)));
        }

        if let Ok(float_val) = value_str.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(float_val) {
                return Ok(Value::Number(num));
            }
        }

        // Default to string
        Ok(Value::String(value_str.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value() {
        assert_eq!(ConfigCli::parse_value("true").unwrap(), Value::Bool(true));
        assert_eq!(
            ConfigCli::parse_value("42").unwrap(),
            Value::Number(serde_json::Number::from(42))
        );
        assert_eq!(
            ConfigCli::parse_value("3.14").unwrap(),
            Value::Number(serde_json::Number::from_f64(3.14).unwrap())
        );
        assert_eq!(
            ConfigCli::parse_value("hello").unwrap(),
            Value::String("hello".to_string())
        );
    }
}
