// Real Agentic System Demo - No Mocks, Real Implementation
use anyhow::Result;
use chrono::Utc;
use fluent_agent::{
    config::{credentials, AgentEngineConfig, ToolConfig},
    context::ExecutionContext,
    goal::{Goal, GoalType},
    memory::{LongTermMemory, MemoryItem, MemoryQuery, MemoryType, SqliteMemoryStore},
    tools::ToolRegistry,
};
use serde_json::json;
use std::collections::HashMap;


#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 Real Agentic System Demo");
    println!("============================");

    // Demo 1: Real Memory System
    println!("\n📚 Demo 1: Real SQLite Memory System");
    demo_memory_system().await?;

    // Demo 2: Real Goal System
    println!("\n🎯 Demo 2: Real Goal Management");
    demo_goal_system().await?;

    // Demo 3: Real Context Management
    println!("\n📋 Demo 3: Real Context Management");
    demo_context_system().await?;

    // Demo 4: Real Tool System
    println!("\n🔧 Demo 4: Real Tool System");
    demo_tool_system().await?;

    // Demo 5: Real Configuration System
    println!("\n⚙️  Demo 5: Real Configuration System");
    demo_config_system().await?;

    println!("\n✅ All demos completed successfully!");
    println!("🚀 The agentic system is fully operational with real implementations!");

    Ok(())
}

async fn demo_memory_system() -> Result<()> {
    // Create real SQLite memory store
    // Note: Using SqliteMemoryStore temporarily until AsyncSqliteMemoryStore implements LongTermMemory
    let memory_store = SqliteMemoryStore::new("demo_agent_memory.db")?;
    println!("✅ Created real SQLite database: demo_agent_memory.db");

    // Store real memory items
    let experiences = vec![
        MemoryItem {
            memory_id: "exp_001".to_string(),
            memory_type: MemoryType::Experience,
            content: "Successfully compiled Rust project with zero warnings".to_string(),
            metadata: {
                let mut map = HashMap::new();
                map.insert("project_type".to_string(), json!("rust"));
                map.insert("outcome".to_string(), json!("success"));
                map
            },
            importance: 0.8,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec![
                "rust".to_string(),
                "compilation".to_string(),
                "success".to_string(),
            ],
            embedding: None,
        },
        MemoryItem {
            memory_id: "learn_001".to_string(),
            memory_type: MemoryType::Learning,
            content: "When using async/await in Rust, always handle Result types properly"
                .to_string(),
            metadata: {
                let mut map = HashMap::new();
                map.insert("language".to_string(), json!("rust"));
                map.insert("concept".to_string(), json!("async_programming"));
                map
            },
            importance: 0.9,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 3,
            tags: vec![
                "rust".to_string(),
                "async".to_string(),
                "best_practice".to_string(),
            ],
            embedding: None,
        },
    ];

    // Store memories in real database
    for memory in &experiences {
        let stored_id = memory_store.store(memory.clone()).await?;
        println!(
            "✅ Stored memory: {:?} -> {}",
            memory.memory_type, stored_id
        );
    }

    // Query real memories
    let query = MemoryQuery {
        memory_types: vec![MemoryType::Experience, MemoryType::Learning],
        importance_threshold: Some(0.7),
        limit: Some(10),
        query_text: "rust".to_string(),
        tags: vec![],
        time_range: None,
    };

    let retrieved = memory_store.search(query).await?;
    println!("✅ Found {} memories matching search criteria", retrieved.len());

    for memory in retrieved {
        println!("   📝 {:?}: {}", memory.memory_type, memory.content);
        println!(
            "      Importance: {}, Access count: {}",
            memory.importance, memory.access_count
        );
    }

    Ok(())
}

async fn demo_goal_system() -> Result<()> {
    // Create real goals with different types
    let goals = vec![
        Goal::builder(
            "Create a Rust function that calculates fibonacci numbers recursively".to_string(),
            GoalType::CodeGeneration,
        )
        .max_iterations(10)
        .success_criterion("Function compiles without errors".to_string())
        .success_criterion("Function returns correct fibonacci sequence".to_string())
        .success_criterion("Function includes proper documentation".to_string())
        .build()?,
        Goal::builder(
            "Analyze existing codebase for potential security vulnerabilities".to_string(),
            GoalType::Analysis,
        )
        .max_iterations(20)
        .success_criterion("Scan all Rust files for unsafe code blocks".to_string())
        .success_criterion("Check for potential buffer overflow conditions".to_string())
        .build()?,
        Goal::builder(
            "Optimize database query performance in the application".to_string(),
            GoalType::Refactoring,
        )
        .max_iterations(15)
        .success_criterion("Identify slow queries using EXPLAIN ANALYZE".to_string())
        .success_criterion("Implement appropriate database indexes".to_string())
        .success_criterion("Achieve 50% performance improvement".to_string())
        .build()?,
    ];

    for goal in goals {
        println!("✅ Created goal: {}", goal.description);
        println!("   Type: {:?}", goal.goal_type);
        println!("   Max iterations: {:?}", goal.max_iterations);
        println!("   Success criteria: {} items", goal.success_criteria.len());

        // Demonstrate goal complexity calculation
        let complexity = goal.get_complexity();
        println!("   Calculated complexity: {:?}", complexity);
    }

    Ok(())
}

