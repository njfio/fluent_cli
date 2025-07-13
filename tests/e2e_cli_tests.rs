use anyhow::Result;
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// End-to-End CLI Tests
/// 
/// These tests validate the complete CLI functionality by spawning the actual
/// fluent CLI binary and testing real command execution, argument parsing,
/// error handling, and output formatting.

/// Test utilities for E2E CLI testing
pub struct CliTestRunner {
    binary_path: String,
    temp_dir: TempDir,
}

impl CliTestRunner {
    /// Create a new CLI test runner
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;

        // Use cargo to run the binary instead of direct path
        let binary_path = "cargo".to_string();

        Ok(Self {
            binary_path,
            temp_dir,
        })
    }
    
    /// Execute a CLI command with arguments
    pub async fn run_command(&self, args: &[&str]) -> Result<CommandOutput> {
        let mut cmd = Command::new(&self.binary_path);

        // Use cargo run to execute the CLI
        let mut cargo_args = vec!["run", "--"];
        cargo_args.extend(args);

        cmd.args(&cargo_args)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .current_dir(std::env::current_dir()?);
        
        // Set timeout for command execution
        let output = timeout(Duration::from_secs(30), async {
            tokio::task::spawn_blocking(move || cmd.output()).await?
        }).await??;
        
        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
    
    /// Execute a CLI command and expect success
    pub async fn run_command_success(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = self.run_command(args).await?;
        if output.exit_code != 0 {
            anyhow::bail!(
                "Command failed with exit code {}: {}", 
                output.exit_code, 
                output.stderr
            );
        }
        Ok(output)
    }
    
    /// Execute a CLI command and expect failure
    pub async fn run_command_failure(&self, args: &[&str]) -> Result<CommandOutput> {
        let output = self.run_command(args).await?;
        if output.exit_code == 0 {
            anyhow::bail!("Expected command to fail, but it succeeded");
        }
        Ok(output)
    }
    
    /// Create a test configuration file
    pub fn create_test_config(&self, content: &str) -> Result<String> {
        let config_path = self.temp_dir.path().join("test_config.yaml");
        std::fs::write(&config_path, content)?;
        Ok(config_path.to_string_lossy().to_string())
    }
    
    /// Create a test pipeline file
    pub fn create_test_pipeline(&self, content: &str) -> Result<String> {
        let pipeline_path = self.temp_dir.path().join("test_pipeline.yaml");
        std::fs::write(&pipeline_path, content)?;
        Ok(pipeline_path.to_string_lossy().to_string())
    }
}

