//! Comprehensive Option Tests for Fluent CLI
//! 
//! This test suite validates all individual CLI options and their combinations
//! to ensure complete coverage of the CLI interface.

use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

/// Test runner for comprehensive option tests
pub struct ComprehensiveOptionTestRunner {
    temp_dir: TempDir,
}

impl ComprehensiveOptionTestRunner {
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

/// Global Option Tests
mod global_option_tests {
    use super::*;

    #[test]
    fn test_help_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test long form help
        runner.run_command(&["--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("fluent"))
            .stdout(predicate::str::contains("A powerful CLI for interacting with various AI engines"));
        
        // Test short form help
        runner.run_command(&["-h"])
            .assert()
            .success()
            .stdout(predicate::str::contains("fluent"));
        
        println!("✅ Global help options test passed");
        Ok(())
    }

    #[test]
    fn test_version_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test long form version
        runner.run_command(&["--version"])
            .assert()
            .success()
            .stdout(predicate::str::contains("0.1.0"));
        
        // Test short form version
        runner.run_command(&["-V"])
            .assert()
            .success()
            .stdout(predicate::str::contains("0.1.0"));
        
        println!("✅ Global version options test passed");
        Ok(())
    }

    #[test]
    fn test_config_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
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
        
        // Test long form config
        runner.run_command(&["--config", &config_path, "--help"])
            .assert()
            .success();
        
        // Test short form config
        runner.run_command(&["-c", &config_path, "--help"])
            .assert()
            .success();
        
        println!("✅ Global config options test passed");
        Ok(())
    }
}

/// Pipeline Command Option Tests
mod pipeline_option_tests {
    use super::*;

    #[test]
    fn test_pipeline_file_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Create a test pipeline
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
"#;
        let pipeline_path = runner.create_test_pipeline(pipeline_content)?;
        
        // Test long form file option
        runner.run_command(&["pipeline", "--file", &pipeline_path, "--dry-run"])
            .assert()
            .success();
        
        // Test short form file option
        runner.run_command(&["pipeline", "-f", &pipeline_path, "--dry-run"])
            .assert()
            .success();
        
        println!("✅ Pipeline file options test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_input_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Create a test pipeline
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
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
        
        // Test long form input option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--config", &config_path,
            "--input", "test input",
            "--dry-run"
        ])
        .assert()
        .success();
        
        // Test short form input option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--config", &config_path,
            "-i", "test input",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Pipeline input options test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_variables_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Create a test pipeline
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
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
        
        // Test long form variables option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--config", &config_path,
            "--variables", "key1=value1",
            "--variables", "key2=value2",
            "--dry-run"
        ])
        .assert()
        .success();
        
        // Test short form variables option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--config", &config_path,
            "-v", "key1=value1",
            "-v", "key2=value2",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Pipeline variables options test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_force_fresh_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Create a test pipeline
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
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
        
        // Test force fresh option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--config", &config_path,
            "--force-fresh",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Pipeline force fresh option test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_run_id_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Create a test pipeline
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
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
        
        // Test run id option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--config", &config_path,
            "--run-id", "test-run-123",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Pipeline run id option test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_dry_run_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Create a test pipeline
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
"#;
        let pipeline_path = runner.create_test_pipeline(pipeline_content)?;
        
        // Test dry run option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Pipeline dry run option test passed");
        Ok(())
    }

    #[test]
    fn test_pipeline_json_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Create a test pipeline
        let pipeline_content = r#"
name: test_pipeline
steps:
  - name: test_step
    engine: test_engine
    request: "Hello, world!"
"#;
        let pipeline_path = runner.create_test_pipeline(pipeline_content)?;
        
        // Test json option
        runner.run_command(&[
            "pipeline",
            "--file", &pipeline_path,
            "--json",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Pipeline json option test passed");
        Ok(())
    }
}

/// Agent Command Option Tests
mod agent_option_tests {
    use super::*;