async fn demo_context_system() -> Result<()> {
    // Create a demo goal for the context
    let demo_goal = Goal::builder(
        "Demonstrate context management capabilities".to_string(),
        GoalType::Planning,
    )
    .success_criterion("Successfully set context variables".to_string())
    .success_criterion("Demonstrate metadata management".to_string())
    .build()?;

    // Create real execution context
    let mut context = ExecutionContext::new(demo_goal);

    // Add real context variables using the context's variable system
    let variables = vec![
        ("project_path", "/Users/dev/my_project"),
        ("target_language", "rust"),
        ("compilation_target", "x86_64-unknown-linux-gnu"),
        ("optimization_level", "release"),
    ];

    for (name, value) in variables {
        context.set_variable(name.to_string(), value.to_string());
        println!("✅ Set context variable: {} = {}", name, value);
    }

    // Demonstrate context operations by adding metadata
    context.add_metadata("compilation_status".to_string(), json!("success"));
    context.add_metadata("testing_status".to_string(), json!("passed"));
    context.add_metadata("linting_status".to_string(), json!("clean"));

    println!("✅ Context summary: {}", context.get_summary());
    println!("✅ Context stats: {:?}", context.get_stats());

    Ok(())
}

async fn demo_tool_system() -> Result<()> {
    // Create tool configuration
    let tool_config = fluent_agent::config::ToolConfig {
        file_operations: true,
        shell_commands: true,
        rust_compiler: true,
        git_operations: false,
        allowed_paths: Some(vec![
            "./".to_string(),
            "./examples/".to_string(),
            "./target/".to_string(),
            "./crates/".to_string(),
        ]),
        allowed_commands: Some(vec![
            "cargo".to_string(),
            "rustc".to_string(),
            "ls".to_string(),
            "cat".to_string(),
        ]),
    };

    // Create tool registry with all standard tools
    let tool_registry = ToolRegistry::with_standard_tools(&tool_config);

    println!("✅ Created tool registry with standard tools");

    // List available tools
    let available_tools = tool_registry.get_all_available_tools();
    println!("✅ Tool registry contains {} tools", available_tools.len());

    for tool_info in available_tools {
        println!(
            "   🔧 Available tool: {} ({})",
            tool_info.name, tool_info.description
        );
    }

    Ok(())
}

async fn demo_config_system() -> Result<()> {
    // Load real credentials from environment
    let credentials = credentials::load_from_environment();
    println!(
        "✅ Loaded {} credentials from environment",
        credentials.len()
    );

    // Show available credential keys (without values for security)
    for key in credentials.keys() {
        println!("   🔑 Available credential: {}", key);
    }

    // Create real agent configuration
    let agent_config = AgentEngineConfig {
        reasoning_engine: "openai".to_string(),
        action_engine: "openai".to_string(),
        reflection_engine: "openai".to_string(),
        memory_database: "sqlite://./demo_agent_memory.db".to_string(),
        tools: ToolConfig {
            file_operations: true,
            shell_commands: true,
            rust_compiler: true,
            git_operations: true,
            allowed_paths: Some(vec!["./".to_string(), "./examples/".to_string()]),
            allowed_commands: Some(vec!["cargo".to_string(), "rustc".to_string()]),
        },
        config_path: None,
        max_iterations: Some(50),
        timeout_seconds: Some(300),
    };

    // Validate configuration
    agent_config.validate()?;
    println!("✅ Agent configuration validated successfully");
    println!("   Reasoning engine: {}", agent_config.reasoning_engine);
    println!("   Action engine: {}", agent_config.action_engine);
    println!("   Reflection engine: {}", agent_config.reflection_engine);
    println!("   Memory database: {}", agent_config.memory_database);
    println!("   Max iterations: {:?}", agent_config.max_iterations);
    println!("   Timeout: {:?} seconds", agent_config.timeout_seconds);
    println!(
        "   Tools enabled: file_ops={}, shell={}, rust={}, git={}",
        agent_config.tools.file_operations,
        agent_config.tools.shell_commands,
        agent_config.tools.rust_compiler,
        agent_config.tools.git_operations
    );

    Ok(())
}