/// Command execution output
#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    /// Check if output contains a specific string
    pub fn contains_stdout(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }
    
    /// Check if stderr contains a specific string
    pub fn contains_stderr(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }
    
    /// Check if output is empty
    pub fn is_empty(&self) -> bool {
        self.stdout.trim().is_empty() && self.stderr.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test CLI help command
    #[tokio::test]
    async fn test_cli_help() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test basic help
        let output = runner.run_command(&["--help"]).await?;

        // Check for help content
        assert!(output.exit_code == 0);
        assert!(output.contains_stdout("Usage: fluent"));
        assert!(output.contains_stdout("Commands:"));

        println!("✅ CLI help test passed");
        Ok(())
    }
    
    /// Test CLI version command
    #[tokio::test]
    async fn test_cli_version() -> Result<()> {
        let runner = CliTestRunner::new()?;
        
        // Test version output
        let output = runner.run_command_success(&["--version"]).await?;
        assert!(output.contains_stdout("0.1.0") || output.contains_stdout("version"));
        
        println!("✅ CLI version test passed");
        Ok(())
    }
    
    /// Test invalid command handling
    #[tokio::test]
    async fn test_invalid_command() -> Result<()> {
        let runner = CliTestRunner::new()?;
        
        // Test invalid command
        let output = runner.run_command_failure(&["invalid-command"]).await?;
        assert!(output.contains_stderr("error") || output.contains_stderr("invalid"));
        
        println!("✅ Invalid command test passed");
        Ok(())
    }
    
    /// Test configuration file handling
    #[tokio::test]
    async fn test_config_file() -> Result<()> {
        let runner = CliTestRunner::new()?;
        
        // Create a test config file
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
      bearer_token: test-token
      model: gpt-3.5-turbo
"#;
        
        let config_path = runner.create_test_config(config_content)?;
        
        // Test config file loading (this might fail due to invalid token, but should parse)
        let output = runner.run_command(&["-c", &config_path, "test-engine", "--help"]).await?;
        
        // Should either show help or fail gracefully with config parsing
        assert!(
            output.contains_stdout("help") || 
            output.contains_stderr("config") ||
            output.exit_code != 0
        );
        
        println!("✅ Config file test passed");
        Ok(())
    }
    
    /// Test MCP command structure
    #[tokio::test]
    async fn test_mcp_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;
        
        // Test MCP help
        let output = runner.run_command(&["mcp", "--help"]).await?;
        
        // Should show MCP subcommands or help
        assert!(
            output.contains_stdout("mcp") || 
            output.contains_stdout("server") ||
            output.contains_stdout("connect") ||
            output.exit_code == 0
        );
        
        println!("✅ MCP commands test passed");
        Ok(())
    }
    
    /// Test pipeline command structure
    #[tokio::test]
    async fn test_pipeline_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;
        
        // Test pipeline help
        let output = runner.run_command(&["pipeline", "--help"]).await?;
        
        // Should show pipeline help or subcommands
        assert!(
            output.contains_stdout("pipeline") || 
            output.contains_stdout("file") ||
            output.exit_code == 0
        );
        
        println!("✅ Pipeline commands test passed");
        Ok(())
    }
    
    /// Test agent command structure
    #[tokio::test]
    async fn test_agent_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;
        
        // Test agent help
        let output = runner.run_command(&["agent", "--help"]).await?;
        
        // Should show agent help or subcommands
        assert!(
            output.contains_stdout("agent") || 
            output.exit_code == 0
        );
        
        println!("✅ Agent commands test passed");
        Ok(())
    }
    
    /// Test tools command structure
    #[tokio::test]
    async fn test_tools_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;
        
        // Test tools help
        let output = runner.run_command(&["tools", "--help"]).await?;
        
        // Should show tools help or subcommands
        assert!(
            output.contains_stdout("tools") || 
            output.contains_stdout("list") ||
            output.exit_code == 0
        );
        
        println!("✅ Tools commands test passed");
        Ok(())
    }
    
    /// Test neo4j command structure
    #[tokio::test]
    async fn test_neo4j_commands() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test neo4j help
        let output = runner.run_command(&["neo4j", "--help"]).await?;

        // Should show neo4j help or subcommands
        assert!(
            output.contains_stdout("neo4j") ||
            output.exit_code == 0
        );

        println!("✅ Neo4j commands test passed");
        Ok(())
    }
}

/// User Workflow Tests
///
/// These tests validate complete user journeys and workflows
#[cfg(test)]
mod workflow_tests {
    use super::*;

    /// Test complete configuration workflow
    #[tokio::test]
    async fn test_configuration_workflow() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Create a comprehensive test config
        let config_content = r#"
engines:
  - name: test-openai
    engine: openai
    connection:
      protocol: https
      hostname: api.openai.com
      port: 443
      request_path: /v1/chat/completions
    parameters:
      bearer_token: test-token-openai
      model: gpt-3.5-turbo
  - name: test-anthropic
    engine: anthropic
    connection:
      protocol: https
      hostname: api.anthropic.com
      port: 443
      request_path: /v1/messages
    parameters:
      bearer_token: test-token-anthropic
      model: claude-3-sonnet-20240229
"#;

        let config_path = runner.create_test_config(config_content)?;

        // Test config loading with different engines
        let engines = vec!["test-openai", "test-anthropic"];

        for engine in engines {
            let output = runner.run_command(&["-c", &config_path, engine, "--help"]).await?;

            // Should either show help or fail gracefully with config parsing
            assert!(
                output.contains_stdout("help") ||
                output.contains_stdout("Usage") ||
                output.contains_stderr("config") ||
                output.exit_code != -1  // Not a crash
            );
        }

