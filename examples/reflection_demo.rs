use anyhow::Result;
use async_trait::async_trait;
use fluent_agent::{
    ExecutionContext, ReflectionEngine, ReflectionConfig,
    Goal, GoalType, GoalPriority, Task, TaskType, TaskPriority,
    ReasoningEngine,
};
use fluent_agent::reflection::{ReflectionTrigger, ReflectionType};
use std::collections::HashMap;
use std::time::SystemTime;
use tokio;

/// Mock reasoning engine for demonstration
struct MockReasoningEngine;

#[async_trait]
impl ReasoningEngine for MockReasoningEngine {
    async fn reason(&self, context: &ExecutionContext) -> Result<fluent_agent::orchestrator::ReasoningResult> {
        Ok(fluent_agent::orchestrator::ReasoningResult {
            reasoning_type: fluent_agent::orchestrator::ReasoningType::SelfReflection,
            input_context: context.get_summary(),
            reasoning_output: "Mock reasoning analysis of current situation".to_string(),
            confidence_score: 0.75,
            goal_achieved_confidence: 0.6,
            next_actions: vec![
                "Continue with current approach".to_string(),
                "Monitor progress closely".to_string(),
                "Consider alternative strategies if needed".to_string(),
            ],
        })
    }

    fn get_capabilities(&self) -> Vec<fluent_agent::ReasoningCapability> {
        vec![
            fluent_agent::ReasoningCapability::SelfReflection,
            fluent_agent::ReasoningCapability::StrategyFormulation,
            fluent_agent::ReasoningCapability::ProgressEvaluation,
        ]
    }

    fn can_handle(&self, reasoning_type: &fluent_agent::orchestrator::ReasoningType) -> bool {
        matches!(reasoning_type, fluent_agent::orchestrator::ReasoningType::SelfReflection)
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
            task_type: TaskType::Analysis,
            priority: TaskPriority::Medium,
            dependencies: Vec::new(),
            estimated_duration: None,
            created_at: SystemTime::now(),
            completed_at: None,
            success: None,
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
                    println!("      - {}: {}", 
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
                    println!("      - {}: {}", 
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
    
    println!("   Manual reflection completed:");
    println!("   Confidence: {:.2}", manual_reflection.confidence_assessment);
    println!("   Performance: {:.2}", manual_reflection.performance_assessment);
    println!("   Total insights: {}", manual_reflection.learning_insights.len());

    // Show reflection statistics
    println!("\n📈 Reflection Statistics:");
    let stats = reflection_engine.get_reflection_statistics();
    println!("   Total Learning Experiences: {}", stats.total_learning_experiences);
    println!("   Total Strategy Patterns: {}", stats.total_strategy_patterns);
    println!("   Average Success Rate: {:.2}", stats.average_success_rate);
    println!("   Learning Velocity: {:.2}", stats.learning_velocity);
    println!("   Reflection Frequency: {}", stats.reflection_frequency);

    // Demonstrate different reflection triggers
    println!("\n🚨 Testing Different Reflection Triggers:");
    
    // Low confidence trigger
    let low_confidence_reflection = reflection_engine.reflect(
        &context,
        &reasoning_engine,
        ReflectionTrigger::LowConfidence(0.3)
    ).await?;
    println!("   Low Confidence Reflection - Adjustments: {}", 
             low_confidence_reflection.strategy_adjustments.len());
    
    // Poor performance trigger
    let poor_performance_reflection = reflection_engine.reflect(
        &context,
        &reasoning_engine,
        ReflectionTrigger::PoorPerformance(0.4)
    ).await?;
    println!("   Poor Performance Reflection - Adjustments: {}", 
             poor_performance_reflection.strategy_adjustments.len());
    
    // Crisis trigger
    let crisis_reflection = reflection_engine.reflect(
        &context,
        &reasoning_engine,
        ReflectionTrigger::CriticalError("System failure detected".to_string())
    ).await?;
    println!("   Crisis Reflection - Type: {:?}", crisis_reflection.reflection_type);
    println!("   Crisis Reflection - Recommendations: {}", 
             crisis_reflection.recommendations.len());

    // Final context summary
    println!("\n📋 Final Context Summary:");
    println!("   {}", context.get_summary());
    println!("   {}", context.get_progress_summary());
    println!("   Strategy Adjustments Made: {}", context.strategy_adjustments.len());

    // Demonstrate learning experience analysis
    println!("\n📚 Learning Experience Analysis:");
    let final_stats = reflection_engine.get_reflection_statistics();
    if final_stats.total_learning_experiences > 0 {
        println!("   Total experiences captured: {}", final_stats.total_learning_experiences);
        println!("   Average success rate: {:.2}", final_stats.average_success_rate);
        println!("   Learning velocity: {:.2}", final_stats.learning_velocity);

        if final_stats.average_success_rate > 0.7 {
            println!("   ✅ Agent is performing well and learning effectively");
        } else if final_stats.average_success_rate > 0.5 {
            println!("   ⚠️  Agent performance is moderate - more learning needed");
        } else {
            println!("   ❌ Agent performance is poor - significant improvements required");
        }
    }

    // Show improvement recommendations
    println!("\n💡 Improvement Recommendations:");
    println!("   1. Continue regular reflection cycles for ongoing learning");
    println!("   2. Monitor strategy adjustment effectiveness");
    println!("   3. Analyze failure patterns to prevent recurring issues");
    println!("   4. Leverage successful patterns for similar future tasks");
    println!("   5. Adjust reflection frequency based on task complexity");

    println!("\n🎉 Self-Reflection & Strategy Adjustment Demo Complete!");
    println!("   The agent has demonstrated advanced self-awareness and learning capabilities");
    println!("   Key achievements:");
    println!("   - ✅ Automatic reflection triggering based on performance");
    println!("   - ✅ Strategy adjustment generation and application");
    println!("   - ✅ Learning insight extraction and retention");
    println!("   - ✅ Performance pattern recognition");
    println!("   - ✅ Crisis detection and emergency response");
    println!("   - ✅ Meta-reflection for process improvement");

    Ok(())
}
