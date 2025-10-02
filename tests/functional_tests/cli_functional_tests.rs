//! Comprehensive functional tests for all Fluent CLI commands and options
//!
//! This test suite validates that all CLI commands and their options work correctly
//! by using assert_cmd to execute the fluent binary with various arguments.

use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Test runner for CLI functional tests
pub struct CliFunctionalTestRunner {
    temp_dir: TempDir,
}

impl CliFunctionalTestRunner {
    /// Create a new test runner
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        Ok(Self { temp_dir })
    }

    /// Execute a CLI command with arguments
    pub fn run_command(&self, args: &[&str]) -> Command {
        let mut cmd = Command::cargo_bin("fluent").expect("Failed to find fluent binary");
        cmd.args(args);
        cmd.current_dir(self.temp_dir.path());
        cmd
    }

    /// Create a test configuration file
    pub fn create_test_config(&self, content: &str) -> Result<String> {
        let config_path = self.temp_dir.path().join("test_config.yaml");
        fs::write(&config_path, content)?;
        Ok(config_path.to_string_lossy().to_string())
    }

    /// Create a test pipeline file
    pub fn create_test_pipeline(&self, content: &str) -> Result<String> {
        let pipeline_path = self.temp_dir.path().join("test_pipeline.yaml");
        fs::write(&pipeline_path, content)?;
        Ok(pipeline_path.to_string_lossy().to_string())
    }

    /// Create a test goal file
    pub fn create_test_goal(&self, content: &str) -> Result<String> {
        let goal_path = self.temp_dir.path().join("test_goal.toml");
        fs::write(&goal_path, content)?;
        Ok(goal_path.to_string_lossy().to_string())
    }
}

/// Global CLI Options Tests
mod global_options_tests {
    use super::*;

    #[test]
    fn test_help_option() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test --help
        runner
            .run_command(&["--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("fluent"))
            .stdout(predicate::str::contains(
                "A powerful CLI for interacting with various AI engines",
            ));

        // Test -h
        runner
            .run_command(&["-h"])
            .assert()
            .success()
            .stdout(predicate::str::contains("fluent"));

        println!("✅ Global help options test passed");
        Ok(())
    }

    #[test]
    fn test_version_option() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test --version
        runner
            .run_command(&["--version"])
            .assert()
            .success()
            .stdout(predicate::str::contains("0.1.0"));

        // Test -V
        runner
            .run_command(&["-V"])
            .assert()
            .success()
            .stdout(predicate::str::contains("0.1.0"));

        println!("✅ Global version options test passed");
        Ok(())
    }

    #[test]
    fn test_config_option() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

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

        // Test --config
        runner
            .run_command(&["--config", &config_path, "--help"])
            .assert()
            .success();

        // Test -c
        runner
            .run_command(&["-c", &config_path, "--help"])
            .assert()
            .success();

        println!("✅ Global config options test passed");
        Ok(())
    }
}

/// Pipeline Command Tests
mod pipeline_tests {
    use super::*;

    #[test]
    fn test_pipeline_help() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        runner
            .run_command(&["pipeline", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Execute a pipeline from a YAML file",
            ))
            .stdout(predicate::str::contains("--file"))
            .stdout(predicate::str::contains("--input"))
            .stdout(predicate::str::contains("--variables"));