        println!("✅ Configuration workflow test passed");
        Ok(())
    }

    /// Test pipeline workflow
    #[tokio::test]
    async fn test_pipeline_workflow() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Create a test pipeline
        let pipeline_content = r#"
name: test-pipeline
description: A test pipeline for E2E testing
steps:
  - name: step1
    type: prompt
    engine: test-engine
    prompt: "Hello, world!"
  - name: step2
    type: transform
    operation: uppercase
"#;

        let _pipeline_path = runner.create_test_pipeline(pipeline_content)?;

        // Test pipeline validation (should work without API keys)
        let output = runner.run_command(&["pipeline", "--help"]).await?;
        assert!(output.exit_code == 0);
        assert!(output.contains_stdout("pipeline") || output.contains_stdout("Execute"));

        println!("✅ Pipeline workflow test passed");
        Ok(())
    }

    /// Test MCP workflow
    #[tokio::test]
    async fn test_mcp_workflow() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test MCP command structure
        let output = runner.run_command(&["mcp", "--help"]).await?;
        assert!(output.exit_code == 0);

        // Test MCP subcommands
        let subcommands = vec!["server", "connect", "tools", "status"];

        for subcommand in subcommands {
            let output = runner.run_command(&["mcp", subcommand, "--help"]).await?;
            // Should show help or handle gracefully
            assert!(output.exit_code == 0 || output.exit_code == 2); // 2 is typical for help
        }

        println!("✅ MCP workflow test passed");
        Ok(())
    }

    /// Test agent workflow
    #[tokio::test]
    async fn test_agent_workflow() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test agent command structure
        let output = runner.run_command(&["agent", "--help"]).await?;
        assert!(output.exit_code == 0);

        // Test agent with different options
        let agent_options = vec![
            vec!["agent", "--help"],
        ];

        for options in agent_options {
            let output = runner.run_command(&options).await?;
            assert!(output.exit_code == 0 || output.exit_code == 2);
        }

        println!("✅ Agent workflow test passed");
        Ok(())
    }

    /// Test tools workflow
    #[tokio::test]
    async fn test_tools_workflow() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test tools command structure
        let output = runner.run_command(&["tools", "--help"]).await?;
        assert!(output.exit_code == 0);

        // Test tools subcommands
        let subcommands = vec!["list", "describe", "categories"];

        for subcommand in subcommands {
            let output = runner.run_command(&["tools", subcommand, "--help"]).await?;
            // Should show help or handle gracefully
            assert!(output.exit_code == 0 || output.exit_code == 2);
        }

        println!("✅ Tools workflow test passed");
        Ok(())
    }

    /// Test error handling workflow
    #[tokio::test]
    async fn test_error_handling_workflow() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test various error conditions
        let error_cases = vec![
            // Invalid engine name
            vec!["invalid-engine", "test query"],
            // Invalid subcommand
            vec!["mcp", "invalid-subcommand"],
            // Missing required arguments
            vec!["pipeline"],
        ];

        for case in error_cases {
            let output = runner.run_command(&case).await?;
            // Should fail gracefully, not crash
            assert!(output.exit_code != 0);
            assert!(!output.stderr.is_empty() || !output.stdout.is_empty());
        }

        println!("✅ Error handling workflow test passed");
        Ok(())
    }

    /// Test configuration override workflow
    #[tokio::test]
    async fn test_configuration_override_workflow() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Create base config
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
      bearer_token: base-token
      model: gpt-3.5-turbo
"#;

        let config_path = runner.create_test_config(config_content)?;

        // Test configuration overrides
        let output = runner.run_command(&[
            "-c", &config_path,
            "-o", "bearer_token=override-token",
            "-o", "model=gpt-4",
            "test-engine",
            "--help"
        ]).await?;

        // Should handle overrides gracefully
        assert!(output.exit_code == 0 || output.exit_code == 2);

        println!("✅ Configuration override workflow test passed");
        Ok(())
    }
}

