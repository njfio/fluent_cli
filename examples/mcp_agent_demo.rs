use anyhow::Result;
use fluent_agent::agent_with_mcp::AgentWithMcp;
use fluent_agent::memory::SqliteMemoryStore;
use fluent_agent::reasoning::LLMReasoningEngine;
use fluent_core::traits::Engine;
use fluent_engines::openai::OpenAIEngine;
use serde_json::json;
use std::sync::Arc;
use tokio;

/// Comprehensive demo of MCP agent capabilities
/// 
/// This example demonstrates:
/// 1. Connecting to multiple MCP servers
/// 2. AI-powered tool selection and execution
/// 3. Learning from tool usage patterns
/// 4. Complex multi-step task execution
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Fluent CLI MCP Agent Demo");
    
    // Initialize the agent with memory and reasoning
    let agent = setup_agent().await?;
    
    // Connect to MCP servers
    setup_mcp_servers(&agent).await?;
    
    // Demonstrate various capabilities
    demo_tool_discovery(&agent).await?;
    demo_intelligent_task_execution(&agent).await?;
    demo_learning_and_adaptation(&agent).await?;
    demo_complex_workflow(&agent).await?;
    
    println!("✅ Demo completed successfully!");
    Ok(())
}

/// Setup the agent with memory and reasoning capabilities
async fn setup_agent() -> Result<AgentWithMcp> {
    println!("🧠 Setting up agent with memory and reasoning...");
    
    // Create memory system
    let memory = Arc::new(SqliteMemoryStore::new("demo_agent_memory.db").await?);
    
    // Create reasoning engine (mock for demo - in real usage, use actual LLM)
    let reasoning_engine = Box::new(MockReasoningEngine::new());
    
    let agent = AgentWithMcp::new(memory, reasoning_engine);
    
    println!("✅ Agent initialized with persistent memory");
    Ok(agent)
}

/// Connect to various MCP servers for different capabilities
async fn setup_mcp_servers(agent: &AgentWithMcp) -> Result<()> {
    println!("🔌 Connecting to MCP servers...");
    
    // Connect to filesystem server
    match agent.connect_to_mcp_server(
        "filesystem".to_string(),
        "mcp-server-filesystem",
        &["--stdio"]
    ).await {
        Ok(_) => println!("✅ Connected to filesystem server"),
        Err(e) => println!("⚠️ Filesystem server not available: {}", e),
    }
    
    // Connect to git server
    match agent.connect_to_mcp_server(
        "git".to_string(),
        "mcp-server-git", 
        &["--stdio"]
    ).await {
        Ok(_) => println!("✅ Connected to git server"),
        Err(e) => println!("⚠️ Git server not available: {}", e),
    }
    
    // Connect to web/fetch server
    match agent.connect_to_mcp_server(
        "web".to_string(),
        "mcp-server-fetch",
        &["--stdio"]
    ).await {
        Ok(_) => println!("✅ Connected to web server"),
        Err(e) => println!("⚠️ Web server not available: {}", e),
    }
    
    // Connect to database server
    match agent.connect_to_mcp_server(
        "database".to_string(),
        "mcp-server-sqlite",
        &["--stdio"]
    ).await {
        Ok(_) => println!("✅ Connected to database server"),
        Err(e) => println!("⚠️ Database server not available: {}", e),
    }
    
    Ok(())
}

/// Demonstrate tool discovery across multiple servers
async fn demo_tool_discovery(agent: &AgentWithMcp) -> Result<()> {
    println!("\n🔍 Discovering available MCP tools...");
    
    let tools = agent.get_available_tools().await;
    
    for (server_name, server_tools) in &tools {
        println!("📦 Server '{}' provides {} tools:", server_name, server_tools.len());
        for tool in server_tools {
            println!("  🔧 {} - {}", tool.name, tool.description);
        }
    }
    
    if tools.is_empty() {
        println!("⚠️ No MCP servers connected. Using simulated tools for demo.");
        simulate_tool_discovery();
    }
    
    Ok(())
}

/// Demonstrate intelligent task execution with AI-powered tool selection
async fn demo_intelligent_task_execution(agent: &AgentWithMcp) -> Result<()> {
    println!("\n🤖 Demonstrating intelligent task execution...");
    
    let tasks = vec![
        "Read the README.md file and summarize its contents",
        "Check the git status of the current repository", 
        "List all files in the current directory",
        "Fetch the latest news from a technology website",
        "Query the database for user statistics",
    ];
    
    for task in tasks {
        println!("\n📋 Task: {}", task);
        
        match agent.execute_task_with_mcp(task).await {
            Ok(result) => {
                println!("✅ Result: {}", result);
            }
            Err(e) => {
                println!("⚠️ Task failed: {}", e);
                // Demonstrate fallback behavior
                println!("🔄 Attempting alternative approach...");
                demonstrate_fallback_behavior(task).await?;
            }
        }
    }
    
    Ok(())
}

