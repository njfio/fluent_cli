use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::{Result, anyhow};

/// Memory profiling metrics for reflection operations
#[derive(Debug, Clone)]
pub struct MemoryProfile {
    pub operation_name: String,
    pub peak_bytes: usize,
    pub allocation_count: usize,
    pub duration: Duration,
    pub timestamp: Instant,
    pub operation_breakdown: HashMap<String, usize>,
}

/// Memory profiler for tracking reflection system performance
pub struct ReflectionMemoryProfiler {
    profiles: Arc<Mutex<Vec<MemoryProfile>>>,
    current_operation: Arc<Mutex<Option<String>>>,
    start_time: Instant,
    baseline_memory: usize,
}

impl ReflectionMemoryProfiler {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(Mutex::new(Vec::new())),
            current_operation: Arc::new(Mutex::new(None)),
            start_time: Instant::now(),
            baseline_memory: Self::get_current_memory_usage(),
        }
    }

    /// Start profiling a reflection operation
    pub fn start_operation(&self, operation_name: &str) {
        if let Ok(mut current) = self.current_operation.lock() {
            *current = Some(operation_name.to_string());
        }
    }

    /// End profiling and record the results
    pub fn end_operation(&self) -> Result<MemoryProfile> {
        let operation_name = {
            let mut current = self.current_operation.lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock current operation: {}", e))?;
            current.take().unwrap_or_else(|| "unknown_operation".to_string())
        };

        let current_memory = Self::get_current_memory_usage();
        let peak_bytes = current_memory.saturating_sub(self.baseline_memory);
        
        let profile = MemoryProfile {
            operation_name: operation_name.clone(),
            peak_bytes,
            allocation_count: 1, // Simplified for demo
            duration: self.start_time.elapsed(),
            timestamp: Instant::now(),
            operation_breakdown: HashMap::new(),
        };

        if let Ok(mut profiles) = self.profiles.lock() {
            profiles.push(profile.clone());
        }

        Ok(profile)
    }

    /// Profile a closure and return the memory profile
    pub fn profile_operation<F, T>(&self, operation_name: &str, operation: F) -> Result<(T, MemoryProfile)>
    where
        F: FnOnce() -> T,
    {
        let start_memory = Self::get_current_memory_usage();
        let start_time = Instant::now();
        
        self.start_operation(operation_name);
        let result = operation();
        
        let end_memory = Self::get_current_memory_usage();
        let duration = start_time.elapsed();
        
        let profile = MemoryProfile {
            operation_name: operation_name.to_string(),
            peak_bytes: end_memory.saturating_sub(start_memory),
            allocation_count: 1,
            duration,
            timestamp: start_time,
            operation_breakdown: HashMap::new(),
        };

        if let Ok(mut profiles) = self.profiles.lock() {
            profiles.push(profile.clone());
        }

        Ok((result, profile))
    }

    /// Profile an async operation
    pub async fn profile_async_operation<F, Fut, T>(
        &self, 
        operation_name: &str, 
        operation: F
    ) -> Result<(T, MemoryProfile)>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start_memory = Self::get_current_memory_usage();
        let start_time = Instant::now();
        
        self.start_operation(operation_name);
        let result = operation().await;
        
        let end_memory = Self::get_current_memory_usage();
        let duration = start_time.elapsed();
        
        let profile = MemoryProfile {
            operation_name: operation_name.to_string(),
            peak_bytes: end_memory.saturating_sub(start_memory),
            allocation_count: 1,
            duration,
            timestamp: start_time,
            operation_breakdown: HashMap::new(),
        };

        if let Ok(mut profiles) = self.profiles.lock() {
            profiles.push(profile.clone());
        }

        Ok((result, profile))
    }

    /// Get all recorded profiles
    pub fn get_profiles(&self) -> Vec<MemoryProfile> {
        self.profiles.lock()
            .map(|profiles| profiles.clone())
            .unwrap_or_default()
    }

    /// Generate a comprehensive memory usage report
    pub fn generate_report(&self) -> String {
        let profiles = self.get_profiles();
        
        if profiles.is_empty() {
            return "No memory profiles recorded".to_string();
        }

        let mut report = String::new();
        report.push_str("Memory Profiling Report\n");
        report.push_str("======================\n\n");

        // Summary statistics
        let total_operations = profiles.len();
        let total_memory: usize = profiles.iter().map(|p| p.peak_bytes).sum();
        let avg_memory = total_memory / total_operations;
        let max_memory = profiles.iter().map(|p| p.peak_bytes).max().unwrap_or(0);
        let total_duration: Duration = profiles.iter().map(|p| p.duration).sum();

        report.push_str(&format!("Summary:\n"));
        report.push_str(&format!("  Total Operations: {}\n", total_operations));
        report.push_str(&format!("  Total Memory Used: {} bytes\n", total_memory));
        report.push_str(&format!("  Average Memory per Operation: {} bytes\n", avg_memory));
        report.push_str(&format!("  Peak Memory Usage: {} bytes\n", max_memory));
        report.push_str(&format!("  Total Duration: {:?}\n\n", total_duration));

        // Per-operation details
        report.push_str("Operation Details:\n");
        report.push_str("------------------\n");
        
        for (i, profile) in profiles.iter().enumerate() {
            report.push_str(&format!("{}. Operation: {}\n", i + 1, profile.operation_name));
            report.push_str(&format!("   Memory Used: {} bytes\n", profile.peak_bytes));
            report.push_str(&format!("   Duration: {:?}\n", profile.duration));
            report.push_str(&format!("   Timestamp: {:?}\n\n", profile.timestamp));
        }

        // Performance analysis
        report.push_str("Performance Analysis:\n");
        report.push_str("--------------------\n");
        
        let high_memory_ops: Vec<_> = profiles.iter()
            .filter(|p| p.peak_bytes > avg_memory * 2)
            .collect();
            
        if !high_memory_ops.is_empty() {
            report.push_str("High Memory Operations:\n");
            for op in high_memory_ops {
                report.push_str(&format!("  - {}: {} bytes\n", op.operation_name, op.peak_bytes));
            }
        } else {
            report.push_str("No high memory usage operations detected.\n");
        }

        report
    }

    /// Save the report to a file asynchronously
    pub async fn save_report(&self, filename: &str) -> Result<()> {
        let report = self.generate_report();
        tokio::fs::write(filename, report).await?;
        Ok(())
    }

    /// Get current memory usage (cross-platform implementation)
    fn get_current_memory_usage() -> usize {
        // Use a blocking approach for constructor compatibility
        // In a real implementation, you might want to use a different approach
        match std::thread::spawn(|| {
            tokio::runtime::Handle::try_current()
                .map(|handle| {
                    handle.block_on(get_process_memory_usage())
                })
                .unwrap_or_else(|_| {
                    // If no tokio runtime, create a minimal one
                    match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt.block_on(get_process_memory_usage()),
                        Err(_) => {
                            // Fallback to a default value if runtime creation fails
                            Ok(1024 * 1024) // 1MB default
                        }
                    }
                })
        }).join() {
            Ok(Ok(memory)) => memory,
            _ => {
                // Fallback: return a reasonable estimate
                std::mem::size_of::<Self>() * 1000
            }
        }
    }
}

