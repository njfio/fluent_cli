# String Replace Editor Tool

The String Replace Editor is a powerful tool for making surgical edits to files with precision targeting. It provides capabilities similar to Anthropic's `string_replace_editor` tool while offering additional enterprise-grade features.

## Overview

The String Replace Editor allows you to:
- Replace specific strings in files with surgical precision
- Target specific occurrences (first, last, all, or by index)
- Restrict replacements to specific line ranges
- Preview changes before applying them
- Create automatic backups for safety
- Handle case-sensitive or case-insensitive matching

## Usage

### Basic Replacement

```rust
use fluent_agent::tools::{ToolRegistry, StringReplaceEditor};
use fluent_agent::config::ToolConfig;
use serde_json::json;
use std::collections::HashMap;

// Create tool registry with string replace editor
let tool_config = ToolConfig {
    file_operations: true,
    shell_commands: false,
    rust_compiler: false,
    git_operations: false,
    allowed_paths: Some(vec!["./src".to_string()]),
    allowed_commands: Some(vec![]),
};

let registry = ToolRegistry::with_standard_tools(&tool_config);

// Replace first occurrence
let mut params = HashMap::new();
params.insert("file_path".to_string(), json!("src/main.rs"));
params.insert("old_str".to_string(), json!("println!"));
params.insert("new_str".to_string(), json!("log::info!"));
params.insert("occurrence".to_string(), json!("First"));

let result = registry.execute_tool("string_replace", &params).await?;
```

## Parameters

### Required Parameters

- **`file_path`** (string): Path to the file to edit
- **`old_str`** (string): String to search for and replace
- **`new_str`** (string): Replacement string

### Optional Parameters

- **`occurrence`** (string|object): Which occurrence(s) to replace
  - `"First"` - Replace only the first occurrence (default)
  - `"Last"` - Replace only the last occurrence
  - `"All"` - Replace all occurrences
  - `{"Index": N}` - Replace the Nth occurrence (1-based)

- **`line_range`** (array): Restrict replacement to specific lines
  - Format: `[start_line, end_line]` (1-based, inclusive)
  - Example: `[10, 20]` - Only search within lines 10-20

- **`create_backup`** (boolean): Create a timestamped backup file
  - Default: `true`

- **`dry_run`** (boolean): Preview changes without modifying the file
  - Default: `false`

## Examples

### Replace All Occurrences

```rust
let mut params = HashMap::new();
params.insert("file_path".to_string(), json!("config.toml"));
params.insert("old_str".to_string(), json!("debug = false"));
params.insert("new_str".to_string(), json!("debug = true"));
params.insert("occurrence".to_string(), json!("All"));
params.insert("create_backup".to_string(), json!(true));
```

### Line Range Replacement

```rust
let mut params = HashMap::new();
params.insert("file_path".to_string(), json!("lib.rs"));
params.insert("old_str".to_string(), json!("i32"));
params.insert("new_str".to_string(), json!("u32"));
params.insert("occurrence".to_string(), json!("All"));
params.insert("line_range".to_string(), json!([15, 25])); // Lines 15-25 only
```

### Indexed Replacement

```rust
let mut params = HashMap::new();
params.insert("file_path".to_string(), json!("app.rs"));
params.insert("old_str".to_string(), json!("TODO"));
params.insert("new_str".to_string(), json!("FIXME"));
params.insert("occurrence".to_string(), json!({"Index": 3})); // Replace 3rd occurrence
```

### Dry Run Preview

```rust
let mut params = HashMap::new();
params.insert("file_path".to_string(), json!("main.rs"));
params.insert("old_str".to_string(), json!("HashMap"));
params.insert("new_str".to_string(), json!("BTreeMap"));
params.insert("occurrence".to_string(), json!("All"));
params.insert("dry_run".to_string(), json!(true)); // Preview only
```

## Response Format

The tool returns a JSON response with the following structure:

```json
{
  "success": true,
  "replacements_made": 3,
  "original_content": "...",
  "new_content": "...",
  "backup_path": "/path/to/backup.20231201_143022",
  "preview": null,
  "error": null
}
```

### Response Fields

