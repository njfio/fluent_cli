use anyhow::Result;
use fluent_agent::{
    ExecutionContext, StateManager, StateManagerConfig,
    Goal, GoalType, GoalPriority,
};
use fluent_agent::context::CheckpointType;
use std::collections::HashMap;
use tempfile::tempdir;
use tokio;

/// Demonstrates the enhanced execution context and state management capabilities
#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Fluent Agent State Management Demo");
    println!("=====================================\n");

    // Create a temporary directory for state storage
    let temp_dir = tempdir()?;
    println!("📁 State directory: {:?}\n", temp_dir.path());

    // Configure state manager (disable auto-save for demo)
    let config = StateManagerConfig {
        state_directory: temp_dir.path().to_path_buf(),
        auto_save_enabled: false, // Disable auto-save to avoid hanging
        auto_save_interval_seconds: 5,
        max_checkpoints: 20,
        compression_enabled: false,
        backup_retention_days: 7,
    };

    // Create state manager
    let state_manager = StateManager::new(config).await?;
    println!("✅ State manager initialized");

    // Create a sample goal
    let goal = Goal {
        goal_id: "demo-goal-001".to_string(),
        description: "Demonstrate state management capabilities".to_string(),
        goal_type: GoalType::Analysis,
        priority: GoalPriority::High,
        success_criteria: vec![
            "Create execution context".to_string(),
            "Demonstrate checkpoints".to_string(),
            "Show state persistence".to_string(),
        ],
        max_iterations: Some(10),
        timeout: None,
        metadata: HashMap::new(),
    };

    // Create execution context
    let mut context = ExecutionContext::new(goal);
    println!("✅ Execution context created: {}", context.context_id);

    // Demonstrate variable management
    println!("\n📊 Variable Management:");
    context.set_variable("demo_var".to_string(), "initial_value".to_string());
    context.set_variable("iteration_count".to_string(), "0".to_string());
    println!("   Set variables: demo_var=initial_value, iteration_count=0");

    // Set context in state manager
    state_manager.set_context(context.clone()).await?;
    println!("✅ Context set in state manager");

    // Demonstrate checkpoint creation
    println!("\n🔄 Checkpoint Management:");
    
    // Create manual checkpoint
    let checkpoint_id = state_manager.create_checkpoint(
        CheckpointType::Manual,
        "Initial state checkpoint".to_string()
    ).await?;
    println!("   Created manual checkpoint: {}", checkpoint_id);

    // Simulate some iterations with automatic checkpoints
    for i in 1..=6 {
        // Update context
        let mut current_context = state_manager.get_context().await.unwrap();
        current_context.set_variable("iteration_count".to_string(), i.to_string());
        current_context.increment_iteration();
        
        // Create checkpoint before action
        let before_checkpoint = state_manager.create_checkpoint(
            CheckpointType::BeforeAction,
            format!("Before action at iteration {}", i)
        ).await?;
        
        // Simulate some work
        current_context.set_variable(
            "last_action".to_string(), 
            format!("action_{}", i)
        );
        
        // Create checkpoint after action
        let after_checkpoint = state_manager.create_checkpoint(
            CheckpointType::AfterAction,
            format!("After action at iteration {}", i)
        ).await?;
        
        // Update state manager
        state_manager.set_context(current_context.clone()).await?;
        
        println!("   Iteration {}: Before={}, After={}", 
                 i, &before_checkpoint[..8], &after_checkpoint[..8]);
    }

    // Demonstrate state persistence
    println!("\n💾 State Persistence:");
    
    // Save current state
    state_manager.save_context().await?;
    println!("   Saved current context to disk");

    // Get current context for comparison
    let original_context = state_manager.get_context().await.unwrap();
    let original_id = original_context.context_id.clone();
    
    // Load context from disk
    let loaded_context = state_manager.load_context(&original_id).await?;
    println!("   Loaded context from disk: {}", loaded_context.context_id);
    
    // Verify data integrity
    assert_eq!(original_context.context_id, loaded_context.context_id);
    assert_eq!(original_context.iteration_count, loaded_context.iteration_count);
    assert_eq!(original_context.variables, loaded_context.variables);
    println!("   ✅ Data integrity verified");

    // Demonstrate state validation
    println!("\n🔍 State Validation:");
    let validation_result = loaded_context.validate_state();
    match validation_result {
        Ok(_) => println!("   ✅ State validation passed"),
        Err(e) => println!("   ❌ State validation failed: {}", e),
    }

    // Demonstrate recovery information
    println!("\n🔧 Recovery Information:");
    let recovery_info = state_manager.get_recovery_info(&original_id).await?;
    println!("   Context ID: {}", recovery_info.context_id);
    println!("   Iteration Count: {}", recovery_info.iteration_count);
    println!("   Checkpoint Count: {}", recovery_info.checkpoint_count);
    println!("   State Version: {}", recovery_info.state_version);
    println!("   Recovery Possible: {}", recovery_info.recovery_possible);
    println!("   Corruption Detected: {}", recovery_info.corruption_detected);

    // Demonstrate checkpoint restoration
    println!("\n🔄 Checkpoint Restoration:");
    let checkpoints = loaded_context.get_checkpoints_by_type(&CheckpointType::Manual);
    if let Some(checkpoint) = checkpoints.first() {
        let mut test_context = loaded_context.clone();
        test_context.restore_from_checkpoint(checkpoint);
        println!("   Restored context from checkpoint: {}", checkpoint.checkpoint_id);
        println!("   State version after restoration: {}", test_context.get_state_version());
    }

    // Show statistics
    println!("\n📈 State Manager Statistics:");
    let stats = state_manager.get_statistics().await?;
    println!("   Total Contexts: {}", stats.total_contexts);
    println!("   Total Checkpoints: {}", stats.total_checkpoints);
    println!("   Total Size: {} bytes", stats.total_size_bytes);
    println!("   Auto-save Enabled: {}", stats.auto_save_enabled);

    // Show context statistics
    println!("\n📊 Context Statistics:");
    let context_stats = loaded_context.get_stats();
    println!("   Total Observations: {}", context_stats.total_observations);
    println!("   Active Tasks: {}", context_stats.active_tasks);
    println!("   Completed Tasks: {}", context_stats.completed_tasks);
    println!("   Variables Count: {}", context_stats.variables_count);
    println!("   Execution Events: {}", context_stats.execution_events);
    println!("   Strategy Adjustments: {}", context_stats.strategy_adjustments);
    println!("   Execution Duration: {:?}", context_stats.execution_duration);
    println!("   Iteration Count: {}", context_stats.iteration_count);

    // Demonstrate context summary
    println!("\n📋 Context Summary:");
    println!("   {}", loaded_context.get_summary());
    println!("   {}", loaded_context.get_progress_summary());

    // List all available contexts
    println!("\n📂 Available Contexts:");
    let contexts = state_manager.list_contexts().await?;
    for context_id in contexts {
        println!("   - {}", context_id);
    }

    println!("\n🎉 State Management Demo Complete!");
    println!("   All features demonstrated successfully");

    Ok(())
}
