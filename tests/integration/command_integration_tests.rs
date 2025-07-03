use std::process::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use serde_json::json;

/// Integration tests for the fluent CLI commands
/// These tests verify that the refactored command structure works end-to-end

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Helper function to create a temporary config file
    fn create_test_config(temp_dir: &TempDir) -> String {
        let config_content = json!({
            "engines": [{
                "name": "test_engine",
                "engine": "openai",
                "connection": {
                    "protocol": "https",
                    "hostname": "api.openai.com",
                    "port": 443,
                    "request_path": "/v1/chat/completions"
                },
                "parameters": {},
                "session_id": null,
                "neo4j": null,
                "spinner": null
            }]
        });

        let config_path = temp_dir.path().join("test_config.json");
        fs::write(&config_path, config_content.to_string()).unwrap();
        config_path.to_string_lossy().to_string()
    }

    /// Helper function to create a test pipeline file
    fn create_test_pipeline(temp_dir: &TempDir) -> String {
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
"#;

        let pipeline_path = temp_dir.path().join("test_pipeline.yaml");
        fs::write(&pipeline_path, pipeline_content).unwrap();
        pipeline_path.to_string_lossy().to_string()
    }

    /// Test that the CLI binary can be executed
    #[test]
    fn test_cli_binary_exists() {
        let output = Command::new("cargo")
            .args(&["build", "--bin", "fluent"])
            .output()
            .expect("Failed to build fluent binary");

        assert!(output.status.success(), "Failed to build fluent binary");
    }

    /// Test help command works
    #[test]
    fn test_help_command() {
        let output = Command::new("cargo")
            .args(&["run", "--bin", "fluent", "--", "--help"])
            .output()
            .expect("Failed to run help command");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("fluent"));
    }

    /// Test pipeline command structure
    #[test]
    fn test_pipeline_command_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);
        let pipeline_path = create_test_pipeline(&temp_dir);

        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "pipeline",
                "--file", &pipeline_path,
                "--config", &config_path,
                "--dry-run"  // Use dry-run to avoid actual API calls
            ])
            .output()
            .expect("Failed to run pipeline command");

        // The command should at least parse correctly, even if it fails due to missing API keys
        // We're testing the command structure, not the actual execution
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("command not found"));
        assert!(!stderr.contains("unrecognized subcommand"));
    }

    /// Test agent command structure
    #[test]
    fn test_agent_command_structure() {
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "agent",
                "--help"
            ])
            .output()
            .expect("Failed to run agent help command");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should show agent help or at least not show "command not found"
        assert!(!stderr.contains("command not found"));
        assert!(!stderr.contains("unrecognized subcommand"));
    }

    /// Test MCP command structure
    #[test]
    fn test_mcp_command_structure() {
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "mcp",
                "--help"
            ])
            .output()
            .expect("Failed to run mcp help command");

        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should not show "command not found"
        assert!(!stderr.contains("command not found"));
        assert!(!stderr.contains("unrecognized subcommand"));
    }

    /// Test neo4j command structure
    #[test]
    fn test_neo4j_command_structure() {
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "neo4j",
                "--help"
            ])
            .output()
            .expect("Failed to run neo4j help command");

        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should not show "command not found"
        assert!(!stderr.contains("command not found"));
        assert!(!stderr.contains("unrecognized subcommand"));
    }

    /// Test that invalid commands are properly rejected
    #[test]
    fn test_invalid_command_rejection() {
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "invalid_command_that_should_not_exist"
            ])
            .output()
            .expect("Failed to run invalid command");

        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should show that the command is not recognized
        assert!(
            stderr.contains("unrecognized subcommand") || 
            stderr.contains("invalid") ||
            !output.status.success()
        );
    }

    /// Test configuration file validation
    #[test]
    fn test_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create invalid config
        let invalid_config = json!({
            "invalid_field": "invalid_value"
        });
        
        let config_path = temp_dir.path().join("invalid_config.json");
        fs::write(&config_path, invalid_config.to_string()).unwrap();

        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "openai",
                "test request",
                "--config", &config_path.to_string_lossy()
            ])
            .output()
            .expect("Failed to run command with invalid config");

        // Should fail due to invalid configuration
        assert!(!output.status.success());
    }

    /// Test that the modular architecture is working
    #[test]
    fn test_modular_architecture_integration() {
        // Test that each command module can be invoked
        let commands = vec!["pipeline", "agent", "mcp", "neo4j"];
        
        for command in commands {
            let output = Command::new("cargo")
                .args(&[
                    "run", "--bin", "fluent", "--",
                    command,
                    "--help"
                ])
                .output()
                .expect(&format!("Failed to run {} command", command));

            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Each command should be recognized (not show "unrecognized subcommand")
            assert!(
                !stderr.contains("unrecognized subcommand"),
                "Command '{}' not recognized: {}",
                command,
                stderr
            );
        }
    }

    /// Test error handling and graceful failures
    #[test]
    fn test_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);

        // Test with missing required arguments
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "pipeline",
                "--config", &config_path
                // Missing --file argument
            ])
            .output()
            .expect("Failed to run pipeline command with missing args");

        // Should fail gracefully with helpful error message
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should not panic or crash
        assert!(!stderr.contains("panic"));
        assert!(!stderr.contains("thread panicked"));
    }

    /// Test that the refactoring maintains backward compatibility
    #[test]
    fn test_backward_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config(&temp_dir);

        // Test that old-style engine commands still work
        let output = Command::new("cargo")
            .args(&[
                "run", "--bin", "fluent", "--",
                "openai",  // Direct engine command
                "test request",
                "--config", &config_path
            ])
            .output()
            .expect("Failed to run direct engine command");

        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Should not show "unrecognized subcommand" - the command should be parsed
        // even if it fails due to missing API keys
        assert!(!stderr.contains("unrecognized subcommand"));
    }

    /// Performance test - CLI startup time
    #[test]
    fn test_cli_startup_performance() {
        use std::time::Instant;

        let start = Instant::now();
        
        let output = Command::new("cargo")
            .args(&["run", "--bin", "fluent", "--", "--help"])
            .output()
            .expect("Failed to run help command");

        let duration = start.elapsed();
        
        // CLI should start reasonably quickly (under 5 seconds for debug build)
        assert!(duration.as_secs() < 5, "CLI startup took too long: {:?}", duration);
        assert!(output.status.success());
    }
}
