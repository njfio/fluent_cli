//! Tool Execution Framework with Behavioral Reminders
//!
//! This module provides a comprehensive tool execution framework for the fluent-agent system.
//! A key feature is the automatic inclusion of **behavioral reminders** in tool outputs to guide
//! the agent's next actions.
//!
//! ## Behavioral Reminders
//!
//! When tools are executed through the `ToolRegistry`, the results are automatically enhanced with
//! contextual reminders that guide the agent based on:
//! - The specific tool that was executed
//! - Whether the execution succeeded or failed
//!
//! ### Success Reminders
//! These guide the agent on what to do next after a successful operation:
//! - After `read_file`: Analyze content before making changes
//! - After `write_file`: Verify the file works by running tests
//! - After `cargo_build`: Run tests to ensure functionality
//! - After `cargo_test`: Review output and move forward if tests pass
//!
//! ### Failure Reminders
//! These help the agent recover from errors:
//! - After failed commands: Analyze errors and try alternatives
//! - After compilation failures: Read error messages and fix specific issues
//! - After file operation failures: Check paths and permissions
//!
//! ### Implementation
//! Behavioral reminders are implemented in `validation::append_behavioral_reminder()` and
//! automatically applied by `ToolRegistry::execute_tool()`.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub mod filesystem;
pub mod rust_compiler;
pub mod shell;
pub mod string_replace_editor;
pub mod workflow;

#[cfg(test)]
mod string_replace_editor_tests;

pub use filesystem::FileSystemExecutor;
pub use rust_compiler::RustCompilerExecutor;
pub use shell::ShellExecutor;
pub use string_replace_editor::StringReplaceEditor;
pub use workflow::WorkflowExecutor;

/// Trait for tool executors that can perform actions in the environment
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool with the given parameters
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String>;

    /// Get the list of available tools this executor provides
    fn get_available_tools(&self) -> Vec<String>;

    /// Get the description of a specific tool
    fn get_tool_description(&self, tool_name: &str) -> Option<String>;

    /// Validate that a tool execution request is safe and allowed
    fn validate_tool_request(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<()>;
}

