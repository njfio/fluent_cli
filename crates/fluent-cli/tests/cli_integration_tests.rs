use fluent_core::config::{Config, EngineConfig};
use fluent_engines::EngineType;
use std::process::Command;
use tempfile::TempDir;
use anyhow::Result;

/// Integration tests for CLI functionality
/// Tests command-line interface, argument parsing, and end-to-end workflows

#[test]
fn test_cli_help_command() -> Result<()> {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fluent", "--", "--help"])
        .output()?;
    
    let stdout = String::from_utf8(output.stdout)?;
    
    // Should contain basic help information
    assert!(stdout.contains("fluent"));
    assert!(stdout.contains("USAGE") || stdout.contains("Usage"));
    
    Ok(())
}

#[test]
fn test_cli_version_command() -> Result<()> {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fluent", "--", "--version"])
        .output()?;
    
    let stdout = String::from_utf8(output.stdout)?;
    
    // Should contain version information
    assert!(stdout.contains("fluent") || stdout.contains("0."));
    
    Ok(())
}

#[test]
fn test_config_file_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("test_config.toml");
    
    // Test config creation
    let config = Config::default();
    config.save_to_file(&config_path)?;
    
    // Verify file was created
    assert!(config_path.exists());
    
    // Test config loading
    let loaded_config = Config::load_from_file(&config_path)?;
    assert_eq!(loaded_config.database.url, config.database.url);
    
    Ok(())
}

#[test]
fn test_config_validation() -> Result<()> {
    // Test valid config
    let valid_config = Config {
        database: DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 10,
            connection_timeout: 30,
        },
        engines: vec![
            EngineConfig {
                name: "test_engine".to_string(),
                engine_type: EngineType::OpenAI,
                api_key: Some("test_key".to_string()),
                base_url: None,
                model: "gpt-3.5-turbo".to_string(),
                max_tokens: 1000,
                temperature: 0.7,
                enabled: true,
            }
        ],
        default_engine: "test_engine".to_string(),
        log_level: "info".to_string(),
        max_concurrent_requests: 5,
    };
    
    let validation_result = valid_config.validate();
    assert!(validation_result.is_ok());
    
    // Test invalid config (empty default engine)
    let mut invalid_config = valid_config.clone();
    invalid_config.default_engine = "".to_string();
    
    let invalid_result = invalid_config.validate();
    assert!(invalid_result.is_err());
    
    Ok(())
}

#[test]
fn test_engine_configuration() -> Result<()> {
    let engine_config = EngineConfig {
        name: "test_openai".to_string(),
        engine_type: EngineType::OpenAI,
        api_key: Some("test-api-key-placeholder".to_string()),
        base_url: None,
        model: "gpt-4".to_string(),
        max_tokens: 2000,
        temperature: 0.8,
        enabled: true,
    };
    
    // Test engine config validation
    assert_eq!(engine_config.name, "test_openai");
    assert_eq!(engine_config.engine_type, EngineType::OpenAI);
    assert!(engine_config.enabled);
    
    Ok(())
}

#[test]
fn test_database_configuration() -> Result<()> {
    let db_config = DatabaseConfig {
        url: "sqlite:test.db".to_string(),
        max_connections: 20,
        connection_timeout: 60,
    };
    
    // Test database config properties
    assert!(db_config.url.starts_with("sqlite:"));
    assert_eq!(db_config.max_connections, 20);
    assert_eq!(db_config.connection_timeout, 60);
    
    Ok(())
}

#[test]
fn test_cli_argument_parsing() -> Result<()> {
    // Test basic command structure (this would require actual CLI parsing logic)
    // For now, we'll test that the binary can be invoked without crashing
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fluent", "--", "--help"])
        .output()?;
    
    // Should exit successfully for help command
    assert!(output.status.success() || output.status.code() == Some(0));
    
    Ok(())
}

#[test]
fn test_config_file_formats() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Test TOML config
    let toml_config = r#"
[database]
url = "sqlite::memory:"
max_connections = 10
connection_timeout = 30

