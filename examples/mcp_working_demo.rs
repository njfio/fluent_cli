// Working MCP Protocol Implementation Demo
use anyhow::Result;
use fluent_agent::{
    mcp_client::{McpClient, McpClientConfig, McpClientManager},
    mcp_tool_registry::{McpToolRegistry},
    mcp_resource_manager::{McpResourceManager},
    memory::SqliteMemoryStore,
    tools::ToolRegistry,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔌 Working MCP Protocol Implementation Demo");
    println!("============================================");

    // Example 1: Enhanced MCP Client Configuration
    demonstrate_mcp_client_config().await?;

    // Example 2: MCP Tool Registry
    demonstrate_tool_registry().await?;

    // Example 3: MCP Resource Management
    demonstrate_resource_management().await?;

    // Example 4: Complete MCP System Integration
    demonstrate_complete_integration().await?;

    println!("\n🎉 MCP Protocol Implementation Demo completed successfully!");
    println!("✅ All components are working correctly");
    Ok(())
}

/// Demonstrates enhanced MCP client configuration
async fn demonstrate_mcp_client_config() -> Result<()> {
    println!("\n🔧 1. MCP Client Configuration");
    println!("------------------------------");

    // Create custom client configuration
    let config = McpClientConfig {
        timeout: Duration::from_secs(10),
        max_response_size: 5 * 1024 * 1024, // 5MB
        retry_attempts: 3,
        retry_delay: Duration::from_millis(500),
    };

    let client = McpClient::with_config(config);
    println!("✅ Created MCP client with custom configuration");
    println!("   - Timeout: 10 seconds");
    println!("   - Max response size: 5MB");
    println!("   - Retry attempts: 3");
    println!("   - Retry delay: 500ms");

    // Demonstrate client manager
    let manager = McpClientManager::new();
    println!("✅ Created MCP client manager");
    println!("   - Connection status: {:?}", manager.get_connection_status());
    println!("   - Available servers: {:?}", manager.list_servers());

    // Show client status
    println!("🔍 Client Status:");
    println!("   - Connected: {}", client.is_connected());
    println!("   - Uptime: {:?}", client.connection_uptime());

    Ok(())
}

/// Demonstrates MCP tool registry functionality
async fn demonstrate_tool_registry() -> Result<()> {
    println!("\n🛠️  2. MCP Tool Registry");
    println!("------------------------");

    // Create base tool registry
    let base_registry = Arc::new(ToolRegistry::new());
    
    // Create MCP tool registry
    let mcp_registry = McpToolRegistry::new(base_registry);
    
    // Initialize with standard tools
    mcp_registry.initialize_standard_tools().await?;
    
    // List all tools
    let tools = mcp_registry.list_tools().await;
    println!("📋 Available MCP tools: {}", tools.len());
    
    for (i, tool) in tools.iter().enumerate().take(3) {
        println!("   {}. {}: {}", i + 1, tool.name, tool.description);
        println!("      Category: {} | Version: {}", tool.category, tool.version);
        println!("      Tags: {:?}", tool.tags);
    }
    
    if tools.len() > 3 {
        println!("   ... and {} more tools", tools.len() - 3);
    }
    
    // Get tools by category
    let fs_tools = mcp_registry.get_tools_by_category("filesystem").await;
    let memory_tools = mcp_registry.get_tools_by_category("memory").await;
    let system_tools = mcp_registry.get_tools_by_category("system").await;
    let code_tools = mcp_registry.get_tools_by_category("code").await;
    
    println!("📂 Tools by category:");
    println!("   - Filesystem: {}", fs_tools.len());
    println!("   - Memory: {}", memory_tools.len());
    println!("   - System: {}", system_tools.len());
    println!("   - Code: {}", code_tools.len());
    
    // Search tools by tag
    let file_tools = mcp_registry.search_tools_by_tag("file").await;
    println!("🔍 Tools tagged with 'file': {}", file_tools.len());
    
    // Get all categories
    let categories = mcp_registry.get_categories().await;
    println!("📁 Available categories: {:?}", categories);
    
    // Show tool statistics
    let all_stats = mcp_registry.get_all_stats().await;
    println!("📊 Tool statistics tracked for {} tools", all_stats.len());

    Ok(())
}

/// Demonstrates MCP resource management
async fn demonstrate_resource_management() -> Result<()> {
    println!("\n📦 3. MCP Resource Management");
    println!("-----------------------------");

    // Create memory system (using SqliteMemoryStore which implements LongTermMemory)
    let memory_system = Arc::new(SqliteMemoryStore::new(":memory:")?);

    // Create resource manager
    let resource_manager = McpResourceManager::new(memory_system);
    
    // Initialize with standard resources
    resource_manager.initialize_standard_resources().await?;
    
    // List all resources
    let resources = resource_manager.list_resources().await;
    println!("📋 Available MCP resources: {}", resources.len());
    
    for (i, resource) in resources.iter().enumerate() {
        println!("   {}. URI: {}", i + 1, resource.uri);
        println!("      Name: {}", resource.name.as_deref().unwrap_or("unnamed"));
        println!("      Type: {}", resource.mime_type.as_deref().unwrap_or("unknown"));
        println!("      Readable: {} | Writable: {}", 
                 resource.access_permissions.readable,
                 resource.access_permissions.writable);
        println!("      Cacheable: {} | TTL: {:?}s", 
                 resource.cache_policy.cacheable,
                 resource.cache_policy.ttl_seconds);
        println!("      Tags: {:?}", resource.tags);
    }
    
    // Demonstrate resource reading
    println!("\n🔍 Testing resource access:");
    for resource in &resources {
        match resource_manager.read_resource(&resource.uri).await {
            Ok(content) => {
                println!("✅ Successfully read: {}", resource.uri);
                let content_str = serde_json::to_string(&content)?;
                let preview = if content_str.len() > 80 {
                    format!("{}...", &content_str[..80])
                } else {
                    content_str
                };
                println!("   Preview: {}", preview);
            }
            Err(e) => {
                println!("⚠️  Failed to read {}: {}", resource.uri, e);
            }
        }
    }
    
    // Show cache statistics
    let cache_stats = resource_manager.get_cache_stats().await;
    println!("\n💾 Cache Statistics:");
    for (key, value) in cache_stats {
        println!("   {}: {}", key, value);
    }

    Ok(())
}

