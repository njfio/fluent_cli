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
                    0 // Success but no code (shouldn't happen)
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

    /// Parse a command string into command and arguments using proper shell lexing
    ///
    /// Uses shlex-style parsing to handle quoted strings properly.
    /// This prevents shell injection via crafted arguments.
    fn parse_command(&self, command_str: &str) -> Result<(String, Vec<String>)> {
        let parts = Self::shell_lex(command_str)?;

        if parts.is_empty() {
            return Err(anyhow!("Empty command"));
        }

        let command = parts[0].clone();
        let args = parts[1..].to_vec();

        Ok((command, args))
    }

    /// Proper shell lexer that handles quotes and escapes safely
    ///
    /// This is a simplified shlex implementation that:
    /// - Handles single and double quoted strings
    /// - Handles backslash escapes
    /// - Does NOT execute shell expansions (no $(), ``, etc.)
    fn shell_lex(input: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut chars = input.chars().peekable();
        let mut in_single_quote = false;
        let mut in_double_quote = false;

        while let Some(c) = chars.next() {
            match c {
                // Backslash escape (only outside single quotes)
                '\\' if !in_single_quote => {
                    if let Some(next_char) = chars.next() {
                        // Only allow escaping specific characters for safety
                        match next_char {
                            '\\' | '"' | '\'' | ' ' | '\t' | 'n' | 't' => {
                                current_token.push(match next_char {
                                    'n' => '\n',
                                    't' => '\t',
                                    _ => next_char,
                                });
                            }
                            _ => {
                                return Err(anyhow!("Invalid escape sequence: \\{}", next_char));
                            }
                        }
                    } else {
                        return Err(anyhow!("Trailing backslash in command"));
                    }
                }

                // Single quote handling
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }

                // Double quote handling
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }

                // Whitespace (token separator when not in quotes)
                ' ' | '\t' if !in_single_quote && !in_double_quote => {
                    if !current_token.is_empty() {
                        tokens.push(current_token);
                        current_token = String::new();
                    }
                }

                // Shell metacharacters - reject these outside quotes
                '$' | '`' | '|' | '&' | ';' | '<' | '>' | '(' | ')' | '{' | '}' | '[' | ']'
                | '!' | '*' | '?' | '~' | '#'
                    if !in_single_quote && !in_double_quote =>
                {
                    return Err(anyhow!(
                        "Shell metacharacter '{}' not allowed outside quotes. \
                         Use quotes to include literal special characters.",
                        c
                    ));
                }

                // Newlines not allowed (prevents multi-line injection)
                '\n' | '\r' => {
                    return Err(anyhow!(
                        "Newlines not allowed in command string. Use run_script for multi-line commands."
                    ));
                }

                // Regular character
                _ => {
                    current_token.push(c);
                }
            }
        }

        // Check for unclosed quotes
        if in_single_quote {
            return Err(anyhow!("Unclosed single quote in command"));
        }
        if in_double_quote {
            return Err(anyhow!("Unclosed double quote in command"));
        }

        // Don't forget the last token
        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        Ok(tokens)
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

                // Parse the command string safely (this will reject metacharacters)
                let (command, args) = self.parse_command(command_str)?;

                // Validate that the command itself is allowed
                self.validate_command(&command)?;

                let result = self.execute_command_safe(&command, &args).await?;

                Ok(serde_json::to_string_pretty(&result)?)
            }

            "run_script" => {
                let script = parameters
                    .get("script")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'script' parameter"))?;

                // SECURITY: Execute each command separately to prevent shell injection
                // Do NOT pass to sh -c which would interpret metacharacters
                let mut combined_stdout = String::new();
                let mut combined_stderr = String::new();
                let mut final_exit_code = 0;
                let mut total_time_ms = 0u64;
                let mut all_success = true;

                for line in script.lines() {
                    let trimmed = line.trim();

                    // Skip empty lines and comments
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }

                    // Parse the command line safely
                    let (command, args) = self.parse_command(trimmed)?;

                    // Validate the command
                    self.validate_command(&command)?;

                    // Execute this individual command
                    let result = self.execute_command_safe(&command, &args).await?;

                    combined_stdout.push_str(&result.stdout);
                    if !result.stdout.ends_with('\n') && !result.stdout.is_empty() {
                        combined_stdout.push('\n');
                    }

                    if !result.stderr.is_empty() {
                        combined_stderr.push_str(&result.stderr);
                        if !result.stderr.ends_with('\n') {
                            combined_stderr.push('\n');
                        }
                    }

                    total_time_ms += result.execution_time_ms;

                    // Track success - stop on first failure
                    if !result.success {
                        all_success = false;
                        final_exit_code = result.exit_code;
                        break;
                    }
                }

                let result = CommandResult {
                    exit_code: final_exit_code,
                    stdout: combined_stdout,
                    stderr: combined_stderr,
                    execution_time_ms: total_time_ms,
                    success: all_success,
                };

                Ok(serde_json::to_string_pretty(&result)?)
            }

            "run_shell" => {
                // Execute a command via sh -c, allowing full shell features (pipes, redirects, etc.)
                // This is intentionally more permissive than run_command for cases where
                // shell features are genuinely needed (e.g., `curl ... | python3`)
                let command_str = parameters
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'command' parameter"))?;

                // Basic validation - check if the command string is reasonable
                if command_str.is_empty() {
                    return Err(anyhow!("Empty command"));
                }

                // Block obviously dangerous patterns
                let dangerous_patterns = [
                    "rm -rf /",
                    "rm -rf /*",
                    "mkfs",
                    "dd if=/dev/",
                    ":(){:|:&};:", // Fork bomb
                    "chmod -R 777 /",
                    "wget -O - | sh", // Blind script execution from unknown source
                    "curl | sh",      // Blind script execution
                ];

                let lower_cmd = command_str.to_lowercase();
                for pattern in &dangerous_patterns {
                    if lower_cmd.contains(pattern) {
                        return Err(anyhow!(
                            "Command contains dangerous pattern '{}' - blocked for safety",
                            pattern
                        ));
                    }
                }

                // Execute via sh -c
                let result = self
                    .execute_command_safe("sh", &["-c".to_string(), command_str.to_string()])
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
            "run_shell".to_string(),
            "run_script".to_string(),
            "get_working_directory".to_string(),
            "check_command_available".to_string(),
        ]
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        let description = match tool_name {
            "run_command" => "Execute a single shell command (safe mode, no pipes or redirects)",
            "run_shell" => "Execute a shell command via sh -c with full shell features (pipes, redirects, etc.). Use this when you need shell features like 'curl ... | python3' or 'echo text > file'",
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
                        // Parse to get the actual command name, not the full string
                        let (command, _args) = self.parse_command(command_str)?;
                        self.validate_command(&command)?;
                    } else {
                        return Err(anyhow!("Command parameter must be a string"));
                    }
                } else {
                    return Err(anyhow!("Missing 'command' parameter"));
                }
            }

            "run_shell" => {
                if let Some(command_value) = parameters.get("command") {
                    if command_value.as_str().is_none() {
                        return Err(anyhow!("Command parameter must be a string"));
                    }
                } else {
                    return Err(anyhow!("Missing 'command' parameter"));
                }
            }

            "run_script" => {
                if let Some(script_value) = parameters.get("script") {
                    if let Some(script_str) = script_value.as_str() {
                        // Validate each line of the script using proper parsing
                        for line in script_str.lines() {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                                // Parse safely to get the command name
                                let (command, _args) = self.parse_command(trimmed)?;
                                self.validate_command(&command)?;
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

    #[test]
    fn test_shell_lex_quoted_strings() {
        // Double quotes
        let tokens = ShellExecutor::shell_lex("echo \"hello world\"").unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);

        // Single quotes
        let tokens = ShellExecutor::shell_lex("echo 'hello world'").unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);

        // Mixed quotes
        let tokens = ShellExecutor::shell_lex("echo \"hello\" 'world'").unwrap();
        assert_eq!(tokens, vec!["echo", "hello", "world"]);

        // Quotes with special chars inside (should be allowed)
        let tokens = ShellExecutor::shell_lex("echo \"$HOME\"").unwrap();
        assert_eq!(tokens, vec!["echo", "$HOME"]);
    }

    #[test]
    fn test_shell_lex_escapes() {
        // Escaped space
        let tokens = ShellExecutor::shell_lex("echo hello\\ world").unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);

        // Escaped quote
        let tokens = ShellExecutor::shell_lex("echo \\\"test\\\"").unwrap();
        assert_eq!(tokens, vec!["echo", "\"test\""]);

        // Escaped backslash
        let tokens = ShellExecutor::shell_lex("echo \\\\").unwrap();
        assert_eq!(tokens, vec!["echo", "\\"]);
    }

    #[test]
    fn test_shell_lex_rejects_metacharacters() {
        // Command substitution
        assert!(ShellExecutor::shell_lex("echo $(whoami)").is_err());
        assert!(ShellExecutor::shell_lex("echo `whoami`").is_err());

        // Pipes and redirects
        assert!(ShellExecutor::shell_lex("echo test | cat").is_err());
        assert!(ShellExecutor::shell_lex("echo test > file").is_err());
        assert!(ShellExecutor::shell_lex("echo test >> file").is_err());
        assert!(ShellExecutor::shell_lex("cat < file").is_err());

        // Command chaining
        assert!(ShellExecutor::shell_lex("echo test; rm -rf /").is_err());
        assert!(ShellExecutor::shell_lex("echo test && rm file").is_err());
        assert!(ShellExecutor::shell_lex("echo test || rm file").is_err());

        // Background
        assert!(ShellExecutor::shell_lex("sleep 100 &").is_err());

        // Glob patterns
        assert!(ShellExecutor::shell_lex("ls *").is_err());
        assert!(ShellExecutor::shell_lex("ls ?.txt").is_err());

        // But these SHOULD work inside quotes
        assert!(ShellExecutor::shell_lex("echo \"test | cat\"").is_ok());
        assert!(ShellExecutor::shell_lex("echo 'test; rm -rf'").is_ok());
    }

    #[test]
    fn test_shell_lex_rejects_newlines() {
        assert!(ShellExecutor::shell_lex("echo test\nrm -rf /").is_err());
        assert!(ShellExecutor::shell_lex("echo test\r\nrm file").is_err());
    }

    #[test]
    fn test_shell_lex_unclosed_quotes() {
        assert!(ShellExecutor::shell_lex("echo \"unclosed").is_err());
        assert!(ShellExecutor::shell_lex("echo 'unclosed").is_err());
    }

    #[test]
    fn test_shell_lex_trailing_backslash() {
        assert!(ShellExecutor::shell_lex("echo test\\").is_err());
    }

    #[test]
    fn test_shell_lex_invalid_escape() {
        assert!(ShellExecutor::shell_lex("echo \\x").is_err());
    }

    #[tokio::test]
    async fn test_run_shell_with_pipe() {
        let temp_dir = tempdir().expect("Failed to create temp directory");

        // For run_shell, we just need sh in the allowed list since it executes via sh -c
        let mut config = ToolExecutionConfig::default();
        config.allowed_commands = vec!["sh".to_string()];

        let executor = ShellExecutor::new(config, temp_dir.path().to_path_buf());

        // Test a simple pipe command
        let mut params = HashMap::new();
        params.insert(
            "command".to_string(),
            serde_json::Value::String("echo 'hello world' | tr 'a-z' 'A-Z'".to_string()),
        );

        let result = executor
            .execute_tool("run_shell", &params)
            .await
            .expect("run_shell execution failed");

        let command_result: CommandResult =
            serde_json::from_str(&result).expect("Failed to parse command result");
        assert!(command_result.success);
        assert!(command_result.stdout.contains("HELLO WORLD"));
        assert_eq!(command_result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_run_shell_blocks_dangerous_patterns() {
        let temp_dir = tempdir().expect("Failed to create temp directory");

        let mut config = ToolExecutionConfig::default();
        config.allowed_commands = vec!["sh".to_string()];

        let executor = ShellExecutor::new(config, temp_dir.path().to_path_buf());

        // Test that dangerous patterns are blocked
        let mut params = HashMap::new();
        params.insert(
            "command".to_string(),
            serde_json::Value::String("rm -rf /".to_string()),
        );

        let result = executor.execute_tool("run_shell", &params).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("dangerous pattern"));
    }
}