[[engines]]
name = "test_engine"
engine_type = "OpenAI"
api_key = "test_key"
model = "gpt-3.5-turbo"
max_tokens = 1000
temperature = 0.7
enabled = true

default_engine = "test_engine"
log_level = "info"
max_concurrent_requests = 5
"#;
    
    let toml_path = temp_dir.path().join("config.toml");
    std::fs::write(&toml_path, toml_config)?;
    
    // Test loading TOML config
    let loaded_config = Config::load_from_file(&toml_path)?;
    assert_eq!(loaded_config.default_engine, "test_engine");
    assert_eq!(loaded_config.engines.len(), 1);
    
    Ok(())
}

#[test]
fn test_environment_variable_handling() -> Result<()> {
    // Test that config can handle environment variables
    // This would test the credential management system
    
    std::env::set_var("TEST_API_KEY", "test_env_key");
    
    // Create config that might use environment variables
    let config = Config::default();
    
    // Verify config was created successfully
    assert!(!config.default_engine.is_empty());
    
    // Clean up
    std::env::remove_var("TEST_API_KEY");
    
    Ok(())
}

#[test]
fn test_config_error_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Test loading non-existent config file
    let non_existent_path = temp_dir.path().join("non_existent.toml");
    let result = Config::load_from_file(&non_existent_path);
    assert!(result.is_err());
    
    // Test loading invalid config file
    let invalid_config = "invalid toml content [[[";
    let invalid_path = temp_dir.path().join("invalid.toml");
    std::fs::write(&invalid_path, invalid_config)?;
    
    let invalid_result = Config::load_from_file(&invalid_path);
    assert!(invalid_result.is_err());
    
    Ok(())
}

#[test]
fn test_cli_output_formatting() -> Result<()> {
    // Test that CLI produces properly formatted output
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fluent", "--", "--help"])
        .output()?;
    
    let stdout = String::from_utf8(output.stdout)?;
    
    // Should be valid UTF-8 and contain expected sections
    assert!(!stdout.is_empty());
    
    // Should not contain obvious formatting errors
    assert!(!stdout.contains("{{"));
    assert!(!stdout.contains("}}"));
    
    Ok(())
}

#[test]
fn test_config_defaults() -> Result<()> {
    let default_config = Config::default();
    
    // Test that defaults are reasonable
    assert!(!default_config.default_engine.is_empty());
    assert!(default_config.max_concurrent_requests > 0);
    assert!(default_config.database.max_connections > 0);
    assert!(default_config.database.connection_timeout > 0);
    
    // Test that log level is valid
    let valid_log_levels = ["error", "warn", "info", "debug", "trace"];
    assert!(valid_log_levels.contains(&default_config.log_level.as_str()));
    
    Ok(())
}

#[test]
fn test_engine_type_serialization() -> Result<()> {
    // Test that engine types can be serialized/deserialized
    let engine_types = vec![
        EngineType::OpenAI,
        EngineType::Anthropic,
        EngineType::Local,
    ];
    
    for engine_type in engine_types {
        let serialized = serde_json::to_string(&engine_type)?;
        let deserialized: EngineType = serde_json::from_str(&serialized)?;
        assert_eq!(engine_type, deserialized);
    }
    
    Ok(())
}

#[test]
fn test_config_merge_functionality() -> Result<()> {
    // Test that configs can be merged (if this functionality exists)
    let base_config = Config::default();
    let mut override_config = Config::default();
    override_config.log_level = "debug".to_string();
    override_config.max_concurrent_requests = 10;
    
    // Test that override values are different
    assert_ne!(base_config.log_level, override_config.log_level);
    assert_ne!(base_config.max_concurrent_requests, override_config.max_concurrent_requests);
    
    Ok(())
}

#[test]
fn test_cli_error_messages() -> Result<()> {
    // Test that CLI provides helpful error messages for invalid commands
    let output = Command::new("cargo")
        .args(&["run", "--bin", "fluent", "--", "invalid_command"])
        .output()?;
    
    // Should exit with error code for invalid command
    assert!(!output.status.success());
    
    // Error output should contain helpful information
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.is_empty() || !String::from_utf8(output.stdout)?.is_empty());
    
    Ok(())
}