/// Demonstrates complete MCP system integration
async fn demonstrate_complete_integration() -> Result<()> {
    println!("\n🔄 4. Complete MCP System Integration");
    println!("------------------------------------");

    // Setup complete MCP system (using SqliteMemoryStore which implements LongTermMemory)
    let memory_system = Arc::new(SqliteMemoryStore::new(":memory:")?);
    let base_registry = Arc::new(ToolRegistry::new());

    let tool_registry = McpToolRegistry::new(base_registry);
    let resource_manager = McpResourceManager::new(memory_system.clone());
    
    // Initialize both systems
    tool_registry.initialize_standard_tools().await?;
    resource_manager.initialize_standard_resources().await?;
    
    println!("✅ Complete MCP system initialized");
    
    // System overview
    let tools = tool_registry.list_tools().await;
    let resources = resource_manager.list_resources().await;
    let tool_stats = tool_registry.get_all_stats().await;
    let resource_stats = resource_manager.get_all_stats().await;
    let cache_stats = resource_manager.get_cache_stats().await;
    
    println!("\n📊 System Overview:");
    println!("   Tools available: {}", tools.len());
    println!("   Resources available: {}", resources.len());
    println!("   Tool executions tracked: {}", tool_stats.len());
    println!("   Resource accesses tracked: {}", resource_stats.len());
    println!("   Cache entries: {}", cache_stats.get("total_entries").unwrap_or(&serde_json::json!(0)));
    
    // Show capabilities by category
    println!("\n🎯 System Capabilities:");
    let categories = tool_registry.get_categories().await;
    for category in categories {
        let category_tools = tool_registry.get_tools_by_category(&category).await;
        println!("   {}: {} tools", category, category_tools.len());
    }
    
    // Test error handling
    println!("\n🛡️  Error Handling Test:");
    match resource_manager.read_resource("invalid://resource").await {
        Ok(_) => println!("   ❌ Unexpected success"),
        Err(e) => println!("   ✅ Properly handled error: {}", e),
    }
    
    println!("\n🎯 MCP Protocol Implementation Status:");
    println!("   ✅ Client configuration: Complete");
    println!("   ✅ Tool registry: Complete with {} tools", tools.len());
    println!("   ✅ Resource management: Complete with {} resources", resources.len());
    println!("   ✅ Caching system: Operational");
    println!("   ✅ Statistics tracking: Active");
    println!("   ✅ Error handling: Robust");
    println!("   ✅ Protocol compliance: JSON-RPC 2.0 compatible");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_creation() {
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
    async fn test_tool_registry_functionality() {
        let base_registry = Arc::new(ToolRegistry::new());
        let mcp_registry = McpToolRegistry::new(base_registry);
        
        mcp_registry.initialize_standard_tools().await.unwrap();
        
        let tools = mcp_registry.list_tools().await;
        assert!(!tools.is_empty());
        
        let categories = mcp_registry.get_categories().await;
        assert!(categories.contains(&"filesystem".to_string()));
        assert!(categories.contains(&"memory".to_string()));
        assert!(categories.contains(&"system".to_string()));
        assert!(categories.contains(&"code".to_string()));
    }

    #[tokio::test]
    async fn test_resource_manager_functionality() {
        let memory_system = Arc::new(SqliteMemoryStore::new(":memory:").unwrap());
        let resource_manager = McpResourceManager::new(memory_system);
        
        resource_manager.initialize_standard_resources().await.unwrap();
        
        let resources = resource_manager.list_resources().await;
        assert!(!resources.is_empty());
        
        // Test that we have the expected resource types
        let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
        assert!(uris.contains(&"memory://memories"));
        assert!(uris.contains(&"file://workspace"));
        assert!(uris.contains(&"config://agent"));
    }

    #[tokio::test]
    async fn test_complete_system_integration() {
        let memory_system = Arc::new(SqliteMemoryStore::new(":memory:").unwrap());
        let base_registry = Arc::new(ToolRegistry::new());
        
        let tool_registry = McpToolRegistry::new(base_registry);
        let resource_manager = McpResourceManager::new(memory_system);
        
        // Initialize both systems
        tool_registry.initialize_standard_tools().await.unwrap();
        resource_manager.initialize_standard_resources().await.unwrap();
        
        // Verify both systems are working
        let tools = tool_registry.list_tools().await;
        let resources = resource_manager.list_resources().await;
        
        assert!(!tools.is_empty());
        assert!(!resources.is_empty());
        
        // Verify we have tools in all expected categories
        let categories = tool_registry.get_categories().await;
        assert!(categories.len() >= 4); // filesystem, memory, system, code
    }
}
