//! Performance profiling utilities for the fluent agent system
//! 
//! This module provides comprehensive profiling capabilities for analyzing
//! the performance characteristics of the reflection system and other
//! agent components.

pub mod memory_profiler;

pub use memory_profiler::{MemoryProfile, ReflectionMemoryProfiler};

/// Performance metrics aggregator
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub memory_usage: usize,
    pub execution_time: std::time::Duration,
    pub operation_count: usize,
    pub success_rate: f64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            memory_usage: 0,
            execution_time: std::time::Duration::from_secs(0),
            operation_count: 0,
            success_rate: 0.0,
        }
    }

    pub fn update(&mut self, memory: usize, duration: std::time::Duration, success: bool) {
        self.memory_usage += memory;
        self.execution_time += duration;
        self.operation_count += 1;
        
        // Update success rate using running average
        let current_successes = (self.success_rate * (self.operation_count - 1) as f64) + if success { 1.0 } else { 0.0 };
        self.success_rate = current_successes / self.operation_count as f64;
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}
