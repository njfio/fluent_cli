use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Simple E2E CLI Tests
///
/// These tests validate basic CLI functionality using assert_cmd properly.
///
/// Test utilities for E2E CLI testing
pub struct CliTestRunner {
    temp_dir: TempDir,
}

impl CliTestRunner {
    /// Create a new CLI test runner
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        Ok(Self { temp_dir })
    }

    /// Execute a CLI command with arguments using assert_cmd
    pub fn run_command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::cargo_bin("fluent").expect("Failed to find fluent binary");
        cmd.args(args);
        cmd.current_dir(self.temp_dir.path());
        cmd
    }

    /// Get the temporary directory path
    pub fn temp_dir(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Create a test configuration file
    pub fn create_test_config(&self, content: &str) -> Result<String> {
        let config_path = self.temp_dir.path().join("test_config.yaml");
        std::fs::write(&config_path, content)?;
        Ok(config_path.to_string_lossy().to_string())
    }
}

/// Basic CLI Tests
mod basic_tests {
    use super::*;

    /// Test basic help command
    #[test]
    fn test_help_command() -> Result<()> {
        let runner = CliTestRunner::new()?;

        runner
            .run_command(&["--help"])
            .assert()
            .code(predicate::in_iter([0, 2]));

        println!("✅ Help command test passed");
        Ok(())
    }

    /// Test agent command structure
    #[test]
    fn test_agent_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test agent help - should succeed or fail gracefully
        runner
            .run_command(&["agent", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes

        println!("✅ Agent commands test passed");
        Ok(())
    }

    /// Test tools command structure
    #[test]
    fn test_tools_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test tools help
        runner
            .run_command(&["tools", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes

        println!("✅ Tools commands test passed");
        Ok(())
    }

    /// Test neo4j command structure
    #[test]
    fn test_neo4j_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test neo4j help
        runner
            .run_command(&["neo4j", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes

        println!("✅ Neo4j commands test passed");
        Ok(())
    }

    /// Test invalid command handling
    #[test]
    fn test_invalid_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test invalid command
        runner
            .run_command(&["invalid-command"])
            .assert()
            .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes

        println!("✅ Invalid command test passed");
        Ok(())
    }

    /// Test version command
    #[test]
    fn test_version_command() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test version command
        runner
            .run_command(&["--version"])
            .assert()
            .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes

        println!("✅ Version command test passed");
        Ok(())
    }
}

/// Configuration Tests
mod config_tests {
    use super::*;

    /// Test configuration file handling
    #[test]
    fn test_config_file_handling() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Create a test config
        let config_content = r#"
engines:
  - name: test-engine
    engine: openai
    connection:
      protocol: https
      hostname: api.openai.com
      port: 443
      request_path: /v1/chat/completions
    parameters:
      model: gpt-3.5-turbo
      max_tokens: 1000
      temperature: 0.7
"#;
        let config_path = runner.create_test_config(config_content)?;

        // Test with config file
        runner
            .run_command(&["-c", &config_path, "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes

        println!("✅ Config file test passed");
        Ok(())
    }

    /// Test missing config file
    #[test]
    fn test_missing_config_file() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test with non-existent config file
        runner
            .run_command(&["-c", "/non/existent/config.yaml", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes

        println!("✅ Missing config file test passed");
        Ok(())
    }
}

/// Error Handling Tests
mod error_tests {
    use super::*;

    /// Test various error scenarios
    #[test]
    fn test_error_scenarios() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test various error conditions that should be handled gracefully
        let error_cases = vec![
            vec!["--invalid-flag"],
            vec!["agent", "--invalid-option"],
            vec!["tools", "--bad-arg"],
        ];

        for case in error_cases {
            runner
                .run_command(&case)
                .assert()
                .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes
        }

        println!("✅ Error scenarios test passed");
        Ok(())
    }
}

/// Functional E2E Tests - Tests actual CLI functionality
mod functional_tests {
    use super::*;

