use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use super::{validation, ToolExecutor};

/// String replacement editor tool for precise file editing
///
/// This tool allows for surgical edits to files by replacing specific strings
/// with new content, similar to Anthropic's string_replace_editor tool.
pub struct StringReplaceEditor {
    config: StringReplaceConfig,
}

/// Configuration for the string replace editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringReplaceConfig {
    pub allowed_paths: Vec<String>,
    pub max_file_size: usize,
    pub backup_enabled: bool,
    pub case_sensitive: bool,
    pub max_replacements: usize,
}

impl Default for StringReplaceConfig {
    fn default() -> Self {
        Self {
            allowed_paths: vec![
                "./".to_string(),
                "./src".to_string(),
                "./examples".to_string(),
                "./crates".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
            backup_enabled: true,
            case_sensitive: true,
            max_replacements: 100,
        }
    }
}

/// Parameters for string replacement operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringReplaceParams {
    pub file_path: String,
    pub old_str: String,
    pub new_str: String,
    pub occurrence: Option<ReplaceOccurrence>,
    pub line_range: Option<(usize, usize)>,
    pub create_backup: Option<bool>,
    pub dry_run: Option<bool>,
}

/// Specifies which occurrence(s) to replace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplaceOccurrence {
    First,
    Last,
    All,
    Index(usize), // 1-based index
}

impl Default for ReplaceOccurrence {
    fn default() -> Self {
        ReplaceOccurrence::First
    }
}

/// Result of a string replacement operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringReplaceResult {
    pub success: bool,
    pub replacements_made: usize,
    pub original_content: Option<String>,
    pub new_content: Option<String>,
    pub backup_path: Option<String>,
    pub preview: Option<String>,
    pub error: Option<String>,
}

impl StringReplaceEditor {
    /// Create a new string replace editor with default configuration
    pub fn new() -> Self {
        Self {
            config: StringReplaceConfig::default(),
        }
    }

    /// Create a new string replace editor with custom configuration
    pub fn with_config(config: StringReplaceConfig) -> Self {
        Self { config }
    }

    /// Perform string replacement in a file
    pub async fn replace_string(&self, params: StringReplaceParams) -> Result<StringReplaceResult> {
        // Validate file path
        let file_path = validation::validate_path(&params.file_path, &self.config.allowed_paths)?;

        // Check if file exists
        if !file_path.exists() {
            return Ok(StringReplaceResult {
                success: false,
                replacements_made: 0,
                original_content: None,
                new_content: None,
                backup_path: None,
                preview: None,
                error: Some(format!("File does not exist: {}", params.file_path)),
            });
        }

        // Check file size
        let metadata = fs::metadata(&file_path).await?;
        if metadata.len() > self.config.max_file_size as u64 {
            return Ok(StringReplaceResult {
                success: false,
                replacements_made: 0,
                original_content: None,
                new_content: None,
                backup_path: None,
                preview: None,
                error: Some(format!(
                    "File too large: {} bytes (max: {})",
                    metadata.len(),
                    self.config.max_file_size
                )),
            });
        }

        // Read file content
        let original_content = fs::read_to_string(&file_path).await?;

        // Perform replacement (handle line range within the replacement function)
        let (new_content, replacements_made) =
            if let Some((start_line, end_line)) = params.line_range {
                self.perform_replacement_with_line_range(
                    &original_content,
                    &params.old_str,
                    &params.new_str,
                    params.occurrence.unwrap_or_default(),
                    start_line,
                    end_line,
                )?
            } else {
                self.perform_replacement(
                    &original_content,
                    &params.old_str,
                    &params.new_str,
                    params.occurrence.unwrap_or_default(),
                )?
            };

        // If no replacements were made, this is still a successful operation
        if replacements_made == 0 {
            let preview = self.create_preview(&original_content, &params.old_str);
            return Ok(StringReplaceResult {
                success: true,
                replacements_made: 0,
                original_content: Some(original_content.clone()),
                new_content: Some(original_content), // Content unchanged
                backup_path: None,
                preview: Some(preview),
                error: None, // No error - this is a valid result
            });
        }

        // If dry run, return preview
        if params.dry_run.unwrap_or(false) {
            let preview = self.create_diff_preview(&original_content, &new_content);
            return Ok(StringReplaceResult {
                success: true,
                replacements_made,
                original_content: Some(original_content),
                new_content: Some(new_content.to_string()),
                backup_path: None,
                preview: Some(preview),
                error: None,
            });
        }

        // Create backup if enabled
        let backup_path = if params.create_backup.unwrap_or(self.config.backup_enabled) {
            let backup_path = self.create_backup(&file_path, &original_content).await?;
            Some(backup_path)
        } else {
            None
        };

        // Write new content to file
        fs::write(&file_path, &new_content).await?;

        Ok(StringReplaceResult {
            success: true,
            replacements_made,
            original_content: Some(original_content),
            new_content: Some(new_content.to_string()),
            backup_path,
            preview: None,
            error: None,
        })
    }