        println!("✅ Pipeline help test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_required_options() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test that --file is required
        runner
            .run_command(&["pipeline"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("required"))
            .stderr(predicate::str::contains("--file"));

        println!("✅ Pipeline required options test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_all_options() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Create a test pipeline with correct YAML format
        let pipeline_content = r#"
name: test_pipeline
steps:
  - !Command
    name: test_step
    command: echo "Hello, world!"
"#;
        let pipeline_path = runner.create_test_pipeline(pipeline_content)?;

        // Create a test config
        let config_content = r#"
engines:
  - name: test_engine
    engine: openai
    connection:
      protocol: https
      hostname: api.openai.com
      port: 443
      request_path: /v1/chat/completions
    parameters: {}
"#;
        let config_path = runner.create_test_config(config_content)?;

        // Test all pipeline options (dry-run to avoid actual execution)
        runner
            .run_command(&[
                "pipeline",
                "--file",
                &pipeline_path,
                "--config",
                &config_path,
                "--input",
                "test input",
                "--variables",
                "key1=value1",
                "--variables",
                "key2=value2",
                "--force-fresh",
                "--run-id",
                "test-run-123",
                "--dry-run",
                "--json",
            ])
            .assert()
            .success(); // Should at least parse correctly

        println!("✅ Pipeline all options test passed");
        Ok(())
    }
}

/// Agent Command Tests
mod agent_tests {
    use super::*;

    #[test]
    fn test_agent_help() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        runner
            .run_command(&["agent", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Run agentic workflows"))
            .stdout(predicate::str::contains("--goal"))
            .stdout(predicate::str::contains("--model"))
            .stdout(predicate::str::contains("--max-iterations"));

        println!("✅ Agent help test passed");
        Ok(())
    }

    #[test]
    fn test_agent_options() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test agent with goal
        runner
            .run_command(&[
                "agent",
                "--goal",
                "Create a simple function",
                "--max-iterations",
                "5",
                "--reflection",
                "--dry-run",
            ])
            .assert()
            .success(); // Should at least parse correctly

        // Test agent with goal file
        let goal_content = r#"
goal_description = "Create a simple function"
max_iterations = 5
success_criteria = ["Function compiles without errors"]
"#;
        let goal_path = runner.create_test_goal(goal_content)?;

        runner
            .run_command(&[
                "agent",
                "--goal-file",
                &goal_path,
                "--model",
                "gpt-4o",
                "--gen-retries",
                "2",
                "--min-html-size",
                "1000",
                "--dry-run",
            ])
            .assert()
            .success(); // Should at least parse correctly

        println!("✅ Agent options test passed");
        Ok(())
    }
}

/// MCP Command Tests
mod mcp_tests {
    use super::*;

    #[test]
    fn test_mcp_help() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        runner
            .run_command(&["mcp", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("MCP server operations"))
            .stdout(predicate::str::contains("server"))
            .stdout(predicate::str::contains("client"));

        println!("✅ MCP help test passed");
        Ok(())
    }

    #[test]
    fn test_mcp_subcommands() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test mcp server help
        runner
            .run_command(&["mcp", "server", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Start MCP server"))
            .stdout(predicate::str::contains("--port"));

        // Test mcp client help
        runner
            .run_command(&["mcp", "client", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Connect as MCP client"))
            .stdout(predicate::str::contains("--server"));

        println!("✅ MCP subcommands test passed");
        Ok(())
    }
}

/// Neo4j Command Tests
mod neo4j_tests {
    use super::*;

    #[test]
    fn test_neo4j_help() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        runner
            .run_command(&["neo4j", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Neo4j database operations"))
            .stdout(predicate::str::contains("--generate-cypher"))
            .stdout(predicate::str::contains("--query"))
            .stdout(predicate::str::contains("--upsert-file"));

        println!("✅ Neo4j help test passed");
        Ok(())
    }

    #[test]
    fn test_neo4j_options() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test neo4j generate-cypher option
        runner
            .run_command(&["neo4j", "--generate-cypher", "--query", "Find all users"])
            .assert()
            .success(); // Should at least parse correctly

        // Test neo4j upsert-file option
        runner
            .run_command(&["neo4j", "--upsert-file", "test.txt"])
            .assert()
            .success(); // Should at least parse correctly

        println!("✅ Neo4j options test passed");
        Ok(())
    }
}

/// Tools Command Tests
mod tools_tests {
    use super::*;

    #[test]
    fn test_tools_help() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        runner
            .run_command(&["tools", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Direct tool access and management",
            ))
            .stdout(predicate::str::contains("list"))
            .stdout(predicate::str::contains("describe"))
            .stdout(predicate::str::contains("exec"))
            .stdout(predicate::str::contains("categories"));

        println!("✅ Tools help test passed");
        Ok(())
    }

    #[test]
    fn test_tools_subcommands() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test tools list help
        runner
            .run_command(&["tools", "list", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("List available tools"))
            .stdout(predicate::str::contains("--category"))
            .stdout(predicate::str::contains("--search"))
            .stdout(predicate::str::contains("--json"));

        // Test tools describe help
        runner
            .run_command(&["tools", "describe", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Describe a specific tool"))
            .stdout(predicate::str::contains("--schema"))
            .stdout(predicate::str::contains("--examples"));

        // Test tools exec help
        runner
            .run_command(&["tools", "exec", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Execute a tool directly"));

        // Test tools categories help
        runner
            .run_command(&["tools", "categories", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("List tool categories"));

        println!("✅ Tools subcommands test passed");
        Ok(())
    }
}

/// Engine Command Tests
mod engine_tests {
    use super::*;

    #[test]
    fn test_engine_help() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        runner
            .run_command(&["engine", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(
                "Engine management and configuration",
            ))
            .stdout(predicate::str::contains("list"))
            .stdout(predicate::str::contains("test"));

        println!("✅ Engine help test passed");
        Ok(())
    }

    #[test]
    fn test_engine_subcommands() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test engine list help
        runner
            .run_command(&["engine", "list", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("List available engines"))
            .stdout(predicate::str::contains("--json"));

        // Test engine test help
        runner
            .run_command(&["engine", "test", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Test engine connectivity"));

        println!("✅ Engine subcommands test passed");
        Ok(())
    }
}

/// Error Handling Tests
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_invalid_command() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test invalid top-level command
        runner
            .run_command(&["invalid-command"])
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("unrecognized").or(predicate::str::contains("unexpected")),
            );

        // Test invalid subcommand
        runner
            .run_command(&["pipeline", "invalid-subcommand"])
            .assert()
            .failure()
            .stderr(
                predicate::str::contains("unrecognized").or(predicate::str::contains("unexpected")),
            );

        println!("✅ Error handling test passed");
        Ok(())
    }

    #[test]
    fn test_missing_required_args() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test missing required args for various commands
        runner
            .run_command(&["pipeline"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("required"))
            .stderr(predicate::str::contains("--file"));

        runner
            .run_command(&["tools", "describe"])
            .assert()
            .failure()
            .stderr(predicate::str::contains("required"))
            .stderr(predicate::str::contains("tool"));

        println!("✅ Missing required args test passed");
        Ok(())
    }
}

/// Comprehensive CLI Options Coverage Tests
mod comprehensive_options_tests {
    use super::*;

    #[test]
    fn test_tools_command_comprehensive() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test tools list with all options
        runner
            .run_command(&[
                "tools",
                "list",
                "--category",
                "file",
                "--search",
                "read",
                "--json",
                "--available",
                "--detailed",
            ])
            .assert()
            .success();

        // Test tools describe with all options
        runner
            .run_command(&[
                "tools",
                "describe",
                "read_file",
                "--json",
                "--schema",
                "--examples",
            ])
            .assert()
            .success();

        // Test tools exec with various options
        runner
            .run_command(&["tools", "exec", "read_file", "--json-output"])
            .assert()
            .success(); // Should at least parse correctly

        // Test tools categories with json option
        runner
            .run_command(&["tools", "categories", "--json"])
            .assert()
            .success();

        println!("✅ Tools command comprehensive tests passed");
        Ok(())
    }

    #[test]
    fn test_engine_command_comprehensive() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test engine list with json option
        runner
            .run_command(&["engine", "list", "--json"])
            .assert()
            .success();

        // Test engine test command
        // Note: This will fail without a valid engine config, but should parse correctly
        runner
            .run_command(&["engine", "test", "nonexistent-engine"])
            .assert()
            .failure(); // Expected to fail due to nonexistent engine, but parsing should work

        println!("✅ Engine command comprehensive tests passed");
        Ok(())
    }

    #[test]
    fn test_complex_option_combinations() -> Result<()> {
        let runner = CliFunctionalTestRunner::new()?;

        // Test multiple global options combined
        runner
            .run_command(&["--config", "nonexistent.toml", "--help"])
            .assert()
            .success();

        // Test nested subcommands with options
        runner
            .run_command(&["tools", "list", "--json", "--category", "file"])
            .assert()
            .success();

        println!("✅ Complex option combinations tests passed");
        Ok(())
    }
}
