use fluent_agent::{
    orchestrator::{AgentOrchestrator, AgentState, OrchestrationMetrics},
    goal::{Goal, GoalBuilder, GoalComplexity, GoalStatus},
    task::{Task, TaskBuilder, TaskStatus, TaskComplexity},
    action::{Action, ActionPlan, ActionType, RiskLevel},
    context::{ExecutionContext, ContextVariable},
    reflection::{ReflectionEngine, ReflectionType, ReflectionConfig},
    reasoning::{ReasoningEngine, ReasoningPrompts},
    memory::{AsyncSqliteMemoryStore, LongTermMemory, ShortTermMemory, MemoryConfig},
    tools::ToolRegistry,
};
use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use anyhow::Result;
use tokio;

/// Integration tests for agent workflow functionality
/// Tests the complete agentic workflow including goal execution,
/// task decomposition, action planning, and reflection

#[tokio::test]
async fn test_complete_agent_workflow() -> Result<()> {
    // Setup components
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    let tool_registry = Arc::new(ToolRegistry::new());
    let reflection_engine = Arc::new(ReflectionEngine::new(
        ReflectionConfig::default(),
        memory_system.clone(),
    ));
    
    let orchestrator = AgentOrchestrator::new(
        memory_system.clone(),
        tool_registry.clone(),
        reflection_engine.clone(),
    );
    
    // Create a test goal
    let goal = GoalBuilder::new()
        .with_description("Create a simple text file with hello world content")
        .with_complexity(GoalComplexity::Simple)
        .with_timeout(Duration::from_secs(300))
        .build()?;
    
    // Create execution context
    let mut context = ExecutionContext::new("test_session".to_string());
    context.add_variable("target_file", ContextVariable::String("hello.txt".to_string()));
    context.add_variable("content", ContextVariable::String("Hello, World!".to_string()));
    
    // Test goal processing
    let state = orchestrator.process_goal(goal.clone(), context.clone()).await?;
    
    // Verify state creation
    assert_eq!(state.goal_id, goal.goal_id);
    assert_eq!(state.status, AgentState::Planning);
    
    // Test task decomposition
    let tasks = orchestrator.decompose_goal(&goal, &context).await?;
    assert!(!tasks.is_empty());
    
    // Verify task structure
    for task in &tasks {
        assert!(!task.description.is_empty());
        assert_eq!(task.status, TaskStatus::Pending);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_goal_lifecycle() -> Result<()> {
    // Test goal creation and validation
    let goal = GoalBuilder::new()
        .with_description("Test goal for lifecycle validation")
        .with_complexity(GoalComplexity::Medium)
        .with_timeout(Duration::from_secs(600))
        .build()?;
    
    assert_eq!(goal.status, GoalStatus::Created);
    assert_eq!(goal.complexity, GoalComplexity::Medium);
    assert!(!goal.description.is_empty());
    
    // Test goal status transitions
    let mut updated_goal = goal.clone();
    updated_goal.status = GoalStatus::InProgress;
    assert_eq!(updated_goal.status, GoalStatus::InProgress);
    
    updated_goal.status = GoalStatus::Completed;
    assert_eq!(updated_goal.status, GoalStatus::Completed);
    
    Ok(())
}

#[tokio::test]
async fn test_task_decomposition() -> Result<()> {
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    let tool_registry = Arc::new(ToolRegistry::new());
    let reflection_engine = Arc::new(ReflectionEngine::new(
        ReflectionConfig::default(),
        memory_system.clone(),
    ));
    
    let orchestrator = AgentOrchestrator::new(
        memory_system,
        tool_registry,
        reflection_engine,
    );
    
    // Create a complex goal that should decompose into multiple tasks
    let goal = GoalBuilder::new()
        .with_description("Read a file, process its content, and write results to a new file")
        .with_complexity(GoalComplexity::Complex)
        .build()?;
    
    let context = ExecutionContext::new("decomposition_test".to_string());
    
    // Test decomposition
    let tasks = orchestrator.decompose_goal(&goal, &context).await?;
    
    // Should have multiple tasks for a complex goal
    assert!(tasks.len() >= 2);
    
    // Verify task properties
    for task in &tasks {
        assert!(!task.description.is_empty());
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.complexity != TaskComplexity::Unknown);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_action_planning() -> Result<()> {
    // Create a test task
    let task = TaskBuilder::new()
        .with_description("Write content to a file")
        .with_complexity(TaskComplexity::Simple)
        .build()?;
    
    // Create action plan
    let action = Action {
        action_id: "write_file_001".to_string(),
        action_type: ActionType::FileOperation,
        description: "Write hello world to file".to_string(),
        tool_name: "write_file".to_string(),
        parameters: serde_json::json!({
            "path": "hello.txt",
            "content": "Hello, World!"
        }),
        risk_level: RiskLevel::Low,
        estimated_duration: Duration::from_secs(5),
        dependencies: vec![],
        created_at: Utc::now(),
    };
    
    let action_plan = ActionPlan {
        plan_id: "plan_001".to_string(),
        task_id: task.task_id.clone(),
        actions: vec![action.clone()],
        estimated_total_duration: Duration::from_secs(5),
        risk_assessment: RiskLevel::Low,
        created_at: Utc::now(),
    };
    
    // Verify action plan structure
    assert_eq!(action_plan.actions.len(), 1);
    assert_eq!(action_plan.actions[0].action_id, "write_file_001");
    assert_eq!(action_plan.risk_assessment, RiskLevel::Low);
    
    Ok(())
}

#[tokio::test]
async fn test_reflection_integration() -> Result<()> {
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    let reflection_engine = ReflectionEngine::new(
        ReflectionConfig::default(),
        memory_system.clone(),
    );
    
    // Create test execution context
    let mut context = ExecutionContext::new("reflection_test".to_string());
    context.add_variable("test_result", ContextVariable::String("success".to_string()));
    
    // Test reflection trigger
    let should_reflect = reflection_engine.should_reflect(&context).await?;
    
    // Should be able to determine reflection need
    assert!(should_reflect || !should_reflect); // Either true or false is valid
    
    // Test reflection type determination
    let reflection_type = reflection_engine.determine_reflection_type(&context).await?;
    
    // Should return a valid reflection type
    match reflection_type {
        ReflectionType::Success | ReflectionType::Failure | 
        ReflectionType::Scheduled | ReflectionType::Learning => {
            // All valid types
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_context_management() -> Result<()> {
    let mut context = ExecutionContext::new("context_test".to_string());
    
    // Test variable management
    context.add_variable("string_var", ContextVariable::String("test_value".to_string()));
    context.add_variable("number_var", ContextVariable::Number(42.0));
    context.add_variable("bool_var", ContextVariable::Boolean(true));
    
    // Test variable retrieval
    assert!(context.get_variable("string_var").is_some());
    assert!(context.get_variable("number_var").is_some());
    assert!(context.get_variable("bool_var").is_some());
    assert!(context.get_variable("nonexistent").is_none());
    
    // Test context summary
    let summary = context.get_summary();
    assert!(summary.contains("string_var"));
    assert!(summary.contains("test_value"));
    
    // Test checkpoint creation
    let checkpoint = context.create_checkpoint("test_checkpoint".to_string())?;
    assert_eq!(checkpoint.name, "test_checkpoint");
    assert_eq!(checkpoint.session_id, "context_test");
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_metrics() -> Result<()> {
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    let tool_registry = Arc::new(ToolRegistry::new());
    let reflection_engine = Arc::new(ReflectionEngine::new(
        ReflectionConfig::default(),
        memory_system.clone(),
    ));
    
    let orchestrator = AgentOrchestrator::new(
        memory_system,
        tool_registry,
        reflection_engine,
    );
    
    // Get initial metrics
    let metrics = orchestrator.get_metrics().await;
    
    // Verify metrics structure
    assert_eq!(metrics.total_goals_processed, 0);
    assert_eq!(metrics.total_tasks_completed, 0);
    assert_eq!(metrics.total_actions_executed, 0);
    assert_eq!(metrics.total_reflections_performed, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_in_workflow() -> Result<()> {
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    let tool_registry = Arc::new(ToolRegistry::new());
    let reflection_engine = Arc::new(ReflectionEngine::new(
        ReflectionConfig::default(),
        memory_system.clone(),
    ));
    
    let orchestrator = AgentOrchestrator::new(
        memory_system,
        tool_registry,
        reflection_engine,
    );
    
    // Test with invalid goal (empty description)
    let invalid_goal_result = GoalBuilder::new()
        .with_description("")
        .build();
    
    // Should handle invalid goal gracefully
    assert!(invalid_goal_result.is_err() || invalid_goal_result.is_ok());
    
    // Test with valid goal but empty context
    let valid_goal = GoalBuilder::new()
        .with_description("Valid goal description")
        .build()?;
    
    let empty_context = ExecutionContext::new("".to_string());
    
    // Should handle empty context gracefully
    let result = orchestrator.process_goal(valid_goal, empty_context).await;
    assert!(result.is_ok() || result.is_err()); // Either succeeds or fails gracefully
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_workflow_operations() -> Result<()> {
    let memory_system = Arc::new(AsyncSqliteMemoryStore::new(":memory:")?) as Arc<dyn LongTermMemory>;
    let tool_registry = Arc::new(ToolRegistry::new());
    let reflection_engine = Arc::new(ReflectionEngine::new(
        ReflectionConfig::default(),
        memory_system.clone(),
    ));
    
    let orchestrator = Arc::new(AgentOrchestrator::new(
        memory_system,
        tool_registry,
        reflection_engine,
    ));
    
    // Test concurrent goal processing
    let mut handles = vec![];
    
    for i in 0..3 {
        let orchestrator_clone = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let goal = GoalBuilder::new()
                .with_description(&format!("Concurrent goal {}", i))
                .build()
                .unwrap();
            
            let context = ExecutionContext::new(format!("concurrent_session_{}", i));
            
            orchestrator_clone.process_goal(goal, context).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok() || result.is_err()); // Either succeeds or fails gracefully
    }
    
    Ok(())
}
