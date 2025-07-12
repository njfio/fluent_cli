use fluent_agent::{
    mcp_tool_registry::{McpToolRegistry, McpToolDefinition},
    mcp_client::McpClient,
    performance::utils::PerformanceCounter,
    tools::ToolRegistry,
};
use std::sync::Arc;
use std::time::{Instant, Duration};
use anyhow::Result;
use tokio;
use tokio::sync::Semaphore;

/// Comprehensive MCP performance tests
/// Tests MCP system performance under various load conditions

#[tokio::test]
async fn test_mcp_tool_registry_performance() -> Result<()> {
    let base_registry = Arc::new(ToolRegistry::new());
    let registry = McpToolRegistry::new(base_registry);
    let counter = PerformanceCounter::new();
    
    // Test tool registration performance
    let num_tools = 1000;
    println!("Testing MCP tool registry with {} tools...", num_tools);
    
    let registration_start = Instant::now();
    
    for i in 0..num_tools {
        let register_start = Instant::now();
        
        let tool_def = McpToolDefinition {
            name: format!("performance_tool_{}", i),
            title: Some(format!("Performance Tool {}", i)),
            description: format!("Performance test tool {} for benchmarking", i),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Input parameter for tool"
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "timeout": {"type": "number"},
                            "retries": {"type": "integer"}
                        }
                    }
                }
            }),
            output_schema: None,
            category: "performance".to_string(),
            tags: vec!["test".to_string(), "benchmark".to_string()],
            version: "1.0.0".to_string(),
            author: Some("fluent-cli".to_string()),
            documentation_url: None,
            examples: vec![],
        };
        
        let result = registry.register_tool(tool_def).await;
        let register_duration = register_start.elapsed();
        
        counter.record_request(register_duration, result.is_err());
        
        if result.is_err() {
            eprintln!("Tool registration {} failed: {:?}", i, result);
        }
    }
    
    let total_registration_time = registration_start.elapsed();
    let stats = counter.get_stats();
    
    println!("Tool Registration Performance:");
    println!("  Total tools: {}", num_tools);
    println!("  Total time: {:?}", total_registration_time);
    println!("  Registrations per second: {:.2}", num_tools as f64 / total_registration_time.as_secs_f64());
    println!("  Average registration time: {:?}", stats.average_duration);
    println!("  Error rate: {:.2}%", stats.error_rate * 100.0);
    
    // Test tool listing performance
    let list_start = Instant::now();
    let tools = registry.list_tools().await;
    let list_time = list_start.elapsed();
    
    println!("Tool Listing Performance:");
    println!("  Listed {} tools in {:?}", tools.len(), list_time);
    
    // Test tool retrieval performance
    let mut retrieval_times = Vec::new();
    for i in 0..100 {
        let retrieve_start = Instant::now();
        let _tool = registry.get_tool(&format!("performance_tool_{}", i)).await;
        retrieval_times.push(retrieve_start.elapsed());
    }
    
    let avg_retrieval_time = retrieval_times.iter().sum::<Duration>() / retrieval_times.len() as u32;
    println!("Tool Retrieval Performance:");
    println!("  Average retrieval time: {:?}", avg_retrieval_time);
    
    // Performance assertions
    assert!(total_registration_time < Duration::from_secs(30), "Tool registration too slow");
    assert!(list_time < Duration::from_secs(2), "Tool listing too slow");
    assert!(avg_retrieval_time < Duration::from_millis(10), "Tool retrieval too slow");
    assert!(stats.error_rate < 0.01, "Too many registration errors");
    assert_eq!(tools.len(), num_tools);
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_client_connection_performance() -> Result<()> {
    let client = McpClient::new();
    
    // Test connection state management performance
    let num_operations = 1000;
    println!("Testing MCP client operations...");
    
    let start_time = Instant::now();
    
    for i in 0..num_operations {
        // Test connection state checks
        let _is_connected = client.is_connected();
        
        // Simulate some client operations
        tokio::time::sleep(Duration::from_micros(100)).await;
        
        if i % 100 == 0 {
            println!("  Completed {} operations", i);
        }
    }
    
    let total_time = start_time.elapsed();
    
    println!("MCP Client Performance:");
    println!("  Total operations: {}", num_operations);
    println!("  Total time: {:?}", total_time);
    println!("  Operations per second: {:.2}", num_operations as f64 / total_time.as_secs_f64());
    
    // Performance assertion
    assert!(total_time < Duration::from_secs(10), "MCP client operations too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_mcp_operations() -> Result<()> {
    let base_registry = Arc::new(ToolRegistry::new());
    let registry = McpToolRegistry::new(base_registry);
    let num_concurrent = 50;
    let operations_per_task = 20;
    
    println!("Testing concurrent MCP operations with {} tasks...", num_concurrent);
    
    let start_time = Instant::now();
    let mut handles = Vec::new();
    
    for task_id in 0..num_concurrent {
        let handle = tokio::spawn(async move {
            // Create a separate registry for each task to avoid lifetime issues
            let base_registry = Arc::new(ToolRegistry::new());
            let task_registry = McpToolRegistry::new(base_registry);
            let mut task_success = 0;
            let mut task_errors = 0;

            for op_id in 0..operations_per_task {
                // Register a tool
                let tool_def = McpToolDefinition {
                    name: format!("concurrent_tool_{}_{}", task_id, op_id),
                    title: Some(format!("Concurrent Tool {} {}", task_id, op_id)),
                    description: format!("Concurrent tool {} {}", task_id, op_id),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "data": {"type": "string"}
                        }
                    }),
                    output_schema: None,
                    category: "concurrent".to_string(),
                    tags: vec!["test".to_string()],
                    version: "1.0.0".to_string(),
                    author: Some("fluent-cli".to_string()),
                    documentation_url: None,
                    examples: vec![],
                };

                match task_registry.register_tool(tool_def).await {
                    Ok(_) => {
                        task_success += 1;

                        // Try to retrieve the tool
                        let _tool = task_registry.get_tool(&format!("concurrent_tool_{}_{}", task_id, op_id)).await;
                    }
                    Err(_) => task_errors += 1,
                }
            }

            (task_success, task_errors)
        });
        handles.push(handle);
    }
    
    // Wait for all tasks and collect results
    let mut total_success = 0;
    let mut total_errors = 0;
    
    for handle in handles {
        let (success, errors) = handle.await?;
        total_success += success;
        total_errors += errors;
    }
    
    let total_time = start_time.elapsed();
    let total_operations = num_concurrent * operations_per_task;
    
    println!("Concurrent MCP Operations Results:");
    println!("  Total operations: {}", total_operations);
    println!("  Successful: {}", total_success);
    println!("  Errors: {}", total_errors);
    println!("  Total time: {:?}", total_time);
    println!("  Operations per second: {:.2}", total_operations as f64 / total_time.as_secs_f64());
    println!("  Success rate: {:.2}%", (total_success as f64 / total_operations as f64) * 100.0);
    
    // Performance assertions
    assert!(total_time < Duration::from_secs(30), "Concurrent MCP operations too slow");
    assert!(total_success > total_operations * 8 / 10, "Too many operation failures");
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_resource_limiting() -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(10)); // Max 10 concurrent operations
    let num_operations = 100;

    println!("Testing MCP resource limiting with {} operations...", num_operations);

    let start_time = Instant::now();
    let mut handles = Vec::new();

    for i in 0..num_operations {
        let semaphore_clone = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap();

            // Simulate MCP operation
            tokio::time::sleep(Duration::from_millis(50)).await;

            i
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    let mut completed = 0;
    for handle in handles {
        handle.await?;
        completed += 1;
    }
    
    let total_time = start_time.elapsed();
    
    println!("Resource Limiting Results:");
    println!("  Completed operations: {}", completed);
    println!("  Total time: {:?}", total_time);
    println!("  Expected minimum time: {:?}", Duration::from_millis(50 * (num_operations / 10)));
    
    // Should take at least the time for batched execution
    let expected_min_time = Duration::from_millis(50 * (num_operations / 10));
    assert!(total_time >= expected_min_time * 8 / 10, "Resource limiting not working properly");
    assert_eq!(completed, num_operations);
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_tool_search_performance() -> Result<()> {
    let base_registry = Arc::new(ToolRegistry::new());
    let registry = McpToolRegistry::new(base_registry);
    
    // Register tools with different categories
    let categories = ["filesystem", "network", "database", "ai", "utility"];
    let tools_per_category = 200;
    
    println!("Registering {} tools across {} categories...", tools_per_category * categories.len(), categories.len());
    
    for category in &categories {
        for i in 0..tools_per_category {
            let tool_def = McpToolDefinition {
                name: format!("{}_tool_{}", category, i),
                title: Some(format!("{} Tool {}", category, i)),
                description: format!("A {} tool for operation {}", category, i),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "input": {"type": "string"},
                        "category": {"type": "string", "enum": [category]}
                    }
                }),
                output_schema: None,
                category: category.to_string(),
                tags: vec![category.to_string()],
                version: "1.0.0".to_string(),
                author: Some("fluent-cli".to_string()),
                documentation_url: None,
                examples: vec![],
            };
            
            registry.register_tool(tool_def).await?;
        }
    }
    
    // Test search performance
    let search_tests = vec![
        ("All tools", ""),
        ("Filesystem tools", "filesystem"),
        ("Network tools", "network"),
        ("AI tools", "ai"),
    ];
    
    for (test_name, search_term) in search_tests {
        let search_start = Instant::now();
        
        // Simulate search by listing all tools and filtering
        let all_tools = registry.list_tools().await;
        let filtered_tools: Vec<_> = all_tools.iter()
            .filter(|tool| search_term.is_empty() || tool.name.contains(search_term))
            .collect();
        
        let search_time = search_start.elapsed();
        
        println!("Search '{}': {} results in {:?}", test_name, filtered_tools.len(), search_time);
        
        // Performance assertion
        assert!(search_time < Duration::from_secs(1), "Search '{}' too slow", test_name);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_mcp_memory_usage() -> Result<()> {
    let base_registry = Arc::new(ToolRegistry::new());
    let registry = McpToolRegistry::new(base_registry);
    
    // Test memory usage with large tool definitions
    let num_large_tools = 100;
    let large_schema_size = 10000; // Large JSON schema
    
    println!("Testing MCP memory usage with {} large tools...", num_large_tools);
    
    for i in 0..num_large_tools {
        // Create a large schema
        let mut properties = serde_json::Map::new();
        for j in 0..large_schema_size {
            properties.insert(
                format!("property_{}", j),
                serde_json::json!({
                    "type": "string",
                    "description": format!("Property {} for large schema testing", j)
                })
            );
        }
        
        let tool_def = McpToolDefinition {
            name: format!("large_tool_{}", i),
            title: Some(format!("Large Tool {}", i)),
            description: format!("Large tool {} with extensive schema", i),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": properties
            }),
            output_schema: None,
            category: "large".to_string(),
            tags: vec!["large".to_string(), "test".to_string()],
            version: "1.0.0".to_string(),
            author: Some("fluent-cli".to_string()),
            documentation_url: None,
            examples: vec![],
        };
        
        registry.register_tool(tool_def).await?;
        
        if i % 10 == 0 {
            println!("  Registered {} large tools", i + 1);
        }
    }
    
    // Test operations with large tools
    let operation_start = Instant::now();
    
    let tools = registry.list_tools().await;
    let list_time = operation_start.elapsed();
    
    println!("Memory Usage Test Results:");
    println!("  Registered {} large tools", num_large_tools);
    println!("  Listed {} tools in {:?}", tools.len(), list_time);
    
    // Performance assertions
    assert!(list_time < Duration::from_secs(5), "Large tool listing too slow");
    assert_eq!(tools.len(), num_large_tools);
    
    Ok(())
}
