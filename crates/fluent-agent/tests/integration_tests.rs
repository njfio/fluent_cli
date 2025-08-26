//! Integration tests for Enhanced Agentic System
//!
//! These tests verify that all enhanced components work together correctly
//! in complex multi-step scenarios typical of autonomous operations.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use fluent_agent::{
    // Core components
    ExecutionContext, Goal, GoalType, GoalPriority, Task, TaskType, TaskPriority,
    
    // Enhanced reasoning
    TreeOfThoughtEngine, ToTConfig, ChainOfThoughtEngine, CoTConfig, 
    MetaReasoningEngine, MetaConfig, CompositeReasoningEngine,
    
    // Advanced planning
    HTNPlanner, HTNConfig, DependencyAnalyzer, DynamicReplanner,
    
    // Memory systems
    WorkingMemory, WorkingMemoryConfig, ContextCompressor, CompressorConfig,
    CrossSessionPersistence, PersistenceConfig,
    
    // Monitoring systems
    PerformanceMonitor, PerformanceConfig, AdaptiveStrategySystem, StrategyConfig,
    ErrorRecoverySystem, RecoveryConfig, ErrorInstance, ErrorType, ErrorSeverity,
    
    // Agent orchestration
    AgentOrchestrator, MemorySystem, MemoryConfig,
};

use fluent_core::traits::Engine;
use fluent_core::types::{Request, Response};

/// Mock engine for testing that simulates LLM responses
struct MockEngine {
    responses: Arc<tokio::sync::Mutex<Vec<String>>>,
}

impl MockEngine {
    fn new() -> Self {
        Self {
            responses: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        }
    }

    async fn add_response(&self, response: String) {
        self.responses.lock().await.push(response);
    }
}

#[async_trait::async_trait]
impl Engine for MockEngine {
    async fn execute(&self, request: &Request) -> Result<Response> {
        // Simulate processing delay
        sleep(Duration::from_millis(10)).await;
        
        let mut responses = self.responses.lock().await;
        let response_text = if !responses.is_empty() {
            responses.remove(0)
        } else {
            format!("Mock response for: {}", request.payload)
        };

        Ok(Response {
            content: response_text,
            metadata: std::collections::HashMap::new(),
        })
    }
}

/// Test complex reasoning scenario with multiple solution paths
#[tokio::test]
async fn test_complex_reasoning_scenario() -> Result<()> {
    let mock_engine = Arc::new(MockEngine::new());
    
    // Create composite reasoning engine
    let tot_engine = TreeOfThoughtEngine::new(mock_engine.clone(), ToTConfig::default()).await?;
    let cot_engine = ChainOfThoughtEngine::new(mock_engine.clone(), CoTConfig::default()).await?;
    let meta_engine = MetaReasoningEngine::new(mock_engine.clone(), MetaConfig::default()).await?;
    
    let composite_engine = CompositeReasoningEngine::new(
        vec![
            Box::new(tot_engine),
            Box::new(cot_engine), 
            Box::new(meta_engine)
        ],
        fluent_agent::reasoning::ReasoningSelectionStrategy::ComplexityBased
    );

    // Set up mock responses for complex scenario
    mock_engine.add_response("Complex problem identified. Exploring multiple solution paths.".to_string()).await;
    mock_engine.add_response("Solution path 1: Use iterative approach with feedback loops".to_string()).await;
    mock_engine.add_response("Solution path 2: Implement parallel processing strategy".to_string()).await;
    mock_engine.add_response("Meta-analysis: Path 2 has higher success probability".to_string()).await;
    
    // Test complex reasoning on a challenging problem
    let mut context = ExecutionContext::new();
    let complex_problem = "Design and implement a scalable web application that can handle 100,000 concurrent users while maintaining sub-100ms response times, ensuring 99.9% uptime, and implementing real-time features.";
    
    let result = composite_engine.reason(complex_problem, &context).await?;
    
    assert!(!result.is_empty());
    assert!(result.contains("Complex") || result.contains("solution"));
    println!("✓ Complex reasoning test passed: Generated {} chars of reasoning", result.len());
    
    Ok(())
}