/// Registry for managing multiple tool executors
pub struct ToolRegistry {
    executors: HashMap<String, Arc<dyn ToolExecutor>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }

    /// Register a tool executor with a given name
    pub fn register(&mut self, name: String, executor: Arc<dyn ToolExecutor>) {
        self.executors.insert(name, executor);
    }

    /// Execute a tool by finding the appropriate executor
    ///
    /// This method finds the appropriate executor, validates the request, executes the tool,
    /// and appends behavioral reminders to guide the agent's next actions.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        // Normalize tool name - map common aliases to actual registered names
        let normalized_name = match tool_name.to_lowercase().as_str() {
            // Shell command aliases
            "run_command" | "execute_command" | "command" | "bash" | "exec" => "shell",
            // File system aliases
            "file_system" | "fs" | "file" | "files" => "filesystem",
            // Read/write file aliases (map to filesystem)
            "read_file" | "write_file" | "list_directory" | "create_directory" | "file_exists" => {
                "filesystem"
            }
            // Rust compiler aliases
            "compiler" | "cargo" | "rustc" | "cargo_build" | "cargo_test" | "cargo_check"
            | "cargo_clippy" => "rust_compiler",
            // String replace aliases
            "str_replace" | "replace" | "edit" | "string_replace_editor" => "string_replace",
            // Use original name if no alias matches
            _ => tool_name,
        };

        // Find the executor that provides this tool (using normalized name)
        for executor in self.executors.values() {
            if executor
                .get_available_tools()
                .contains(&normalized_name.to_string())
            {
                // Validate the request first (use normalized name)
                executor.validate_tool_request(normalized_name, parameters)?;

                // Execute the tool (use normalized name)
                let result = executor.execute_tool(normalized_name, parameters).await;

                // Enhance the result with behavioral reminders
                return match result {
                    Ok(output) => {
                        let enhanced_output =
                            validation::append_behavioral_reminder(normalized_name, output, true);
                        Ok(enhanced_output)
                    }
                    Err(e) => {
                        // Even for errors, provide a reminder to guide recovery
                        let error_msg = e.to_string();
                        let enhanced_error = validation::append_behavioral_reminder(
                            normalized_name,
                            error_msg.clone(),
                            false,
                        );
                        // Return the enhanced error message
                        Err(anyhow::anyhow!("{}", enhanced_error))
                    }
                };
            }
        }

        Err(anyhow::anyhow!(
            "Tool '{}' not found in any registered executor (tried alias: '{}')",
            tool_name,
            normalized_name
        ))
    }

    /// Get all available tools across all executors
    pub fn get_all_available_tools(&self) -> Vec<ToolInfo> {
        let mut tools = Vec::new();

        for (executor_name, executor) in &self.executors {
            for tool_name in executor.get_available_tools() {
                tools.push(ToolInfo {
                    name: tool_name.clone(),
                    executor: executor_name.clone(),
                    description: executor
                        .get_tool_description(&tool_name)
                        .unwrap_or_else(|| "No description available".to_string()),
                });
            }
        }

        tools
    }

    /// Check if a tool is available
    pub fn is_tool_available(&self, tool_name: &str) -> bool {
        self.executors.values().any(|executor| {
            executor
                .get_available_tools()
                .contains(&tool_name.to_string())
        })
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a tool registry with all standard tools configured
    pub fn with_standard_tools(config: &crate::config::ToolConfig) -> Self {
        let mut registry = Self::new();

        // Register file system executor
        if config.file_operations {
            let tool_config = ToolExecutionConfig {
                timeout_seconds: 30,
                max_output_size: 1024 * 1024, // 1MB
                allowed_paths: config.allowed_paths.clone().unwrap_or_else(|| {
                    vec![
                        "./".to_string(),
                        "./src".to_string(),
                        "./examples".to_string(),
                        "./crates".to_string(),
                    ]
                }),
                allowed_commands: config
                    .allowed_commands
                    .clone()
                    .unwrap_or_else(|| vec!["cargo".to_string(), "rustc".to_string()]),
                read_only: false,
            };

            let fs_executor = Arc::new(FileSystemExecutor::new(tool_config));
            registry.register("filesystem".to_string(), fs_executor);

            // Register string replace editor (also requires file operations)
            let string_replace_config = string_replace_editor::StringReplaceConfig {
                allowed_paths: config.allowed_paths.clone().unwrap_or_else(|| {
                    vec![
                        "./".to_string(),
                        "./src".to_string(),
                        "./examples".to_string(),
                        "./crates".to_string(),
                    ]
                }),
                max_file_size: 10 * 1024 * 1024, // 10MB
                backup_enabled: true,
                case_sensitive: true,
                max_replacements: 100,
            };

            let string_replace_executor =
                Arc::new(StringReplaceEditor::with_config(string_replace_config));
            registry.register("string_replace".to_string(), string_replace_executor);
        }

        // Register shell executor
        if config.shell_commands {
            let shell_config = ToolExecutionConfig {
                timeout_seconds: 60,
                max_output_size: 1024 * 1024, // 1MB
                allowed_paths: config
                    .allowed_paths
                    .clone()
                    .unwrap_or_else(|| vec!["./".to_string()]),
                allowed_commands: config.allowed_commands.clone().unwrap_or_else(|| {
                    vec![
                        "cargo".to_string(),
                        "rustc".to_string(),
                        "ls".to_string(),
                        "cat".to_string(),
                        "echo".to_string(),
                    ]
                }),
                read_only: false,
            };

            let working_dir =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let shell_executor = Arc::new(ShellExecutor::new(shell_config, working_dir));
            registry.register("shell".to_string(), shell_executor);
        }

        // Register Rust compiler executor
        if config.rust_compiler {
            let rust_compiler_executor = Arc::new(RustCompilerExecutor::with_defaults(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            ));
            registry.register("rust_compiler".to_string(), rust_compiler_executor);
        }

        registry
    }
}

/// Information about an available tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub executor: String,
    pub description: String,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Configuration for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionConfig {
    pub timeout_seconds: u64,
    pub max_output_size: usize,
    pub allowed_paths: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub read_only: bool,
}

impl Default for ToolExecutionConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_output_size: 1024 * 1024, // 1MB
            allowed_paths: vec![
                "./".to_string(),
                "./src".to_string(),
                "./examples".to_string(),
                "./tests".to_string(),
            ],
            allowed_commands: vec![
                "cargo build".to_string(),
                "cargo test".to_string(),
                "cargo check".to_string(),
                "cargo clippy".to_string(),
            ],
            read_only: false,
        }
    }
}

