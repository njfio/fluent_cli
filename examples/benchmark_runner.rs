//! Benchmark Runner Example
//!
//! This example demonstrates how to run comprehensive benchmarks
//! for the enhanced autonomous task execution system.

use anyhow::Result;
use fluent_agent::{AutonomousBenchmarkSuite, BenchmarkConfig};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Enhanced Agentic System Benchmark Runner");
    println!("==========================================");
    
    // Create benchmark configuration
    let config = BenchmarkConfig {
        enable_performance_benchmarks: true,
        enable_scalability_benchmarks: true,
        enable_quality_benchmarks: true,
        enable_stress_tests: true,
        max_execution_time: std::time::Duration::from_secs(300),
        iterations_per_test: 10,
        concurrent_tasks_limit: 50,
    };
    
    // Create and run benchmark suite
    let mut benchmark_suite = AutonomousBenchmarkSuite::new(config);
    
    println!("Starting comprehensive benchmark execution...\n");
    benchmark_suite.run_all_benchmarks().await?;
    
    // Display results summary
    let results = benchmark_suite.get_results();
    println!("\n📈 Benchmark execution completed successfully!");
    println!("Total benchmarks executed: {}", results.len());
    
    // Calculate overall performance score
    let avg_success_rate: f64 = results.iter()
        .map(|r| r.success_rate)
        .sum::<f64>() / results.len() as f64;
    
    let avg_quality: f64 = results.iter()
        .map(|r| (r.quality_metrics.accuracy_score + 
                  r.quality_metrics.completeness_score + 
                  r.quality_metrics.efficiency_score + 
                  r.quality_metrics.adaptability_score) / 4.0)
        .sum::<f64>() / results.len() as f64;
    
    println!("\n🎯 FINAL ASSESSMENT:");
    println!("   Overall Success Rate: {:.1}%", avg_success_rate * 100.0);
    println!("   Overall Quality Score: {:.2}/1.0", avg_quality);
    
    if avg_success_rate > 0.85 && avg_quality > 0.8 {
        println!("   🏆 Status: EXCELLENT - System ready for production deployment");
    } else if avg_success_rate > 0.7 && avg_quality > 0.7 {
        println!("   ✅ Status: GOOD - System performing well with minor optimization opportunities");
    } else {
        println!("   ⚠️  Status: NEEDS IMPROVEMENT - System requires optimization before deployment");
    }
    
    Ok(())
}