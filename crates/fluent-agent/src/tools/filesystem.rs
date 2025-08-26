use super::{validation, ToolExecutionConfig, ToolExecutor};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// File system tool executor that provides safe file operations
pub struct FileSystemExecutor {
    config: ToolExecutionConfig,
}

impl FileSystemExecutor {
    /// Create a new file system executor with the given configuration
    pub fn new(config: ToolExecutionConfig) -> Self {
        Self { config }
    }

    /// Create a file system executor with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ToolExecutionConfig::default())
    }

    /// Validate that a path is safe to access
    fn validate_path(&self, path: &str) -> Result<PathBuf> {
        // First, use the existing validation
        let validated_path = validation::validate_path(path, &self.config.allowed_paths)?;

        // Additional security checks - handle non-existent files
        let canonical_path = if validated_path.exists() {
            validated_path
                .canonicalize()
                .map_err(|e| anyhow!("Failed to canonicalize path '{}': {}", path, e))?
        } else {
            // For non-existent files, canonicalize the parent directory
            if let Some(parent) = validated_path.parent() {
                if parent.exists() {
                    let canonical_parent = parent.canonicalize().map_err(|e| {
                        anyhow!(
                            "Failed to canonicalize parent path '{}': {}",
                            parent.display(),
                            e
                        )
                    })?;
                    let file_name = validated_path
                        .file_name()
                        .ok_or_else(|| anyhow!("Path '{}' has no file name component", validated_path.display()))?;
                    canonical_parent.join(file_name)
                } else {
                    validated_path.clone()
                }
            } else {
                validated_path.clone()
            }
        };

        // Ensure the canonical path is still within allowed directories
        let mut is_allowed = false;
        for allowed_path in &self.config.allowed_paths {
            let allowed_canonical = PathBuf::from(allowed_path).canonicalize().map_err(|e| {
                anyhow!(
                    "Failed to canonicalize allowed path '{}': {}",
                    allowed_path,
                    e
                )
            })?;

            if canonical_path.starts_with(&allowed_canonical) {
                is_allowed = true;
                break;
            }
        }

        if !is_allowed {
            return Err(anyhow!(
                "Path '{}' (canonical: '{}') is not within any allowed directory",
                path,
                canonical_path.display()
            ));
        }

        Ok(canonical_path)
    }

    /// Read file content with size limits
    async fn read_file_safe(&self, path: &Path) -> Result<String> {
        let metadata = fs::metadata(path)
            .await
            .map_err(|e| anyhow!("Failed to get file metadata: {}", e))?;

        if metadata.len() > self.config.max_output_size as u64 {
            return Err(anyhow!(
                "File size ({} bytes) exceeds maximum allowed size ({} bytes)",
                metadata.len(),
                self.config.max_output_size
            ));
        }

        let mut file = fs::File::open(path)
            .await
            .map_err(|e| anyhow!("Failed to open file: {}", e))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| anyhow!("Failed to read file: {}", e))?;

        Ok(validation::sanitize_output(
            &contents,
            self.config.max_output_size,
        ))
    }

    /// Write file content safely
    async fn write_file_safe(&self, path: &Path, content: &str) -> Result<()> {
        if self.config.read_only {
            return Err(anyhow!("Write operations are disabled in read-only mode"));
        }

        if content.len() > self.config.max_output_size {
            return Err(anyhow!(
                "Content size ({} bytes) exceeds maximum allowed size ({} bytes)",
                content.len(),
                self.config.max_output_size
            ));
        }

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| anyhow!("Failed to create parent directories: {}", e))?;
        }

        let mut file = fs::File::create(path)
            .await
            .map_err(|e| anyhow!("Failed to create file: {}", e))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| anyhow!("Failed to write file: {}", e))?;

        file.flush()
            .await
            .map_err(|e| anyhow!("Failed to flush file: {}", e))?;

        Ok(())
    }
}

#[async_trait]
impl ToolExecutor for FileSystemExecutor {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        match tool_name {
            "read_file" => {
                let path_str = parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;

                let path = self.validate_path(path_str)?;
                let content = self.read_file_safe(&path).await?;
                Ok(content)
            }

            "write_file" => {
                let path_str = parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;
                let content = parameters
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'content' parameter"))?;

                let path = self.validate_path(path_str)?;
                self.write_file_safe(&path, content).await?;
                Ok(format!(
                    "Successfully wrote {} bytes to {}",
                    content.len(),
                    path.display()
                ))
            }

            "list_directory" => {
                let path_str = parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;

                let path = self.validate_path(path_str)?;
                let mut entries = fs::read_dir(&path)
                    .await
                    .map_err(|e| anyhow!("Failed to read directory: {}", e))?;

                let mut files = Vec::new();
                while let Some(entry) = entries
                    .next_entry()
                    .await
                    .map_err(|e| anyhow!("Failed to read directory entry: {}", e))?
                {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let metadata = entry
                        .metadata()
                        .await
                        .map_err(|e| anyhow!("Failed to get file metadata: {}", e))?;

                    let file_info = FileInfo {
                        name: file_name,
                        is_directory: metadata.is_dir(),
                        size: if metadata.is_file() {
                            Some(metadata.len())
                        } else {
                            None
                        },
                        modified: metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs()),
                    };

                    files.push(file_info);
                }

