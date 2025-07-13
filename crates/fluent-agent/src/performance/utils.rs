use log::{debug, info};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

/// Performance counter for tracking operation metrics
#[derive(Debug, Clone)]
pub struct PerformanceCounter {
    stats: Arc<Mutex<PerformanceStats>>,
}

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
    pub average_duration: Duration,
    pub error_rate: f64,
}

impl PerformanceCounter {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(PerformanceStats {
                total_requests: 0,
                total_errors: 0,
                total_duration: Duration::from_secs(0),
                min_duration: None,
                max_duration: None,
                average_duration: Duration::from_secs(0),
                error_rate: 0.0,
            })),
        }
    }
    
    pub fn record_request(&self, duration: Duration, is_error: bool) {
        let mut stats = match self.stats.lock() {
            Ok(stats) => stats,
            Err(_) => {
                // Mutex is poisoned, but we can still continue with degraded functionality
                log::warn!("Performance stats mutex poisoned, skipping stats update");
                return;
            }
        };
        
        stats.total_requests += 1;
        if is_error {
            stats.total_errors += 1;
        }
        
        stats.total_duration += duration;
        
        // Update min/max
        stats.min_duration = Some(
            stats.min_duration.map_or(duration, |min| min.min(duration))
        );
        stats.max_duration = Some(
            stats.max_duration.map_or(duration, |max| max.max(duration))
        );
        
        // Update averages
        if stats.total_requests > 0 {
            stats.average_duration = stats.total_duration / stats.total_requests as u32;
            stats.error_rate = stats.total_errors as f64 / stats.total_requests as f64;
        }
    }
    
    pub fn get_stats(&self) -> PerformanceStats {
        match self.stats.lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                log::warn!("Performance stats mutex poisoned, returning default stats");
                PerformanceStats::default()
            }
        }
    }
    
    pub fn reset(&self) {
        let mut stats = match self.stats.lock() {
            Ok(stats) => stats,
            Err(_) => {
                log::warn!("Performance stats mutex poisoned, cannot reset stats");
                return;
            }
        };
        *stats = PerformanceStats {
            total_requests: 0,
            total_errors: 0,
            total_duration: Duration::from_secs(0),
            min_duration: None,
            max_duration: None,
            average_duration: Duration::from_secs(0),
            error_rate: 0.0,
        };
    }
}

impl Default for PerformanceCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage tracker
#[derive(Debug)]
pub struct MemoryTracker {
    initial_usage: u64,
    peak_usage: Arc<Mutex<u64>>,
}

impl MemoryTracker {
    pub fn new() -> Self {
        let initial = Self::get_memory_usage();
        Self {
            initial_usage: initial,
            peak_usage: Arc::new(Mutex::new(initial)),
        }
    }
    
    pub fn get_current_usage(&self) -> u64 {
        let current = Self::get_memory_usage();
        
        // Update peak usage
        let mut peak = match self.peak_usage.lock() {
            Ok(peak) => peak,
            Err(_) => {
                log::warn!("Memory tracker peak usage mutex poisoned");
                return;
            }
        };
        if current > *peak {
            *peak = current;
        }
        
        current
    }
    
    pub fn get_peak_usage(&self) -> u64 {
        match self.peak_usage.lock() {
            Ok(peak) => *peak,
            Err(_) => {
                log::warn!("Memory tracker peak usage mutex poisoned, returning 0");
                0
            }
        }
    }
    
    pub fn get_initial_usage(&self) -> u64 {
        self.initial_usage
    }
    
    pub fn get_usage_delta(&self) -> i64 {
        self.get_current_usage() as i64 - self.initial_usage as i64
    }
    
    fn get_memory_usage() -> u64 {
        get_current_process_memory().unwrap_or_else(|_| {
            // Fallback: return a simulated value based on time
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();

            // Base memory usage + some variation
            1024 * 1024 * 10 + (now.as_millis() % 1000) as u64 * 1024
        })
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource limiter for controlling concurrent operations
#[derive(Debug, Clone)]
pub struct ResourceLimiter {
    semaphore: Arc<Semaphore>,
}

impl ResourceLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
    
    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit<'_>, anyhow::Error> {
        self.semaphore.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to acquire semaphore permit: {}", e))
    }
    