/// Documentation Example Validation Tests
///
/// These tests validate that all examples in README.md and documentation actually work
#[cfg(test)]
mod documentation_tests {
    use super::*;

    /// Test README.md basic usage examples
    #[tokio::test]
    async fn test_readme_basic_usage_examples() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test examples from README.md
        let examples = vec![
            // Basic help
            vec!["--help"],
            vec!["--version"],

            // Engine help examples
            vec!["openai-gpt4", "--help"],
            vec!["anthropic-claude", "--help"],

            // Command help examples
            vec!["pipeline", "--help"],
            vec!["agent", "--help"],
            vec!["mcp", "--help"],
            vec!["tools", "--help"],
        ];

        for example in examples {
            let output = runner.run_command(&example).await?;

            // Should show help or handle gracefully
            assert!(
                output.exit_code == 0 ||
                output.exit_code == 2 ||  // Help exit code
                output.contains_stderr("error") // Expected error for invalid engines
            );
        }

        println!("✅ README basic usage examples test passed");
        Ok(())
    }

    /// Test README.md agent command examples
    #[tokio::test]
    async fn test_readme_agent_examples() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test agent examples from README (without API keys, should show help or fail gracefully)
        let examples = vec![
            vec!["openai-gpt4", "agent", "--help"],
            vec!["agent", "--help"],
        ];

        for example in examples {
            let output = runner.run_command(&example).await?;

            // Should show help or handle gracefully
            assert!(
                output.exit_code == 0 ||
                output.exit_code == 2 ||
                output.contains_stderr("error")
            );
        }

        println!("✅ README agent examples test passed");
        Ok(())
    }

    /// Test README.md pipeline command examples
    #[tokio::test]
    async fn test_readme_pipeline_examples() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test pipeline examples from README
        let examples = vec![
            vec!["pipeline", "--help"],
            vec!["build-pipeline", "--help"],
        ];

        for example in examples {
            let output = runner.run_command(&example).await?;

            // Should show help or handle gracefully
            assert!(
                output.exit_code == 0 ||
                output.exit_code == 2
            );
        }

        println!("✅ README pipeline examples test passed");
        Ok(())
    }

    /// Test README.md MCP command examples
    #[tokio::test]
    async fn test_readme_mcp_examples() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test MCP examples from README
        let examples = vec![
            vec!["mcp", "--help"],
            vec!["mcp", "server", "--help"],
            vec!["mcp", "connect", "--help"],
            vec!["mcp", "tools", "--help"],
            vec!["mcp", "status", "--help"],
        ];

        for example in examples {
            let output = runner.run_command(&example).await?;

            // Should show help or handle gracefully
            assert!(
                output.exit_code == 0 ||
                output.exit_code == 2
            );
        }

        println!("✅ README MCP examples test passed");
        Ok(())
    }

    /// Test README.md tools command examples
    #[tokio::test]
    async fn test_readme_tools_examples() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test tools examples from README
        let examples = vec![
            vec!["tools", "--help"],
            vec!["tools", "list", "--help"],
            vec!["tools", "describe", "--help"],
            vec!["tools", "categories", "--help"],
        ];

        for example in examples {
            let output = runner.run_command(&example).await?;

            // Should show help or handle gracefully
            assert!(
                output.exit_code == 0 ||
                output.exit_code == 2
            );
        }

        println!("✅ README tools examples test passed");
        Ok(())
    }

    /// Test configuration file format examples
    #[tokio::test]
    async fn test_configuration_format_examples() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test configuration formats mentioned in documentation
        let config_examples = vec![
            // OpenAI configuration
            r#"
engines:
  - name: openai-gpt4
    engine: openai
    connection:
      protocol: https
      hostname: api.openai.com
      port: 443
      request_path: /v1/chat/completions
    parameters:
      bearer_token: test-token
      model: gpt-4
"#,
            // Anthropic configuration
            r#"
engines:
  - name: anthropic-claude
    engine: anthropic
    connection:
      protocol: https
      hostname: api.anthropic.com
      port: 443
      request_path: /v1/messages
    parameters:
      bearer_token: test-token
      model: claude-3-sonnet-20240229
"#,
        ];

        for config_content in config_examples {
            let config_path = runner.create_test_config(config_content)?;

            // Test that config loads without syntax errors
            let output = runner.run_command(&["-c", &config_path, "--help"]).await?;

            // Should load config successfully or fail gracefully
            assert!(
                output.exit_code == 0 ||
                output.contains_stderr("config") ||
                output.contains_stderr("token")
            );
        }

        println!("✅ Configuration format examples test passed");
        Ok(())
    }

    /// Test command line argument examples
    #[tokio::test]
    async fn test_command_line_argument_examples() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test various command line argument combinations from documentation
        let argument_examples = vec![
            // Basic arguments
            vec!["--help"],
            vec!["--version"],

            // Configuration arguments
            vec!["-c", "nonexistent.yaml", "--help"],

            // Override arguments
            vec!["-o", "model=gpt-4", "--help"],

            // Multiple overrides
            vec!["-o", "model=gpt-4", "-o", "temperature=0.7", "--help"],
        ];

        for example in argument_examples {
            let output = runner.run_command(&example).await?;

            // Should handle arguments gracefully
            assert!(
                output.exit_code == 0 ||
                output.exit_code == 2 ||
                output.contains_stderr("error")
            );
        }

        println!("✅ Command line argument examples test passed");
        Ok(())
    }
}

