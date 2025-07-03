// Comprehensive integration test for the StringReplaceEditor tool
use anyhow::Result;
use fluent_agent::config::ToolConfig;
use fluent_agent::tools::{ToolExecutor, ToolRegistry};
use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 String Replace Editor Integration Test");
    println!("==========================================");

    // Test 1: Tool Registry Integration
    test_tool_registry_integration().await?;

    // Test 2: Real File Operations
    test_real_file_operations().await?;

    // Test 3: Advanced Features
    test_advanced_features().await?;

    // Test 4: Error Handling
    test_error_handling().await?;

    // Test 5: Security Validation
    test_security_validation().await?;

    println!("\n✅ All integration tests passed!");
    println!("🎉 String Replace Editor is fully integrated and validated!");

    Ok(())
}

async fn test_tool_registry_integration() -> Result<()> {
    println!("\n🔧 Test 1: Tool Registry Integration");

    // Create tool configuration
    let tool_config = ToolConfig {
        file_operations: true,
        shell_commands: false,
        rust_compiler: false,
        git_operations: false,
        allowed_paths: Some(vec!["./".to_string()]),
        allowed_commands: Some(vec![]),
    };

    // Create tool registry with standard tools
    let registry = ToolRegistry::with_standard_tools(&tool_config);

    // Verify string_replace tool is available
    assert!(
        registry.is_tool_available("string_replace"),
        "string_replace tool should be available"
    );

    // Get all tools and verify string_replace is included
    let all_tools = registry.get_all_available_tools();
    let string_replace_tool = all_tools
        .iter()
        .find(|tool| tool.name == "string_replace")
        .expect("string_replace tool should be in the registry");

    println!("   ✅ String replace tool found in registry");
    println!(
        "   📝 Tool: {} ({})",
        string_replace_tool.name, string_replace_tool.description
    );
    println!("   🏷️  Executor: {}", string_replace_tool.executor);

    // Verify other expected tools are also present
    let expected_tools = vec![
        "read_file",
        "write_file",
        "list_directory",
        "string_replace",
    ];
    for expected_tool in expected_tools {
        assert!(
            registry.is_tool_available(expected_tool),
            "Expected tool '{}' should be available",
            expected_tool
        );
        println!("   ✅ Tool '{}' is available", expected_tool);
    }

    println!("   🎯 Tool registry integration: PASSED");
    Ok(())
}

