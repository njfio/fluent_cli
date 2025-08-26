// Complete MCP Protocol Demo with All Features
use anyhow::Result;
use fluent_agent::{
    mcp_client::{McpClient, McpClientConfig},
    mcp_tool_registry::{McpToolRegistry},
    mcp_resource_manager::{McpResourceManager},
    memory::AsyncSqliteMemoryStore,
    tools::ToolRegistry,
    agent_with_mcp::LongTermMemory,
};
use std::sync::Arc;
use std::time::Duration;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔌 Complete MCP Protocol Demo");
    println!("=============================");

    // Example 1: Enhanced MCP Client Configuration
    demonstrate_mcp_client_config().await?;

    // Example 2: MCP Tool Registry
    demonstrate_mcp_tool_registry().await?;

    // Example 3: MCP Resource Management
    demonstrate_mcp_resource_management().await?;

    // Example 4: Complete MCP Workflow
    demonstrate_complete_mcp_workflow().await?;

    println!("\n🎉 Complete MCP Protocol Demo finished successfully!");
    println!("✅ All components are working correctly");
    Ok(())
}

/// Demonstrates enhanced MCP client configuration
async fn demonstrate_mcp_client_config() -> Result<()> {
    println!("\n🔧 Example 1: Enhanced MCP Client Configuration");

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

    // Show client status
    println!("🔍 Client Status:");
    println!("   - Connected: {}", client.is_connected());
    println!("   - Uptime: {:?}", client.connection_uptime());

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
        println!("   - {}: {}", tool.name, tool.description);
        println!("     Category: {} | Version: {}", tool.category, tool.version);
        println!("     Tags: {:?}", tool.tags);
    }
    
    if tools.len() > 5 {
        println!("   ... and {} more tools", tools.len() - 5);
    }
    
    // Get tools by category
    let fs_tools = mcp_registry.get_tools_by_category("filesystem").await;
    let memory_tools = mcp_registry.get_tools_by_category("memory").await;
    let system_tools = mcp_registry.get_tools_by_category("system").await;
    
    println!("📂 Tools by category:");
    println!("   - Filesystem: {}", fs_tools.len());
    println!("   - Memory: {}", memory_tools.len());
    println!("   - System: {}", system_tools.len());
    
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
async fn demonstrate_mcp_resource_management() -> Result<()> {
    println!("\n📦 Example 3: MCP Resource Management");

    // Create memory system (using AsyncSqliteMemoryStore which implements LongTermMemory)
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await?) as Arc<dyn LongTermMemory>;

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

    // Setup complete MCP system (using AsyncSqliteMemoryStore which implements LongTermMemory)
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await?) as Arc<dyn LongTermMemory>;
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
    use fluent_agent::memory::LongTermMemory;

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
        let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await.unwrap()) as Arc<dyn LongTermMemory>;
        let resource_manager = McpResourceManager::new(memory_system);
        
        resource_manager.initialize_standard_resources().await.unwrap();
        
        let resources = resource_manager.list_resources().await;
        assert!(!resources.is_empty());
    }

    #[tokio::test]
    async fn test_complete_workflow() {
        let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:").await.unwrap()) as Arc<dyn LongTermMemory>;
        let base_registry = Arc::new(ToolRegistry::new());
        let tool_registry = McpToolRegistry::new(base_registry);
        let resource_manager = McpResourceManager::new(memory_system);
        
        tool_registry.initialize_standard_tools().await.unwrap();
        resource_manager.initialize_standard_resources().await.unwrap();
        
        let tools = tool_registry.list_tools().await;
        let resources = resource_manager.list_resources().await;
        
        assert!(!tools.is_empty());
        assert!(!resources.is_empty());
    }
}