/// Configuration for tool capabilities and limits with JSON schema support
///
/// This struct provides comprehensive capability configuration for tool execution
/// including file size limits, path restrictions, command allowlists, and resource limits.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ToolCapabilityConfig {
    /// Maximum file size in bytes for file operations
    #[serde(default = "default_max_file_size")]
    #[schemars(
        description = "Maximum file size in bytes that can be read or written (default: 10MB)"
    )]
    pub max_file_size: usize,

    /// Allowed root paths for file operations
    #[serde(default)]
    #[schemars(
        description = "List of allowed root paths for file operations. Paths outside these directories will be rejected."
    )]
    pub allowed_paths: Vec<String>,

    /// Command allowlist for shell operations
    #[serde(default)]
    #[schemars(
        description = "List of allowed commands for shell execution. Only commands in this list can be executed."
    )]
    pub allowed_commands: Vec<String>,

    /// Maximum output size in bytes
    #[serde(default = "default_max_output_size")]
    #[schemars(
        description = "Maximum output size in bytes for tool execution results (default: 1MB)"
    )]
    pub max_output_size: usize,

    /// Timeout in seconds for tool execution
    #[serde(default = "default_timeout")]
    #[schemars(description = "Timeout in seconds for tool execution (default: 30s)")]
    pub timeout_seconds: u64,

    /// Whether the tool can make network requests
    #[serde(default)]
    #[schemars(
        description = "Whether the tool is allowed to make network requests (default: false)"
    )]
    pub allow_network: bool,

    /// Whether file operations are read-only
    #[serde(default)]
    #[schemars(
        description = "Whether file operations are restricted to read-only mode (default: false)"
    )]
    pub read_only: bool,

    /// Maximum number of concurrent tool executions
    #[serde(default = "default_max_concurrent")]
    #[schemars(description = "Maximum number of concurrent tool executions allowed (default: 5)")]
    pub max_concurrent_executions: usize,
}

fn default_max_file_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_max_output_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_timeout() -> u64 {
    30
}

fn default_max_concurrent() -> usize {
    5
}

impl Default for ToolCapabilityConfig {
    fn default() -> Self {
        Self {
            max_file_size: default_max_file_size(),
            allowed_paths: vec![".".to_string()],
            allowed_commands: vec![],
            max_output_size: default_max_output_size(),
            timeout_seconds: default_timeout(),
            allow_network: false,
            read_only: false,
            max_concurrent_executions: default_max_concurrent(),
        }
    }
}

impl ToolCapabilityConfig {
    /// Generate JSON Schema for this configuration
    ///
    /// Returns a pretty-printed JSON Schema string that can be used for
    /// validation and documentation of tool capability configurations.
    pub fn json_schema() -> String {
        let schema = schemars::schema_for!(ToolCapabilityConfig);
        serde_json::to_string_pretty(&schema).unwrap_or_default()
    }

    /// Create a new ToolCapabilityConfig with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum file size
    pub fn with_max_file_size(mut self, max_file_size: usize) -> Self {
        self.max_file_size = max_file_size;
        self
    }

    /// Set allowed paths
    pub fn with_allowed_paths(mut self, allowed_paths: Vec<String>) -> Self {
        self.allowed_paths = allowed_paths;
        self
    }

    /// Set allowed commands
    pub fn with_allowed_commands(mut self, allowed_commands: Vec<String>) -> Self {
        self.allowed_commands = allowed_commands;
        self
    }

    /// Set maximum output size
    pub fn with_max_output_size(mut self, max_output_size: usize) -> Self {
        self.max_output_size = max_output_size;
        self
    }

    /// Set timeout in seconds
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Enable or disable network access
    pub fn with_network(mut self, allow_network: bool) -> Self {
        self.allow_network = allow_network;
        self
    }

    /// Set read-only mode
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Set maximum concurrent executions
    pub fn with_max_concurrent(mut self, max_concurrent: usize) -> Self {
        self.max_concurrent_executions = max_concurrent;
        self
    }