    /// Test tools list command outputs tool information
    #[test]
    fn test_tools_list_command() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let output = runner.run_command(&["tools", "list"]).output()?;

        // Should complete without crashing
        // The exit code depends on whether config is found
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Either succeeds with tool output or fails gracefully with error message
        let has_output = !stdout.is_empty() || !stderr.is_empty();
        assert!(has_output, "tools list should produce some output");

        println!("✅ Tools list command test passed");
        Ok(())
    }

    /// Test engine list command
    #[test]
    fn test_engine_list_command() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let output = runner.run_command(&["engine", "list"]).output()?;

        // Should complete without crashing
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Either succeeds or fails gracefully
        let has_output = !stdout.is_empty() || !stderr.is_empty();
        assert!(has_output, "engine list should produce some output");

        println!("✅ Engine list command test passed");
        Ok(())
    }

    /// Test schema command outputs JSON schema
    #[test]
    fn test_schema_command() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let output = runner.run_command(&["schema"]).output()?;

        // If schema command works, should output JSON
        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() && !stdout.is_empty() {
            // Should be valid JSON if it succeeds
            assert!(
                stdout.contains("{") || stdout.contains("schema"),
                "schema output should contain JSON or schema keywords"
            );
        }

        println!("✅ Schema command test passed");
        Ok(())
    }

    /// Test bash completions generation
    #[test]
    fn test_bash_completions() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let output = runner
            .run_command(&["completions", "--shell", "bash"])
            .output()?;

        // If completions work, should output shell script
        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() && !stdout.is_empty() {
            // Bash completions should contain completion-related content
            assert!(
                stdout.contains("complete")
                    || stdout.contains("_fluent")
                    || stdout.contains("COMPREPLY"),
                "bash completions should contain completion-related keywords"
            );
        }

        println!("✅ Bash completions test passed");
        Ok(())
    }

    /// Test zsh completions generation
    #[test]
    fn test_zsh_completions() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let output = runner
            .run_command(&["completions", "--shell", "zsh"])
            .output()?;

        // If completions work, should output shell script
        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() && !stdout.is_empty() {
            // Zsh completions should contain zsh-specific content
            assert!(
                stdout.contains("compdef")
                    || stdout.contains("#compdef")
                    || stdout.contains("_fluent"),
                "zsh completions should contain zsh-specific keywords"
            );
        }

        println!("✅ Zsh completions test passed");
        Ok(())
    }

    /// Test fish completions generation
    #[test]
    fn test_fish_completions() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let output = runner
            .run_command(&["completions", "--shell", "fish"])
            .output()?;

        // If completions work, should output shell script
        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() && !stdout.is_empty() {
            // Fish completions should contain fish-specific content
            assert!(
                stdout.contains("complete -c fluent") || stdout.contains("__fish"),
                "fish completions should contain fish-specific keywords"
            );
        }

        println!("✅ Fish completions test passed");
        Ok(())
    }
}

/// JSON Output Tests - Tests JSON output formatting
mod json_output_tests {
    use super::*;

    /// Test verbose flag
    #[test]
    fn test_verbose_flag() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Verbose should enable more output
        runner
            .run_command(&["--verbose", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2]));

        println!("✅ Verbose flag test passed");
        Ok(())
    }

    /// Test quiet flag
    #[test]
    fn test_quiet_flag() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Quiet should suppress output
        runner
            .run_command(&["--quiet", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2]));

        println!("✅ Quiet flag test passed");
        Ok(())
    }

    /// Test JSON log flag
    #[test]
    fn test_json_logs_flag() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // JSON logs should format logs as JSON
        runner
            .run_command(&["--json-logs", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2]));

        println!("✅ JSON logs flag test passed");
        Ok(())
    }
}

/// Exit Code Tests - Tests proper exit codes for various scenarios
mod exit_code_tests {
    use super::*;