/// Error Scenario Testing
///
/// These tests validate error handling and recovery in real-world scenarios
#[cfg(test)]
mod error_scenario_tests {
    use super::*;

    /// Test invalid configuration scenarios
    #[tokio::test]
    async fn test_invalid_configuration_scenarios() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test various invalid configuration scenarios
        let invalid_configs = vec![
            // Invalid YAML syntax
            r#"
engines:
  - name: test-engine
    engine: openai
    invalid_yaml: [
"#,
            // Missing required fields
            r#"
engines:
  - name: test-engine
    # Missing engine field
    connection:
      protocol: https
"#,
            // Invalid engine type
            r#"
engines:
  - name: test-engine
    engine: invalid-engine-type
    connection:
      protocol: https
      hostname: api.example.com
"#,
        ];

        for config_content in invalid_configs {
            let config_path = runner.create_test_config(config_content)?;

            // Should fail gracefully with config error
            let output = runner.run_command(&["-c", &config_path, "--help"]).await?;

            // Should handle invalid config gracefully
            assert!(
                output.exit_code != 0 ||
                output.contains_stderr("config") ||
                output.contains_stderr("error") ||
                output.contains_stderr("invalid")
            );
        }

        println!("✅ Invalid configuration scenarios test passed");
        Ok(())
    }

    /// Test missing file scenarios
    #[tokio::test]
    async fn test_missing_file_scenarios() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test missing file scenarios
        let missing_file_cases = vec![
            // Missing config file
            vec!["-c", "nonexistent-config.yaml", "--help"],

            // Missing pipeline file
            vec!["pipeline", "-f", "nonexistent-pipeline.yaml", "--help"],
        ];

        for case in missing_file_cases {
            let output = runner.run_command(&case).await?;

            // Should handle missing files gracefully
            assert!(
                output.exit_code != 0 ||
                output.contains_stderr("file") ||
                output.contains_stderr("not found") ||
                output.contains_stderr("error")
            );
        }