    /// Extract content within specified line range (utility method for future features)
    #[allow(dead_code)]
    fn extract_line_range(
        &self,
        content: &str,
        start_line: usize,
        end_line: usize,
    ) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();

        if start_line == 0 || start_line > lines.len() {
            return Err(anyhow!("Invalid start line: {}", start_line));
        }

        let end_line = if end_line > lines.len() {
            lines.len()
        } else {
            end_line
        };

        if start_line > end_line {
            return Err(anyhow!(
                "Start line ({}) cannot be greater than end line ({})",
                start_line,
                end_line
            ));
        }

        let selected_lines = &lines[(start_line - 1)..end_line];
        Ok(selected_lines.join("\n"))
    }

    /// Perform string replacement within a specific line range
    fn perform_replacement_with_line_range(
        &self,
        content: &str,
        old_str: &str,
        new_str: &str,
        occurrence: ReplaceOccurrence,
        start_line: usize,
        end_line: usize,
    ) -> Result<(String, usize)> {
        let lines: Vec<&str> = content.lines().collect();

        if start_line == 0 || start_line > lines.len() {
            return Err(anyhow!("Invalid start line: {}", start_line));
        }

        let end_line = if end_line > lines.len() {
            lines.len()
        } else {
            end_line
        };

        if start_line > end_line {
            return Err(anyhow!(
                "Start line ({}) cannot be greater than end line ({})",
                start_line,
                end_line
            ));
        }

        // Extract the target range
        let range_content = lines[(start_line - 1)..end_line].join("\n");

        // Perform replacement on the range
        let (replaced_range, replacements_made) =
            self.perform_replacement(&range_content, old_str, new_str, occurrence)?;

        // Reconstruct the full content
        let mut result_lines = Vec::new();
        result_lines.extend_from_slice(&lines[..(start_line - 1)]);
        result_lines.extend(replaced_range.lines());
        result_lines.extend_from_slice(&lines[end_line..]);

        Ok((result_lines.join("\n"), replacements_made))
    }

    /// Perform the actual string replacement
    fn perform_replacement(
        &self,
        content: &str,
        old_str: &str,
        new_str: &str,
        occurrence: ReplaceOccurrence,
    ) -> Result<(String, usize)> {
        if old_str.is_empty() {
            return Err(anyhow!("Old string cannot be empty"));
        }

        let search_str = if self.config.case_sensitive {
            old_str.to_string()
        } else {
            old_str.to_lowercase()
        };

        let search_content = if self.config.case_sensitive {
            content.to_string()
        } else {
            content.to_lowercase()
        };

        match occurrence {
            ReplaceOccurrence::All => {
                let new_content = if self.config.case_sensitive {
                    content.replace(old_str, new_str)
                } else {
                    // Case-insensitive replacement is more complex
                    self.case_insensitive_replace_all(content, old_str, new_str)
                };
                let count = search_content.matches(&search_str).count();
                Ok((new_content, count))
            }
            ReplaceOccurrence::First => {
                if let Some(pos) = search_content.find(&search_str) {
                    let mut new_content = content.to_string();
                    new_content.replace_range(pos..pos + old_str.len(), new_str);
                    Ok((new_content, 1))
                } else {
                    Ok((content.to_string(), 0))
                }
            }
            ReplaceOccurrence::Last => {
                if let Some(pos) = search_content.rfind(&search_str) {
                    let mut new_content = content.to_string();
                    new_content.replace_range(pos..pos + old_str.len(), new_str);
                    Ok((new_content, 1))
                } else {
                    Ok((content.to_string(), 0))
                }
            }
            ReplaceOccurrence::Index(index) => {
                let matches: Vec<_> = search_content.match_indices(&search_str).collect();
                if index == 0 || index > matches.len() {
                    return Err(anyhow!(
                        "Invalid occurrence index: {} (found {} matches)",
                        index,
                        matches.len()
                    ));
                }

                let (pos, _) = matches[index - 1];
                let mut new_content = content.to_string();
                new_content.replace_range(pos..pos + old_str.len(), new_str);
                Ok((new_content, 1))
            }
        }
    }

    /// Case-insensitive replace all
    fn case_insensitive_replace_all(&self, content: &str, old_str: &str, new_str: &str) -> String {
        let mut result = String::new();
        let mut last_end = 0;
        let old_lower = old_str.to_lowercase();
        let content_lower = content.to_lowercase();

        for (start, _) in content_lower.match_indices(&old_lower) {
            result.push_str(&content[last_end..start]);
            result.push_str(new_str);
            last_end = start + old_str.len();
        }
        result.push_str(&content[last_end..]);
        result
    }

    /// Create a backup of the original file
    async fn create_backup(&self, file_path: &Path, content: &str) -> Result<String> {
        let backup_path = format!(
            "{}.backup.{}",
            file_path.to_string_lossy(),
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        );

        fs::write(&backup_path, content).await?;
        Ok(backup_path)
    }

    /// Create a preview showing where matches would occur
    fn create_preview(&self, content: &str, search_str: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut preview_lines = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let search_line = if self.config.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };

            let search_target = if self.config.case_sensitive {
                search_str.to_string()
            } else {
                search_str.to_lowercase()
            };

            if search_line.contains(&search_target) {
                preview_lines.push(format!("Line {}: {}", line_num + 1, line));
                if preview_lines.len() >= 10 {
                    preview_lines.push("... (showing first 10 matches)".to_string());
                    break;
                }
            }
        }

        if preview_lines.is_empty() {
            "No matches found".to_string()
        } else {
            preview_lines.join("\n")
        }
    }

    /// Create a diff-style preview
    fn create_diff_preview(&self, original: &str, new: &str) -> String {
        let orig_lines: Vec<&str> = original.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        let mut diff = Vec::new();
        let max_lines = orig_lines.len().max(new_lines.len());

        for i in 0..max_lines {
            let orig_line = orig_lines.get(i).unwrap_or(&"");
            let new_line = new_lines.get(i).unwrap_or(&"");

            if orig_line != new_line {
                if !orig_line.is_empty() {
                    diff.push(format!("- {}", orig_line));
                }
                if !new_line.is_empty() {
                    diff.push(format!("+ {}", new_line));
                }
            }
        }

        if diff.is_empty() {
            "No changes".to_string()
        } else {
            diff.join("\n")
        }
    }
}