                Ok(serde_json::to_string_pretty(&files)?)
            }

            "create_directory" => {
                let path_str = parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;

                if self.config.read_only {
                    return Err(anyhow!("Directory creation is disabled in read-only mode"));
                }

                let path = self.validate_path(path_str)?;
                fs::create_dir_all(&path)
                    .await
                    .map_err(|e| anyhow!("Failed to create directory: {}", e))?;

                Ok(format!(
                    "Successfully created directory: {}",
                    path.display()
                ))
            }

            "file_exists" => {
                let path_str = parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;

                let path = self.validate_path(path_str)?;
                let exists = path.exists();
                Ok(exists.to_string())
            }

            "get_file_info" => {
                let path_str = parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'path' parameter"))?;

                let path = self.validate_path(path_str)?;
                let metadata = fs::metadata(&path)
                    .await
                    .map_err(|e| anyhow!("Failed to get file metadata: {}", e))?;

                let file_info = FileInfo {
                    name: path
                        .file_name()
                        .ok_or_else(|| anyhow!("Path '{}' has no file name component", path.display()))?
                        .to_string_lossy()
                        .to_string(),
                    is_directory: metadata.is_dir(),
                    size: if metadata.is_file() {
                        Some(metadata.len())
                    } else {
                        None
                    },
                    modified: metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()),
                };

                Ok(serde_json::to_string_pretty(&file_info)?)
            }

            "concat_files" => {
                // Concatenate multiple files into a destination file
                let paths_val = parameters
                    .get("paths")
                    .ok_or_else(|| anyhow!("Missing 'paths' parameter"))?;
                let dest_str = parameters
                    .get("dest")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'dest' parameter"))?;
                let separator = parameters
                    .get("separator")
                    .and_then(|v| v.as_str())
                    .unwrap_or("\n\n");

                let mut inputs: Vec<String> = Vec::new();
                match paths_val {
                    serde_json::Value::Array(arr) => {
                        for v in arr {
                            if let Some(s) = v.as_str() { inputs.push(s.to_string()); }
                        }
                    }
                    serde_json::Value::String(s) => inputs.push(s.clone()),
                    _ => return Err(anyhow!("'paths' must be an array of strings or a string")),
                }

                if inputs.is_empty() { return Err(anyhow!("No input paths provided")); }

                // Validate and read inputs
                let mut combined = String::new();
                for (idx, p) in inputs.iter().enumerate() {
                    let path = self.validate_path(p)?;
                    let content = self.read_file_safe(&path).await?;
                    if idx > 0 { combined.push_str(separator); }
                    combined.push_str(&content);
                }

                // Write destination
                let dest = self.validate_path(dest_str)?;
                self.write_file_safe(&dest, &combined).await?;
                Ok(format!("Successfully concatenated {} files into {}", inputs.len(), dest.display()))
            }

            _ => Err(anyhow!("Unknown file system tool: {}", tool_name)),
        }
    }

    fn get_available_tools(&self) -> Vec<String> {
        let mut tools = vec![
            "read_file".to_string(),
            "list_directory".to_string(),
            "file_exists".to_string(),
            "get_file_info".to_string(),
            "concat_files".to_string(),
        ];

        if !self.config.read_only {
            tools.extend(vec![
                "write_file".to_string(),
                "create_directory".to_string(),
            ]);
        }

        tools
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        let description = match tool_name {
            "read_file" => "Read the contents of a file",
            "write_file" => "Write content to a file",
            "list_directory" => "List the contents of a directory",
            "create_directory" => "Create a directory and its parent directories",
            "file_exists" => "Check if a file or directory exists",
            "get_file_info" => "Get detailed information about a file or directory",
            "concat_files" => "Concatenate multiple files and write to a destination",
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

        // Validate path parameter if present
        if let Some(path_value) = parameters.get("path") {
            if let Some(path_str) = path_value.as_str() {
                self.validate_path(path_str)?;
            } else {
                return Err(anyhow!("Path parameter must be a string"));
            }
        }

        // Validate content size for write operations
        if tool_name == "write_file" {
            if let Some(content_value) = parameters.get("content") {
                if let Some(content_str) = content_value.as_str() {
                    if content_str.len() > self.config.max_output_size {
                        return Err(anyhow!(
                            "Content size ({} bytes) exceeds maximum allowed size ({} bytes)",
                            content_str.len(),
                            self.config.max_output_size
                        ));
                    }
                } else {
                    return Err(anyhow!("Content parameter must be a string"));
                }
            }
        }

        // Validate concat_files parameters
        if tool_name == "concat_files" {
            if let Some(paths_val) = parameters.get("paths") {
                match paths_val {
                    serde_json::Value::Array(arr) => {
                        for v in arr {
                            if let Some(p) = v.as_str() { let _ = self.validate_path(p)?; }
                        }
                    }
                    serde_json::Value::String(s) => { let _ = self.validate_path(s)?; }
                    _ => return Err(anyhow!("'paths' must be array of strings or string")),
                }
            } else {
                return Err(anyhow!("'paths' parameter required"));
            }
            if let Some(dest_val) = parameters.get("dest") {
                if let Some(dest_str) = dest_val.as_str() { let _ = self.validate_path(dest_str)?; }
                else { return Err(anyhow!("'dest' must be string")); }
            } else { return Err(anyhow!("'dest' parameter required")); }
        }

        Ok(())
    }
}