/// Demonstrate learning from tool usage patterns
async fn demo_learning_and_adaptation(agent: &AgentWithMcp) -> Result<()> {
    println!("\n🧠 Demonstrating learning and adaptation...");
    
    // Learn from file operations
    let file_insights = agent.learn_from_mcp_usage("file operations").await?;
    println!("📚 Insights from file operations:");
    for insight in file_insights {
        println!("  💡 {}", insight);
    }
    
    // Learn from git operations
    let git_insights = agent.learn_from_mcp_usage("git").await?;
    println!("📚 Insights from git operations:");
    for insight in git_insights {
        println!("  💡 {}", insight);
    }
    
    // Get recommendations for new servers
    let recommendations = agent.get_mcp_server_recommendations("development").await;
    println!("🎯 Recommended MCP servers for development:");
    for rec in recommendations {
        println!("  📦 {}", rec);
    }
    
    Ok(())
}

/// Demonstrate complex multi-step workflow
async fn demo_complex_workflow(agent: &AgentWithMcp) -> Result<()> {
    println!("\n🔄 Demonstrating complex multi-step workflow...");
    
    let workflow_steps = vec![
        "Analyze the current project structure",
        "Check for any uncommitted changes in git",
        "Read the project's package.json or Cargo.toml",
        "Identify potential security vulnerabilities",
        "Generate a project health report",
    ];
    
    let mut workflow_results = Vec::new();
    
    for (i, step) in workflow_steps.iter().enumerate() {
        println!("\n🔸 Step {}: {}", i + 1, step);
        
        match agent.execute_task_with_mcp(step).await {
            Ok(result) => {
                println!("✅ Completed: {}", result);
                workflow_results.push(format!("Step {}: {}", i + 1, result));
            }
            Err(e) => {
                println!("⚠️ Step failed: {}", e);
                workflow_results.push(format!("Step {}: Failed - {}", i + 1, e));
            }
        }
    }
    
    // Generate final report
    println!("\n📊 Workflow Summary:");
    for result in workflow_results {
        println!("  📋 {}", result);
    }
    
    Ok(())
}

/// Simulate tool discovery when no real MCP servers are available
fn simulate_tool_discovery() {
    println!("🎭 Simulated MCP Tools:");
    println!("📦 filesystem server:");
    println!("  🔧 read_file - Read contents of a file");
    println!("  🔧 write_file - Write content to a file");
    println!("  🔧 list_directory - List files in a directory");
    
    println!("📦 git server:");
    println!("  🔧 git_status - Get repository status");
    println!("  🔧 git_log - Get commit history");
    println!("  🔧 git_diff - Show changes");
    
    println!("📦 web server:");
    println!("  🔧 fetch_url - Fetch content from URL");
    println!("  🔧 download_file - Download file from web");
}

/// Demonstrate fallback behavior when MCP tools aren't available
async fn demonstrate_fallback_behavior(task: &str) -> Result<()> {
    println!("🔄 Using built-in capabilities for: {}", task);
    
    // Simulate fallback logic
    if task.contains("README") {
        println!("📖 Using built-in file reader...");
        println!("✅ Fallback: Successfully read file using standard library");
    } else if task.contains("git") {
        println!("🔧 Using built-in git commands...");
        println!("✅ Fallback: Executed git command directly");
    } else if task.contains("directory") {
        println!("📁 Using built-in directory listing...");
        println!("✅ Fallback: Listed directory using std::fs");
    } else {
        println!("🤔 No suitable fallback available");
    }
    
    Ok(())
}

/// Mock reasoning engine for demo purposes
struct MockReasoningEngine;

impl MockReasoningEngine {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl fluent_agent::reasoning::ReasoningEngine for MockReasoningEngine {
    async fn reason(&self, context: &fluent_agent::context::ExecutionContext) -> Result<fluent_agent::orchestrator::ReasoningResult> {
        use fluent_agent::orchestrator::{ReasoningResult, ReasoningType};
        
        // Simulate intelligent reasoning about tool selection
        let goal = context.get_current_goal()
            .map(|g| g.description.clone())
            .unwrap_or_default();
            
        let reasoning_output = if goal.contains("file") || goal.contains("README") {
            json!({
                "server": "filesystem",
                "tool": "read_file", 
                "arguments": {"path": "README.md"}
            }).to_string()
        } else if goal.contains("git") {
            json!({
                "server": "git",
                "tool": "git_status",
                "arguments": {}
            }).to_string()
        } else if goal.contains("directory") {
            json!({
                "server": "filesystem", 
                "tool": "list_directory",
                "arguments": {"path": "."}
            }).to_string()
        } else {
            json!({"no_tool": true}).to_string()
        };
        
        Ok(ReasoningResult {
            reasoning_type: ReasoningType::ToolSelection,
            input_context: goal,
            reasoning_output,
            confidence_score: 0.8,
            goal_achieved_confidence: 0.7,
            next_actions: vec!["Execute selected tool".to_string()],
        })
    }
    
    fn get_capabilities(&self) -> Vec<fluent_agent::reasoning::ReasoningCapability> {
        vec![fluent_agent::reasoning::ReasoningCapability::ToolSelection]
    }
    
    fn can_handle(&self, reasoning_type: &fluent_agent::orchestrator::ReasoningType) -> bool {
        matches!(reasoning_type, fluent_agent::orchestrator::ReasoningType::ToolSelection)
    }
}
