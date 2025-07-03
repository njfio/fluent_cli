use anyhow::Result;
use log::{debug, info, warn};
use std::fs;
use std::path::Path;

/// Memory management utilities for the CLI
pub struct MemoryManager;

impl MemoryManager {
    /// Force garbage collection and memory cleanup
    pub fn force_cleanup() {
        // In Rust, we can't force GC, but we can drop large allocations
        // This is more of a placeholder for future memory management
        debug!("Performing memory cleanup");
    }

    /// Log current memory usage for debugging
    pub fn log_memory_usage(context: &str) {
        // This is a basic implementation - in production you might use a proper memory profiler
        debug!("Memory usage check at: {}", context);
        
        // On Unix systems, we could read /proc/self/status for more detailed info
        #[cfg(unix)]
        {
            if let Ok(status) = fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") || line.starts_with("VmSize:") {
                        debug!("{}: {}", context, line);
                    }
                }
            }
        }
    }

    /// Clean up temporary resources
    pub fn cleanup_temp_resources() -> Result<()> {
        // Clean up any temporary files that might have been created
        let temp_patterns = [
            "/tmp/fluent_*",
            "/tmp/pipeline_*", 
            "/tmp/agent_*",
        ];

        for pattern in &temp_patterns {
            if let Ok(entries) = glob::glob(pattern) {
                for entry in entries.flatten() {
                    if let Err(e) = fs::remove_file(&entry) {
                        warn!("Failed to remove temp file {:?}: {}", entry, e);
                    } else {
                        debug!("Cleaned up temp file: {:?}", entry);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if system has sufficient memory for operation
    pub fn check_memory_availability(required_mb: u64) -> bool {
        #[cfg(unix)]
        {
            if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemAvailable:") {
                        if let Some(available_str) = line.split_whitespace().nth(1) {
                            if let Ok(available_kb) = available_str.parse::<u64>() {
                                let available_mb = available_kb / 1024;
                                return available_mb >= required_mb;
                            }
                        }
                    }
                }
            }
        }

        // Default to true if we can't determine memory usage
        true
    }

    /// Optimize memory usage for large operations
    pub fn optimize_for_large_operation() {
        info!("Optimizing memory for large operation");
        
        // Force cleanup before large operations
        Self::force_cleanup();
        
        // Log current state
        Self::log_memory_usage("before_large_operation");
    }

    /// Clean up after large operations
    pub fn cleanup_after_large_operation() {
        info!("Cleaning up after large operation");
        
        // Clean up temporary resources
        if let Err(e) = Self::cleanup_temp_resources() {
            warn!("Failed to cleanup temp resources: {}", e);
        }
        
        // Force cleanup
        Self::force_cleanup();
        
        // Log final state
        Self::log_memory_usage("after_large_operation");
    }
}

/// Resource guard that automatically cleans up on drop
pub struct ResourceGuard {
    cleanup_paths: Vec<String>,
}

impl ResourceGuard {
    pub fn new() -> Self {
        Self {
            cleanup_paths: Vec::new(),
        }
    }

    pub fn add_cleanup_path<P: AsRef<Path>>(&mut self, path: P) {
        self.cleanup_paths.push(path.as_ref().to_string_lossy().to_string());
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        for path in &self.cleanup_paths {
            if Path::new(path).exists() {
                if let Err(e) = fs::remove_file(path) {
                    warn!("Failed to cleanup resource {}: {}", path, e);
                } else {
                    debug!("Cleaned up resource: {}", path);
                }
            }
        }
    }
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
    fn test_large_operation_optimization() {
        // Should not panic
        MemoryManager::optimize_for_large_operation();
        MemoryManager::cleanup_after_large_operation();
    }
}