/// Information about a file or directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<u64>, // Unix timestamp
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_read_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create a test file
        let mut file = fs::File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, world!").await.unwrap();
        file.flush().await.unwrap();

        let mut config = ToolExecutionConfig::default();
        config.allowed_paths = vec![temp_dir.path().to_string_lossy().to_string()];

        let executor = FileSystemExecutor::new(config);

        let mut params = HashMap::new();
        params.insert(
            "path".to_string(),
            serde_json::Value::String(file_path.to_string_lossy().to_string()),
        );

        let result = executor.execute_tool("read_file", &params).await.unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[tokio::test]
    async fn test_write_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test_write.txt");

        let mut config = ToolExecutionConfig::default();
        config.allowed_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.read_only = false;

        let executor = FileSystemExecutor::new(config);

        let mut params = HashMap::new();
        params.insert(
            "path".to_string(),
            serde_json::Value::String(file_path.to_string_lossy().to_string()),
        );
        params.insert(
            "content".to_string(),
            serde_json::Value::String("Test content".to_string()),
        );

        let result = executor.execute_tool("write_file", &params).await.unwrap();
        assert!(result.contains("Successfully wrote"));

        // Verify the file was written
        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_list_directory() {
        let temp_dir = tempdir().unwrap();

        // Create some test files
        fs::File::create(temp_dir.path().join("file1.txt"))
            .await
            .unwrap();
        fs::File::create(temp_dir.path().join("file2.txt"))
            .await
            .unwrap();
        fs::create_dir(temp_dir.path().join("subdir"))
            .await
            .unwrap();

        let mut config = ToolExecutionConfig::default();
        config.allowed_paths = vec![temp_dir.path().to_string_lossy().to_string()];

        let executor = FileSystemExecutor::new(config);

        let mut params = HashMap::new();
        params.insert(
            "path".to_string(),
            serde_json::Value::String(temp_dir.path().to_string_lossy().to_string()),
        );

        let result = executor
            .execute_tool("list_directory", &params)
            .await
            .unwrap();

        // Parse the JSON result
        let files: Vec<FileInfo> = serde_json::from_str(&result).unwrap();
        assert_eq!(files.len(), 3);

        let file_names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert!(file_names.contains(&"file1.txt"));
        assert!(file_names.contains(&"file2.txt"));
        assert!(file_names.contains(&"subdir"));
    }

    #[tokio::test]
    async fn test_path_validation() {
        let temp_dir = tempdir().unwrap();

        let mut config = ToolExecutionConfig::default();
        config.allowed_paths = vec![temp_dir.path().to_string_lossy().to_string()];

        let executor = FileSystemExecutor::new(config);

        // Test valid path
        let valid_path = temp_dir.path().join("test.txt");
        let mut params = HashMap::new();
        params.insert(
            "path".to_string(),
            serde_json::Value::String(valid_path.to_string_lossy().to_string()),
        );

        assert!(executor.validate_tool_request("read_file", &params).is_ok());

        // Test invalid path (outside allowed directories)
        let mut invalid_params = HashMap::new();
        invalid_params.insert(
            "path".to_string(),
            serde_json::Value::String("/etc/passwd".to_string()),
        );

        assert!(executor
            .validate_tool_request("read_file", &invalid_params)
            .is_err());
    }

    #[tokio::test]
    async fn test_read_only_mode() {
        let temp_dir = tempdir().unwrap();

        let mut config = ToolExecutionConfig::default();
        config.allowed_paths = vec![temp_dir.path().to_string_lossy().to_string()];
        config.read_only = true;

        let executor = FileSystemExecutor::new(config);

        // Read operations should work
        assert!(executor
            .get_available_tools()
            .contains(&"read_file".to_string()));

        // Write operations should not be available
        assert!(!executor
            .get_available_tools()
            .contains(&"write_file".to_string()));
        assert!(!executor
            .get_available_tools()
            .contains(&"create_directory".to_string()));
    }
}
