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
pub mod web;
pub mod workflow;

#[cfg(test)]
mod string_replace_editor_tests;

pub use filesystem::FileSystemExecutor;
pub use rust_compiler::RustCompilerExecutor;
pub use shell::ShellExecutor;
pub use string_replace_editor::StringReplaceEditor;
pub use web::WebExecutor;
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
#[derive(Clone)]
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
        let tool_lower = tool_name.to_lowercase();

        // Map tool names to (executor_key, actual_tool_name)
        // The executor_key is used to find the registered executor
        // The actual_tool_name is what the executor expects in execute_tool()
        let (executor_key, actual_tool_name): (&str, &str) = match tool_lower.as_str() {
            // Shell command tools - executor is registered as "shell"
            "run_command" | "execute_command" | "command" | "bash" | "exec" => {
                ("shell", "run_command")
            }
            "run_script" => ("shell", "run_script"),
            "get_working_directory" => ("shell", "get_working_directory"),
            "check_command_available" => ("shell", "check_command_available"),

            // File system tools - executor is registered as "filesystem"
            "file_system" | "fs" | "file" | "files" | "filesystem" => ("filesystem", "read_file"),
            "read_file" => ("filesystem", "read_file"),
            "write_file" => ("filesystem", "write_file"),
            "list_directory" => ("filesystem", "list_directory"),
            "create_directory" => ("filesystem", "create_directory"),
            "file_exists" => ("filesystem", "file_exists"),
            "delete_file" => ("filesystem", "delete_file"),
            "concat_files" => ("filesystem", "concat_files"),

            // Rust compiler tools - executor is registered as "rust_compiler"
            "compiler" | "cargo" | "rustc" | "rust_compiler" => ("rust_compiler", "cargo_build"),
            "cargo_build" => ("rust_compiler", "cargo_build"),
            "cargo_test" => ("rust_compiler", "cargo_test"),
            "cargo_check" => ("rust_compiler", "cargo_check"),
            "cargo_clippy" => ("rust_compiler", "cargo_clippy"),
            "cargo_fmt" => ("rust_compiler", "cargo_fmt"),
            "cargo_run" => ("rust_compiler", "cargo_run"),
            "get_rust_info" => ("rust_compiler", "get_rust_info"),

            // String replace tools - executor is registered as "string_replace"
            "str_replace" | "replace" | "edit" | "string_replace_editor" | "string_replace" => {
                ("string_replace", "str_replace_editor")
            }

            // Web tools - executor is registered as "web"
            "web_search" | "search" | "internet_search" => ("web", "web_search"),
            "fetch_url" | "web_fetch" | "browse" | "http_get" => ("web", "fetch_url"),

            // Fall back to checking all executors for the original tool name
            _ => ("", tool_name),
        };

        // If we have a known executor key, try to find it directly
        if !executor_key.is_empty() {
            if let Some(executor) = self.executors.get(executor_key) {
                // Validate the request
                executor.validate_tool_request(actual_tool_name, parameters)?;

                // Execute the tool
                let result = executor.execute_tool(actual_tool_name, parameters).await;

                // Enhance the result with behavioral reminders
                return match result {
                    Ok(output) => {
                        let enhanced_output =
                            validation::append_behavioral_reminder(actual_tool_name, output, true);
                        Ok(enhanced_output)
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        let enhanced_error = validation::append_behavioral_reminder(
                            actual_tool_name,
                            error_msg.clone(),
                            false,
                        );
                        Err(anyhow::anyhow!("{}", enhanced_error))
                    }
                };
            }
        }

        // Fallback: search all executors for one that provides this tool
        for executor in self.executors.values() {
            if executor
                .get_available_tools()
                .contains(&tool_name.to_string())
            {
                executor.validate_tool_request(tool_name, parameters)?;
                let result = executor.execute_tool(tool_name, parameters).await;

                return match result {
                    Ok(output) => {
                        let enhanced_output =
                            validation::append_behavioral_reminder(tool_name, output, true);
                        Ok(enhanced_output)
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        let enhanced_error = validation::append_behavioral_reminder(
                            tool_name,
                            error_msg.clone(),
                            false,
                        );
                        Err(anyhow::anyhow!("{}", enhanced_error))
                    }
                };
            }
        }

        Err(anyhow::anyhow!(
            "Tool '{}' not found in any registered executor",
            tool_name
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

        // Register web executor for browsing and search
        if config.web_browsing {
            let web_executor = Arc::new(WebExecutor::with_defaults());
            registry.register("web".to_string(), web_executor);
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

    // ==================== Semantic Validation ====================
    //
    // Semantic checks validate the meaning and intent of tool operations,
    // not just syntax. These help catch logical errors and provide warnings
    // for potentially problematic operations.

    /// Result of semantic validation - can be Ok, Warning, or Error
    #[derive(Debug, Clone, PartialEq)]
    pub enum SemanticValidationResult {
        /// Operation is semantically valid
        Ok,
        /// Operation has potential issues but can proceed
        Warning(String),
        /// Operation is semantically invalid and should be rejected
        Error(String),
    }

    impl SemanticValidationResult {
        pub fn is_ok(&self) -> bool {
            matches!(self, SemanticValidationResult::Ok)
        }

        pub fn is_warning(&self) -> bool {
            matches!(self, SemanticValidationResult::Warning(_))
        }

        pub fn is_error(&self) -> bool {
            matches!(self, SemanticValidationResult::Error(_))
        }
    }

    /// Semantic validation for string_replace operations
    pub fn validate_string_replace_semantic(
        old_string: &str,
        new_string: &str,
        file_path: &str,
    ) -> SemanticValidationResult {
        // Check for no-op replacement (identical strings)
        if old_string == new_string {
            return SemanticValidationResult::Warning(
                "string_replace: old_string and new_string are identical - this is a no-op"
                    .to_string(),
            );
        }

        // Check for empty old_string (would match everything)
        if old_string.is_empty() {
            return SemanticValidationResult::Error(
                "string_replace: old_string cannot be empty".to_string(),
            );
        }

        // Check for suspiciously short replacement that could be too broad
        if old_string.len() < 3 && !old_string.contains('\n') {
            return SemanticValidationResult::Warning(format!(
                "string_replace: very short old_string '{}' may match unintended occurrences",
                old_string.escape_debug()
            ));
        }

        // Check for file extension mismatch in code content
        let file_ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Detect language indicators in the new content
        let has_rust_syntax = new_string.contains("fn ")
            || new_string.contains("let ")
            || new_string.contains("impl ")
            || new_string.contains("::");
        let has_python_syntax = new_string.contains("def ")
            || new_string.contains("import ")
            || new_string.contains("self.") && !new_string.contains("::");
        let has_js_syntax = new_string.contains("function ")
            || new_string.contains("const ")
            || new_string.contains("=>")
            || new_string.contains("require(");

        // Warn about potential language mismatches
        if file_ext == "rs" && has_python_syntax && !has_rust_syntax {
            return SemanticValidationResult::Warning(
                "string_replace: Python-like syntax detected in a .rs file".to_string(),
            );
        }
        if file_ext == "py" && has_rust_syntax && !has_python_syntax {
            return SemanticValidationResult::Warning(
                "string_replace: Rust-like syntax detected in a .py file".to_string(),
            );
        }
        if file_ext == "js" && has_rust_syntax && !has_js_syntax {
            return SemanticValidationResult::Warning(
                "string_replace: Rust-like syntax detected in a .js file".to_string(),
            );
        }

        SemanticValidationResult::Ok
    }

    /// Semantic validation for file write operations
    pub fn validate_file_write_semantic(
        file_path: &str,
        content: &str,
    ) -> SemanticValidationResult {
        let path = Path::new(file_path);
        let file_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Check for writing to backup files
        if file_path.ends_with(".bak")
            || file_path.ends_with(".orig")
            || file_path.ends_with(".backup")
            || file_path.ends_with("~")
        {
            return SemanticValidationResult::Warning(
                "write_file: Writing to a backup file pattern - is this intentional?".to_string(),
            );
        }

        // Check for hidden files (except common ones like .gitignore)
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let allowed_hidden = [
            ".gitignore",
            ".gitattributes",
            ".editorconfig",
            ".env",
            ".env.example",
            ".dockerignore",
            ".prettierrc",
            ".eslintrc",
            ".cargo",
            ".rustfmt.toml",
            ".clippy.toml",
        ];
        if file_name.starts_with('.') && !allowed_hidden.iter().any(|h| file_name.starts_with(h)) {
            return SemanticValidationResult::Warning(format!(
                "write_file: Creating hidden file '{}' - verify this is intentional",
                file_name
            ));
        }

        // Check for empty content
        if content.is_empty() {
            return SemanticValidationResult::Warning(
                "write_file: Writing empty content to file".to_string(),
            );
        }

        // Check for content that looks like it might overwrite important files
        let sensitive_patterns = [
            "PRIVATE KEY",
            "BEGIN RSA",
            "password=",
            "secret=",
            "api_key=",
            "AWS_SECRET",
        ];
        for pattern in sensitive_patterns {
            if content.contains(pattern) {
                return SemanticValidationResult::Warning(format!(
                    "write_file: Content appears to contain sensitive data ('{}')",
                    pattern
                ));
            }
        }

        // Check content type matches file extension
        if file_ext == "json"
            && !content.trim().is_empty()
            && !content.trim().starts_with('{')
            && !content.trim().starts_with('[')
        {
            return SemanticValidationResult::Warning(
                "write_file: Content doesn't look like JSON for .json file".to_string(),
            );
        }

        if file_ext == "yaml" || file_ext == "yml" {
            // YAML files shouldn't start with { unless they're JSON
            if content.trim().starts_with('{') {
                return SemanticValidationResult::Warning(
                    "write_file: Content looks like JSON for .yaml file".to_string(),
                );
            }
        }

        SemanticValidationResult::Ok
    }

    /// Semantic validation for file read operations
    pub fn validate_file_read_semantic(file_path: &str) -> SemanticValidationResult {
        let path = Path::new(file_path);

        // Warn about reading very large binary file types
        let binary_extensions = [
            "exe", "dll", "so", "dylib", "bin", "o", "a", "jpg", "jpeg", "png", "gif", "bmp",
            "ico", "webp", "mp3", "mp4", "avi", "mov", "mkv", "wav", "zip", "tar", "gz", "rar",
            "7z", "bz2", "pdf", "doc", "docx", "xls", "xlsx",
        ];

        let file_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if binary_extensions.contains(&file_ext.as_str()) {
            return SemanticValidationResult::Warning(format!(
                "read_file: '{}' appears to be a binary file - reading may produce unreadable output",
                file_path
            ));
        }

        // Warn about reading lock files
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name.ends_with(".lock")
            || file_name == "package-lock.json"
            || file_name == "yarn.lock"
            || file_name == "Cargo.lock"
        {
            return SemanticValidationResult::Warning(format!(
                "read_file: '{}' is a lock file - usually auto-generated and very large",
                file_name
            ));
        }

        SemanticValidationResult::Ok
    }

    /// Semantic validation for command execution
    pub fn validate_command_semantic(command: &str, args: &[String]) -> SemanticValidationResult {
        let full_command = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        // Check for potentially destructive operations
        let destructive_patterns = [
            ("rm -rf /", "Attempting to remove root filesystem"),
            ("rm -rf ~", "Attempting to remove home directory"),
            ("rm -rf *", "Recursive deletion with wildcard"),
            ("chmod 777", "Setting world-writable permissions"),
            (
                "chmod -R 777",
                "Recursively setting world-writable permissions",
            ),
            ("dd if=", "Low-level disk write operation"),
            ("mkfs", "Filesystem format operation"),
            (":(){:|:&};:", "Fork bomb pattern detected"),
            (">(){ >|>&", "Fork bomb variant detected"),
        ];

        for (pattern, message) in destructive_patterns {
            if full_command.contains(pattern) {
                return SemanticValidationResult::Error(format!(
                    "command: {} - operation blocked",
                    message
                ));
            }
        }

        // Warning for operations that could have wide impact
        let warning_patterns = [
            ("rm -r", "Recursive deletion - ensure path is correct"),
            ("chmod -R", "Recursive permission change"),
            ("chown -R", "Recursive ownership change"),
            ("find . -delete", "Find with delete - very dangerous"),
            (
                "git reset --hard",
                "Hard reset will discard uncommitted changes",
            ),
            (
                "git push --force",
                "Force push can overwrite remote history",
            ),
            ("git clean -fd", "Clean will remove untracked files"),
            ("npm install -g", "Global npm install affects system"),
            ("pip install", "Installing Python packages"),
            ("cargo install", "Installing Cargo packages"),
        ];

        for (pattern, message) in warning_patterns {
            if full_command.contains(pattern) {
                return SemanticValidationResult::Warning(format!(
                    "command: {} - proceed with caution",
                    message
                ));
            }
        }

        // Check for commands without a clear target
        if (command == "rm" || command == "mv" || command == "cp") && args.is_empty() {
            return SemanticValidationResult::Error(format!(
                "command: '{}' requires arguments specifying target files",
                command
            ));
        }

        SemanticValidationResult::Ok
    }

    /// Semantic validation for directory creation
    pub fn validate_create_directory_semantic(dir_path: &str) -> SemanticValidationResult {
        let path = Path::new(dir_path);
        let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check for suspicious directory names
        if dir_name.starts_with('.')
            && dir_name != ".github"
            && dir_name != ".vscode"
            && dir_name != ".cargo"
            && dir_name != ".config"
        {
            return SemanticValidationResult::Warning(format!(
                "create_directory: Creating hidden directory '{}' - verify this is intentional",
                dir_name
            ));
        }

        // Check for temp/cache directory patterns
        let temp_patterns = [
            "tmp",
            "temp",
            "cache",
            ".cache",
            "node_modules",
            "__pycache__",
            ".pytest_cache",
            "target",
            "build",
            "dist",
        ];
        if temp_patterns.contains(&dir_name) {
            return SemanticValidationResult::Warning(format!(
                "create_directory: '{}' is typically an auto-generated directory - verify this is needed",
                dir_name
            ));
        }

        SemanticValidationResult::Ok
    }

    /// Validate tool parameters against a JSON schema
    pub fn validate_schema(
        params: &HashMap<String, serde_json::Value>,
        required_fields: &[&str],
        optional_fields: &[&str],
    ) -> SemanticValidationResult {
        // Check for missing required fields
        let missing: Vec<_> = required_fields
            .iter()
            .filter(|&&f| !params.contains_key(f))
            .collect();

        if !missing.is_empty() {
            return SemanticValidationResult::Error(format!(
                "Missing required parameters: {}",
                missing.into_iter().copied().collect::<Vec<_>>().join(", ")
            ));
        }

        // Check for unknown fields
        let known_fields: std::collections::HashSet<_> = required_fields
            .iter()
            .chain(optional_fields.iter())
            .cloned()
            .collect();

        let unknown: Vec<_> = params
            .keys()
            .filter(|k| !known_fields.contains(k.as_str()))
            .collect();

        if !unknown.is_empty() {
            return SemanticValidationResult::Warning(format!(
                "Unknown parameters (may be ignored): {}",
                unknown
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        SemanticValidationResult::Ok
    }

    /// Perform comprehensive semantic validation for a tool operation
    pub fn validate_tool_semantic(
        tool_name: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> SemanticValidationResult {
        match tool_name {
            "string_replace" | "str_replace_editor" => {
                let old_string = params
                    .get("old_string")
                    .or_else(|| params.get("old_str"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new_string = params
                    .get("new_string")
                    .or_else(|| params.get("new_str"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let file_path = params
                    .get("path")
                    .or_else(|| params.get("file_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                validate_string_replace_semantic(old_string, new_string, file_path)
            }
            "write_file" | "write" => {
                let file_path = params
                    .get("path")
                    .or_else(|| params.get("file_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");

                validate_file_write_semantic(file_path, content)
            }
            "read_file" | "read" => {
                let file_path = params
                    .get("path")
                    .or_else(|| params.get("file_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                validate_file_read_semantic(file_path)
            }
            "run_command" | "shell" | "bash" | "execute" => {
                let command = params.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let args: Vec<String> = params
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                validate_command_semantic(command, &args)
            }
            "create_directory" | "mkdir" => {
                let dir_path = params
                    .get("path")
                    .or_else(|| params.get("directory"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                validate_create_directory_semantic(dir_path)
            }
            _ => SemanticValidationResult::Ok,
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

    // ==================== Semantic Validation Tests ====================

    #[test]
    fn test_semantic_validation_result_methods() {
        let ok = validation::SemanticValidationResult::Ok;
        assert!(ok.is_ok());
        assert!(!ok.is_warning());
        assert!(!ok.is_error());

        let warning = validation::SemanticValidationResult::Warning("test".to_string());
        assert!(!warning.is_ok());
        assert!(warning.is_warning());
        assert!(!warning.is_error());

        let error = validation::SemanticValidationResult::Error("test".to_string());
        assert!(!error.is_ok());
        assert!(!error.is_warning());
        assert!(error.is_error());
    }

    #[test]
    fn test_string_replace_semantic_identical_strings() {
        let result = validation::validate_string_replace_semantic("hello", "hello", "test.rs");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("no-op"));
        }
    }

    #[test]
    fn test_string_replace_semantic_empty_old_string() {
        let result = validation::validate_string_replace_semantic("", "new", "test.rs");
        assert!(result.is_error());
        if let validation::SemanticValidationResult::Error(msg) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[test]
    fn test_string_replace_semantic_short_old_string() {
        let result = validation::validate_string_replace_semantic("ab", "newvalue", "test.rs");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("short"));
        }
    }

    #[test]
    fn test_string_replace_semantic_valid() {
        let result = validation::validate_string_replace_semantic(
            "fn old_function() {}",
            "fn new_function() {}",
            "test.rs",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_string_replace_semantic_language_mismatch_python_in_rust() {
        let result = validation::validate_string_replace_semantic(
            "old_code",
            "def new_function():\n    import os",
            "test.rs",
        );
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("Python"));
        }
    }

    #[test]
    fn test_string_replace_semantic_language_mismatch_rust_in_python() {
        let result = validation::validate_string_replace_semantic(
            "old_code",
            "fn new_function() -> i32 { let x = 5; }",
            "test.py",
        );
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("Rust"));
        }
    }

    #[test]
    fn test_file_write_semantic_backup_file() {
        let result = validation::validate_file_write_semantic("test.bak", "content");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("backup"));
        }

        let result2 = validation::validate_file_write_semantic("test.orig", "content");
        assert!(result2.is_warning());
    }

    #[test]
    fn test_file_write_semantic_hidden_file() {
        let result = validation::validate_file_write_semantic(".secret", "content");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("hidden"));
        }
    }

    #[test]
    fn test_file_write_semantic_allowed_hidden_files() {
        let result = validation::validate_file_write_semantic(".gitignore", "*.log");
        assert!(result.is_ok());

        // .env is in the allowed hidden files list, so simple content doesn't trigger warning
        let result2 = validation::validate_file_write_semantic(".env", "KEY=value");
        assert!(result2.is_ok());

        // But .env with sensitive patterns will trigger warning
        let result3 = validation::validate_file_write_semantic(".env", "password=secret123");
        assert!(result3.is_warning());
    }

    #[test]
    fn test_file_write_semantic_empty_content() {
        let result = validation::validate_file_write_semantic("test.txt", "");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[test]
    fn test_file_write_semantic_sensitive_content() {
        let result = validation::validate_file_write_semantic("config.txt", "password=secret123");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("sensitive"));
        }
    }

    #[test]
    fn test_file_write_semantic_json_mismatch() {
        let result = validation::validate_file_write_semantic("config.json", "not json content");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("JSON"));
        }
    }

    #[test]
    fn test_file_write_semantic_valid_json() {
        let result = validation::validate_file_write_semantic("config.json", r#"{"key": "value"}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_write_semantic_yaml_with_json() {
        let result = validation::validate_file_write_semantic("config.yaml", r#"{"key": "value"}"#);
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("JSON"));
        }
    }

    #[test]
    fn test_file_read_semantic_binary_file() {
        let result = validation::validate_file_read_semantic("image.png");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("binary"));
        }

        let result2 = validation::validate_file_read_semantic("archive.zip");
        assert!(result2.is_warning());
    }

    #[test]
    fn test_file_read_semantic_lock_file() {
        let result = validation::validate_file_read_semantic("Cargo.lock");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("lock file"));
        }

        let result2 = validation::validate_file_read_semantic("package-lock.json");
        assert!(result2.is_warning());
    }

    #[test]
    fn test_file_read_semantic_valid() {
        let result = validation::validate_file_read_semantic("main.rs");
        assert!(result.is_ok());

        let result2 = validation::validate_file_read_semantic("README.md");
        assert!(result2.is_ok());
    }

    #[test]
    fn test_command_semantic_destructive_operations() {
        let result =
            validation::validate_command_semantic("rm", &["-rf".to_string(), "/".to_string()]);
        assert!(result.is_error());
        if let validation::SemanticValidationResult::Error(msg) = result {
            assert!(msg.contains("root filesystem"));
        }

        let result2 = validation::validate_command_semantic(
            "chmod",
            &["777".to_string(), "file".to_string()],
        );
        assert!(result2.is_error());
    }

    #[test]
    fn test_command_semantic_warning_operations() {
        let result =
            validation::validate_command_semantic("rm", &["-r".to_string(), "dir".to_string()]);
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("Recursive deletion"));
        }

        let result2 = validation::validate_command_semantic(
            "git",
            &["push".to_string(), "--force".to_string()],
        );
        assert!(result2.is_warning());
    }

    #[test]
    fn test_command_semantic_missing_args() {
        let result = validation::validate_command_semantic("rm", &[]);
        assert!(result.is_error());
        if let validation::SemanticValidationResult::Error(msg) = result {
            assert!(msg.contains("requires arguments"));
        }

        let result2 = validation::validate_command_semantic("mv", &[]);
        assert!(result2.is_error());
    }

    #[test]
    fn test_command_semantic_valid() {
        let result = validation::validate_command_semantic("ls", &["-la".to_string()]);
        assert!(result.is_ok());

        let result2 = validation::validate_command_semantic("cargo", &["build".to_string()]);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_create_directory_semantic_hidden() {
        let result = validation::validate_create_directory_semantic(".hidden_dir");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("hidden"));
        }
    }

    #[test]
    fn test_create_directory_semantic_allowed_hidden() {
        let result = validation::validate_create_directory_semantic(".github");
        assert!(result.is_ok());

        let result2 = validation::validate_create_directory_semantic(".vscode");
        assert!(result2.is_ok());
    }

    #[test]
    fn test_create_directory_semantic_temp_patterns() {
        let result = validation::validate_create_directory_semantic("node_modules");
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("auto-generated"));
        }

        let result2 = validation::validate_create_directory_semantic("__pycache__");
        assert!(result2.is_warning());
    }

    #[test]
    fn test_create_directory_semantic_valid() {
        let result = validation::validate_create_directory_semantic("src/modules");
        assert!(result.is_ok());

        let result2 = validation::validate_create_directory_semantic("tests");
        assert!(result2.is_ok());
    }

    #[test]
    fn test_validate_schema_missing_required() {
        let mut params = HashMap::new();
        params.insert("optional".to_string(), serde_json::json!("value"));

        let result =
            validation::validate_schema(&params, &["required1", "required2"], &["optional"]);
        assert!(result.is_error());
        if let validation::SemanticValidationResult::Error(msg) = result {
            assert!(msg.contains("required1"));
            assert!(msg.contains("required2"));
        }
    }

    #[test]
    fn test_validate_schema_unknown_fields() {
        let mut params = HashMap::new();
        params.insert("required".to_string(), serde_json::json!("value"));
        params.insert("unknown".to_string(), serde_json::json!("value"));

        let result = validation::validate_schema(&params, &["required"], &[]);
        assert!(result.is_warning());
        if let validation::SemanticValidationResult::Warning(msg) = result {
            assert!(msg.contains("unknown"));
        }
    }

    #[test]
    fn test_validate_schema_valid() {
        let mut params = HashMap::new();
        params.insert("required".to_string(), serde_json::json!("value"));
        params.insert("optional".to_string(), serde_json::json!("value"));

        let result = validation::validate_schema(&params, &["required"], &["optional"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_tool_semantic_string_replace() {
        let mut params = HashMap::new();
        params.insert("old_string".to_string(), serde_json::json!("old"));
        params.insert("new_string".to_string(), serde_json::json!("old"));
        params.insert("path".to_string(), serde_json::json!("test.rs"));

        let result = validation::validate_tool_semantic("string_replace", &params);
        assert!(result.is_warning());
    }

    #[test]
    fn test_validate_tool_semantic_write_file() {
        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::json!("test.bak"));
        params.insert("content".to_string(), serde_json::json!("content"));

        let result = validation::validate_tool_semantic("write_file", &params);
        assert!(result.is_warning());
    }

    #[test]
    fn test_validate_tool_semantic_read_file() {
        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::json!("image.jpg"));

        let result = validation::validate_tool_semantic("read_file", &params);
        assert!(result.is_warning());
    }

    #[test]
    fn test_validate_tool_semantic_run_command() {
        let mut params = HashMap::new();
        params.insert("command".to_string(), serde_json::json!("rm"));
        params.insert("args".to_string(), serde_json::json!(["-rf", "/"]));

        let result = validation::validate_tool_semantic("run_command", &params);
        assert!(result.is_error());
    }

    #[test]
    fn test_validate_tool_semantic_create_directory() {
        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::json!("node_modules"));

        let result = validation::validate_tool_semantic("create_directory", &params);
        assert!(result.is_warning());
    }

    #[test]
    fn test_validate_tool_semantic_unknown_tool() {
        let params = HashMap::new();
        let result = validation::validate_tool_semantic("unknown_tool", &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_semantic_validation_result_equality() {
        let ok1 = validation::SemanticValidationResult::Ok;
        let ok2 = validation::SemanticValidationResult::Ok;
        assert_eq!(ok1, ok2);

        let warning1 = validation::SemanticValidationResult::Warning("test".to_string());
        let warning2 = validation::SemanticValidationResult::Warning("test".to_string());
        assert_eq!(warning1, warning2);

        let warning3 = validation::SemanticValidationResult::Warning("different".to_string());
        assert_ne!(warning1, warning3);
    }

    #[test]
    fn test_semantic_validation_result_clone() {
        let original = validation::SemanticValidationResult::Warning("test".to_string());
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