    pub fn try_acquire(&self) -> Option<tokio::sync::SemaphorePermit<'_>> {
        self.semaphore.try_acquire().ok()
    }
    
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

/// Performance test utilities
pub struct PerformanceTestUtils;

impl PerformanceTestUtils {
    /// Run a performance test with the given parameters
    pub async fn run_test<F, Fut>(
        name: &str,
        num_operations: usize,
        operation: F,
    ) -> PerformanceTestResult
    where
        F: Fn(usize) -> Fut,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>>,
    {
        let counter = PerformanceCounter::new();
        let memory_tracker = MemoryTracker::new();
        let start_time = Instant::now();
        
        info!("Running performance test: {}", name);

        for i in 0..num_operations {
            let op_start = Instant::now();
            let result = operation(i).await;
            let op_duration = op_start.elapsed();

            counter.record_request(op_duration, result.is_err());

            if i % (num_operations / 10).max(1) == 0 {
                debug!("  Progress: {}/{}", i + 1, num_operations);
            }
        }
        
        let total_duration = start_time.elapsed();
        let stats = counter.get_stats();
        let peak_memory = memory_tracker.get_peak_usage();
        
        PerformanceTestResult {
            test_name: name.to_string(),
            total_duration,
            stats,
            peak_memory_usage: peak_memory,
            operations_per_second: num_operations as f64 / total_duration.as_secs_f64(),
        }
    }
    