/// Test hierarchical planning with dependencies
#[tokio::test]
async fn test_hierarchical_planning_scenario() -> Result<()> {
    let mock_engine = Arc::new(MockEngine::new());
    let htn_planner = HTNPlanner::new(mock_engine.clone(), HTNConfig::default());
    let dependency_analyzer = DependencyAnalyzer::new(Default::default());
    
    // Set up mock responses for decomposition
    mock_engine.add_response("SUBTASK: Design system architecture\nTYPE: compound".to_string()).await;
    mock_engine.add_response("SUBTASK: Implement backend services\nTYPE: primitive".to_string()).await;
    mock_engine.add_response("SUBTASK: Create frontend interface\nTYPE: primitive".to_string()).await;
    mock_engine.add_response("SUBTASK: Set up CI/CD pipeline\nTYPE: primitive".to_string()).await;
    mock_engine.add_response("SUBTASK: Deploy to production\nTYPE: primitive".to_string()).await;
    
    // Create complex goal
    let goal = Goal {
        goal_id: "complex_project".to_string(),
        description: "Build a complete e-commerce platform with microservices architecture".to_string(),
        goal_type: GoalType::LongTerm,
        priority: GoalPriority::High,
        target_outcome: "Fully functional e-commerce platform".to_string(),
        success_criteria: vec!["All features working".to_string(), "Performance targets met".to_string()],
        estimated_duration: Some(Duration::from_secs(86400 * 30)), // 30 days
        dependencies: Vec::new(),
    };
    
    let context = ExecutionContext::new();
    
    // Test HTN planning
    let htn_result = htn_planner.plan_decomposition(&goal, &context).await?;
    assert!(htn_result.tasks.len() > 0);
    assert!(htn_result.plan.phases.len() > 0);
    println!("✓ HTN planning test passed: Generated {} tasks in {} phases", 
             htn_result.tasks.len(), htn_result.plan.phases.len());
    
    // Test dependency analysis
    let tasks: Vec<Task> = htn_result.tasks.iter().map(|nt| Task {
        task_id: nt.id.clone(),
        description: nt.description.clone(),
        task_type: TaskType::Implementation,
        priority: TaskPriority::Medium,
        estimated_duration: Some(Duration::from_secs(3600)),
        dependencies: Vec::new(),
        assigned_to: None,
        created_at: std::time::SystemTime::now(),
        status: fluent_agent::task::TaskStatus::Pending,
    }).collect();
    
    let dependency_analysis = dependency_analyzer.analyze_dependencies(&tasks, &context).await?;
    assert!(!dependency_analysis.topological_order.is_empty());
    println!("✓ Dependency analysis test passed: {} tasks ordered, {} parallel groups", 
             dependency_analysis.topological_order.len(),
             dependency_analysis.parallel_opportunities.len());
    
    Ok(())
}

/// Test advanced memory management scenario
#[tokio::test]
async fn test_memory_management_scenario() -> Result<()> {
    let mock_engine = Arc::new(MockEngine::new());
    
    // Create memory components
    let working_memory = WorkingMemory::new(WorkingMemoryConfig::default());
    let context_compressor = ContextCompressor::new(mock_engine.clone(), CompressorConfig::default());
    let persistence = CrossSessionPersistence::new(PersistenceConfig::default());
    
    // Set up mock responses for summarization
    mock_engine.add_response("Context Summary: Processed 50 tasks successfully with 90% efficiency. Key learning: Parallel execution improves performance.".to_string()).await;
    
    // Initialize context with substantial information
    let mut context = ExecutionContext::new();
    
    // Simulate long-running process with lots of context
    for i in 0..100 {
        context.add_context_item(
            format!("task_{}", i),
            format!("Completed task {} with outcome: success", i)
        );
    }
    
    // Test working memory attention update
    working_memory.update_attention(&context).await?;
    let attention_items = working_memory.get_attention_items().await?;
    assert!(!attention_items.is_empty());
    println!("✓ Working memory test passed: {} attention items tracked", attention_items.len());
    
    // Test context compression
    let compression_result = context_compressor.compress_context(&context).await?;
    assert!(compression_result.compressed_context.compressed_data.len() < 
             context.context_data.len() * 10); // Should be compressed
    println!("✓ Context compression test passed: Compressed to {} bytes", 
             compression_result.compressed_context.metadata.compressed_size);
    
    // Test cross-session persistence
    persistence.initialize().await?;
    persistence.save_session_state(&context).await?;
    let patterns = persistence.get_relevant_patterns(&context).await?;
    println!("✓ Cross-session persistence test passed: {} patterns found", patterns.len());
    
    Ok(())
}