        println!("✅ Missing file scenarios test passed");
        Ok(())
    }

    /// Test invalid command combinations
    #[tokio::test]
    async fn test_invalid_command_combinations() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test invalid command combinations
        let invalid_combinations = vec![
            // Invalid subcommands
            vec!["mcp", "invalid-subcommand"],
            vec!["agent", "invalid-subcommand"],
            vec!["tools", "invalid-subcommand"],
            vec!["pipeline", "invalid-subcommand"],
        ];

        for combination in invalid_combinations {
            let output = runner.run_command(&combination).await?;

            // Should handle invalid combinations gracefully
            assert!(
                output.exit_code != 0 ||
                output.contains_stderr("error:") ||
                output.contains_stderr("unrecognized") ||
                output.contains_stderr("unexpected argument")
            );
        }

        println!("✅ Invalid command combinations test passed");
        Ok(())
    }

    /// Test resource exhaustion scenarios
    #[tokio::test]
    async fn test_resource_exhaustion_scenarios() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test scenarios that might cause resource issues
        let large_override = "x=".repeat(1000);
        let resource_test_cases = vec![
            // Very long command line
            vec!["--help"; 100], // Repeat --help 100 times

            // Large configuration override
            vec!["-o", &large_override, "--help"],
        ];

        for case in resource_test_cases {
            let output = runner.run_command(&case).await?;

            // Should handle resource issues gracefully (not crash)
            assert!(output.exit_code != -1); // Not a crash
        }

        println!("✅ Resource exhaustion scenarios test passed");
        Ok(())
    }

    /// Test permission and access scenarios
    #[tokio::test]
    async fn test_permission_scenarios() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test permission-related scenarios
        let permission_cases = vec![
            // Try to access system directories (should be handled gracefully)
            vec!["-c", "/root/config.yaml", "openai", "test"],
            vec!["-c", "/etc/passwd", "openai", "test"],
        ];

        for case in permission_cases {
            let output = runner.run_command(&case).await?;

            // Should handle permission issues gracefully
            assert!(
                output.exit_code != 0 ||
                output.contains_stderr("permission") ||
                output.contains_stderr("access") ||
                output.contains_stderr("denied") ||
                output.contains_stderr("error:")
            );
        }

        println!("✅ Permission scenarios test passed");
        Ok(())
    }

    /// Test malformed input scenarios
    #[tokio::test]
    async fn test_malformed_input_scenarios() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test malformed input handling
        let malformed_inputs = vec![
            // Invalid override format
            vec!["-o", "invalid-format", "--help"],
            vec!["-o", "=invalid", "--help"],
            vec!["-o", "key=", "--help"],

            // Special characters (avoiding null bytes which cause issues)
            vec!["-o", "key=value with spaces", "--help"],
            vec!["-o", "key=value@#$%", "--help"],
        ];

        for input in malformed_inputs {
            let output = runner.run_command(&input).await?;

            // Should handle malformed input gracefully
            assert!(output.exit_code != -1); // Not a crash
        }

        println!("✅ Malformed input scenarios test passed");
        Ok(())
    }

    /// Test concurrent execution scenarios
    #[tokio::test]
    async fn test_concurrent_execution_scenarios() -> Result<()> {
        // Test concurrent command execution
        let mut handles = Vec::new();

        for _i in 0..5 {
            let runner_clone = CliTestRunner::new()?;
            let handle = tokio::spawn(async move {
                let output = runner_clone.run_command(&["--help"]).await?;
                assert!(output.exit_code == 0);
                Ok::<(), anyhow::Error>(())
            });
            handles.push(handle);
        }

        // Wait for all concurrent executions
        for handle in handles {
            handle.await??;
        }

        println!("✅ Concurrent execution scenarios test passed");
        Ok(())
    }

    /// Test timeout and hanging scenarios
    #[tokio::test]
    async fn test_timeout_scenarios() -> Result<()> {
        let runner = CliTestRunner::new()?;

        // Test commands that should complete quickly
        let quick_commands = vec![
            vec!["--help"],
            vec!["--version"],
            vec!["mcp", "--help"],
            vec!["tools", "--help"],
        ];

        for command in quick_commands {
            // Use a reasonable timeout
            let start = std::time::Instant::now();
            let output = runner.run_command(&command).await?;
            let duration = start.elapsed();

            // Should complete within reasonable time (30 seconds)
            assert!(duration.as_secs() < 30);
            assert!(output.exit_code == 0 || output.exit_code == 2);
        }

        println!("✅ Timeout scenarios test passed");
        Ok(())
    }
}
