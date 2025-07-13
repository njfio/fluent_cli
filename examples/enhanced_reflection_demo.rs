use anyhow::Result;
use async_trait::async_trait;
use fluent_agent::{
    ExecutionContext, ReflectionEngine, ReflectionConfig,
    Goal, GoalType, GoalPriority, Task, TaskType, TaskPriority,
    ReasoningEngine,
};

use fluent_agent::profiling::ReflectionMemoryProfiler;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use tokio;

/// Enhanced reasoning engine with memory profiling
struct ProfiledReasoningEngine {
    profiler: ReflectionMemoryProfiler,
}

impl ProfiledReasoningEngine {
    fn new() -> Self {
        Self {
            profiler: ReflectionMemoryProfiler::new(),
        }
    }

    fn get_profiler(&self) -> &ReflectionMemoryProfiler {
        &self.profiler
    }
}

#[async_trait]
impl ReasoningEngine for ProfiledReasoningEngine {
    async fn reason(&self, context: &ExecutionContext) -> Result<fluent_agent::orchestrator::ReasoningResult> {
        // Profile the reasoning operation
        let (result, profile) = self.profiler.profile_async_operation("reasoning_operation", || async {
            // Simulate complex reasoning with memory allocation
            let analysis_data = vec![0u8; 1024 * 100]; // 100KB of analysis data
            
            // Simulate processing time
            tokio::time::sleep(Duration::from_millis(50)).await;
            
            fluent_agent::orchestrator::ReasoningResult {
                reasoning_type: fluent_agent::orchestrator::ReasoningType::SelfReflection,
                input_context: context.get_summary(),
                reasoning_output: format!("Enhanced reasoning analysis with {} bytes of data", analysis_data.len()),
                confidence_score: 0.75 + (context.iteration_count() as f64 * 0.02).min(0.2),
                goal_achieved_confidence: 0.6 + (context.iteration_count() as f64 * 0.03).min(0.3),
                next_actions: vec![
                    "Continue with enhanced approach".to_string(),
                    "Monitor memory usage patterns".to_string(),
                    "Optimize based on profiling data".to_string(),
                ],
            }
        }).await?;

        println!("   🔍 Reasoning Memory Profile: {} bytes, {:?}", 
                 profile.peak_bytes, profile.duration);

        Ok(result)
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

/// Demonstrates enhanced self-reflection with memory profiling
#[tokio::main]
async fn main() -> Result<()> {
    println!("🧠 Enhanced Fluent Agent Self-Reflection with Memory Profiling");
    println!("==============================================================\n");

    // Create memory profiler for the entire demo
    let demo_profiler = ReflectionMemoryProfiler::new();

    // Configure reflection engine with optimized settings
    let reflection_config = ReflectionConfig {
        reflection_frequency: 3,
        deep_reflection_frequency: 6,
        learning_retention_days: 30,
        confidence_threshold: 0.6,
        performance_threshold: 0.7,
        enable_meta_reflection: true,
        strategy_adjustment_sensitivity: 0.8,
    };

    // Create reflection engine
    let mut reflection_engine = ReflectionEngine::with_config(reflection_config);
    println!("✅ Enhanced reflection engine initialized");

    // Create a complex goal for testing
    let goal = Goal {
        goal_id: "enhanced-reflection-demo".to_string(),
        description: "Demonstrate enhanced self-reflection with memory profiling and performance optimization".to_string(),
        goal_type: GoalType::Analysis,
        priority: GoalPriority::High,
        success_criteria: vec![
            "Complete memory-profiled reflection analysis".to_string(),
            "Generate optimized strategy adjustments".to_string(),
            "Demonstrate performance improvements".to_string(),
            "Produce comprehensive profiling report".to_string(),
        ],
        max_iterations: Some(15),
        timeout: None,
        metadata: HashMap::new(),
    };

    // Create execution context
    let mut context = ExecutionContext::new(goal);
    println!("✅ Execution context created: {}", context.context_id);

    // Create profiled reasoning engine
    let reasoning_engine = ProfiledReasoningEngine::new();
    println!("✅ Profiled reasoning engine initialized");

    // Simulate enhanced agent execution with memory profiling
    println!("\n🔄 Enhanced Agent Execution with Memory Profiling:");
    
    for iteration in 1..=10 {
        println!("\n--- Iteration {} ---", iteration);
        
        // Profile the entire iteration
        let (_, iteration_profile) = demo_profiler.profile_operation(
            &format!("iteration_{}", iteration),
            || {
                // Simulate iteration work
                context.increment_iteration();
                context.set_variable("current_iteration".to_string(), iteration.to_string());
                
                // Create and execute tasks with varying complexity
                let task_complexity = if iteration % 3 == 0 { "high" } else { "medium" };
                let task = Task {
                    task_id: format!("enhanced-task-{}", iteration),
                    description: format!("Enhanced task for iteration {} (complexity: {})", iteration, task_complexity),
                    task_type: TaskType::CodeAnalysis,
                    priority: TaskPriority::Medium,
                    dependencies: Vec::new(),
                    inputs: HashMap::new(),
                    expected_outputs: vec!["enhanced_analysis_result".to_string()],
                    success_criteria: vec!["Task completed with profiling".to_string()],
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
                
                // Simulate task execution with memory allocation
                let _work_data = vec![0u8; 1024 * iteration]; // Increasing memory usage
                
                // Task success rate improves over time (learning effect)
                let success = iteration <= 2 || iteration % 4 != 0 || iteration > 7;
                context.complete_task(&task.task_id, success);
                
                if success {
                    println!("   ✅ Enhanced task completed successfully");
                } else {
                    println!("   ❌ Enhanced task failed");
                }
            }
        )?;

        println!("   📊 Iteration Memory: {} bytes, Duration: {:?}", 
                 iteration_profile.peak_bytes, iteration_profile.duration);

        // Check for reflection triggers
        if let Some(trigger) = reflection_engine.should_reflect(&context) {
            println!("   🧠 Enhanced reflection triggered: {:?}", trigger);
            
            // Profile the reflection operation
            let (reflection_result, reflection_profile) = demo_profiler.profile_async_operation(
                "reflection_operation",
                || reflection_engine.reflect(&context, &reasoning_engine, trigger)
            ).await?;

            let reflection_result = reflection_result?;

            println!("   🔍 Reflection Memory: {} bytes, Duration: {:?}", 
                     reflection_profile.peak_bytes, reflection_profile.duration);
            
            println!("   📊 Enhanced Reflection Results:");
            println!("      Type: {:?}", reflection_result.reflection_type);
            println!("      Confidence: {:.2}", reflection_result.confidence_assessment);
            println!("      Performance: {:.2}", reflection_result.performance_assessment);
            println!("      Learning Insights: {}", reflection_result.learning_insights.len());
            println!("      Strategy Adjustments: {}", reflection_result.strategy_adjustments.len());
            
            // Display memory-optimized strategy adjustments
            if !reflection_result.strategy_adjustments.is_empty() {
                println!("   🔧 Memory-Optimized Strategy Adjustments:");
                for adjustment in &reflection_result.strategy_adjustments {
                    println!("      - {:?}: {}",
                            adjustment.adjustment_type,
                            adjustment.description);
                    if adjustment.description.contains("memory") || adjustment.description.contains("performance") {
                        println!("        🎯 Performance-focused adjustment detected");
                    }
                }
            }
            
            // Display learning insights with memory context
            if !reflection_result.learning_insights.is_empty() {
                println!("   💡 Memory-Aware Learning Insights:");
                for insight in &reflection_result.learning_insights {
                    println!("      - {:?}: {}",
                            insight.insight_type,
                            insight.description);
                    println!("        Confidence: {:.2}, Retention: {:.2}", 
                            insight.confidence, insight.retention_value);
                }
            }
        } else {
            println!("   ⏭️  No reflection needed");
        }
    }

    // Generate comprehensive profiling report
    println!("\n📈 Memory Profiling Report:");
    println!("==========================");
    
    let demo_report = demo_profiler.generate_report();
    println!("{}", demo_report);
    
    // Save profiling report to file
    demo_profiler.save_report("enhanced_reflection_profiling_report.txt").await?;
    println!("✅ Profiling report saved to: enhanced_reflection_profiling_report.txt");

    // Get reasoning engine profiling data
    println!("\n🧠 Reasoning Engine Memory Analysis:");
    let reasoning_report = reasoning_engine.get_profiler().generate_report();
    println!("{}", reasoning_report);
    
    reasoning_engine.get_profiler().save_report("reasoning_engine_profiling_report.txt").await?;
    println!("✅ Reasoning profiling report saved to: reasoning_engine_profiling_report.txt");

    // Final reflection statistics with memory context
    println!("\n📊 Enhanced Reflection Statistics:");
    let stats = reflection_engine.get_reflection_statistics();
    println!("   Total Learning Experiences: {}", stats.total_learning_experiences);
    println!("   Total Strategy Patterns: {}", stats.total_strategy_patterns);
    println!("   Average Success Rate: {:.2}", stats.average_success_rate);
    println!("   Learning Velocity: {:.2}", stats.learning_velocity);
    println!("   Reflection Frequency: {}", stats.reflection_frequency);

    // Performance analysis
    let all_profiles = demo_profiler.get_profiles();
    if !all_profiles.is_empty() {
        let total_memory: usize = all_profiles.iter().map(|p| p.peak_bytes).sum();
        let avg_memory = total_memory / all_profiles.len();
        let max_memory = all_profiles.iter().map(|p| p.peak_bytes).max().unwrap_or(0);
        
        println!("\n🎯 Performance Optimization Insights:");
        println!("   Total Memory Tracked: {} bytes", total_memory);
        println!("   Average Memory per Operation: {} bytes", avg_memory);
        println!("   Peak Memory Usage: {} bytes", max_memory);
        
        if max_memory > avg_memory * 3 {
            println!("   ⚠️  High memory variance detected - optimization recommended");
        } else {
            println!("   ✅ Memory usage is consistent across operations");
        }
    }

    println!("\n🎉 Enhanced Self-Reflection Demo Complete!");
    println!("   Key achievements:");
    println!("   - ✅ Memory profiling integrated into reflection system");
    println!("   - ✅ Performance-aware strategy adjustments");
    println!("   - ✅ Comprehensive profiling reports generated");
    println!("   - ✅ Memory optimization insights provided");
    println!("   - ✅ Real-time performance monitoring");

    Ok(())
}
