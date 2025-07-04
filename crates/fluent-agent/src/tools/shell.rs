use super::{validation, ToolExecutionConfig, ToolExecutor};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Shell command executor that provides safe command execution
pub struct ShellExecutor {
    config: ToolExecutionConfig,
    working_directory: PathBuf,
}

impl ShellExecutor {
    /// Create a new shell executor with the given configuration and working directory
    pub fn new(config: ToolExecutionConfig, working_directory: PathBuf) -> Self {
        Self {
            config,
            working_directory,
        }
    }

    /// Create a shell executor with default configuration
    pub fn with_defaults(working_directory: PathBuf) -> Self {
        Self::new(ToolExecutionConfig::default(), working_directory)
    }

    /// Validate that a command is safe to execute
    fn validate_command(&self, command: &str) -> Result<()> {
        validation::validate_command(command, &self.config.allowed_commands)
    }

    /// Execute a command safely with timeout and output limits
    async fn execute_command_safe(&self, command: &str, args: &[String]) -> Result<CommandResult> {
        let start_time = std::time::Instant::now();

        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(&self.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);

        let output = timeout(timeout_duration, cmd.output())
            .await
            .map_err(|_| {
                anyhow!(
                    "Command timed out after {} seconds",
                    self.config.timeout_seconds
                )
            })?
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        let execution_time = start_time.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let sanitized_stdout = validation::sanitize_output(&stdout, self.config.max_output_size);
        let sanitized_stderr = validation::sanitize_output(&stderr, self.config.max_output_size);

        Ok(CommandResult {
            exit_code: output.status.code().unwrap_or_else(|| {
                // Different error codes for different failure scenarios
                if output.status.success() {
                    0  // Success but no code (shouldn't happen)
                } else {
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        // Check if process was terminated by signal
                        if let Some(signal) = output.status.signal() {
                            return -signal; // Negative signal number
                        }
                    }
                    -1 // Generic failure
                }
            }),
            stdout: sanitized_stdout,
            stderr: sanitized_stderr,
            execution_time_ms: execution_time.as_millis() as u64,
            success: output.status.success(),
        })
    }

    /// Parse a command string into command and arguments
    fn parse_command(&self, command_str: &str) -> Result<(String, Vec<String>)> {
        let parts: Vec<&str> = command_str.split_whitespace().collect();

        if parts.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        let command = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok((command, args))
    }
}

#[async_trait]
impl ToolExecutor for ShellExecutor {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        match tool_name {
            "run_command" => {
                let command_str = parameters
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'command' parameter"))?;

                self.validate_command(command_str)?;

                let (command, args) = self.parse_command(command_str)?;
                let result = self.execute_command_safe(&command, &args).await?;

                Ok(serde_json::to_string_pretty(&result)?)
            }

            "run_script" => {
                let script = parameters
                    .get("script")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'script' parameter"))?;

                // For security, we'll execute the script through sh -c
                // but still validate against allowed commands
                for line in script.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        self.validate_command(trimmed)?;
                    }
                }

                let result = self
                    .execute_command_safe("sh", &["-c".to_string(), script.to_string()])
                    .await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "get_working_directory" => Ok(self.working_directory.to_string_lossy().to_string()),

            "check_command_available" => {
                let command = parameters
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'command' parameter"))?;

                let result = self
                    .execute_command_safe("which", &[command.to_string()])
                    .await?;

                let available = CommandAvailability {
                    command: command.to_string(),
                    available: result.success,
                    path: if result.success {
                        Some(result.stdout.trim().to_string())
                    } else {
                        None
                    },
                };

                Ok(serde_json::to_string_pretty(&available)?)
            }

            _ => Err(anyhow!("Unknown shell tool: {}", tool_name)),
        }
    }

    fn get_available_tools(&self) -> Vec<String> {
        vec![
            "run_command".to_string(),
            "run_script".to_string(),
            "get_working_directory".to_string(),
            "check_command_available".to_string(),
        ]
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        let description = match tool_name {
            "run_command" => "Execute a single shell command",
            "run_script" => "Execute a multi-line shell script",
            "get_working_directory" => "Get the current working directory",
            "check_command_available" => "Check if a command is available in the system PATH",
            _ => return None,
        };

        Some(description.to_string())
    }

    fn validate_tool_request(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // Check if tool is available
        if !self.get_available_tools().contains(&tool_name.to_string()) {
            return Err(anyhow!("Tool '{}' is not available", tool_name));
        }

        match tool_name {
            "run_command" => {
                if let Some(command_value) = parameters.get("command") {
                    if let Some(command_str) = command_value.as_str() {
                        self.validate_command(command_str)?;
                    } else {
                        return Err(anyhow!("Command parameter must be a string"));
                    }
                } else {
                    return Err(anyhow!("Missing 'command' parameter"));
                }
            }

            "run_script" => {
                if let Some(script_value) = parameters.get("script") {
                    if let Some(script_str) = script_value.as_str() {
                        // Validate each line of the script
                        for line in script_str.lines() {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                                self.validate_command(trimmed)?;
                            }
                        }
                    } else {
                        return Err(anyhow!("Script parameter must be a string"));
                    }
                } else {
                    return Err(anyhow!("Missing 'script' parameter"));
                }
            }

            "check_command_available" => {
                if parameters.get("command").is_none() {
                    return Err(anyhow!("Missing 'command' parameter"));
                }
            }

            _ => {} // Other tools don't need special validation
        }

        Ok(())
    }
}