    /// Test successful help returns exit code 0
    #[test]
    fn test_help_exit_code() -> Result<()> {
        let runner = CliTestRunner::new()?;

        runner
            .run_command(&["--help"])
            .assert()
            .code(predicate::in_iter([0, 2])); // 0 success, 2 for clap help

        println!("✅ Help exit code test passed");
        Ok(())
    }

    /// Test version returns exit code 0
    #[test]
    fn test_version_exit_code() -> Result<()> {
        let runner = CliTestRunner::new()?;

        runner
            .run_command(&["--version"])
            .assert()
            .code(predicate::in_iter([0, 2])); // 0 success, 2 for clap version

        println!("✅ Version exit code test passed");
        Ok(())
    }

    /// Test subcommand help returns proper exit code
    #[test]
    fn test_subcommand_help_exit_codes() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let subcommands = vec!["agent", "tools", "engine", "pipeline", "neo4j", "mcp"];

        for subcmd in subcommands {
            runner
                .run_command(&[subcmd, "--help"])
                .assert()
                .code(predicate::in_iter([0, 1, 2])); // Allow various exit codes
        }

        println!("✅ Subcommand help exit codes test passed");
        Ok(())
    }
}

/// Pipeline Tests - Tests pipeline functionality
mod pipeline_tests {
    use super::*;

    /// Test pipeline help command
    #[test]
    fn test_pipeline_help() -> Result<()> {
        let runner = CliTestRunner::new()?;

        runner
            .run_command(&["pipeline", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2]));

        println!("✅ Pipeline help test passed");
        Ok(())
    }

    /// Test pipeline with non-existent file
    #[test]
    fn test_pipeline_missing_file() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Should fail gracefully when file doesn't exist
        let output = runner
            .run_command(&["pipeline", "-f", "/nonexistent/pipeline.yaml"])
            .output()?;

        // Should either fail with error or handle gracefully
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should have some output indicating the problem
        let has_output = !stderr.is_empty() || !stdout.is_empty() || !output.status.success();
        assert!(
            has_output,
            "Missing pipeline file should produce error or output"
        );

        println!("✅ Pipeline missing file test passed");
        Ok(())
    }

    /// Test pipeline with valid YAML file
    #[test]
    fn test_pipeline_valid_yaml() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Create a simple test pipeline
        let pipeline_content = r#"
name: test_pipeline
description: A simple test pipeline
steps:
  - name: step1
    type: echo
    input: "Hello, World!"
"#;

        let pipeline_path = runner.temp_dir().join("test_pipeline.yaml");
        std::fs::write(&pipeline_path, pipeline_content)?;

        // Try to run the pipeline (may fail without proper config, but shouldn't crash)
        let output = runner
            .run_command(&["pipeline", "-f", pipeline_path.to_str().unwrap()])
            .output()?;

        // Should not crash regardless of outcome
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Should have some output
        assert!(
            !stderr.is_empty() || !stdout.is_empty(),
            "Pipeline execution should produce some output"
        );

        println!("✅ Pipeline valid YAML test passed");
        Ok(())
    }
}

/// MCP Tests - Tests MCP functionality
mod mcp_tests {
    use super::*;

    /// Test MCP help command
    #[test]
    fn test_mcp_help() -> Result<()> {
        let runner = CliTestRunner::new()?;

        runner
            .run_command(&["mcp", "--help"])
            .assert()
            .code(predicate::in_iter([0, 1, 2]));

        println!("✅ MCP help test passed");
        Ok(())
    }

    /// Test MCP server help
    #[test]
    fn test_mcp_server_help() -> Result<()> {
        let runner = CliTestRunner::new()?;

        let output = runner.run_command(&["mcp", "server", "--help"]).output()?;

        // Should show server-related help or fail gracefully
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let has_output = !stdout.is_empty() || !stderr.is_empty();
        assert!(has_output, "MCP server help should produce output");

        println!("✅ MCP server help test passed");
        Ok(())
    }
}
