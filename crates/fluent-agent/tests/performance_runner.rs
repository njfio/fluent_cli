use fluent_agent::performance::utils::{PerformanceCounter, MemoryTracker};
use std::time::{Instant, Duration};
use anyhow::Result;
use tokio;

/// Performance test runner and reporting utilities
/// Provides comprehensive performance testing framework

pub struct PerformanceTestRunner {
    pub name: String,
    pub counter: PerformanceCounter,
    pub memory_tracker: MemoryTracker,
    pub start_time: Option<Instant>,
}

impl PerformanceTestRunner {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            counter: PerformanceCounter::new(),
            memory_tracker: MemoryTracker::new(),
            start_time: None,
        }
    }
    
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        println!("=== Starting Performance Test: {} ===", self.name);
    }
    
    pub fn record_operation(&self, duration: Duration, success: bool) {
        self.counter.record_request(duration, !success);
    }
    
    pub fn checkpoint(&self, checkpoint_name: &str) {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            let stats = self.counter.get_stats();
            let memory_usage = self.memory_tracker.get_current_usage();
            
            println!("  Checkpoint '{}' at {:?}:", checkpoint_name, elapsed);
            println!("    Operations: {}", stats.total_requests);
            println!("    Errors: {}", stats.total_errors);
            println!("    Avg duration: {:?}", stats.average_duration);
            println!("    Memory usage: {} bytes", memory_usage);
        }
    }
    
    pub fn finish(&self) -> PerformanceReport {
        let total_duration = self.start_time.map(|start| start.elapsed()).unwrap_or_default();
        let stats = self.counter.get_stats();
        let peak_memory = self.memory_tracker.get_peak_usage();
        let current_memory = self.memory_tracker.get_current_usage();
        
        let report = PerformanceReport {
            test_name: self.name.clone(),
            total_duration,
            total_operations: stats.total_requests,
            successful_operations: stats.total_requests - stats.total_errors,
            failed_operations: stats.total_errors,
            average_operation_time: stats.average_duration,
            operations_per_second: if total_duration.as_secs_f64() > 0.0 {
                stats.total_requests as f64 / total_duration.as_secs_f64()
            } else {
                0.0
            },
            peak_memory_usage: peak_memory,
            final_memory_usage: current_memory,
            success_rate: if stats.total_requests > 0 {
                (stats.total_requests - stats.total_errors) as f64 / stats.total_requests as f64
            } else {
                0.0
            },
        };
        
        report.print();
        report
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub test_name: String,
    pub total_duration: Duration,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub average_operation_time: Duration,
    pub operations_per_second: f64,
    pub peak_memory_usage: u64,
    pub final_memory_usage: u64,
    pub success_rate: f64,
}

impl PerformanceReport {
    pub fn print(&self) {
        println!("=== Performance Report: {} ===", self.test_name);
        println!("  Total Duration: {:?}", self.total_duration);
        println!("  Total Operations: {}", self.total_operations);
        println!("  Successful: {}", self.successful_operations);
        println!("  Failed: {}", self.failed_operations);
        println!("  Success Rate: {:.2}%", self.success_rate * 100.0);
        println!("  Average Operation Time: {:?}", self.average_operation_time);
        println!("  Operations per Second: {:.2}", self.operations_per_second);
        println!("  Peak Memory Usage: {} bytes ({:.2} MB)", 
                 self.peak_memory_usage, 
                 self.peak_memory_usage as f64 / 1024.0 / 1024.0);
        println!("  Final Memory Usage: {} bytes ({:.2} MB)", 
                 self.final_memory_usage, 
                 self.final_memory_usage as f64 / 1024.0 / 1024.0);
        println!("=== End Report ===\n");
    }
    