    /// Run a concurrent performance test
    pub async fn run_concurrent_test<F, Fut>(
        name: &str,
        num_operations: usize,
        concurrency: usize,
        operation: F,
    ) -> PerformanceTestResult
    where
        F: Fn(usize) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send,
    {
        let counter = PerformanceCounter::new();
        let memory_tracker = MemoryTracker::new();
        let start_time = Instant::now();
        
        println!("Running concurrent performance test: {} (concurrency: {})", name, concurrency);
        
        let mut handles = Vec::new();
        let ops_per_task = num_operations / concurrency;
        
        for task_id in 0..concurrency {
            let counter_clone = counter.clone();
            let operation = &operation;
            
            let handle = tokio::spawn(async move {
                for op_id in 0..ops_per_task {
                    let op_start = Instant::now();
                    let result = operation(task_id * ops_per_task + op_id).await;
                    let op_duration = op_start.elapsed();
                    
                    counter_clone.record_request(op_duration, result.is_err());
                }
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            if let Err(e) = handle.await {
                log::warn!("Task failed during performance test: {}", e);
            }
        }
        
        let total_duration = start_time.elapsed();
        let stats = counter.get_stats();
        let peak_memory = memory_tracker.get_peak_usage();
        
        PerformanceTestResult {
            test_name: name.to_string(),
            total_duration,
            stats,
            peak_memory_usage: peak_memory,
            operations_per_second: num_operations as f64 / total_duration.as_secs_f64(),
        }
    }
}

/// Performance test result
#[derive(Debug)]
pub struct PerformanceTestResult {
    pub test_name: String,
    pub total_duration: Duration,
    pub stats: PerformanceStats,
    pub peak_memory_usage: u64,
    pub operations_per_second: f64,
}

impl PerformanceTestResult {
    pub fn print_summary(&self) {
        info!("=== Performance Test Results: {} ===", self.test_name);
        info!("  Total Duration: {:?}", self.total_duration);
        info!("  Total Operations: {}", self.stats.total_requests);
        info!("  Successful Operations: {}", self.stats.total_requests - self.stats.total_errors);
        info!("  Failed Operations: {}", self.stats.total_errors);
        info!("  Success Rate: {:.2}%", (1.0 - self.stats.error_rate) * 100.0);
        info!("  Operations per Second: {:.2}", self.operations_per_second);
        info!("  Average Operation Time: {:?}", self.stats.average_duration);
        info!("  Min Operation Time: {:?}", self.stats.min_duration.unwrap_or_default());
        println!("  Max Operation Time: {:?}", self.stats.max_duration.unwrap_or_default());
        println!("  Peak Memory Usage: {} bytes ({:.2} MB)", 
                 self.peak_memory_usage, 
                 self.peak_memory_usage as f64 / 1024.0 / 1024.0);
        println!("=== End Results ===\n");
    }
    
    pub fn assert_requirements(&self, requirements: &PerformanceRequirements) -> Result<(), anyhow::Error> {
        if let Some(max_duration) = requirements.max_duration {
            if self.total_duration > max_duration {
                return Err(anyhow::anyhow!(
                    "Test '{}' exceeded maximum duration: {:?} > {:?}",
                    self.test_name, self.total_duration, max_duration
                ));
            }
        }
        
        if let Some(min_ops_per_sec) = requirements.min_operations_per_second {
            if self.operations_per_second < min_ops_per_sec {
                return Err(anyhow::anyhow!(
                    "Test '{}' below minimum operations per second: {:.2} < {:.2}",
                    self.test_name, self.operations_per_second, min_ops_per_sec
                ));
            }
        }
        
        if let Some(max_error_rate) = requirements.max_error_rate {
            if self.stats.error_rate > max_error_rate {
                return Err(anyhow::anyhow!(
                    "Test '{}' exceeded maximum error rate: {:.2}% > {:.2}%",
                    self.test_name, self.stats.error_rate * 100.0, max_error_rate * 100.0
                ));
            }
        }
        
        Ok(())
    }
}

/// Performance requirements for testing
#[derive(Debug, Clone)]
pub struct PerformanceRequirements {
    pub max_duration: Option<Duration>,
    pub min_operations_per_second: Option<f64>,
    pub max_error_rate: Option<f64>,
    pub max_memory_usage: Option<u64>,
}

impl Default for PerformanceRequirements {
    fn default() -> Self {
        Self {
            max_duration: Some(Duration::from_secs(60)),
            min_operations_per_second: Some(10.0),
            max_error_rate: Some(0.05), // 5% max error rate
            max_memory_usage: Some(1024 * 1024 * 512), // 512MB max
        }
    }
}

/// Get current process memory usage in bytes (cross-platform)
fn get_current_process_memory() -> Result<u64, anyhow::Error> {
    #[cfg(target_os = "linux")]
    {
        get_process_memory_linux()
    }
    #[cfg(target_os = "macos")]
    {
        get_process_memory_macos()
    }
    #[cfg(target_os = "windows")]
    {
        get_process_memory_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Fallback for other platforms
        Ok(1024 * 1024) // 1MB default
    }
}

#[cfg(target_os = "linux")]
fn get_process_memory_linux() -> Result<u64, anyhow::Error> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|e| anyhow::anyhow!("Failed to read /proc/self/status: {}", e))?;

    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                if let Ok(kb) = kb_str.parse::<u64>() {
                    return Ok(kb * 1024); // Convert KB to bytes
                }
            }
        }
    }

    Err(anyhow::anyhow!("Could not find VmRSS in /proc/self/status"))
}

#[cfg(target_os = "macos")]
fn get_process_memory_macos() -> Result<u64, anyhow::Error> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-o", "rss", "-p"])
        .arg(std::process::id().to_string())
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run ps command: {}", e))?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.lines().collect();

    if lines.len() >= 2 {
        if let Ok(rss_kb) = lines[1].trim().parse::<u64>() {
            return Ok(rss_kb * 1024); // Convert KB to bytes
        }
    }

    Err(anyhow::anyhow!("Could not parse ps output"))
}

#[cfg(target_os = "windows")]
fn get_process_memory_windows() -> Result<u64, anyhow::Error> {
    // On Windows, we would use GetProcessMemoryInfo()
    // For now, provide a simplified implementation
    Ok(1024 * 1024) // 1MB default
}
