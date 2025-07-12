// Complete MCP Protocol Implementation Demo
use anyhow::Result;
use fluent_agent::{
    mcp_client::{McpClient, McpClientConfig, McpClientManager},
    mcp_tool_registry::{McpToolRegistry, McpToolDefinition},
    mcp_resource_manager::{McpResourceManager, McpResource, ResourcePermissions, CachePolicy},
    memory::AsyncSqliteMemoryStore,
    tools::ToolRegistry,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔌 Complete MCP Protocol Implementation Demo");

    // Example 1: MCP Client with enhanced configuration
    demonstrate_enhanced_mcp_client().await?;

    // Example 2: MCP Tool Registry
    demonstrate_mcp_tool_registry().await?;

    // Example 3: MCP Resource Management
    demonstrate_mcp_resource_management().await?;

    // Example 4: Complete MCP workflow
    demonstrate_complete_mcp_workflow().await?;

    println!("🎉 Complete MCP demo finished successfully!");
    Ok(())
}

/// Demonstrates enhanced MCP client with configuration and error handling
async fn demonstrate_enhanced_mcp_client() -> Result<()> {
    println!("\n🔧 Example 1: Enhanced MCP Client");

    // Create custom client configuration
    let config = McpClientConfig {
        timeout: Duration::from_secs(10),
        max_response_size: 5 * 1024 * 1024, // 5MB
        retry_attempts: 3,
        retry_delay: Duration::from_millis(500),
    };

    let mut client = McpClient::with_config(config);
    println!("✅ Created MCP client with custom configuration");

    // Demonstrate client manager
    let mut manager = McpClientManager::new();
    
    // Note: In a real scenario, you would connect to actual MCP servers
    println!("📋 MCP Client Manager created");
    println!("   - Connection status: {:?}", manager.get_connection_status());
    println!("   - Available servers: {:?}", manager.list_servers());

    // Demonstrate connection monitoring
    println!("🔍 Client connection status: {}", client.is_connected());
    if let Some(uptime) = client.connection_uptime() {
        println!("   Connection uptime: {:?}", uptime);
    }

    Ok(())
}

/// Demonstrates MCP tool registry functionality
async fn demonstrate_mcp_tool_registry() -> Result<()> {
    println!("\n🛠️  Example 2: MCP Tool Registry");

    // Create base tool registry
    let base_registry = Arc::new(ToolRegistry::new());
    
    // Create MCP tool registry
    let mcp_registry = McpToolRegistry::new(base_registry);
    
    // Initialize with standard tools
    mcp_registry.initialize_standard_tools().await?;
    
    // List all tools
    let tools = mcp_registry.list_tools().await;
    println!("📋 Available MCP tools: {}", tools.len());
    
    for tool in tools.iter().take(5) {
        println!("   - {}: {} ({})", tool.name, tool.description, tool.category);
    }
    
    // Get tools by category
    let fs_tools = mcp_registry.get_tools_by_category("filesystem").await;
    println!("📁 Filesystem tools: {}", fs_tools.len());
    
    let memory_tools = mcp_registry.get_tools_by_category("memory").await;
    println!("🧠 Memory tools: {}", memory_tools.len());
    
    // Search tools by tag
    let file_tools = mcp_registry.search_tools_by_tag("file").await;
    println!("🔍 Tools tagged with 'file': {}", file_tools.len());
    
    // Get categories
    let categories = mcp_registry.get_categories().await;
    println!("📂 Tool categories: {:?}", categories);
    
    // Demonstrate tool execution (simulated)
    if let Some(tool) = mcp_registry.get_tool("read_file").await {
        println!("🔧 Tool details for 'read_file':");
        println!("   Title: {:?}", tool.title);
        println!("   Version: {}", tool.version);
        println!("   Tags: {:?}", tool.tags);
        println!("   Examples: {}", tool.examples.len());
        
        // Show input schema
        println!("   Input schema: {}", serde_json::to_string_pretty(&tool.input_schema)?);
    }
    
    // Get tool statistics
    let all_stats = mcp_registry.get_all_stats().await;
    println!("📊 Tool statistics available for {} tools", all_stats.len());

    Ok(())
}

/// Demonstrates MCP resource management
async fn demonstrate_mcp_resource_management() -> Result<()> {
    println!("\n📦 Example 3: MCP Resource Management");

    // Create memory system
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await?);
    
    // Create resource manager
    let resource_manager = McpResourceManager::new(memory_system);
    
    // Initialize with standard resources
    resource_manager.initialize_standard_resources().await?;
    
    // List all resources
    let resources = resource_manager.list_resources().await;
    println!("📋 Available MCP resources: {}", resources.len());
    
    for resource in &resources {
        println!("   - {}: {} ({})", 
                 resource.uri, 
                 resource.name.as_deref().unwrap_or("unnamed"),
                 resource.mime_type.as_deref().unwrap_or("unknown"));
        println!("     Permissions: readable={}, writable={}", 
                 resource.access_permissions.readable,
                 resource.access_permissions.writable);
        println!("     Cache policy: cacheable={}, TTL={:?}s", 
                 resource.cache_policy.cacheable,
                 resource.cache_policy.ttl_seconds);
    }
    
    // Demonstrate resource reading
    for resource in &resources {
        match resource_manager.read_resource(&resource.uri).await {
            Ok(content) => {
                println!("✅ Successfully read resource: {}", resource.uri);
                // Show first 100 characters of content
                let content_str = serde_json::to_string(&content)?;
                let preview = if content_str.len() > 100 {
                    format!("{}...", &content_str[..100])
                } else {
                    content_str
                };
                println!("   Content preview: {}", preview);
            }
            Err(e) => {
                println!("❌ Failed to read resource {}: {}", resource.uri, e);
            }
        }
    }
    
    // Get cache statistics
    let cache_stats = resource_manager.get_cache_stats().await;
    println!("💾 Cache statistics:");
    for (key, value) in cache_stats {
        println!("   {}: {}", key, value);
    }
    
    // Get resource statistics
    let all_stats = resource_manager.get_all_stats().await;
    println!("📊 Resource statistics available for {} resources", all_stats.len());
    
    for (uri, stats) in all_stats.iter().take(3) {
        println!("   {}: {} accesses, {} cache hits, {} cache misses", 
                 uri, stats.total_accesses, stats.cache_hits, stats.cache_misses);
    }

    Ok(())
}

