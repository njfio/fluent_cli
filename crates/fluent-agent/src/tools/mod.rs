use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

pub mod filesystem;
pub mod shell;
pub mod rust_compiler;
pub mod string_replace_editor;

pub use filesystem::FileSystemExecutor;
pub use shell::ShellExecutor;
pub use rust_compiler::RustCompilerExecutor;
pub use string_replace_editor::StringReplaceEditor;

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
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        // Find the executor that provides this tool
        for executor in self.executors.values() {
            if executor.get_available_tools().contains(&tool_name.to_string()) {
                // Validate the request first
                executor.validate_tool_request(tool_name, parameters)?;
                
                // Execute the tool
                return executor.execute_tool(tool_name, parameters).await;
            }
        }

        Err(anyhow::anyhow!("Tool '{}' not found in any registered executor", tool_name))
    }

    /// Get all available tools across all executors
    pub fn get_all_available_tools(&self) -> Vec<ToolInfo> {
        let mut tools = Vec::new();
        
        for (executor_name, executor) in &self.executors {
            for tool_name in executor.get_available_tools() {
                tools.push(ToolInfo {
                    name: tool_name.clone(),
                    executor: executor_name.clone(),
                    description: executor.get_tool_description(&tool_name)
                        .unwrap_or_else(|| "No description available".to_string()),
                });
            }
        }

        tools
    }

    /// Check if a tool is available
    pub fn is_tool_available(&self, tool_name: &str) -> bool {
        self.executors.values()
            .any(|executor| executor.get_available_tools().contains(&tool_name.to_string()))
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
                allowed_paths: config.allowed_paths.clone().unwrap_or_else(|| vec![
                    "./".to_string(),
                    "./src".to_string(),
                    "./examples".to_string(),
                    "./crates".to_string(),
                ]),
                allowed_commands: config.allowed_commands.clone().unwrap_or_else(|| vec![
                    "cargo".to_string(),
                    "rustc".to_string(),
                ]),
                read_only: false,
            };

            let fs_executor = Arc::new(FileSystemExecutor::new(tool_config));
            registry.register("filesystem".to_string(), fs_executor);

            // Register string replace editor (also requires file operations)
            let string_replace_config = string_replace_editor::StringReplaceConfig {
                allowed_paths: config.allowed_paths.clone().unwrap_or_else(|| vec![
                    "./".to_string(),
                    "./src".to_string(),
                    "./examples".to_string(),
                    "./crates".to_string(),
                ]),
                max_file_size: 10 * 1024 * 1024, // 10MB
                backup_enabled: true,
                case_sensitive: true,
                max_replacements: 100,
            };

            let string_replace_executor = Arc::new(StringReplaceEditor::with_config(string_replace_config));
            registry.register("string_replace".to_string(), string_replace_executor);
        }

        // Register shell executor
        if config.shell_commands {
            let shell_config = ToolExecutionConfig {
                timeout_seconds: 60,
                max_output_size: 1024 * 1024, // 1MB
                allowed_paths: config.allowed_paths.clone().unwrap_or_else(|| vec![
                    "./".to_string(),
                ]),
                allowed_commands: config.allowed_commands.clone().unwrap_or_else(|| vec![
                    "cargo".to_string(),
                    "rustc".to_string(),
                    "ls".to_string(),
                    "cat".to_string(),
                    "echo".to_string(),
                ]),
                read_only: false,
            };

            let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            let shell_executor = Arc::new(ShellExecutor::new(shell_config, working_dir));
            registry.register("shell".to_string(), shell_executor);
        }

        // Register Rust compiler executor
        if config.rust_compiler {
            let rust_compiler_executor = Arc::new(RustCompilerExecutor::with_defaults(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
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

/// Utility functions for tool validation
pub mod validation {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Validate that a path is within allowed directories
    pub fn validate_path(path: &str, allowed_paths: &[String]) -> Result<PathBuf> {
        let path = Path::new(path);

        // Try to canonicalize the path, but if it fails (e.g., file doesn't exist),
        // try to canonicalize the parent directory and append the filename
        let canonical_path = if let Ok(canonical) = path.canonicalize() {
            canonical
        } else if let Some(parent) = path.parent() {
            if let Some(filename) = path.file_name() {
                let canonical_parent = parent.canonicalize()
                    .map_err(|e| anyhow::anyhow!("Failed to canonicalize parent path '{}': {}", parent.display(), e))?;
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

    /// Validate that a command is in the allowed list
    pub fn validate_command(command: &str, allowed_commands: &[String]) -> Result<()> {
        let command_lower = command.to_lowercase();
        
        for allowed in allowed_commands {
            if command_lower.starts_with(&allowed.to_lowercase()) {
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "Command '{}' is not in the allowed commands list",
            command
        ))
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
        let allowed_commands = vec!["cargo build".to_string(), "cargo test".to_string()];
        
        assert!(validation::validate_command("cargo build", &allowed_commands).is_ok());
        assert!(validation::validate_command("cargo test --lib", &allowed_commands).is_ok());
        assert!(validation::validate_command("rm -rf /", &allowed_commands).is_err());
    }

    #[test]
    fn test_output_sanitization() {
        let short_output = "Hello, world!";
        let long_output = "a".repeat(2000);
        
        assert_eq!(validation::sanitize_output(short_output, 1000), short_output);
        
        let sanitized = validation::sanitize_output(&long_output, 100);
        assert!(sanitized.len() < long_output.len());
        assert!(sanitized.contains("truncated"));
    }
}