#[async_trait]
impl ToolExecutor for StringReplaceEditor {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        match tool_name {
            "string_replace" => {
                let params: StringReplaceParams = serde_json::from_value(
                    serde_json::Value::Object(parameters.clone().into_iter().collect()),
                )?;

                let result = self.replace_string(params).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    fn get_available_tools(&self) -> Vec<String> {
        vec!["string_replace".to_string()]
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        match tool_name {
            "string_replace" => Some(
                "Replace specific strings in files with surgical precision. \
                Supports first/last/all/indexed occurrences, line ranges, \
                case sensitivity, dry runs, and automatic backups."
                    .to_string(),
            ),
            _ => None,
        }
    }

    fn validate_tool_request(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        match tool_name {
            "string_replace" => {
                // Validate required parameters
                if !parameters.contains_key("file_path") {
                    return Err(anyhow!("Missing required parameter: file_path"));
                }
                if !parameters.contains_key("old_str") {
                    return Err(anyhow!("Missing required parameter: old_str"));
                }
                if !parameters.contains_key("new_str") {
                    return Err(anyhow!("Missing required parameter: new_str"));
                }

                // Validate file path
                if let Some(file_path) = parameters.get("file_path").and_then(|v| v.as_str()) {
                    validation::validate_path(file_path, &self.config.allowed_paths)?;
                }

                Ok(())
            }
            _ => Err(anyhow!("Unknown tool: {}", tool_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_string_replace_basic() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file
        let original_content = "Hello world\nThis is a test\nHello again";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "Hello".to_string(),
            new_str: "Hi".to_string(),
            occurrence: Some(ReplaceOccurrence::First),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "Hi world\nThis is a test\nHello again");
    }

    #[tokio::test]
    async fn test_string_replace_all() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "foo bar foo baz foo";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "foo".to_string(),
            new_str: "FOO".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 3);

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(new_content, "FOO bar FOO baz FOO");
    }

    #[tokio::test]
    async fn test_dry_run() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Hello world";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "Hello".to_string(),
            new_str: "Hi".to_string(),
            occurrence: Some(ReplaceOccurrence::First),
            line_range: None,
            create_backup: Some(false),
            dry_run: Some(true),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 1);
        assert!(result.preview.is_some());

        // File should remain unchanged
        let file_content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(file_content, original_content);
    }

    #[tokio::test]
    async fn test_line_range_replacement() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create test file with multiple lines
        let original_content = "Line 1: foo\nLine 2: foo\nLine 3: foo\nLine 4: foo\nLine 5: foo";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        // Replace only in lines 2-4
        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "foo".to_string(),
            new_str: "bar".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: Some((2, 4)),
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 3); // Lines 2, 3, 4

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        let expected = "Line 1: foo\nLine 2: bar\nLine 3: bar\nLine 4: bar\nLine 5: foo";
        assert_eq!(new_content, expected);
    }

    #[tokio::test]
    async fn test_line_range_single_line() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let original_content = "Line 1: foo\nLine 2: foo bar foo\nLine 3: foo";
        fs::write(&file_path, original_content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        // Replace only in line 2
        let params = StringReplaceParams {
            file_path: file_path.to_string_lossy().to_string(),
            old_str: "foo".to_string(),
            new_str: "baz".to_string(),
            occurrence: Some(ReplaceOccurrence::All),
            line_range: Some((2, 2)),
            create_backup: Some(false),
            dry_run: Some(false),
        };

        let result = editor.replace_string(params).await.unwrap();

        assert!(result.success);
        assert_eq!(result.replacements_made, 2); // Two "foo" in line 2

        let new_content = fs::read_to_string(&file_path).await.unwrap();
        let expected = "Line 1: foo\nLine 2: baz bar baz\nLine 3: foo";
        assert_eq!(new_content, expected);
    }
}