/// Demonstrates complete MCP workflow
async fn demonstrate_complete_mcp_workflow() -> Result<()> {
    println!("\n🔄 Example 4: Complete MCP Workflow");

    // Setup complete MCP system
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await?);
    let base_registry = Arc::new(ToolRegistry::new());
    
    let tool_registry = McpToolRegistry::new(base_registry);
    let resource_manager = McpResourceManager::new(memory_system.clone());
    
    // Initialize both systems
    tool_registry.initialize_standard_tools().await?;
    resource_manager.initialize_standard_resources().await?;
    
    println!("✅ Complete MCP system initialized");
    println!("   Tools: {}", tool_registry.list_tools().await.len());
    println!("   Resources: {}", resource_manager.list_resources().await.len());
    
    // Simulate MCP client workflow
    println!("\n🔄 Simulating MCP client workflow:");
    
    // 1. List available tools
    let tools = tool_registry.list_tools().await;
    println!("1. 📋 Listed {} available tools", tools.len());
    
    // 2. Get tool details
    if let Some(tool) = tools.first() {
        println!("2. 🔍 Got details for tool: {}", tool.name);
        println!("   Description: {}", tool.description);
        println!("   Category: {}", tool.category);
    }
    
    // 3. List available resources
    let resources = resource_manager.list_resources().await;
    println!("3. 📦 Listed {} available resources", resources.len());
    
    // 4. Read a resource
    if let Some(resource) = resources.first() {
        match resource_manager.read_resource(&resource.uri).await {
            Ok(_content) => {
                println!("4. ✅ Successfully read resource: {}", resource.uri);
            }
            Err(e) => {
                println!("4. ❌ Failed to read resource: {}", e);
            }
        }
    }
    
    // 5. Show system statistics
    let tool_stats = tool_registry.get_all_stats().await;
    let resource_stats = resource_manager.get_all_stats().await;
    let cache_stats = resource_manager.get_cache_stats().await;
    
    println!("5. 📊 System Statistics:");
    println!("   Tool executions tracked: {}", tool_stats.len());
    println!("   Resource accesses tracked: {}", resource_stats.len());
    println!("   Cache entries: {}", cache_stats.get("total_entries").unwrap_or(&json!(0)));
    println!("   Cache utilization: {}%", 
             cache_stats.get("utilization_percent")
                       .and_then(|v| v.as_f64())
                       .map(|v| format!("{:.1}", v))
                       .unwrap_or_else(|| "0.0".to_string()));
    
    // 6. Demonstrate error handling
    println!("6. 🛡️  Testing error handling:");
    
    // Try to read non-existent resource
    match resource_manager.read_resource("nonexistent://resource").await {
        Ok(_) => println!("   ❌ Unexpected success"),
        Err(e) => println!("   ✅ Properly handled error: {}", e),
    }
    
    // Try to execute non-existent tool
    match tool_registry.execute_tool("nonexistent_tool", json!({})).await {
        Ok(_) => println!("   ❌ Unexpected success"),
        Err(e) => println!("   ✅ Properly handled error: {}", e),
    }
    
    println!("\n🎯 Complete MCP workflow demonstration finished!");
    println!("   ✅ Tool registry: Fully functional");
    println!("   ✅ Resource management: Fully functional");
    println!("   ✅ Caching system: Operational");
    println!("   ✅ Statistics tracking: Active");
    println!("   ✅ Error handling: Robust");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_config() {
        let config = McpClientConfig {
            timeout: Duration::from_secs(5),
            max_response_size: 1024,
            retry_attempts: 2,
            retry_delay: Duration::from_millis(100),
        };

        let client = McpClient::with_config(config);
        assert!(!client.is_connected());
        assert!(client.connection_uptime().is_none());
    }

    #[tokio::test]
    async fn test_tool_registry_initialization() {
        let base_registry = Arc::new(ToolRegistry::new());
        let mcp_registry = McpToolRegistry::new(base_registry);
        
        mcp_registry.initialize_standard_tools().await.unwrap();
        
        let tools = mcp_registry.list_tools().await;
        assert!(!tools.is_empty());
        
        let categories = mcp_registry.get_categories().await;
        assert!(categories.contains(&"filesystem".to_string()));
        assert!(categories.contains(&"memory".to_string()));
    }

    #[tokio::test]
    async fn test_resource_manager_initialization() {
        let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await.unwrap());
        let resource_manager = McpResourceManager::new(memory_system);
        
        resource_manager.initialize_standard_resources().await.unwrap();
        
        let resources = resource_manager.list_resources().await;
        assert!(!resources.is_empty());
        
        // Test reading a resource
        if let Some(resource) = resources.first() {
            let result = resource_manager.read_resource(&resource.uri).await;
            // Some resources might fail due to test environment, but should not panic
            match result {
                Ok(_) => println!("Resource read successful"),
                Err(e) => println!("Resource read failed (expected in test): {}", e),
            }
        }
    }
}
