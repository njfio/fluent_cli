// String Replace Editor Tool Demo
use anyhow::Result;
use fluent_agent::tools::{StringReplaceEditor, ToolExecutor};
use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔧 String Replace Editor Tool Demo");
    println!("===================================");

    // Create a temporary directory for our demo
    let temp_dir = tempdir()?;
    let demo_file = temp_dir.path().join("demo_code.rs");

    // Create a sample Rust file to edit
    let original_content = r#"use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("hello", "world");
    map.insert("foo", "bar");
    
    println!("Hello, world!");
    println!("This is a test");
    println!("Hello again!");
    
    // TODO: Add more functionality
    let result = calculate_sum(5, 10);
    println!("Sum: {}", result);
}

fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

// Helper function
fn helper_function() {
    println!("This is a helper");
}
"#;

    fs::write(&demo_file, original_content).await?;
    println!("✅ Created demo file: {}", demo_file.display());

    // Create string replace editor with custom config
    let config = fluent_agent::tools::string_replace_editor::StringReplaceConfig {
        allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
        max_file_size: 1024 * 1024, // 1MB
        backup_enabled: true,
        case_sensitive: true,
        max_replacements: 50,
    };

    let editor = StringReplaceEditor::with_config(config);

    // Demo 1: Replace first occurrence
    println!("\n📝 Demo 1: Replace first occurrence of 'Hello'");
    demo_replace_first(&editor, &demo_file).await?;

    // Demo 2: Replace all occurrences
    println!("\n📝 Demo 2: Replace all occurrences of 'println!'");
    demo_replace_all(&editor, &demo_file).await?;

    // Demo 3: Dry run preview
    println!("\n📝 Demo 3: Dry run preview");
    demo_dry_run(&editor, &demo_file).await?;

    // Demo 4: Replace with line range
    println!("\n📝 Demo 4: Replace within line range");
    demo_line_range(&editor, &demo_file).await?;

    // Demo 5: Replace specific occurrence by index
    println!("\n📝 Demo 5: Replace specific occurrence by index");
    demo_specific_occurrence(&editor, &demo_file).await?;

    // Demo 6: Error handling
    println!("\n📝 Demo 6: Error handling");
    demo_error_handling(&editor, &demo_file).await?;

    // Show final file content
    println!("\n📄 Final file content:");
    let final_content = fs::read_to_string(&demo_file).await?;
    println!("{}", final_content);

    println!("\n✅ String Replace Editor Demo completed successfully!");

    Ok(())
}

async fn demo_replace_first(
    editor: &StringReplaceEditor,
    file_path: &std::path::Path,
) -> Result<()> {
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(file_path.to_string_lossy()));
    params.insert("old_str".to_string(), json!("Hello"));
    params.insert("new_str".to_string(), json!("Hi"));
    params.insert("occurrence".to_string(), json!("First"));
    params.insert("create_backup".to_string(), json!(false));

    let result = editor.execute_tool("string_replace", &params).await?;
    println!("Result: {}", result);

    Ok(())
}