impl Default for ReflectionMemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current process memory usage in bytes (cross-platform)
async fn get_process_memory_usage() -> Result<usize> {
    #[cfg(target_os = "linux")]
    {
        get_process_memory_usage_linux().await
    }
    #[cfg(target_os = "macos")]
    {
        get_process_memory_usage_macos()
    }
    #[cfg(target_os = "windows")]
    {
        get_process_memory_usage_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Fallback for other platforms
        Ok(1024 * 1024) // 1MB default
    }
}

#[cfg(target_os = "linux")]
async fn get_process_memory_usage_linux() -> Result<usize> {
    let status = tokio::fs::read_to_string("/proc/self/status")
        .await
        .map_err(|e| anyhow!("Failed to read /proc/self/status: {}", e))?;

    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                if let Ok(kb) = kb_str.parse::<usize>() {
                    return Ok(kb * 1024); // Convert KB to bytes
                }
            }
        }
    }

    Err(anyhow!("Could not find VmRSS in /proc/self/status"))
}

#[cfg(target_os = "macos")]
fn get_process_memory_usage_macos() -> Result<usize> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-o", "rss", "-p"])
        .arg(std::process::id().to_string())
        .output()
        .map_err(|e| anyhow!("Failed to run ps command: {}", e))?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.lines().collect();

    if lines.len() >= 2 {
        if let Ok(rss_kb) = lines[1].trim().parse::<usize>() {
            return Ok(rss_kb * 1024); // Convert KB to bytes
        }
    }

    Err(anyhow!("Could not parse ps output"))
}

#[cfg(target_os = "windows")]
fn get_process_memory_usage_windows() -> Result<usize> {
    // On Windows, we would use GetProcessMemoryInfo()
    // For now, provide a simplified implementation
    Ok(1024 * 1024) // 1MB default
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_memory_profiler_basic() {
        let profiler = ReflectionMemoryProfiler::new();
        
        let (result, profile) = profiler.profile_operation("test_operation", || {
            // Simulate some work
            let _data = vec![0u8; 1024]; // Allocate 1KB
            42
        }).unwrap();
        
        assert_eq!(result, 42);
        assert_eq!(profile.operation_name, "test_operation");
        assert!(profile.duration > Duration::from_nanos(0));
    }

    #[test]
    fn test_memory_profiler_multiple_operations() {
        let profiler = ReflectionMemoryProfiler::new();
        
        // Profile multiple operations
        for i in 0..3 {
            let operation_name = format!("operation_{}", i);
            profiler.profile_operation(&operation_name, || {
                thread::sleep(Duration::from_millis(10));
            }).unwrap();
        }
        
        let profiles = profiler.get_profiles();
        assert_eq!(profiles.len(), 3);
        
        let report = profiler.generate_report();
        assert!(report.contains("Memory Profiling Report"));
        assert!(report.contains("Total Operations: 3"));
    }

    #[tokio::test]
    async fn test_async_profiling() {
        let profiler = ReflectionMemoryProfiler::new();
        
        let (result, profile) = profiler.profile_async_operation("async_test", || async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            "async_result"
        }).await.unwrap();
        
        assert_eq!(result, "async_result");
        assert_eq!(profile.operation_name, "async_test");
        assert!(profile.duration >= Duration::from_millis(10));
    }
}