    /// Convert to ToolExecutionConfig for backward compatibility
    pub fn to_execution_config(&self) -> ToolExecutionConfig {
        ToolExecutionConfig {
            timeout_seconds: self.timeout_seconds,
            max_output_size: self.max_output_size,
            allowed_paths: self.allowed_paths.clone(),
            allowed_commands: self.allowed_commands.clone(),
            read_only: self.read_only,
        }
    }
}

/// Utility functions for tool validation and result enhancement
pub mod validation {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Append a behavioral reminder to a tool result output
    ///
    /// This enhances tool results with contextual reminders that guide the agent's
    /// next actions based on the tool that was executed and whether it succeeded.
    pub fn append_behavioral_reminder(tool_name: &str, output: String, success: bool) -> String {
        let reminder = get_tool_reminder(tool_name, success);
        if reminder.is_empty() {
            output
        } else {
            format!("{}\n\n{}", output, reminder)
        }
    }

    /// Get the behavioral reminder for a specific tool
    fn get_tool_reminder(tool_name: &str, success: bool) -> String {
        if !success {
            // Common failure reminders
            return match tool_name {
                "run_command" | "run_script" => {
                    "🔴 Remember: Analyze the error output carefully. Consider:\n\
                     - Is the command syntax correct?\n\
                     - Are all required files/dependencies present?\n\
                     - Try an alternative approach or fix the underlying issue"
                        .to_string()
                }
                "cargo_build" | "cargo_test" | "cargo_check" | "cargo_clippy" => {
                    "🔴 Remember: Compilation/test failed. Next steps:\n\
                     - Read the error messages carefully to identify the issue\n\
                     - Fix the specific errors mentioned\n\
                     - Re-run the command to verify the fix"
                        .to_string()
                }
                "write_file" | "string_replace" => {
                    "🔴 Remember: File operation failed. Consider:\n\
                     - Does the directory exist?\n\
                     - Are the file paths correct?\n\
                     - Check permissions and path restrictions"
                        .to_string()
                }
                _ => {
                    "🔴 Remember: This operation failed. Analyze the error and try an alternative approach"
                        .to_string()
                }
            };
        }

        // Success reminders - guide next actions
        match tool_name {
            "read_file" => "✓ Remember: Now that you've read the file:\n\
                 - Analyze the content carefully before making changes\n\
                 - Plan your modifications to preserve existing functionality\n\
                 - Use surgical edits (string_replace) when possible"
                .to_string(),
            "write_file" => "✓ Remember: File written successfully. Next steps:\n\
                 - Verify the file works by running relevant tests\n\
                 - Check for syntax errors if it's code\n\
                 - Consider if any other files need updating"
                .to_string(),
            "string_replace" => "✓ Remember: Edit applied successfully. Validate the change:\n\
                 - Run tests to ensure nothing broke\n\
                 - Check if related code needs similar updates\n\
                 - Verify the logic is still correct"
                .to_string(),
            "run_command" | "run_script" => {
                "✓ Remember: Command executed successfully. Review the output:\n\
                 - Check if the output matches expectations\n\
                 - Look for warnings or issues in the output\n\
                 - Determine if follow-up actions are needed"
                    .to_string()
            }
            "cargo_build" => "✓ Remember: Build succeeded. Recommended next steps:\n\
                 - Run tests to ensure functionality works: cargo_test\n\
                 - Consider running clippy for code quality: cargo_clippy\n\
                 - Verify the binary works as expected"
                .to_string(),
            "cargo_test" => "✓ Remember: Tests passed. Good progress!\n\
                 - Review test output for any warnings\n\
                 - Consider if more tests are needed\n\
                 - Move on to the next task if tests cover your changes"
                .to_string(),
            "cargo_check" => "✓ Remember: Check passed (no compilation errors).\n\
                 - This only checks compilation, not functionality\n\
                 - Run tests to verify behavior: cargo_test\n\
                 - Consider running clippy for code quality"
                .to_string(),
            "cargo_clippy" => "✓ Remember: Clippy analysis complete.\n\
                 - Address any warnings or suggestions shown\n\
                 - Some warnings indicate potential bugs or bad practices\n\
                 - Run tests after fixing issues"
                .to_string(),
            "cargo_fmt" => "✓ Remember: Code formatting complete.\n\
                 - Code style is now consistent\n\
                 - Continue with building or testing\n\
                 - This doesn't affect functionality"
                .to_string(),
            "list_directory" => "✓ Remember: Directory listing retrieved.\n\
                 - Use this information to understand the project structure\n\
                 - Identify which files you need to read or modify\n\
                 - Check for files you might have missed"
                .to_string(),
            "create_directory" => "✓ Remember: Directory created successfully.\n\
                 - You can now create files in this directory\n\
                 - Ensure parent modules/configs reference this directory if needed"
                .to_string(),
            "file_exists" => "✓ Remember: File existence checked.\n\
                 - Use this information to decide next actions\n\
                 - If file doesn't exist, you may need to create it\n\
                 - If it exists, you may need to read it first"
                .to_string(),
            _ => String::new(), // No reminder for tools not listed
        }
    }