- **`success`** (boolean): Whether the operation succeeded
- **`replacements_made`** (number): Number of replacements performed
- **`original_content`** (string|null): Original file content (for dry runs)
- **`new_content`** (string|null): Modified file content
- **`backup_path`** (string|null): Path to backup file if created
- **`preview`** (string|null): Preview of changes (for dry runs or errors)
- **`error`** (string|null): Error message if operation failed

## Configuration

The String Replace Editor can be configured with the following options:

```rust
use fluent_agent::tools::string_replace_editor::StringReplaceConfig;

let config = StringReplaceConfig {
    allowed_paths: vec![
        "./src".to_string(),
        "./examples".to_string(),
        "./docs".to_string(),
    ],
    max_file_size: 10 * 1024 * 1024, // 10MB
    backup_enabled: true,
    case_sensitive: true,
    max_replacements: 100,
};

let editor = StringReplaceEditor::with_config(config);
```

### Configuration Options

- **`allowed_paths`**: List of directories where file operations are permitted
- **`max_file_size`**: Maximum file size in bytes (default: 10MB)
- **`backup_enabled`**: Whether to create backups by default (default: true)
- **`case_sensitive`**: Whether string matching is case-sensitive (default: true)
- **`max_replacements`**: Maximum number of replacements per operation (default: 100)

## Security Features

### Path Validation
- Only files within `allowed_paths` can be accessed
- Path traversal attacks are prevented
- Symbolic links are resolved safely

### Input Validation
- All parameters are validated before execution
- File size limits prevent memory exhaustion
- Replacement limits prevent runaway operations

### Backup Protection
- Automatic timestamped backups for safety
- Configurable backup behavior
- Backup files include full timestamp for uniqueness

## Error Handling

The tool provides comprehensive error handling for common scenarios:

- **File not found**: Clear error message with file path
- **Permission denied**: Detailed access error information
- **File too large**: Size limit exceeded notification
- **No matches found**: Preview of search context
- **Invalid parameters**: Specific validation error messages
- **Path restrictions**: Security violation notifications

## Best Practices

### 1. Use Dry Runs for Safety
Always preview changes for critical files:
```rust
params.insert("dry_run".to_string(), json!(true));
```

### 2. Enable Backups for Important Files
```rust
params.insert("create_backup".to_string(), json!(true));
```

### 3. Use Line Ranges for Targeted Changes
Restrict changes to specific sections:
```rust
params.insert("line_range".to_string(), json!([10, 50]));
```

### 4. Validate Paths
Ensure file paths are within allowed directories and use absolute paths when possible.

### 5. Handle Errors Gracefully
Always check the `success` field and handle errors appropriately:
```rust
let response: StringReplaceResult = serde_json::from_str(&result)?;
if !response.success {
    eprintln!("Error: {}", response.error.unwrap_or("Unknown error".to_string()));
}
```

## Integration with Agentic Systems

The String Replace Editor is designed for seamless integration with autonomous agents:

- **Tool Registry Integration**: Automatically available in agent tool registries
- **Configuration Inheritance**: Respects agent security and path constraints
- **Error Recovery**: Provides detailed feedback for agent decision-making
- **Batch Operations**: Supports multiple file modifications in sequence
- **State Tracking**: Compatible with agent memory and context systems

## Examples in Practice

### Code Modernization
```rust
// Update logging throughout a codebase
params.insert("old_str".to_string(), json!("println!"));
params.insert("new_str".to_string(), json!("log::info!"));
params.insert("occurrence".to_string(), json!("All"));
```

### Configuration Updates
```rust
// Enable debug mode in config files
params.insert("old_str".to_string(), json!("debug = false"));
params.insert("new_str".to_string(), json!("debug = true"));
params.insert("occurrence".to_string(), json!("First"));
```

### Type System Improvements
```rust
// Update error types in specific functions
params.insert("old_str".to_string(), json!("Box<dyn std::error::Error>"));
params.insert("new_str".to_string(), json!("anyhow::Error"));
params.insert("line_range".to_string(), json!([20, 30]));
```

The String Replace Editor provides the precision and safety needed for autonomous code modification while maintaining the flexibility required for complex editing tasks.