    #[test]
    fn test_agent_agentic_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test agentic option
        runner.run_command(&[
            "agent",
            "--agentic",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent agentic option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_preview_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test preview option
        runner.run_command(&[
            "agent",
            "--preview",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        // Test preview-path option
        runner.run_command(&[
            "agent",
            "--preview-path", "examples/web_tetris.html",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent preview options test passed");
        Ok(())
    }

    #[test]
    fn test_agent_goal_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test long form goal option
        runner.run_command(&[
            "agent",
            "--goal", "Create a simple function",
            "--dry-run"
        ])
        .assert()
        .success();
        
        // Test short form goal option
        runner.run_command(&[
            "agent",
            "-g", "Create a simple function",
            "--dry-run"
        ])
        .assert()
        .success();
        
        // Create a test goal file
        let goal_content = r#"
goal_description = "Create a simple function"
max_iterations = 5
success_criteria = ["Function compiles without errors"]
"#;
        let goal_path = runner.create_test_goal(goal_content)?;
        
        // Test goal-file option
        runner.run_command(&[
            "agent",
            "--goal-file", &goal_path,
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent goal options test passed");
        Ok(())
    }

    #[test]
    fn test_agent_model_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test model option
        runner.run_command(&[
            "agent",
            "--model", "gpt-4o",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent model option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_max_iterations_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test long form max-iterations option
        runner.run_command(&[
            "agent",
            "--max-iterations", "5",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent max iterations option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_reflection_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test reflection option
        runner.run_command(&[
            "agent",
            "--reflection",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent reflection option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_enable_tools_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test enable-tools option
        runner.run_command(&[
            "agent",
            "--enable-tools",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent enable tools option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_config_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test agent-config option
        runner.run_command(&[
            "agent",
            "--agent-config", "agent_config.json",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent config options test passed");
        Ok(())
    }

    #[test]
    fn test_agent_dry_run_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test dry-run option
        runner.run_command(&[
            "agent",
            "--dry-run",
            "--goal", "Test goal"
        ])
        .assert()
        .success();
        
        println!("✅ Agent dry run option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_gen_retries_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test gen-retries option
        runner.run_command(&[
            "agent",
            "--gen-retries", "2",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent gen retries option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_min_html_size_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test min-html-size option
        runner.run_command(&[
            "agent",
            "--min-html-size", "1000",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent min html size option test passed");
        Ok(())
    }

    #[test]
    fn test_agent_task_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test long form task option
        runner.run_command(&[
            "agent",
            "--task", "Create a function",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        // Test short form task option
        runner.run_command(&[
            "agent",
            "-t", "Create a function",
            "--goal", "Test goal",
            "--dry-run"
        ])
        .assert()
        .success();
        
        println!("✅ Agent task options test passed");
        Ok(())
    }
}

/// MCP Command Option Tests
mod mcp_option_tests {
    use super::*;

    #[test]
    fn test_mcp_server_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test server help
        runner.run_command(&["mcp", "server", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--port"))
            .stdout(predicate::str::contains("-p"));
        
        // Test port option
        runner.run_command(&["mcp", "server", "--port", "8081"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test short form port option
        runner.run_command(&["mcp", "server", "-p", "8082"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ MCP server options test passed");
        Ok(())
    }

    #[test]
    fn test_mcp_client_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test client help
        runner.run_command(&["mcp", "client", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--server"))
            .stdout(predicate::str::contains("-s"));
        
        // Test server option
        runner.run_command(&["mcp", "client", "--server", "http://localhost:8080"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test short form server option
        runner.run_command(&["mcp", "client", "-s", "http://localhost:8080"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ MCP client options test passed");
        Ok(())
    }
}

/// Neo4j Command Option Tests
mod neo4j_option_tests {
    use super::*;

    #[test]
    fn test_neo4j_generate_cypher_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test generate-cypher option
        runner.run_command(&["neo4j", "--generate-cypher", "--query", "Find all users"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Neo4j generate cypher option test passed");
        Ok(())
    }

    #[test]
    fn test_neo4j_query_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test long form query option
        runner.run_command(&["neo4j", "--query", "MATCH (n) RETURN n LIMIT 10"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test short form query option
        runner.run_command(&["neo4j", "-q", "MATCH (n) RETURN n LIMIT 10"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Neo4j query options test passed");
        Ok(())
    }

    #[test]
    fn test_neo4j_upsert_file_option() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test upsert-file option
        runner.run_command(&["neo4j", "--upsert-file", "test.txt"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Neo4j upsert file option test passed");
        Ok(())
    }
}

/// Tools Command Option Tests
mod tools_option_tests {
    use super::*;

    #[test]
    fn test_tools_list_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test list help
        runner.run_command(&["tools", "list", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--category"))
            .stdout(predicate::str::contains("--search"))
            .stdout(predicate::str::contains("--json"))
            .stdout(predicate::str::contains("--available"))
            .stdout(predicate::str::contains("--detailed"));
        
        // Test category option
        runner.run_command(&["tools", "list", "--category", "file"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test search option
        runner.run_command(&["tools", "list", "--search", "read"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test json option
        runner.run_command(&["tools", "list", "--json"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test available option
        runner.run_command(&["tools", "list", "--available"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test detailed option
        runner.run_command(&["tools", "list", "--detailed"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test all options combined
        runner.run_command(&["tools", "list", "--category", "file", "--search", "read", "--json", "--available", "--detailed"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Tools list options test passed");
        Ok(())
    }

    #[test]
    fn test_tools_describe_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test describe help
        runner.run_command(&["tools", "describe", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--json"))
            .stdout(predicate::str::contains("--schema"))
            .stdout(predicate::str::contains("--examples"));
        
        // Test tool argument
        runner.run_command(&["tools", "describe", "read_file"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test json option
        runner.run_command(&["tools", "describe", "read_file", "--json"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test schema option
        runner.run_command(&["tools", "describe", "read_file", "--schema"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test examples option
        runner.run_command(&["tools", "describe", "read_file", "--examples"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test all options combined
        runner.run_command(&["tools", "describe", "read_file", "--json", "--schema", "--examples"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Tools describe options test passed");
        Ok(())
    }

    #[test]
    fn test_tools_exec_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test exec help
        runner.run_command(&["tools", "exec", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--json-output"));
        
        // Test tool argument
        runner.run_command(&["tools", "exec", "read_file"])
            .assert()
            .success(); // Should at least parse correctly
        
        // Test json-output option
        runner.run_command(&["tools", "exec", "read_file", "--json-output"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Tools exec options test passed");
        Ok(())
    }

    #[test]
    fn test_tools_categories_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test categories help
        runner.run_command(&["tools", "categories", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--json"));
        
        // Test json option
        runner.run_command(&["tools", "categories", "--json"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Tools categories options test passed");
        Ok(())
    }
}

/// Engine Command Option Tests
mod engine_option_tests {
    use super::*;

    #[test]
    fn test_engine_list_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test list help
        runner.run_command(&["engine", "list", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--json"));
        
        // Test json option
        runner.run_command(&["engine", "list", "--json"])
            .assert()
            .success(); // Should at least parse correctly
        
        println!("✅ Engine list options test passed");
        Ok(())
    }

    #[test]
    fn test_engine_test_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test test help
        runner.run_command(&["engine", "test", "--help"])
            .assert()
            .success();
        
        // Test engine argument
        runner.run_command(&["engine", "test", "test-engine"])
            .assert()
            .failure(); // Will fail without valid config, but should parse correctly
        
        println!("✅ Engine test options test passed");
        Ok(())
    }
}

/// Complex Option Combination Tests
mod complex_combination_tests {
    use super::*;

    #[test]
    fn test_multiple_global_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
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
        
        // Test multiple global options combined
        runner.run_command(&["--config", &config_path, "--help"])
            .assert()
            .success();
        
        println!("✅ Multiple global options test passed");
        Ok(())
    }

    #[test]
    fn test_nested_subcommands_with_options() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test nested subcommands with options
        runner.run_command(&["tools", "list", "--json", "--category", "file"])
            .assert()
            .success();
        
        println!("✅ Nested subcommands with options test passed");
        Ok(())
    }

    #[test]
    fn test_all_major_commands_help() -> Result<()> {
        let runner = ComprehensiveOptionTestRunner::new()?;
        
        // Test all major commands help
        let commands = [
            ["pipeline", "--help"],
            ["agent", "--help"],
            ["mcp", "--help"],
            ["neo4j", "--help"],
            ["tools", "--help"],
            ["engine", "--help"],
        ];
        
        for cmd in &commands {
            runner.run_command(cmd)
                .assert()
                .success();
        }
        
        println!("✅ All major commands help test passed");
        Ok(())
    }
}