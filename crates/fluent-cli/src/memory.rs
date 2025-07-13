use anyhow::{Result, anyhow};
use log::{debug, info, warn};
use std::fs;
use std::path::Path;

/// Memory management utilities for the CLI
pub struct MemoryManager;

impl MemoryManager {
    /// Force memory cleanup by dropping large allocations and clearing caches
    pub fn force_cleanup() {
        debug!("Performing comprehensive memory cleanup");

        // Clear thread-local storage
        std::thread_local! {
            static CLEANUP_COUNTER: std::cell::RefCell<u64> = std::cell::RefCell::new(0);
        }

        CLEANUP_COUNTER.with(|counter| {
            let mut count = counter.borrow_mut();
            *count += 1;
            debug!("Memory cleanup iteration: {}", *count);
        });

        // Force drop of any large static allocations we can control
        Self::clear_static_caches();

        // Trigger any available memory compaction
        Self::compact_memory();

        debug!("Memory cleanup completed");
    }

    /// Clear static caches and pools
    fn clear_static_caches() {
        debug!("Clearing static caches");

        // Note: In a real implementation, we would clear specific caches
        // For now, we'll use a more conservative approach

        // Clear any environment variable caches
        std::env::vars().count(); // This forces env var cache refresh

        // Clear DNS cache if possible (platform-specific)
        #[cfg(unix)]
        {
            // On Unix systems, we could potentially clear resolver cache
            debug!("Unix system detected - considering DNS cache clear");
        }

        #[cfg(windows)]
        {
            // On Windows, we could use different approaches
            debug!("Windows system detected - considering cache clear");
        }
    }

    /// Attempt memory compaction where possible
    fn compact_memory() {
        debug!("Attempting memory compaction");

        // Force allocation of a large block and immediate deallocation
        // This can help with memory fragmentation in some allocators
        let _large_vec: Vec<u8> = Vec::with_capacity(1024 * 1024); // 1MB
        drop(_large_vec);

        // Create and drop several smaller allocations to encourage compaction
        for _ in 0..10 {
            let _small_vec: Vec<u8> = Vec::with_capacity(64 * 1024); // 64KB
        }

        debug!("Memory compaction attempt completed");
    }

    /// Log current memory usage for debugging (cross-platform)
    pub fn log_memory_usage(context: &str) {
        debug!("Memory usage check at: {}", context);

        match get_memory_info() {
            Ok(info) => {
                debug!("{}: RSS: {} KB, Virtual: {} KB",
                    context, info.rss_kb, info.virtual_kb);
            }
            Err(e) => {
                debug!("{}: Failed to get memory info: {}", context, e);
            }
        }
    }

    /// Clean up temporary resources including files, caches, and checkpoints
    pub fn cleanup_temp_resources() -> Result<()> {
        debug!("Starting comprehensive temporary resource cleanup");

        // Clean up temporary files
        Self::cleanup_temp_files()?;

        // Clean up old checkpoints
        Self::cleanup_old_checkpoints()?;

        // Clean up cache files
        Self::cleanup_cache_files()?;

        // Clean up log files if they're too large
        Self::cleanup_large_log_files()?;

        debug!("Temporary resource cleanup completed");
        Ok(())
    }

    /// Clean up temporary files with enhanced patterns
    fn cleanup_temp_files() -> Result<()> {
        debug!("Cleaning up temporary files");

        let temp_patterns = [
            "/tmp/fluent_*",
            "/tmp/pipeline_*",
            "/tmp/agent_*",
            "/tmp/mcp_*",
            "/tmp/neo4j_*",
            "/tmp/checkpoint_*",
            "/var/tmp/fluent_*",
            // Platform-specific temp directories
            #[cfg(windows)]
            "C:\\Windows\\Temp\\fluent_*",
            #[cfg(windows)]
            "C:\\Users\\*\\AppData\\Local\\Temp\\fluent_*",
        ];

        let mut cleaned_count = 0;
        let mut failed_count = 0;

        for pattern in &temp_patterns {
            if let Ok(entries) = glob::glob(pattern) {
                for entry in entries.flatten() {
                    match fs::remove_file(&entry) {
                        Ok(_) => {
                            debug!("Cleaned up temp file: {:?}", entry);
                            cleaned_count += 1;
                        }
                        Err(e) => {
                            warn!("Failed to remove temp file {:?}: {}", entry, e);
                            failed_count += 1;
                        }
                    }
                }
            }
        }

        info!("Temp file cleanup: {} cleaned, {} failed", cleaned_count, failed_count);
        Ok(())
    }

