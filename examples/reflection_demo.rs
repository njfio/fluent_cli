use anyhow::Result;
use async_trait::async_trait;
use fluent_agent::{
    ExecutionContext, ReflectionEngine, ReflectionConfig,
    Goal, GoalType, GoalPriority, Task, TaskType, TaskPriority,
    ReasoningEngine,
};
use fluent_agent::reflection::ReflectionTrigger;
use std::collections::HashMap;
use std::time::SystemTime;
use tokio;

/// Mock reasoning engine for demonstration
struct MockReasoningEngine;

#[async_trait]
impl ReasoningEngine for MockReasoningEngine {
    async fn reason(&self, prompt: &str, context: &ExecutionContext) -> Result<String> {
        Ok(format!("Mock reasoning analysis of current situation for prompt: {}", prompt))
    }

    async fn get_capabilities(&self) -> Vec<fluent_agent::reasoning::ReasoningCapability> {
        vec![
            fluent_agent::reasoning::ReasoningCapability::SelfReflection,
            fluent_agent::reasoning::ReasoningCapability::StrategyFormulation,
            fluent_agent::reasoning::ReasoningCapability::ProgressEvaluation,
        ]
    }

    async fn get_confidence(&self) -> f64 {
        0.75
    }
}

/// Demonstrates the advanced self-reflection and strategy adjustment capabilities
#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 Fluent Agent Self-Reflection & Strategy Adjustment Demo");
    println!("=========================================================\n");

    // Configure reflection engine
    let reflection_config = ReflectionConfig {
        reflection_frequency: 3, // Reflect every 3 iterations for demo
        deep_reflection_frequency: 6,
        learning_retention_days: 30,
        confidence_threshold: 0.6,
        performance_threshold: 0.7,
        enable_meta_reflection: true,
        strategy_adjustment_sensitivity: 0.8,
    };

    // Create reflection engine
    let mut reflection_engine = ReflectionEngine::with_config(reflection_config);
    println!("✅ Reflection engine initialized with custom configuration");

    // Create a sample goal
    let goal = Goal {
        goal_id: "demo-goal-reflection".to_string(),
        description: "Demonstrate self-reflection and strategy adjustment".to_string(),
        goal_type: GoalType::Analysis,
        priority: GoalPriority::High,
        success_criteria: vec![
            "Complete reflection analysis".to_string(),
            "Generate strategy adjustments".to_string(),
            "Learn from experience".to_string(),
        ],
        max_iterations: Some(15),
        timeout: None,
        metadata: HashMap::new(),
    };

    // Create execution context
    let mut context = ExecutionContext::new(goal);
    println!("✅ Execution context created: {}", context.context_id);

    // Create mock reasoning engine
    let reasoning_engine = MockReasoningEngine;

    // Simulate agent execution with reflection
    println!("\n🔄 Simulating Agent Execution with Reflection:");
    
    for iteration in 1..=12 {
        println!("\n--- Iteration {} ---", iteration);
        
        // Simulate some work
        context.increment_iteration();
        context.set_variable("current_iteration".to_string(), iteration.to_string());
        
        // Add some tasks and complete them with varying success
        let task = Task {
            task_id: format!("task-{}", iteration),
            description: format!("Task for iteration {}", iteration),
            task_type: TaskType::CodeAnalysis,
            priority: TaskPriority::Medium,
            dependencies: Vec::new(),
            inputs: HashMap::new(),
            expected_outputs: vec!["analysis_result".to_string()],
            success_criteria: vec!["Task completed successfully".to_string()],
            estimated_duration: None,
            max_attempts: 3,
            current_attempt: 1,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            success: None,
            error_message: None,
            metadata: HashMap::new(),
        };
        
        context.start_task(task.clone());
        
        // Simulate task completion with some failures for demonstration
        let success = iteration % 4 != 0; // Fail every 4th iteration
        context.complete_task(&task.task_id, success);
        
        if success {
            println!("   ✅ Task completed successfully");
        } else {
            println!("   ❌ Task failed");
        }

        // Check if reflection should be triggered
        if let Some(trigger) = reflection_engine.should_reflect(&context) {
            println!("   🧠 Reflection triggered: {:?}", trigger);
            
            // Perform reflection
            let reflection_result = reflection_engine.reflect(
                &context,
                &reasoning_engine,
                trigger
            ).await?;
            
            println!("   📊 Reflection Results:");
            println!("      Type: {:?}", reflection_result.reflection_type);
            println!("      Confidence: {:.2}", reflection_result.confidence_assessment);
            println!("      Performance: {:.2}", reflection_result.performance_assessment);
            println!("      Learning Insights: {}", reflection_result.learning_insights.len());
            println!("      Strategy Adjustments: {}", reflection_result.strategy_adjustments.len());
            println!("      Recommendations: {}", reflection_result.recommendations.len());
            
            // Display strategy adjustments
            if !reflection_result.strategy_adjustments.is_empty() {
                println!("   🔧 Strategy Adjustments:");
                for adjustment in &reflection_result.strategy_adjustments {
                    println!("      - {}: {}", 
                            adjustment.adjustment_type, 
                            adjustment.description);
                    println!("        Expected Impact: {:?}", adjustment.expected_impact);
                    println!("        Steps: {:?}", adjustment.implementation_steps);
                }
            }
            
            // Display learning insights
            if !reflection_result.learning_insights.is_empty() {
                println!("   💡 Learning Insights:");
                for insight in &reflection_result.learning_insights {
                    println!("      - {:?}: {}",
                            insight.insight_type,
                            insight.description);
                    println!("        Confidence: {:.2}", insight.confidence);
                    println!("        Retention Value: {:.2}", insight.retention_value);
                }
            }
            
            // Display recommendations
            if !reflection_result.recommendations.is_empty() {
                println!("   📋 Recommendations:");
                for recommendation in &reflection_result.recommendations {
                    println!("      - {:?}: {}",
                            recommendation.recommendation_type,
                            recommendation.description);
                    println!("        Priority: {:?}, Urgency: {:?}", 
                            recommendation.priority, 
                            recommendation.urgency);
                }
            }
        } else {
            println!("   ⏭️  No reflection needed");
        }
    }

    // Demonstrate manual reflection trigger
    println!("\n🎯 Manual Reflection Trigger:");
    let manual_reflection = reflection_engine.reflect(
        &context,
        &reasoning_engine,
        ReflectionTrigger::UserRequest
    ).await?;
    
    println!("📊 Manual Reflection Results:");
    println!("   Type: {:?}", manual_reflection.reflection_type);
    println!("   Confidence: {:.2}", manual_reflection.confidence_assessment);
    println!("   Performance: {:.2}", manual_reflection.performance_assessment);
    println!("   Learning Insights: {}", manual_reflection.learning_insights.len());
    println!("   Strategy Adjustments: {}", manual_reflection.strategy_adjustments.len());

    println!("\n✅ Demo completed successfully!");
    println!("💡 Key takeaways:");
    println!("   - Reflection engine automatically triggers based on configured frequency");
    println!("   - Reflection provides valuable insights for strategy adjustment");
    println!("   - Learning insights are retained for future decision making");
    println!("   - Strategy adjustments help improve performance over time");

    Ok(())
}