async fn demo_replace_all(editor: &StringReplaceEditor, file_path: &std::path::Path) -> Result<()> {
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(file_path.to_string_lossy()));
    params.insert("old_str".to_string(), json!("println!"));
    params.insert("new_str".to_string(), json!("eprintln!"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("create_backup".to_string(), json!(false));

    let result = editor.execute_tool("string_replace", &params).await?;
    println!("Result: {}", result);

    Ok(())
}

async fn demo_dry_run(editor: &StringReplaceEditor, file_path: &std::path::Path) -> Result<()> {
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(file_path.to_string_lossy()));
    params.insert("old_str".to_string(), json!("calculate_sum"));
    params.insert("new_str".to_string(), json!("add_numbers"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("dry_run".to_string(), json!(true));

    let result = editor.execute_tool("string_replace", &params).await?;
    println!("Dry run result: {}", result);

    Ok(())
}

async fn demo_line_range(editor: &StringReplaceEditor, file_path: &std::path::Path) -> Result<()> {
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(file_path.to_string_lossy()));
    params.insert("old_str".to_string(), json!("i32"));
    params.insert("new_str".to_string(), json!("u32"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("line_range".to_string(), json!([15, 20])); // Only in function definition area
    params.insert("create_backup".to_string(), json!(false));

    let result = editor.execute_tool("string_replace", &params).await?;
    println!("Line range result: {}", result);

    Ok(())
}

async fn demo_specific_occurrence(
    editor: &StringReplaceEditor,
    file_path: &std::path::Path,
) -> Result<()> {
    // First, let's add some duplicate content to demonstrate index-based replacement
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(file_path.to_string_lossy()));
    params.insert("old_str".to_string(), json!("// Helper function"));
    params.insert("new_str".to_string(), json!("// Helper function\n// Another helper function\nfn another_helper() {\n    println!(\"Another helper\");\n}\n\n// Helper function"));
    params.insert("occurrence".to_string(), json!("First"));
    params.insert("create_backup".to_string(), json!(false));

    let _result = editor.execute_tool("string_replace", &params).await?;

    // Now replace the second occurrence of "Helper function"
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(file_path.to_string_lossy()));
    params.insert("old_str".to_string(), json!("Helper function"));
    params.insert("new_str".to_string(), json!("Utility function"));
    params.insert("occurrence".to_string(), json!({"Index": 2}));
    params.insert("create_backup".to_string(), json!(false));

    let result = editor.execute_tool("string_replace", &params).await?;
    println!("Specific occurrence result: {}", result);

    Ok(())
}

async fn demo_error_handling(
    editor: &StringReplaceEditor,
    file_path: &std::path::Path,
) -> Result<()> {
    // Try to replace something that doesn't exist
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(file_path.to_string_lossy()));
    params.insert("old_str".to_string(), json!("nonexistent_string"));
    params.insert("new_str".to_string(), json!("replacement"));
    params.insert("occurrence".to_string(), json!("First"));

    let result = editor.execute_tool("string_replace", &params).await?;
    println!("Error handling result: {}", result);

    // Try to access a file outside allowed paths
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!("/etc/passwd"));
    params.insert("old_str".to_string(), json!("root"));
    params.insert("new_str".to_string(), json!("admin"));

    match editor.execute_tool("string_replace", &params).await {
        Ok(result) => println!("Unexpected success: {}", result),
        Err(e) => println!("Expected error (path validation): {}", e),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_agent::tools::string_replace_editor::StringReplaceConfig;

    #[tokio::test]
    async fn test_string_replace_integration() {
        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        let content = "fn main() {\n    println!(\"Hello, world!\");\n}";
        fs::write(&test_file, content).await.unwrap();

        let config = StringReplaceConfig {
            allowed_paths: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let editor = StringReplaceEditor::with_config(config);

        let mut params = HashMap::new();
        params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
        params.insert("old_str".to_string(), json!("Hello, world!"));
        params.insert("new_str".to_string(), json!("Hello, Rust!"));
        params.insert("occurrence".to_string(), json!("First"));
        params.insert("create_backup".to_string(), json!(false));

        let result = editor
            .execute_tool("string_replace", &params)
            .await
            .unwrap();
        assert!(result.contains("\"success\": true"));

        let new_content = fs::read_to_string(&test_file).await.unwrap();
        assert!(new_content.contains("Hello, Rust!"));
        assert!(!new_content.contains("Hello, world!"));
    }

    #[tokio::test]
    async fn test_tool_validation() {
        let editor = StringReplaceEditor::new();

        // Test missing required parameters
        let params = HashMap::new();
        let result = editor.validate_tool_request("string_replace", &params);
        assert!(result.is_err());

        // Test valid parameters
        let mut params = HashMap::new();
        params.insert("file_path".to_string(), json!("./test.txt"));
        params.insert("old_str".to_string(), json!("old"));
        params.insert("new_str".to_string(), json!("new"));

        let result = editor.validate_tool_request("string_replace", &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tool_descriptions() {
        let editor = StringReplaceEditor::new();

        let tools = editor.get_available_tools();
        assert_eq!(tools, vec!["string_replace"]);

        let description = editor.get_tool_description("string_replace");
        assert!(description.is_some());
        assert!(description.unwrap().contains("Replace specific strings"));
    }
}
