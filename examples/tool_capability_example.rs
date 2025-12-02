use fluent_agent::tools::ToolCapabilityConfig;

fn main() {
    // Example 1: Create a default configuration
    let default_config = ToolCapabilityConfig::default();
    println!("Default configuration:");
    println!("  Max file size: {} bytes", default_config.max_file_size);
    println!("  Timeout: {} seconds", default_config.timeout_seconds);
    println!("  Allow network: {}", default_config.allow_network);
    println!();

    // Example 2: Create a custom configuration using the builder pattern
    let custom_config = ToolCapabilityConfig::new()
        .with_max_file_size(5 * 1024 * 1024) // 5MB
        .with_allowed_paths(vec![
            "./src".to_string(),
            "./tests".to_string(),
            "./examples".to_string(),
        ])
        .with_allowed_commands(vec![
            "cargo".to_string(),
            "rustc".to_string(),
            "git".to_string(),
        ])
        .with_max_output_size(2 * 1024 * 1024) // 2MB
        .with_timeout(60)
        .with_network(true)
        .with_read_only(false)
        .with_max_concurrent(10);

    println!("Custom configuration:");
    println!("  Max file size: {} bytes", custom_config.max_file_size);
    println!("  Timeout: {} seconds", custom_config.timeout_seconds);
    println!("  Allow network: {}", custom_config.allow_network);
    println!("  Allowed paths: {:?}", custom_config.allowed_paths);
    println!("  Max concurrent executions: {}", custom_config.max_concurrent_executions);
    println!();

    // Example 3: Serialize to JSON
    let json = serde_json::to_string_pretty(&custom_config).unwrap();
    println!("Configuration as JSON:");
    println!("{}", json);
    println!();

    // Example 4: Convert to ToolExecutionConfig for backward compatibility
    let execution_config = custom_config.to_execution_config();
    println!("Converted to ToolExecutionConfig:");
    println!("  Timeout: {} seconds", execution_config.timeout_seconds);
    println!("  Max output size: {} bytes", execution_config.max_output_size);
    println!("  Read only: {}", execution_config.read_only);
    println!();

    // Example 5: Load from JSON
    let json_config = r#"{
        "max_file_size": 20971520,
        "allowed_paths": ["./"],
        "allowed_commands": ["cargo", "git"],
        "max_output_size": 2097152,
        "timeout_seconds": 45,
        "allow_network": false,
        "read_only": true,
        "max_concurrent_executions": 3
    }"#;

    let loaded_config: ToolCapabilityConfig =
        serde_json::from_str(json_config).expect("Failed to parse JSON");
    println!("Loaded configuration from JSON:");
    println!("  Max file size: {} bytes", loaded_config.max_file_size);
    println!("  Read only: {}", loaded_config.read_only);
    println!("  Max concurrent: {}", loaded_config.max_concurrent_executions);
}