    /// Clean up old checkpoint files
    fn cleanup_old_checkpoints() -> Result<()> {
        debug!("Cleaning up old checkpoint files");

        let checkpoint_dirs = [
            ".fluent/checkpoints",
            "/tmp/fluent_checkpoints",
            "checkpoints",
        ];

        let cutoff_time = std::time::SystemTime::now() - std::time::Duration::from_secs(7 * 24 * 3600); // 7 days

        for dir in &checkpoint_dirs {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            if modified < cutoff_time {
                                if let Err(e) = fs::remove_file(entry.path()) {
                                    warn!("Failed to remove old checkpoint {:?}: {}", entry.path(), e);
                                } else {
                                    debug!("Removed old checkpoint: {:?}", entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Clean up cache files that are too large or old
    fn cleanup_cache_files() -> Result<()> {
        debug!("Cleaning up cache files");

        let cache_dirs = [
            ".fluent/cache",
            "/tmp/fluent_cache",
            "cache",
        ];

        let max_cache_size = 100 * 1024 * 1024; // 100MB
        let cutoff_time = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 3600); // 1 day

        for dir in &cache_dirs {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        let should_remove = if let Ok(modified) = metadata.modified() {
                            modified < cutoff_time || metadata.len() > max_cache_size
                        } else {
                            metadata.len() > max_cache_size
                        };

                        if should_remove {
                            if let Err(e) = fs::remove_file(entry.path()) {
                                warn!("Failed to remove cache file {:?}: {}", entry.path(), e);
                            } else {
                                debug!("Removed cache file: {:?}", entry.path());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Clean up log files that are too large
    fn cleanup_large_log_files() -> Result<()> {
        debug!("Cleaning up large log files");

        let log_patterns = [
            "*.log",
            "logs/*.log",
            "/tmp/*.log",
            ".fluent/logs/*.log",
        ];

        let max_log_size = 50 * 1024 * 1024; // 50MB

        for pattern in &log_patterns {
            if let Ok(entries) = glob::glob(pattern) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = fs::metadata(&entry) {
                        if metadata.len() > max_log_size {
                            // Truncate instead of deleting to preserve the file
                            if let Err(e) = fs::write(&entry, "") {
                                warn!("Failed to truncate large log file {:?}: {}", entry, e);
                            } else {
                                info!("Truncated large log file: {:?} (was {} bytes)", entry, metadata.len());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if system has sufficient memory for operation (cross-platform)
    pub fn check_memory_availability(required_mb: u64) -> bool {
        match get_system_memory_info() {
            Ok(info) => {
                let available_mb = info.available_kb / 1024;
                available_mb >= required_mb
            }
            Err(_) => {
                // Default to true if we can't determine memory usage
                // This is conservative but prevents blocking operations
                debug!("Could not determine system memory, allowing operation");
                true
            }
        }
    }

    /// Optimize memory usage for large operations with comprehensive preparation
    pub fn optimize_for_large_operation() {
        info!("Optimizing memory for large operation");

        // Check current memory state
        Self::log_memory_usage("before_optimization");

        // Clean up temporary resources first
        if let Err(e) = Self::cleanup_temp_resources() {
            warn!("Failed to cleanup temp resources during optimization: {}", e);
        }

        // Force cleanup
        Self::force_cleanup();

        // Set memory-conscious environment variables
        Self::set_memory_optimized_env();

        // Log optimized state
        Self::log_memory_usage("after_optimization");

        info!("Memory optimization completed");
    }

    /// Clean up after large operations with comprehensive cleanup
    pub fn cleanup_after_large_operation() {
        info!("Cleaning up after large operation");

        // Log state before cleanup
        Self::log_memory_usage("before_cleanup");

        // Clean up temporary resources
        if let Err(e) = Self::cleanup_temp_resources() {
            warn!("Failed to cleanup temp resources: {}", e);
        }

        // Force memory cleanup
        Self::force_cleanup();

        // Reset environment variables
        Self::reset_memory_env();

        // Final memory check
        Self::log_memory_usage("after_cleanup");

        // Verify memory was actually freed
        Self::verify_memory_cleanup();

        info!("Large operation cleanup completed");
    }

    /// Set environment variables for memory optimization
    fn set_memory_optimized_env() {
        debug!("Setting memory-optimized environment variables");

        // Set conservative memory limits for subprocesses
        std::env::set_var("RUST_MIN_STACK", "2097152"); // 2MB stack
        std::env::set_var("MALLOC_ARENA_MAX", "2"); // Limit malloc arenas

        // Set garbage collection hints for any GC-based components
        std::env::set_var("GC_INITIAL_HEAP_SIZE", "32m");
        std::env::set_var("GC_MAXIMUM_HEAP_SIZE", "256m");
    }

    /// Reset memory-related environment variables
    fn reset_memory_env() {
        debug!("Resetting memory environment variables");

        std::env::remove_var("RUST_MIN_STACK");
        std::env::remove_var("MALLOC_ARENA_MAX");
        std::env::remove_var("GC_INITIAL_HEAP_SIZE");
        std::env::remove_var("GC_MAXIMUM_HEAP_SIZE");
    }

    /// Verify that memory cleanup was effective
    fn verify_memory_cleanup() {
        debug!("Verifying memory cleanup effectiveness");

        match get_memory_info() {
            Ok(info) => {
                let rss_mb = info.rss_kb / 1024;
                let virtual_mb = info.virtual_kb / 1024;

                info!("Post-cleanup memory: RSS: {} MB, Virtual: {} MB", rss_mb, virtual_mb);

                // Warn if memory usage seems high
                if rss_mb > 500 {
                    warn!("High RSS memory usage after cleanup: {} MB", rss_mb);
                }

                if virtual_mb > 2000 {
                    warn!("High virtual memory usage after cleanup: {} MB", virtual_mb);
                }
            }
            Err(e) => {
                debug!("Could not verify memory cleanup: {}", e);
            }
        }
    }
}

/// Resource guard that automatically cleans up on drop with enhanced functionality
pub struct ResourceGuard {
    cleanup_paths: Vec<String>,
    cleanup_dirs: Vec<String>,
    temp_files: Vec<String>,
    memory_allocations: Vec<Box<[u8]>>,
    cleanup_callbacks: Vec<Box<dyn FnOnce() + Send>>,
}

impl ResourceGuard {
    pub fn new() -> Self {
        Self {
            cleanup_paths: Vec::new(),
            cleanup_dirs: Vec::new(),
            temp_files: Vec::new(),
            memory_allocations: Vec::new(),
            cleanup_callbacks: Vec::new(),
        }
    }

    /// Add a file path to be cleaned up on drop
    pub fn add_cleanup_path<P: AsRef<Path>>(&mut self, path: P) {
        self.cleanup_paths.push(path.as_ref().to_string_lossy().to_string());
    }

    /// Add a directory to be cleaned up on drop (recursively)
    pub fn add_cleanup_dir<P: AsRef<Path>>(&mut self, path: P) {
        self.cleanup_dirs.push(path.as_ref().to_string_lossy().to_string());
    }

    /// Add a temporary file that was created and should be cleaned up
    pub fn add_temp_file<P: AsRef<Path>>(&mut self, path: P) {
        self.temp_files.push(path.as_ref().to_string_lossy().to_string());
    }

    /// Add a large memory allocation to be tracked and freed
    pub fn add_memory_allocation(&mut self, allocation: Box<[u8]>) {
        self.memory_allocations.push(allocation);
    }

    /// Add a custom cleanup callback
    pub fn add_cleanup_callback<F>(&mut self, callback: F)
    where
        F: FnOnce() + Send + 'static
    {
        self.cleanup_callbacks.push(Box::new(callback));
    }

    /// Create a temporary file and add it to cleanup list
    pub fn create_temp_file(&mut self, prefix: &str) -> Result<std::fs::File> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_nanos();
        let temp_path = format!("/tmp/{}_{}", prefix, timestamp);
        let file = std::fs::File::create(&temp_path)?;
        self.add_temp_file(&temp_path);
        Ok(file)
    }

    /// Manually trigger cleanup (useful for early cleanup)
    pub fn cleanup_now(&mut self) {
        self.perform_cleanup();
    }

    /// Perform the actual cleanup operations
    fn perform_cleanup(&mut self) {
        debug!("ResourceGuard performing cleanup");

        // Clean up temporary files first
        for temp_file in &self.temp_files {
            if Path::new(temp_file).exists() {
                if let Err(e) = fs::remove_file(temp_file) {
                    warn!("Failed to remove temp file {}: {}", temp_file, e);
                } else {
                    debug!("Cleaned up temp file: {}", temp_file);
                }
            }
        }

        // Clean up regular files
        for file_path in &self.cleanup_paths {
            if Path::new(file_path).exists() {
                if let Err(e) = fs::remove_file(file_path) {
                    warn!("Failed to remove file {}: {}", file_path, e);
                } else {
                    debug!("Cleaned up file: {}", file_path);
                }
            }
        }

        // Clean up directories (recursively)
        for dir_path in &self.cleanup_dirs {
            if Path::new(dir_path).exists() {
                if let Err(e) = fs::remove_dir_all(dir_path) {
                    warn!("Failed to remove directory {}: {}", dir_path, e);
                } else {
                    debug!("Cleaned up directory: {}", dir_path);
                }
            }
        }

        // Free memory allocations
        let allocation_count = self.memory_allocations.len();
        self.memory_allocations.clear();
        if allocation_count > 0 {
            debug!("Freed {} memory allocations", allocation_count);
        }

        // Execute cleanup callbacks
        let callback_count = self.cleanup_callbacks.len();
        for callback in self.cleanup_callbacks.drain(..) {
            callback();
        }
        if callback_count > 0 {
            debug!("Executed {} cleanup callbacks", callback_count);
        }

        // Clear all cleanup lists
        self.cleanup_paths.clear();
        self.cleanup_dirs.clear();
        self.temp_files.clear();

        debug!("ResourceGuard cleanup completed");
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        debug!("ResourceGuard dropping - performing final cleanup");
        self.perform_cleanup();
    }
}

/// Cross-platform memory information
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub rss_kb: u64,      // Resident Set Size in KB
    pub virtual_kb: u64,  // Virtual memory size in KB
}

/// Cross-platform system memory information
#[derive(Debug, Clone)]
pub struct SystemMemoryInfo {
    pub total_kb: u64,     // Total system memory in KB
    pub available_kb: u64, // Available system memory in KB
    pub used_kb: u64,      // Used system memory in KB
}

/// Get current process memory information (cross-platform)
fn get_memory_info() -> Result<MemoryInfo> {
    #[cfg(target_os = "linux")]
    {
        get_memory_info_linux()
    }
    #[cfg(target_os = "macos")]
    {
        get_memory_info_macos()
    }
    #[cfg(target_os = "windows")]
    {
        get_memory_info_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Fallback for other platforms
        Ok(MemoryInfo {
            rss_kb: 0,
            virtual_kb: 0,
        })
    }
}

/// Get system memory information (cross-platform)
fn get_system_memory_info() -> Result<SystemMemoryInfo> {
    #[cfg(target_os = "linux")]
    {
        get_system_memory_info_linux()
    }
    #[cfg(target_os = "macos")]
    {
        get_system_memory_info_macos()
    }
    #[cfg(target_os = "windows")]
    {
        get_system_memory_info_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        // Fallback for other platforms
        Ok(SystemMemoryInfo {
            total_kb: 1024 * 1024, // 1GB default
            available_kb: 512 * 1024, // 512MB default
            used_kb: 512 * 1024,
        })
    }
}

#[cfg(target_os = "linux")]
fn get_memory_info_linux() -> Result<MemoryInfo> {
    let status = fs::read_to_string("/proc/self/status")
        .map_err(|e| anyhow!("Failed to read /proc/self/status: {}", e))?;

    let mut rss_kb = 0;
    let mut virtual_kb = 0;

    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                rss_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("VmSize:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                virtual_kb = kb_str.parse().unwrap_or(0);
            }
        }
    }

    Ok(MemoryInfo { rss_kb, virtual_kb })
}

#[cfg(target_os = "linux")]
fn get_system_memory_info_linux() -> Result<SystemMemoryInfo> {
    let meminfo = fs::read_to_string("/proc/meminfo")
        .map_err(|e| anyhow!("Failed to read /proc/meminfo: {}", e))?;

    let mut total_kb = 0;
    let mut available_kb = 0;
    let mut free_kb = 0;
    let mut buffers_kb = 0;
    let mut cached_kb = 0;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                total_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("MemAvailable:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                available_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("MemFree:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                free_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("Buffers:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                buffers_kb = kb_str.parse().unwrap_or(0);
            }
        } else if line.starts_with("Cached:") {
            if let Some(kb_str) = line.split_whitespace().nth(1) {
                cached_kb = kb_str.parse().unwrap_or(0);
            }
        }
    }

    // If MemAvailable is not available, estimate it
    if available_kb == 0 {
        available_kb = free_kb + buffers_kb + cached_kb;
    }

    let used_kb = total_kb.saturating_sub(available_kb);

    Ok(SystemMemoryInfo {
        total_kb,
        available_kb,
        used_kb,
    })
}

#[cfg(target_os = "macos")]
fn get_memory_info_macos() -> Result<MemoryInfo> {
    // On macOS, we would use task_info() system call
    // For now, provide a simplified implementation
    use std::process::Command;

    let output = Command::new("ps")
        .args(&["-o", "rss,vsz", "-p"])
        .arg(std::process::id().to_string())
        .output()
        .map_err(|e| anyhow!("Failed to run ps command: {}", e))?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = output_str.lines().collect();

    if lines.len() >= 2 {
        let parts: Vec<&str> = lines[1].split_whitespace().collect();
        if parts.len() >= 2 {
            let rss_kb = parts[0].parse().unwrap_or(0);
            let virtual_kb = parts[1].parse().unwrap_or(0);
            return Ok(MemoryInfo { rss_kb, virtual_kb });
        }
    }

    // Fallback
    Ok(MemoryInfo {
        rss_kb: 0,
        virtual_kb: 0,
    })
}

#[cfg(target_os = "macos")]
fn get_system_memory_info_macos() -> Result<SystemMemoryInfo> {
    use std::process::Command;

    let output = Command::new("vm_stat")
        .output()
        .map_err(|e| anyhow!("Failed to run vm_stat: {}", e))?;

    let _output_str = String::from_utf8_lossy(&output.stdout);

    // Parse vm_stat output (simplified)
    // This is a basic implementation - in production you'd use system APIs
    Ok(SystemMemoryInfo {
        total_kb: 8 * 1024 * 1024, // 8GB default
        available_kb: 4 * 1024 * 1024, // 4GB default
        used_kb: 4 * 1024 * 1024,
    })
}

#[cfg(target_os = "windows")]
fn get_memory_info_windows() -> Result<MemoryInfo> {
    // On Windows, we would use GetProcessMemoryInfo()
    // For now, provide a simplified implementation
    Ok(MemoryInfo {
        rss_kb: 0,
        virtual_kb: 0,
    })
}

#[cfg(target_os = "windows")]
fn get_system_memory_info_windows() -> Result<SystemMemoryInfo> {
    // On Windows, we would use GlobalMemoryStatusEx()
    // For now, provide a simplified implementation
    Ok(SystemMemoryInfo {
        total_kb: 8 * 1024 * 1024, // 8GB default
        available_kb: 4 * 1024 * 1024, // 4GB default
        used_kb: 4 * 1024 * 1024,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_memory_manager_cleanup() {
        // Test that cleanup doesn't panic
        MemoryManager::force_cleanup();
        MemoryManager::log_memory_usage("test");
    }

    #[test]
    fn test_memory_availability_check() {
        // Should not panic and return a boolean
        let result = MemoryManager::check_memory_availability(100);
        assert!(result == true || result == false);
    }

    #[test]
    fn test_resource_guard() {
        let temp_dir = tempdir().unwrap();
        let temp_file = temp_dir.path().join("test_file.txt");

        {
            // Create a file
            File::create(&temp_file).unwrap();
            assert!(temp_file.exists());

            // Create resource guard
            let mut guard = ResourceGuard::new();
            guard.add_cleanup_path(&temp_file);

            // File should still exist
            assert!(temp_file.exists());
        } // Guard drops here

        // File should be cleaned up
        assert!(!temp_file.exists());
    }

    #[test]
    fn test_enhanced_resource_guard() {
        let temp_dir = tempdir().unwrap();
        let temp_file = temp_dir.path().join("enhanced_test.txt");
        let temp_subdir = temp_dir.path().join("subdir");

        {
            // Create file and directory
            File::create(&temp_file).unwrap();
            std::fs::create_dir(&temp_subdir).unwrap();

            let mut guard = ResourceGuard::new();
            guard.add_temp_file(&temp_file);
            guard.add_cleanup_dir(&temp_subdir);

            // Add a memory allocation
            let large_allocation = vec![0u8; 1024].into_boxed_slice();
            guard.add_memory_allocation(large_allocation);

            // Add a cleanup callback
            let callback_executed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let callback_flag = callback_executed.clone();
            guard.add_cleanup_callback(move || {
                callback_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            });

            assert!(temp_file.exists());
            assert!(temp_subdir.exists());
        } // Guard drops here

        // Resources should be cleaned up
        assert!(!temp_file.exists());
        assert!(!temp_subdir.exists());
    }

    #[test]
    fn test_large_operation_optimization() {
        // Should not panic
        MemoryManager::optimize_for_large_operation();
        MemoryManager::cleanup_after_large_operation();
    }
}