async fn test_real_file_operations() -> Result<()> {
    println!("\n📁 Test 2: Real File Operations");

    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("integration_test.rs");

    // Create a test file
    let original_content = r#"use std::collections::HashMap;

fn main() {
    let mut data = HashMap::new();
    data.insert("key1", "value1");
    data.insert("key2", "value2");
    
    println!("Hello, world!");
    println!("Testing string replacement");
    
    // TODO: Add more functionality
    let result = calculate_sum(10, 20);
    println!("Result: {}", result);
}

fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}
"#;

    fs::write(&test_file, original_content).await?;
    println!("   📝 Created test file: {}", test_file.display());

    // Create tool registry
    let tool_config = ToolConfig {
        file_operations: true,
        shell_commands: false,
        rust_compiler: false,
        git_operations: false,
        allowed_paths: Some(vec![temp_dir.path().to_string_lossy().to_string()]),
        allowed_commands: Some(vec![]),
    };

    let registry = ToolRegistry::with_standard_tools(&tool_config);

    // Test 1: Replace first occurrence
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("println!"));
    params.insert("new_str".to_string(), json!("eprintln!"));
    params.insert("occurrence".to_string(), json!("First"));
    params.insert("create_backup".to_string(), json!(false));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!(
        "   🔄 First replacement result: {}",
        result.lines().take(3).collect::<Vec<_>>().join(" ")
    );

    // Verify the change
    let content = fs::read_to_string(&test_file).await?;
    assert!(content.contains("eprintln!(\"Hello, world!\")"));
    assert!(content.contains("println!(\"Testing string replacement\")"));
    println!("   ✅ First occurrence replacement: PASSED");

    // Test 2: Replace all occurrences
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("i32"));
    params.insert("new_str".to_string(), json!("u32"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("create_backup".to_string(), json!(false));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!(
        "   🔄 All replacements result: {}",
        result.lines().take(3).collect::<Vec<_>>().join(" ")
    );

    // Verify all changes
    let content = fs::read_to_string(&test_file).await?;
    assert!(content.contains("fn calculate_sum(a: u32, b: u32) -> u32"));
    assert!(!content.contains("i32"));
    println!("   ✅ All occurrences replacement: PASSED");

    println!("   🎯 Real file operations: PASSED");
    Ok(())
}

async fn test_advanced_features() -> Result<()> {
    println!("\n⚡ Test 3: Advanced Features");

    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("advanced_test.rs");

    // Create a test file with multiple sections
    let original_content = r#"// Section 1
fn function_one() {
    let value = "test";
    println!("Function one: {}", value);
}

// Section 2
fn function_two() {
    let value = "test";
    println!("Function two: {}", value);
}

// Section 3
fn function_three() {
    let value = "test";
    println!("Function three: {}", value);
}
"#;

    fs::write(&test_file, original_content).await?;

    let tool_config = ToolConfig {
        file_operations: true,
        shell_commands: false,
        rust_compiler: false,
        git_operations: false,
        allowed_paths: Some(vec![temp_dir.path().to_string_lossy().to_string()]),
        allowed_commands: Some(vec![]),
    };

    let registry = ToolRegistry::with_standard_tools(&tool_config);

    // Test 1: Dry run preview
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("\"test\""));
    params.insert("new_str".to_string(), json!("\"production\""));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("dry_run".to_string(), json!(true));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!(
        "   🔍 Dry run preview: {}",
        result.lines().take(5).collect::<Vec<_>>().join(" ")
    );

    // Verify file wasn't changed
    let content = fs::read_to_string(&test_file).await?;
    assert!(content.contains("\"test\""));
    println!("   ✅ Dry run preview: PASSED");

    // Test 2: Line range replacement
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("value"));
    params.insert("new_str".to_string(), json!("data"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("line_range".to_string(), json!([7, 12])); // Only function_two
    params.insert("create_backup".to_string(), json!(false));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!(
        "   📏 Line range replacement: {}",
        result.lines().take(3).collect::<Vec<_>>().join(" ")
    );

    // Verify only function_two was changed
    let content = fs::read_to_string(&test_file).await?;
    assert!(content.contains("let value = \"test\";") && content.contains("Function one"));
    assert!(content.contains("let data = \"test\";") && content.contains("Function two"));
    assert!(content.contains("let value = \"test\";") && content.contains("Function three"));
    println!("   ✅ Line range replacement: PASSED");

    // Test 3: Indexed occurrence replacement - simplified test
    let fresh_file = temp_dir.path().join("indexed_test.rs");
    let fresh_content = "FIRST println! SECOND println! THIRD println!";
    fs::write(&fresh_file, fresh_content).await?;

    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(fresh_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("println!"));
    params.insert("new_str".to_string(), json!("eprintln!"));
    params.insert("occurrence".to_string(), json!({"Index": 2})); // Second occurrence
    params.insert("create_backup".to_string(), json!(false));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!(
        "   🎯 Indexed replacement: {}",
        result.lines().take(3).collect::<Vec<_>>().join(" ")
    );

    // Verify the indexed replacement worked correctly
    let content = fs::read_to_string(&fresh_file).await?;
    println!("   🔍 Content after indexed replacement: {}", content);

    // Should have replaced only the 2nd occurrence
    assert!(
        content.contains("FIRST println!"),
        "First occurrence should remain"
    );
    assert!(
        content.contains("SECOND eprintln!"),
        "Second occurrence should be changed"
    );
    assert!(
        content.contains("THIRD println!"),
        "Third occurrence should remain"
    );

    println!("   ✅ Indexed occurrence replacement: PASSED");

    println!("   🎯 Advanced features: PASSED");
    Ok(())
}

async fn test_error_handling() -> Result<()> {
    println!("\n❌ Test 4: Error Handling");

    let temp_dir = tempdir()?;
    let tool_config = ToolConfig {
        file_operations: true,
        shell_commands: false,
        rust_compiler: false,
        git_operations: false,
        allowed_paths: Some(vec![temp_dir.path().to_string_lossy().to_string()]),
        allowed_commands: Some(vec![]),
    };

    let registry = ToolRegistry::with_standard_tools(&tool_config);

    // Test 1: Non-existent file
    let mut params = HashMap::new();
    params.insert(
        "file_path".to_string(),
        json!(temp_dir.path().join("nonexistent.txt").to_string_lossy()),
    );
    params.insert("old_str".to_string(), json!("test"));
    params.insert("new_str".to_string(), json!("replacement"));

    let result = registry.execute_tool("string_replace", &params).await?;
    assert!(result.contains("\"success\": false"));
    assert!(result.contains("File does not exist"));
    println!("   ✅ Non-existent file error: PASSED");

    // Test 2: No matches found
    let test_file = temp_dir.path().join("empty_test.txt");
    fs::write(&test_file, "Hello world").await?;

    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("nonexistent_string"));
    params.insert("new_str".to_string(), json!("replacement"));

    let result = registry.execute_tool("string_replace", &params).await?;
    assert!(result.contains("\"success\": false"));
    assert!(result.contains("No matches found"));
    println!("   ✅ No matches found error: PASSED");

    println!("   🎯 Error handling: PASSED");
    Ok(())
}

async fn test_security_validation() -> Result<()> {
    println!("\n🔒 Test 5: Security Validation");

    let temp_dir = tempdir()?;
    let tool_config = ToolConfig {
        file_operations: true,
        shell_commands: false,
        rust_compiler: false,
        git_operations: false,
        allowed_paths: Some(vec![temp_dir.path().to_string_lossy().to_string()]),
        allowed_commands: Some(vec![]),
    };

    let registry = ToolRegistry::with_standard_tools(&tool_config);

    // Test 1: Path outside allowed directories
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!("/etc/passwd"));
    params.insert("old_str".to_string(), json!("root"));
    params.insert("new_str".to_string(), json!("admin"));

    let result = registry.execute_tool("string_replace", &params).await;
    assert!(result.is_err());
    println!("   ✅ Path validation: PASSED");

    // Test 2: Missing required parameters
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!("test.txt"));
    // Missing old_str and new_str

    let result = registry.execute_tool("string_replace", &params).await;
    assert!(result.is_err());
    println!("   ✅ Parameter validation: PASSED");

    println!("   🎯 Security validation: PASSED");
    Ok(())
}