    /// Validate that a path is within allowed directories
    pub fn validate_path(path: &str, allowed_paths: &[String]) -> Result<PathBuf> {
        let path = Path::new(path);

        // Try to canonicalize the path, but if it fails (e.g., file doesn't exist),
        // try to canonicalize the parent directory and append the filename
        let canonical_path = if let Ok(canonical) = path.canonicalize() {
            canonical
        } else if let Some(parent) = path.parent() {
            if let Some(filename) = path.file_name() {
                let canonical_parent = parent.canonicalize().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to canonicalize parent path '{}': {}",
                        parent.display(),
                        e
                    )
                })?;
                canonical_parent.join(filename)
            } else {
                return Err(anyhow::anyhow!("Invalid path: {}", path.display()));
            }
        } else {
            return Err(anyhow::anyhow!("Cannot validate path: {}", path.display()));
        };

        for allowed in allowed_paths {
            let allowed_path = Path::new(allowed);
            if let Ok(canonical_allowed) = allowed_path.canonicalize() {
                if canonical_path.starts_with(&canonical_allowed) {
                    return Ok(canonical_path);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Path '{}' is not within any allowed directory",
            path.display()
        ))
    }

    /// Validate that a command is in the allowed list with enhanced security checks
    ///
    /// This function now uses the unified CommandValidator for consistency across the codebase.
    pub fn validate_command(command: &str, allowed_commands: &[String]) -> Result<()> {
        use crate::security::command_validator::CommandValidator;

        // Parse the command to extract command name and arguments
        let parts: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();

        if parts.is_empty() {
            return Err(anyhow::anyhow!("Command cannot be empty"));
        }

        let cmd_name = &parts[0];
        let args = if parts.len() > 1 {
            parts[1..].to_vec()
        } else {
            Vec::new()
        };

        // Use the unified validator
        let validator = CommandValidator::new(allowed_commands.to_vec());
        validator.validate(cmd_name, &args)
    }

    /// Sanitize output to prevent excessive memory usage
    pub fn sanitize_output(output: &str, max_size: usize) -> String {
        if output.len() <= max_size {
            output.to_string()
        } else {
            let truncated = &output[..max_size];
            format!("{}... (truncated from {} bytes)", truncated, output.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockToolExecutor {
        tools: Vec<String>,
    }

    #[async_trait]
    impl ToolExecutor for MockToolExecutor {
        async fn execute_tool(
            &self,
            tool_name: &str,
            _parameters: &HashMap<String, serde_json::Value>,
        ) -> Result<String> {
            Ok(format!("Executed {}", tool_name))
        }

        fn get_available_tools(&self) -> Vec<String> {
            self.tools.clone()
        }

        fn get_tool_description(&self, tool_name: &str) -> Option<String> {
            if self.tools.contains(&tool_name.to_string()) {
                Some(format!("Description for {}", tool_name))
            } else {
                None
            }
        }

        fn validate_tool_request(
            &self,
            _tool_name: &str,
            _parameters: &HashMap<String, serde_json::Value>,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();

        let executor = Arc::new(MockToolExecutor {
            tools: vec!["test_tool".to_string()],
        });

        registry.register("mock".to_string(), executor);

        assert!(registry.is_tool_available("test_tool"));
        assert!(!registry.is_tool_available("nonexistent_tool"));

        let result = registry.execute_tool("test_tool", &HashMap::new()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Executed test_tool");
    }

    #[test]
    fn test_path_validation() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let allowed_paths = vec![temp_dir.path().to_string_lossy().to_string()];

        // Test with existing file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test").unwrap();

        let result = validation::validate_path(&test_file.to_string_lossy(), &allowed_paths);
        assert!(result.is_ok());

        // Test with non-existing file in allowed directory
        let non_existing = temp_dir.path().join("non_existing.txt");
        let result = validation::validate_path(&non_existing.to_string_lossy(), &allowed_paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_validation() {
        // Note: The unified validator now requires exact command names (not prefixes like "cargo build")
        // This is more secure as it prevents "cargo" from matching "cargo-malicious"
        let allowed_commands = vec!["cargo".to_string(), "rm".to_string()];

        assert!(validation::validate_command("cargo build", &allowed_commands).is_ok());
        assert!(validation::validate_command("cargo test --lib", &allowed_commands).is_ok());

        // rm should be rejected because it has dangerous patterns (even though in allowlist)
        // The unified validator checks patterns in addition to allowlist
        assert!(validation::validate_command("rm -rf /", &allowed_commands).is_err());

        // Command not in allowlist should fail
        let allowed_commands_no_rm = vec!["cargo".to_string()];
        assert!(validation::validate_command("rm -rf /", &allowed_commands_no_rm).is_err());
    }

    #[test]
    fn test_output_sanitization() {
        let short_output = "Hello, world!";
        let long_output = "a".repeat(2000);

        assert_eq!(
            validation::sanitize_output(short_output, 1000),
            short_output
        );

        let sanitized = validation::sanitize_output(&long_output, 100);
        assert!(sanitized.len() < long_output.len());
        assert!(sanitized.contains("truncated"));
    }

    #[test]
    fn test_tool_capability_config_default() {
        let config = ToolCapabilityConfig::default();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_output_size, 1024 * 1024);
        assert_eq!(config.max_concurrent_executions, 5);
        assert!(!config.allow_network);
        assert!(!config.read_only);
        assert_eq!(config.allowed_paths, vec![".".to_string()]);
        assert!(config.allowed_commands.is_empty());
    }

    #[test]
    fn test_tool_capability_config_builder() {
        let config = ToolCapabilityConfig::new()
            .with_max_file_size(5 * 1024 * 1024)
            .with_allowed_paths(vec!["./src".to_string(), "./tests".to_string()])
            .with_allowed_commands(vec!["cargo".to_string(), "git".to_string()])
            .with_max_output_size(512 * 1024)
            .with_timeout(60)
            .with_network(true)
            .with_read_only(true)
            .with_max_concurrent(10);

        assert_eq!(config.max_file_size, 5 * 1024 * 1024);
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.max_output_size, 512 * 1024);
        assert_eq!(config.max_concurrent_executions, 10);
        assert!(config.allow_network);
        assert!(config.read_only);
        assert_eq!(config.allowed_paths.len(), 2);
        assert_eq!(config.allowed_commands.len(), 2);
    }

    #[test]
    fn test_tool_capability_config_json_schema_generation() {
        let schema = ToolCapabilityConfig::json_schema();
        assert!(schema.contains("max_file_size"));
        assert!(schema.contains("allowed_paths"));
        assert!(schema.contains("allowed_commands"));
        assert!(schema.contains("max_output_size"));
        assert!(schema.contains("timeout_seconds"));
        assert!(schema.contains("allow_network"));
        assert!(schema.contains("read_only"));
        assert!(schema.contains("max_concurrent_executions"));

        // Verify it's valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&schema).expect("Schema should be valid JSON");
        assert!(parsed.is_object());
    }

    #[test]
    fn test_tool_capability_config_serialization() {
        let config = ToolCapabilityConfig::new()
            .with_max_file_size(5 * 1024 * 1024)
            .with_allowed_paths(vec!["./src".to_string()])
            .with_timeout(45);

        // Test serialization
        let json = serde_json::to_string(&config).expect("Should serialize to JSON");
        assert!(json.contains("max_file_size"));
        assert!(json.contains("5242880")); // 5MB in bytes

        // Test deserialization
        let deserialized: ToolCapabilityConfig =
            serde_json::from_str(&json).expect("Should deserialize from JSON");
        assert_eq!(deserialized.max_file_size, config.max_file_size);
        assert_eq!(deserialized.timeout_seconds, config.timeout_seconds);
        assert_eq!(deserialized.allowed_paths, config.allowed_paths);
    }

    #[test]
    fn test_tool_capability_config_to_execution_config() {
        let capability_config = ToolCapabilityConfig::new()
            .with_max_output_size(2 * 1024 * 1024)
            .with_allowed_paths(vec!["./src".to_string()])
            .with_allowed_commands(vec!["cargo".to_string()])
            .with_timeout(120)
            .with_read_only(true);

        let execution_config = capability_config.to_execution_config();

        assert_eq!(execution_config.timeout_seconds, 120);
        assert_eq!(execution_config.max_output_size, 2 * 1024 * 1024);
        assert_eq!(execution_config.allowed_paths, vec!["./src".to_string()]);
        assert_eq!(execution_config.allowed_commands, vec!["cargo".to_string()]);
        assert!(execution_config.read_only);
    }

    #[test]
    fn test_tool_capability_config_default_values() {
        // Test that serde defaults work correctly
        let json = "{}";
        let config: ToolCapabilityConfig =
            serde_json::from_str(json).expect("Should deserialize with defaults");

        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.timeout_seconds, 30);
        assert_eq!(config.max_output_size, 1024 * 1024);
        assert_eq!(config.max_concurrent_executions, 5);
        assert!(!config.allow_network);
        assert!(!config.read_only);
        assert!(config.allowed_paths.is_empty());
        assert!(config.allowed_commands.is_empty());
    }

    #[test]
    fn test_behavioral_reminders_success() {
        // Test success reminders for different tools
        let read_reminder =
            validation::append_behavioral_reminder("read_file", "file contents".to_string(), true);
        assert!(read_reminder.contains("file contents"));
        assert!(read_reminder.contains("Remember"));
        assert!(read_reminder.contains("Analyze the content"));

        let write_reminder =
            validation::append_behavioral_reminder("write_file", "File written".to_string(), true);
        assert!(write_reminder.contains("Remember"));
        assert!(write_reminder.contains("running relevant tests"));

        let build_reminder = validation::append_behavioral_reminder(
            "cargo_build",
            "Build successful".to_string(),
            true,
        );
        assert!(build_reminder.contains("Remember"));
        assert!(build_reminder.contains("cargo_test"));
    }

    #[test]
    fn test_behavioral_reminders_failure() {
        // Test failure reminders for different tools
        let cmd_reminder = validation::append_behavioral_reminder(
            "run_command",
            "Command failed".to_string(),
            false,
        );
        assert!(cmd_reminder.contains("Remember"));
        assert!(cmd_reminder.contains("🔴"));
        assert!(cmd_reminder.contains("alternative approach"));

        let build_reminder = validation::append_behavioral_reminder(
            "cargo_build",
            "Build failed".to_string(),
            false,
        );
        assert!(build_reminder.contains("Remember"));
        assert!(build_reminder.contains("error messages"));

        let file_reminder =
            validation::append_behavioral_reminder("write_file", "Write failed".to_string(), false);
        assert!(file_reminder.contains("Remember"));
        assert!(file_reminder.contains("permissions"));
    }

    #[test]
    fn test_behavioral_reminders_unknown_tool() {
        // Unknown tools should still get base reminders
        let unknown_success =
            validation::append_behavioral_reminder("unknown_tool", "output".to_string(), true);
        // Should just return output unchanged for unknown tools
        assert_eq!(unknown_success, "output");

        let unknown_failure =
            validation::append_behavioral_reminder("unknown_tool", "error".to_string(), false);
        assert!(unknown_failure.contains("Remember"));
        assert!(unknown_failure.contains("alternative approach"));
    }

    #[tokio::test]
    async fn test_tool_registry_with_reminders() {
        let mut registry = ToolRegistry::new();

        let executor = Arc::new(MockToolExecutor {
            tools: vec!["test_tool".to_string()],
        });

        registry.register("mock".to_string(), executor);

        let result = registry.execute_tool("test_tool", &HashMap::new()).await;
        assert!(result.is_ok());

        // The result should contain the original output but won't have a reminder
        // since "test_tool" is not in our reminder list
        let output = result.unwrap();
        assert!(output.contains("Executed test_tool"));
    }
}