/// Result of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub success: bool,
}

/// Information about command availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAvailability {
    pub command: String,
    pub available: bool,
    pub path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_run_command() {
        let temp_dir = tempdir().expect("Failed to create temp directory");

        let mut config = ToolExecutionConfig::default();
        config.allowed_commands = vec!["echo".to_string(), "ls".to_string()];

        let executor = ShellExecutor::new(config, temp_dir.path().to_path_buf());

        let mut params = HashMap::new();
        params.insert(
            "command".to_string(),
            serde_json::Value::String("echo Hello World".to_string()),
        );

        let result = executor
            .execute_tool("run_command", &params)
            .await
            .expect("Command execution failed");

        let command_result: CommandResult =
            serde_json::from_str(&result).expect("Failed to parse command result");
        assert!(command_result.success);
        assert!(command_result.stdout.contains("Hello World"));
        assert_eq!(command_result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_command_validation() {
        let temp_dir = tempdir().expect("Failed to create temp directory");

        let mut config = ToolExecutionConfig::default();
        config.allowed_commands = vec!["echo".to_string()];

        let executor = ShellExecutor::new(config, temp_dir.path().to_path_buf());

        // Valid command
        let mut valid_params = HashMap::new();
        valid_params.insert(
            "command".to_string(),
            serde_json::Value::String("echo test".to_string()),
        );
        assert!(executor
            .validate_tool_request("run_command", &valid_params)
            .is_ok());

        // Invalid command
        let mut invalid_params = HashMap::new();
        invalid_params.insert(
            "command".to_string(),
            serde_json::Value::String("rm -rf /".to_string()),
        );
        assert!(executor
            .validate_tool_request("run_command", &invalid_params)
            .is_err());
    }

    #[tokio::test]
    async fn test_get_working_directory() {
        let temp_dir = tempdir().unwrap();
        let executor = ShellExecutor::with_defaults(temp_dir.path().to_path_buf());

        let result = executor
            .execute_tool("get_working_directory", &HashMap::new())
            .await
            .unwrap();
        assert_eq!(result, temp_dir.path().to_string_lossy());
    }

    #[tokio::test]
    async fn test_check_command_available() {
        let temp_dir = tempdir().unwrap();
        let executor = ShellExecutor::with_defaults(temp_dir.path().to_path_buf());

        let mut params = HashMap::new();
        params.insert(
            "command".to_string(),
            serde_json::Value::String("echo".to_string()),
        );

        let result = executor
            .execute_tool("check_command_available", &params)
            .await
            .unwrap();

        let availability: CommandAvailability = serde_json::from_str(&result).unwrap();
        assert_eq!(availability.command, "echo");
        assert!(availability.available); // echo should be available on most systems
    }

    #[tokio::test]
    async fn test_run_script() {
        let temp_dir = tempdir().expect("Failed to create temp directory");

        let mut config = ToolExecutionConfig::default();
        config.allowed_commands = vec!["echo".to_string(), "ls".to_string()];

        let executor = ShellExecutor::new(config, temp_dir.path().to_path_buf());

        let script = r#"
            echo "Line 1"
            echo "Line 2"
        "#;

        let mut params = HashMap::new();
        params.insert(
            "script".to_string(),
            serde_json::Value::String(script.to_string()),
        );

        let result = executor
            .execute_tool("run_script", &params)
            .await
            .expect("Script execution failed");

        let command_result: CommandResult =
            serde_json::from_str(&result).expect("Failed to parse command result");
        assert!(command_result.success);
        assert!(command_result.stdout.contains("Line 1"));
        assert!(command_result.stdout.contains("Line 2"));
    }

    #[test]
    fn test_parse_command() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let executor = ShellExecutor::with_defaults(temp_dir.path().to_path_buf());

        let (command, args) = executor
            .parse_command("echo hello world")
            .expect("Failed to parse command");
        assert_eq!(command, "echo");
        assert_eq!(args, vec!["hello", "world"]);

        let (command, args) = executor
            .parse_command("ls")
            .expect("Failed to parse command");
        assert_eq!(command, "ls");
        assert!(args.is_empty());
    }
}