    pub fn assert_performance_requirements(&self, requirements: &PerformanceRequirements) -> Result<()> {
        if let Some(max_duration) = requirements.max_total_duration {
            if self.total_duration > max_duration {
                return Err(anyhow::anyhow!(
                    "Test '{}' took {:?}, exceeding maximum of {:?}",
                    self.test_name, self.total_duration, max_duration
                ));
            }
        }
        
        if let Some(min_ops_per_sec) = requirements.min_operations_per_second {
            if self.operations_per_second < min_ops_per_sec {
                return Err(anyhow::anyhow!(
                    "Test '{}' achieved {:.2} ops/sec, below minimum of {:.2}",
                    self.test_name, self.operations_per_second, min_ops_per_sec
                ));
            }
        }
        
        if let Some(min_success_rate) = requirements.min_success_rate {
            if self.success_rate < min_success_rate {
                return Err(anyhow::anyhow!(
                    "Test '{}' achieved {:.2}% success rate, below minimum of {:.2}%",
                    self.test_name, self.success_rate * 100.0, min_success_rate * 100.0
                ));
            }
        }
        
        if let Some(max_memory) = requirements.max_memory_usage {
            if self.peak_memory_usage > max_memory {
                return Err(anyhow::anyhow!(
                    "Test '{}' used {} bytes memory, exceeding maximum of {} bytes",
                    self.test_name, self.peak_memory_usage, max_memory
                ));
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceRequirements {
    pub max_total_duration: Option<Duration>,
    pub min_operations_per_second: Option<f64>,
    pub min_success_rate: Option<f64>,
    pub max_memory_usage: Option<u64>,
}

impl Default for PerformanceRequirements {
    fn default() -> Self {
        Self {
            max_total_duration: Some(Duration::from_secs(300)), // 5 minutes max
            min_operations_per_second: Some(10.0), // At least 10 ops/sec
            min_success_rate: Some(0.95), // 95% success rate
            max_memory_usage: Some(1024 * 1024 * 1024), // 1GB max
        }
    }
}

/// Comprehensive performance test suite
#[tokio::test]
async fn test_performance_runner_functionality() -> Result<()> {
    let mut runner = PerformanceTestRunner::new("Performance Runner Test");
    runner.start();
    
    // Simulate some operations
    for i in 0..100 {
        let op_start = Instant::now();
        
        // Simulate work
        tokio::time::sleep(Duration::from_millis(1)).await;
        
        let op_duration = op_start.elapsed();
        let success = i % 10 != 0; // 90% success rate
        
        runner.record_operation(op_duration, success);
        
        if i % 25 == 0 {
            runner.checkpoint(&format!("Operation {}", i));
        }
    }
    
    let report = runner.finish();
    
    // Verify report
    assert_eq!(report.total_operations, 100);
    assert_eq!(report.failed_operations, 10); // 10% failure rate
    assert_eq!(report.successful_operations, 90);
    assert!((report.success_rate - 0.9).abs() < 0.01); // ~90% success rate
    
    // Test performance requirements
    let requirements = PerformanceRequirements {
        max_total_duration: Some(Duration::from_secs(10)),
        min_operations_per_second: Some(5.0),
        min_success_rate: Some(0.8),
        max_memory_usage: Some(1024 * 1024 * 100), // 100MB
    };
    
    report.assert_performance_requirements(&requirements)?;
    
    Ok(())
}

#[tokio::test]
async fn test_performance_requirements_validation() -> Result<()> {
    // Create a report that should fail requirements
    let failing_report = PerformanceReport {
        test_name: "Failing Test".to_string(),
        total_duration: Duration::from_secs(100),
        total_operations: 10,
        successful_operations: 5,
        failed_operations: 5,
        average_operation_time: Duration::from_secs(10),
        operations_per_second: 0.1,
        peak_memory_usage: 1024 * 1024 * 1024 * 2, // 2GB
        final_memory_usage: 1024 * 1024 * 1024,
        success_rate: 0.5,
    };
    
    let strict_requirements = PerformanceRequirements {
        max_total_duration: Some(Duration::from_secs(10)),
        min_operations_per_second: Some(10.0),
        min_success_rate: Some(0.95),
        max_memory_usage: Some(1024 * 1024 * 100),
    };
    
    // Should fail all requirements
    let result = failing_report.assert_performance_requirements(&strict_requirements);
    assert!(result.is_err());
    
    // Create a report that should pass
    let passing_report = PerformanceReport {
        test_name: "Passing Test".to_string(),
        total_duration: Duration::from_secs(5),
        total_operations: 100,
        successful_operations: 98,
        failed_operations: 2,
        average_operation_time: Duration::from_millis(50),
        operations_per_second: 20.0,
        peak_memory_usage: 1024 * 1024 * 50, // 50MB
        final_memory_usage: 1024 * 1024 * 40,
        success_rate: 0.98,
    };
    
    let result = passing_report.assert_performance_requirements(&strict_requirements);
    assert!(result.is_ok());
    
    Ok(())
}

/// Performance test utilities
pub mod utils {
    use super::*;

    pub async fn run_simple_test(
        name: &str,
        num_operations: usize,
    ) -> Result<PerformanceReport>
    {
        let mut runner = PerformanceTestRunner::new(name);
        runner.start();

        for i in 0..num_operations {
            let op_start = Instant::now();

            // Simulate work
            tokio::time::sleep(Duration::from_millis(1)).await;

            let op_duration = op_start.elapsed();
            let success = i % 10 != 0; // 90% success rate

            runner.record_operation(op_duration, success);
        }

        Ok(runner.finish())
    }
}