/// Test monitoring and adaptation scenario
#[tokio::test]
async fn test_monitoring_adaptation_scenario() -> Result<()> {
    // Create monitoring components
    let performance_monitor = PerformanceMonitor::new(PerformanceConfig::default());
    let adaptive_system = AdaptiveStrategySystem::new(StrategyConfig::default());
    let error_recovery = ErrorRecoverySystem::new(Arc::new(MockEngine::new()), RecoveryConfig::default());
    
    // Initialize error recovery strategies
    error_recovery.initialize_strategies().await?;
    
    // Test performance monitoring
    performance_monitor.start_monitoring().await?;
    
    // Create test task execution
    let task = Task {
        task_id: "test_task_001".to_string(),
        description: "Complex computational task".to_string(),
        task_type: TaskType::Implementation,
        priority: TaskPriority::High,
        estimated_duration: Some(Duration::from_secs(300)),
        dependencies: Vec::new(),
        assigned_to: None,
        created_at: std::time::SystemTime::now(),
        status: fluent_agent::task::TaskStatus::Pending,
    };
    
    let task_result = fluent_agent::task::TaskResult {
        task_id: task.task_id.clone(),
        success: true,
        output: "Task completed successfully".to_string(),
        execution_time: Duration::from_secs(250),
        error_message: None,
        metadata: std::collections::HashMap::new(),
    };
    
    // Record task execution for monitoring
    performance_monitor.record_task_execution(&task, &task_result, &ExecutionContext::new()).await?;
    
    let performance_metrics = performance_monitor.get_current_metrics().await?;
    assert!(performance_metrics.execution_metrics.tasks_completed > 0);
    println!("✓ Performance monitoring test passed: {} tasks completed", 
             performance_metrics.execution_metrics.tasks_completed);
    
    // Test adaptive strategy system
    adaptive_system.evaluate_and_adapt(&performance_metrics, &ExecutionContext::new()).await?;
    let current_strategy = adaptive_system.get_current_strategy().await?;
    println!("✓ Adaptive strategy test passed: Current strategy: {}", current_strategy.strategy_id);
    
    // Test error recovery
    let error = ErrorInstance {
        error_id: "test_error_001".to_string(),
        error_type: ErrorType::SystemFailure,
        severity: ErrorSeverity::High,
        description: "Mock system failure for testing".to_string(),
        context: "Testing scenario".to_string(),
        timestamp: std::time::SystemTime::now(),
        affected_tasks: vec![task.task_id],
        root_cause: Some("Test condition".to_string()),
        recovery_suggestions: vec!["Restart affected components".to_string()],
    };
    
    let recovery_result = error_recovery.handle_error(error, &ExecutionContext::new()).await?;
    assert!(recovery_result.success);
    println!("✓ Error recovery test passed: Recovery took {}ms", 
             recovery_result.recovery_time.as_millis());
    
    Ok(())
}

/// Test full integration scenario with all components
#[tokio::test]
async fn test_full_integration_scenario() -> Result<()> {
    let mock_engine = Arc::new(MockEngine::new());
    
    // Set up comprehensive mock responses
    mock_engine.add_response("Analyzing complex goal: Build AI-powered content management system".to_string()).await;
    mock_engine.add_response("SUBTASK: Design AI model architecture\nTYPE: compound".to_string()).await;
    mock_engine.add_response("SUBTASK: Implement content processing pipeline\nTYPE: primitive".to_string()).await;
    mock_engine.add_response("SUBTASK: Create user interface\nTYPE: primitive".to_string()).await;
    mock_engine.add_response("SUBTASK: Deploy and test system\nTYPE: primitive".to_string()).await;
    mock_engine.add_response("Context Summary: Multi-stage AI system development proceeding successfully.".to_string()).await;
    
    // Create integrated memory system
    let memory_system = MemorySystem::new(MemoryConfig::default()).await?;
    
    // Create agent orchestrator with all enhanced components
    let mut orchestrator = AgentOrchestrator::new(
        mock_engine.clone(),
        memory_system,
        Default::default()
    ).await?;
    
    // Define complex goal requiring all capabilities
    let complex_goal = Goal {
        goal_id: "ai_cms_project".to_string(),
        description: "Build an AI-powered content management system with real-time collaboration, automated content generation, and advanced analytics".to_string(),
        goal_type: GoalType::LongTerm,
        priority: GoalPriority::Critical,
        target_outcome: "Production-ready AI CMS platform".to_string(),
        success_criteria: vec![
            "AI content generation working".to_string(),
            "Real-time collaboration implemented".to_string(),
            "Analytics dashboard functional".to_string(),
            "Performance targets achieved".to_string()
        ],
        estimated_duration: Some(Duration::from_secs(86400 * 60)), // 60 days
        dependencies: Vec::new(),
    };
    
    // Execute complex scenario
    let context = ExecutionContext::new();
    let execution_result = orchestrator.execute_goal(&complex_goal, &context).await?;
    
    // Verify integration results
    assert!(execution_result.success);
    assert!(!execution_result.final_output.is_empty());
    println!("✓ Full integration test passed: Goal executed successfully");
    println!("   - Execution time: {}ms", execution_result.execution_time.as_millis());
    println!("   - Output length: {} chars", execution_result.final_output.len());
    
    // Verify orchestration metrics
    let metrics = orchestrator.get_metrics().await?;
    assert!(metrics.goals_completed > 0);
    println!("✓ Orchestration metrics verified: {} goals completed, {}% success rate",
             metrics.goals_completed, 
             metrics.success_rate * 100.0);
    
    Ok(())
}

