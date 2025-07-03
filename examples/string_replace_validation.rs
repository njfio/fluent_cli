// Final validation test for the integrated StringReplaceEditor tool
use anyhow::Result;
use fluent_agent::config::ToolConfig;
use fluent_agent::tools::ToolRegistry;
use serde_json::json;
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 String Replace Editor - Final Validation Test");
    println!("=================================================");

    // Create a temporary directory for our validation
    let temp_dir = tempdir()?;
    let test_file = temp_dir.path().join("validation_test.rs");

    // Create a realistic Rust file to edit
    let original_content = r#"use std::collections::HashMap;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = HashMap::new();
    config.insert("debug", "false");
    config.insert("log_level", "info");
    config.insert("max_connections", "100");
    
    println!("Starting application...");
    println!("Configuration loaded: {:?}", config);
    
    let result = process_data("input.txt")?;
    println!("Processing complete: {}", result);
    
    Ok(())
}

fn process_data(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(filename)?;
    println!("Read {} bytes from {}", content.len(), filename);
    Ok(format!("Processed: {}", content.trim()))
}
"#;

    fs::write(&test_file, original_content).await?;
    println!("✅ Created test file: {}", test_file.display());

    // Create tool configuration with the string replace editor
    let tool_config = ToolConfig {
        file_operations: true,
        shell_commands: false,
        rust_compiler: false,
        git_operations: false,
        allowed_paths: Some(vec![temp_dir.path().to_string_lossy().to_string()]),
        allowed_commands: Some(vec![]),
    };

    // Create tool registry with all standard tools (including string_replace)
    let registry = ToolRegistry::with_standard_tools(&tool_config);

    println!(
        "✅ Created tool registry with {} tools",
        registry.get_all_available_tools().len()
    );

    // Validation Test 1: Replace debug configuration
    println!("\n🔧 Test 1: Replace debug configuration");
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("\"false\""));
    params.insert("new_str".to_string(), json!("\"true\""));
    params.insert("occurrence".to_string(), json!("First"));
    params.insert("create_backup".to_string(), json!(true));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!("   ✅ Debug configuration updated");

    // Validation Test 2: Update all println! to use a logging macro
    println!("\n🔧 Test 2: Replace println! with log::info!");
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("println!"));
    params.insert("new_str".to_string(), json!("log::info!"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("create_backup".to_string(), json!(false));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!("   ✅ All println! statements updated to log::info!");

    // Validation Test 3: Update function signature with line range
    println!("\n🔧 Test 3: Update function signature (line range)");
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("Box<dyn std::error::Error>"));
    params.insert("new_str".to_string(), json!("anyhow::Error"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("line_range".to_string(), json!([18, 25])); // Broader range to catch process_data function
    params.insert("create_backup".to_string(), json!(false));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!("   ✅ Function signature updated in specified range");

    // Validation Test 4: Dry run preview of a major change
    println!("\n🔧 Test 4: Dry run preview of HashMap replacement");
    let mut params = HashMap::new();
    params.insert("file_path".to_string(), json!(test_file.to_string_lossy()));
    params.insert("old_str".to_string(), json!("HashMap"));
    params.insert("new_str".to_string(), json!("BTreeMap"));
    params.insert("occurrence".to_string(), json!("All"));
    params.insert("dry_run".to_string(), json!(true));

    let result = registry.execute_tool("string_replace", &params).await?;
    println!("   ✅ Dry run completed - no changes made to file");

    // Show final file content
    println!("\n📄 Final file content after all modifications:");
    let final_content = fs::read_to_string(&test_file).await?;
    println!("{}", final_content);

    // Verify expected changes
    assert!(
        final_content.contains("\"true\""),
        "Debug should be set to true"
    );
    assert!(
        final_content.contains("log::info!"),
        "Should use log::info! instead of println!"
    );
    assert!(
        !final_content.contains("println!"),
        "Should not contain any println! statements"
    );
    assert!(
        final_content.contains("anyhow::Error"),
        "Should use anyhow::Error in process_data function"
    );
    assert!(
        final_content.contains("HashMap"),
        "HashMap should still exist (dry run didn't change it)"
    );

    println!("\n✅ All validation tests passed!");
    println!("🎉 String Replace Editor is fully integrated and working perfectly!");
    println!("🚀 Ready for production use in agentic workflows!");

    Ok(())
}
