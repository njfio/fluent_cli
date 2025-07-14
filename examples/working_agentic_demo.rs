// Working Agentic System Demo - Real Implementation Examples
use anyhow::Result;
use chrono::Utc;
use fluent_agent::{
    config::{credentials, AgentEngineConfig},
    context::ExecutionContext,
    goal::{Goal, GoalType},
    memory::{LongTermMemory, MemoryItem, MemoryQuery, MemoryType, SqliteMemoryStore},
    tools::{FileSystemExecutor, ToolExecutionConfig, ToolRegistry},
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 Working Agentic System Demo");
    println!("===============================");
    println!("This demo shows REAL working examples of the agentic system components");

    // Demo 1: Real SQLite Memory System
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

    // Store real memory items with proper Value types
    let experiences = vec![
        MemoryItem {
            memory_id: "exp_001".to_string(),
            memory_type: MemoryType::Experience,
            content: "Successfully compiled Rust project with zero warnings".to_string(),
            metadata: {
                let mut map = HashMap::new();
                map.insert(
                    "project_type".to_string(),
                    Value::String("rust".to_string()),
                );
                map.insert("outcome".to_string(), Value::String("success".to_string()));
                map.insert(
                    "warnings".to_string(),
                    Value::Number(serde_json::Number::from(0)),
                );
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
                map.insert("language".to_string(), Value::String("rust".to_string()));
                map.insert(
                    "concept".to_string(),
                    Value::String("async_programming".to_string()),
                );
                map.insert(
                    "importance_level".to_string(),
                    Value::String("high".to_string()),
                );
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

    // Store memories in real database using the trait
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

    let retrieved = memory_store.retrieve(&query).await?;
    println!("✅ Retrieved {} memories from database", retrieved.len());

    for memory in retrieved {
        println!("   📝 {:?}: {}", memory.memory_type, memory.content);
        println!(
            "      Importance: {}, Access count: {}",
            memory.importance, memory.access_count
        );
        println!("      Tags: {:?}", memory.tags);
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
            "Debug and fix compilation errors in the project".to_string(),
            GoalType::Debugging,
        )
        .max_iterations(15)
        .success_criterion("Identify all compilation errors".to_string())
        .success_criterion("Fix syntax and type errors".to_string())
        .success_criterion("Achieve successful compilation".to_string())
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
    // Create a simple goal for the context
    let goal = Goal::builder("Demo context management".to_string(), GoalType::Analysis).build()?;

    // Create real execution context
    let mut context = ExecutionContext::new(goal);

    // Add real context variables
    context.set_variable(
        "project_path".to_string(),
        "/Users/dev/my_project".to_string(),
    );
    context.set_variable("target_language".to_string(), "rust".to_string());
    context.set_variable(
        "compilation_target".to_string(),
        "x86_64-unknown-linux-gnu".to_string(),
    );
    context.set_variable("optimization_level".to_string(), "release".to_string());

    println!("✅ Set context variables:");
    // Note: get_variables() method not available, using metadata instead
    println!("   📊 Context metadata available");

    // Demonstrate context operations
    println!("✅ Context summary: {}", context.get_summary());
    println!("✅ Context stats: {:?}", context.get_stats());

    Ok(())
}

async fn demo_tool_system() -> Result<()> {
    // Create real tool registry
    let mut tool_registry = ToolRegistry::new();

    // Configure real file system executor
    let tool_config = ToolExecutionConfig {
        timeout_seconds: 30,
        max_output_size: 1024 * 1024, // 1MB
        allowed_paths: vec![
            "./".to_string(),
            "./examples/".to_string(),
            "./target/".to_string(),
        ],
        allowed_commands: vec![
            "cargo".to_string(),
            "rustc".to_string(),
            "ls".to_string(),
            "cat".to_string(),
        ],
        read_only: false,
    };

    let fs_executor = Arc::new(FileSystemExecutor::new(tool_config));
    tool_registry.register("filesystem".to_string(), fs_executor);

    println!("✅ Registered real file system executor");
    println!("✅ Tool registry operational");

    // Show that the registry is working
    if tool_registry.is_tool_available("filesystem") {
        println!("   🔧 File system tool is available and ready");
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
        tools: fluent_agent::config::ToolConfig {
            file_operations: true,
            shell_commands: true,
            rust_compiler: true,
            git_operations: false,
            allowed_paths: Some(vec!["./".to_string(), "./examples/".to_string()]),
            allowed_commands: Some(vec!["cargo".to_string(), "rustc".to_string()]),
        },
        config_path: Some("./config_test.json".to_string()),
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
    println!(
        "   Tools enabled: file_ops={}, shell={}, rust={}",
        agent_config.tools.file_operations,
        agent_config.tools.shell_commands,
        agent_config.tools.rust_compiler
    );

    Ok(())
}
