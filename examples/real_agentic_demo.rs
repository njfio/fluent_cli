// Real Agentic System Demo - No Mocks, Real Implementation
use anyhow::Result;
use fluent_agent::{
    config::{AgentEngineConfig, credentials},
    goal::{Goal, GoalType},
    memory::{MemorySystem, SqliteMemoryStore, MemoryItem, MemoryType, MemoryQuery},
    context::{ExecutionContext, ContextVariable},
    tools::{ToolRegistry, FileSystemExecutor, ToolExecutionConfig},
};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;

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
                map.insert("project_type".to_string(), "rust".to_string());
                map.insert("outcome".to_string(), "success".to_string());
                map
            },
            importance: 0.8,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            tags: vec!["rust".to_string(), "compilation".to_string(), "success".to_string()],
            embedding: None,
        },
        MemoryItem {
            memory_id: "learn_001".to_string(),
            memory_type: MemoryType::Learning,
            content: "When using async/await in Rust, always handle Result types properly".to_string(),
            metadata: {
                let mut map = HashMap::new();
                map.insert("language".to_string(), "rust".to_string());
                map.insert("concept".to_string(), "async_programming".to_string());
                map
            },
            importance: 0.9,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 3,
            tags: vec!["rust".to_string(), "async".to_string(), "best_practice".to_string()],
            embedding: None,
        },
    ];
    
    // Store memories in real database
    for memory in &experiences {
        let stored_id = memory_store.store(memory.clone()).await?;
        println!("✅ Stored memory: {} -> {}", memory.memory_type, stored_id);
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
        println!("   📝 {}: {}", memory.memory_type, memory.content);
        println!("      Importance: {}, Access count: {}", memory.importance, memory.access_count);
    }
    
    Ok(())
}

async fn demo_goal_system() -> Result<()> {
    // Create real goals with different types
    let goals = vec![
        Goal::builder(
            "Create a Rust function that calculates fibonacci numbers recursively".to_string(),
            GoalType::CodeGeneration
        )
        .max_iterations(10)
        .add_success_criterion("Function compiles without errors".to_string())
        .add_success_criterion("Function returns correct fibonacci sequence".to_string())
        .add_success_criterion("Function includes proper documentation".to_string())
        .build()?,
        
        Goal::builder(
            "Analyze existing codebase for potential security vulnerabilities".to_string(),
            GoalType::Analysis
        )
        .max_iterations(20)
        .add_success_criterion("Scan all Rust files for unsafe code blocks".to_string())
        .add_success_criterion("Check for potential buffer overflow conditions".to_string())
        .build()?,
        
        Goal::builder(
            "Optimize database query performance in the application".to_string(),
            GoalType::Optimization
        )
        .max_iterations(15)
        .add_success_criterion("Identify slow queries using EXPLAIN ANALYZE".to_string())
        .add_success_criterion("Implement appropriate database indexes".to_string())
        .add_success_criterion("Achieve 50% performance improvement".to_string())
        .build()?,
    ];
    
    for goal in goals {
        println!("✅ Created goal: {}", goal.description);
        println!("   Type: {:?}", goal.goal_type);
        println!("   Max iterations: {:?}", goal.max_iterations);
        println!("   Success criteria: {} items", goal.success_criteria.len());
        
        // Demonstrate goal complexity calculation
        let complexity = goal.calculate_complexity();
        println!("   Calculated complexity: {:?}", complexity);
    }
    
    Ok(())
}

async fn demo_context_system() -> Result<()> {
    // Create real execution context
    let mut context = ExecutionContext::new("demo_session".to_string());
    
    // Add real context variables
    let variables = vec![
        ContextVariable::new("project_path".to_string(), "/Users/dev/my_project".to_string()),
        ContextVariable::new("target_language".to_string(), "rust".to_string()),
        ContextVariable::new("compilation_target".to_string(), "x86_64-unknown-linux-gnu".to_string()),
        ContextVariable::new("optimization_level".to_string(), "release".to_string()),
    ];
    
    for var in variables {
        context.set_variable(var.name.clone(), var.value.clone());
        println!("✅ Set context variable: {} = {}", var.name, var.value);
    }
    
    // Demonstrate context operations
    context.add_step_result("compilation".to_string(), "success".to_string());
    context.add_step_result("testing".to_string(), "passed".to_string());
    context.add_step_result("linting".to_string(), "clean".to_string());
    
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
    println!("✅ Tool registry contains {} tools", tool_registry.list_tools().len());
    
    // List available tools
    for tool_name in tool_registry.list_tools() {
        println!("   🔧 Available tool: {}", tool_name);
    }
    
    Ok(())
}

async fn demo_config_system() -> Result<()> {
    // Load real credentials from environment
    let credentials = credentials::load_from_environment();
    println!("✅ Loaded {} credentials from environment", credentials.len());
    
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
        tools: fluent_agent::config::ToolsConfig {
            file_operations: true,
            shell_commands: true,
            rust_compiler: true,
            allowed_paths: Some(vec!["./".to_string(), "./examples/".to_string()]),
            allowed_commands: Some(vec!["cargo".to_string(), "rustc".to_string()]),
        },
        reasoning: fluent_agent::config::ReasoningConfig {
            max_context_length: 8000,
            temperature: 0.1,
            enable_chain_of_thought: true,
        },
        action: fluent_agent::config::ActionConfig {
            max_retries: 3,
            timeout_seconds: 60,
            enable_parallel_execution: false,
        },
        reflection: fluent_agent::config::ReflectionConfig {
            enable_self_correction: true,
            reflection_frequency: 5,
            learning_rate: 0.1,
        },
    };
    
    // Validate configuration
    agent_config.validate()?;
    println!("✅ Agent configuration validated successfully");
    println!("   Reasoning engine: {}", agent_config.reasoning_engine);
    println!("   Action engine: {}", agent_config.action_engine);
    println!("   Reflection engine: {}", agent_config.reflection_engine);
    println!("   Tools enabled: file_ops={}, shell={}, rust={}", 
        agent_config.tools.file_operations,
        agent_config.tools.shell_commands,
        agent_config.tools.rust_compiler
    );
    
    Ok(())
}
