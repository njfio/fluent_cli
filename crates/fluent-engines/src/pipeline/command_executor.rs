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

/// Handles execution of command and shell command steps
pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute a regular command
    pub async fn execute_command(
        command: &str,
        save_output: &Option<String>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing command: {}", command);
        
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;

        let stdout = String::from_utf8(output.stdout)?;
        let mut result = HashMap::new();
        
        if let Some(key) = save_output {
            result.insert(key.clone(), stdout.trim().to_string());
        }
        
        Ok(result)
    }

    /// Execute a shell command
    pub async fn execute_shell_command(
        command: &str,
        save_output: &Option<String>,
    ) -> Result<HashMap<String, String>, Error> {
        debug!("Executing shell command: {}", command);
        
        let output = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await?;

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