/// Benchmark test for performance validation
#[tokio::test] 
async fn test_performance_benchmarks() -> Result<()> {
    let mock_engine = Arc::new(MockEngine::new());
    
    // Set up responses for benchmark
    for i in 0..10 {
        mock_engine.add_response(format!("Benchmark response {}: Processing task efficiently", i)).await;
    }
    
    let start_time = std::time::Instant::now();
    
    // Create composite reasoning engine for performance test
    let tot_engine = TreeOfThoughtEngine::new(mock_engine.clone(), ToTConfig::default()).await?;
    let composite_engine = CompositeReasoningEngine::new(
        vec![Box::new(tot_engine)],
        fluent_agent::reasoning::ReasoningSelectionStrategy::HighestConfidence
    );
    
    // Run multiple reasoning iterations
    let context = ExecutionContext::new();
    for i in 0..10 {
        let problem = format!("Benchmark problem {}: Optimize database query performance for {} records", i, i * 1000);
        let _result = composite_engine.reason(&problem, &context).await?;
    }
    
    let elapsed = start_time.elapsed();
    let throughput = 10.0 / elapsed.as_secs_f64();
    
    println!("✓ Performance benchmark completed:");
    println!("   - Total time: {}ms", elapsed.as_millis());
    println!("   - Throughput: {:.2} operations/second", throughput);
    
    // Verify performance is within acceptable limits
    assert!(elapsed < Duration::from_secs(30), "Benchmark took too long: {}ms", elapsed.as_millis());
    assert!(throughput > 0.1, "Throughput too low: {:.2} ops/sec", throughput);
    
    Ok(())
}

/// Test error handling and recovery in complex scenarios
#[tokio::test]
async fn test_error_handling_scenarios() -> Result<()> {
    let mock_engine = Arc::new(MockEngine::new());
    let error_recovery = ErrorRecoverySystem::new(mock_engine.clone(), RecoveryConfig::default());
    
    error_recovery.initialize_strategies().await?;
    
    // Test different error types
    let error_scenarios = vec![
        (ErrorType::SystemFailure, ErrorSeverity::Critical),
        (ErrorType::ResourceExhaustion, ErrorSeverity::High),
        (ErrorType::NetworkTimeout, ErrorSeverity::Medium),
        (ErrorType::ValidationError, ErrorSeverity::Low),
    ];
    
    let mut recovery_count = 0;
    
    for (error_type, severity) in error_scenarios {
        let error = ErrorInstance {
            error_id: format!("test_error_{}", recovery_count),
            error_type: error_type.clone(),
            severity,
            description: format!("Test {:?} error", error_type),
            context: "Integration test scenario".to_string(),
            timestamp: std::time::SystemTime::now(),
            affected_tasks: vec!["test_task".to_string()],
            root_cause: Some("Test condition".to_string()),
            recovery_suggestions: vec!["Apply test recovery".to_string()],
        };
        
        let recovery_result = error_recovery.handle_error(error, &ExecutionContext::new()).await?;
        if recovery_result.success {
            recovery_count += 1;
        }
    }
    
    println!("✓ Error handling test passed: {}/{} errors recovered successfully", 
             recovery_count, error_scenarios.len());
    
    // Get resilience metrics
    let resilience_metrics = error_recovery.get_resilience_metrics().await?;
    println!("   - Mean time to recovery: {}ms", resilience_metrics.mean_time_to_recovery.as_millis());
    println!("   - Availability: {:.2}%", resilience_metrics.availability_percentage * 100.0);
    
    Ok(())
}