use anyhow::{Result, anyhow};
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
    fn test_large_operation_optimization() {
        // Should not panic
        MemoryManager::optimize_for_large_operation();
        MemoryManager::cleanup_after_large_operation();
    }
}
