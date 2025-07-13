// Lock timeout utilities and monitoring
use crate::error::{FluentError, LockTimeoutConfig};
use log::warn;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

/// Lock contention monitor for tracking lock performance
#[derive(Debug)]
pub struct LockContentionMonitor {
    /// Total number of lock acquisitions
    total_acquisitions: AtomicU64,
    /// Total number of timeouts
    total_timeouts: AtomicU64,
    /// Total time spent waiting for locks (in milliseconds)
    total_wait_time_ms: AtomicU64,
    /// Current number of waiters
    current_waiters: AtomicU32,
    /// Maximum number of concurrent waiters seen
    max_concurrent_waiters: AtomicU32,
    /// Number of contention warnings issued
    contention_warnings: AtomicU64,
}

impl LockContentionMonitor {
    /// Create a new lock contention monitor
    pub fn new() -> Self {
        Self {
            total_acquisitions: AtomicU64::new(0),
            total_timeouts: AtomicU64::new(0),
            total_wait_time_ms: AtomicU64::new(0),
            current_waiters: AtomicU32::new(0),
            max_concurrent_waiters: AtomicU32::new(0),
            contention_warnings: AtomicU64::new(0),
        }
    }

    /// Record the start of a lock acquisition attempt
    pub fn record_acquisition_start(&self) -> u32 {
        let current = self.current_waiters.fetch_add(1, Ordering::Relaxed) + 1;
        
        // Update max concurrent waiters
        let mut max = self.max_concurrent_waiters.load(Ordering::Relaxed);
        while current > max {
            match self.max_concurrent_waiters.compare_exchange_weak(
                max, current, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }
        
        current
    }

    /// Record the completion of a lock acquisition attempt
    pub fn record_acquisition_complete(&self, start_time: Instant, success: bool) {
        self.current_waiters.fetch_sub(1, Ordering::Relaxed);
        self.total_acquisitions.fetch_add(1, Ordering::Relaxed);
        
        let wait_time = start_time.elapsed();
        self.total_wait_time_ms.fetch_add(
            wait_time.as_millis() as u64, 
            Ordering::Relaxed
        );
        
        if !success {
            self.total_timeouts.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a contention warning
    pub fn record_contention_warning(&self) {
        self.contention_warnings.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current statistics
    pub fn get_stats(&self) -> LockContentionStats {
        let total_acquisitions = self.total_acquisitions.load(Ordering::Relaxed);
        let total_timeouts = self.total_timeouts.load(Ordering::Relaxed);
        let total_wait_time_ms = self.total_wait_time_ms.load(Ordering::Relaxed);
        
        LockContentionStats {
            total_acquisitions,
            total_timeouts,
            timeout_rate: if total_acquisitions > 0 {
                (total_timeouts as f64 / total_acquisitions as f64) * 100.0
            } else {
                0.0
            },
            average_wait_time_ms: if total_acquisitions > 0 {
                total_wait_time_ms as f64 / total_acquisitions as f64
            } else {
                0.0
            },
            current_waiters: self.current_waiters.load(Ordering::Relaxed),
            max_concurrent_waiters: self.max_concurrent_waiters.load(Ordering::Relaxed),
            contention_warnings: self.contention_warnings.load(Ordering::Relaxed),
        }
    }

    /// Reset all statistics
    pub fn reset_stats(&self) {
        self.total_acquisitions.store(0, Ordering::Relaxed);
        self.total_timeouts.store(0, Ordering::Relaxed);
        self.total_wait_time_ms.store(0, Ordering::Relaxed);
        self.current_waiters.store(0, Ordering::Relaxed);
        self.max_concurrent_waiters.store(0, Ordering::Relaxed);
        self.contention_warnings.store(0, Ordering::Relaxed);
    }
}

impl Default for LockContentionMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about lock contention
#[derive(Debug, Clone)]
pub struct LockContentionStats {
    /// Total number of lock acquisitions attempted
    pub total_acquisitions: u64,
    /// Total number of timeouts
    pub total_timeouts: u64,
    /// Timeout rate as a percentage
    pub timeout_rate: f64,
    /// Average wait time in milliseconds
    pub average_wait_time_ms: f64,
    /// Current number of waiters
    pub current_waiters: u32,
    /// Maximum number of concurrent waiters seen
    pub max_concurrent_waiters: u32,
    /// Number of contention warnings issued
    pub contention_warnings: u64,
}

/// Enhanced lock timeout utilities with monitoring
pub struct LockTimeoutUtils;

impl LockTimeoutUtils {
    /// Execute an operation on a tokio mutex with timeout and monitoring
    pub async fn execute_with_timeout_and_monitoring<T, R, F>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
        config: &LockTimeoutConfig,
        monitor: Option<&LockContentionMonitor>,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: FnOnce(&mut T) -> Result<R, FluentError>,
    {
        let start_time = Instant::now();
        let current_waiters = if let Some(mon) = monitor {
            Some(mon.record_acquisition_start())
        } else {
            None
        };

        // Check for contention warning
        if let (Some(mon), Some(waiters)) = (monitor, current_waiters) {
            if waiters > config.max_waiters_warning_threshold {
                mon.record_contention_warning();
                if config.log_timeout_events {
                    eprintln!(
                        "⚠️  High lock contention detected in {}: {} waiters (threshold: {})",
                        context, waiters, config.max_waiters_warning_threshold
                    );
                }
            }
        }

        // Attempt to acquire lock with timeout
        let result = match tokio::time::timeout(config.timeout, mutex.lock()).await {
            Ok(mut guard) => {
                let result = operation(&mut *guard);
                if let Some(mon) = monitor {
                    mon.record_acquisition_complete(start_time, true);
                }
                result
            }
            Err(_) => {
                if let Some(mon) = monitor {
                    mon.record_acquisition_complete(start_time, false);
                }
                
                let elapsed = start_time.elapsed();
                if config.log_timeout_events {
                    warn!(
                        "⏰ Mutex lock timeout in {} after {:?} (timeout: {:?})",
                        context, elapsed, config.timeout
                    );
                }
                
                Err(FluentError::LockTimeout(format!(
                    "Mutex lock timeout in {} after {:?} (timeout: {:?})",
                    context, elapsed, config.timeout
                )))
            }
        };

        result
    }

    /// Execute a read operation on a tokio RwLock with timeout and monitoring
    pub async fn execute_read_with_timeout_and_monitoring<T, R, F>(
        rwlock: &Arc<RwLock<T>>,
        context: &str,
        config: &LockTimeoutConfig,
        monitor: Option<&LockContentionMonitor>,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: FnOnce(&T) -> Result<R, FluentError>,
    {
        let start_time = Instant::now();
        let current_waiters = if let Some(mon) = monitor {
            Some(mon.record_acquisition_start())
        } else {
            None
        };

        // Check for contention warning
        if let (Some(mon), Some(waiters)) = (monitor, current_waiters) {
            if waiters > config.max_waiters_warning_threshold {
                mon.record_contention_warning();
                if config.log_timeout_events {
                    eprintln!(
                        "⚠️  High RwLock read contention detected in {}: {} waiters (threshold: {})",
                        context, waiters, config.max_waiters_warning_threshold
                    );
                }
            }
        }

        // Attempt to acquire read lock with timeout
        let result = match tokio::time::timeout(config.timeout, rwlock.read()).await {
            Ok(guard) => {
                let result = operation(&*guard);
                if let Some(mon) = monitor {
                    mon.record_acquisition_complete(start_time, true);
                }
                result
            }
            Err(_) => {
                if let Some(mon) = monitor {
                    mon.record_acquisition_complete(start_time, false);
                }
                
                let elapsed = start_time.elapsed();
                if config.log_timeout_events {
                    eprintln!(
                        "⏰ RwLock read lock timeout in {} after {:?} (timeout: {:?})",
                        context, elapsed, config.timeout
                    );
                }
                
                Err(FluentError::LockTimeout(format!(
                    "RwLock read lock timeout in {} after {:?} (timeout: {:?})",
                    context, elapsed, config.timeout
                )))
            }
        };

        result
    }

    /// Execute a write operation on a tokio RwLock with timeout and monitoring
    pub async fn execute_write_with_timeout_and_monitoring<T, R, F>(
        rwlock: &Arc<RwLock<T>>,
        context: &str,
        config: &LockTimeoutConfig,
        monitor: Option<&LockContentionMonitor>,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: FnOnce(&mut T) -> Result<R, FluentError>,
    {
        let start_time = Instant::now();
        let current_waiters = if let Some(mon) = monitor {
            Some(mon.record_acquisition_start())
        } else {
            None
        };

        // Check for contention warning
        if let (Some(mon), Some(waiters)) = (monitor, current_waiters) {
            if waiters > config.max_waiters_warning_threshold {
                mon.record_contention_warning();
                if config.log_timeout_events {
                    eprintln!(
                        "⚠️  High RwLock write contention detected in {}: {} waiters (threshold: {})",
                        context, waiters, config.max_waiters_warning_threshold
                    );
                }
            }
        }

        // Attempt to acquire write lock with timeout
        let result = match tokio::time::timeout(config.timeout, rwlock.write()).await {
            Ok(mut guard) => {
                let result = operation(&mut *guard);
                if let Some(mon) = monitor {
                    mon.record_acquisition_complete(start_time, true);
                }
                result
            }
            Err(_) => {
                if let Some(mon) = monitor {
                    mon.record_acquisition_complete(start_time, false);
                }
                
                let elapsed = start_time.elapsed();
                if config.log_timeout_events {
                    eprintln!(
                        "⏰ RwLock write lock timeout in {} after {:?} (timeout: {:?})",
                        context, elapsed, config.timeout
                    );
                }
                
                Err(FluentError::LockTimeout(format!(
                    "RwLock write lock timeout in {} after {:?} (timeout: {:?})",
                    context, elapsed, config.timeout
                )))
            }
        };

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_lock_contention_monitor() {
        let monitor = LockContentionMonitor::new();
        
        // Test initial state
        let stats = monitor.get_stats();
        assert_eq!(stats.total_acquisitions, 0);
        assert_eq!(stats.total_timeouts, 0);
        assert_eq!(stats.current_waiters, 0);

        // Test recording acquisitions
        let start_time = Instant::now();
        let waiters = monitor.record_acquisition_start();
        assert_eq!(waiters, 1);
        
        std::thread::sleep(Duration::from_millis(10));
        monitor.record_acquisition_complete(start_time, true);
        
        let stats = monitor.get_stats();
        assert_eq!(stats.total_acquisitions, 1);
        assert_eq!(stats.total_timeouts, 0);
        assert_eq!(stats.current_waiters, 0);
        assert!(stats.average_wait_time_ms >= 10.0);
    }

    #[tokio::test]
    async fn test_lock_timeout_utils() {
        let mutex = Arc::new(Mutex::new(42));
        let config = LockTimeoutConfig::short_timeout();
        let monitor = LockContentionMonitor::new();

        let result = LockTimeoutUtils::execute_with_timeout_and_monitoring(
            &mutex,
            "test_operation",
            &config,
            Some(&monitor),
            |data| {
                *data += 1;
                Ok(*data)
            },
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 43);

        let stats = monitor.get_stats();
        assert_eq!(stats.total_acquisitions, 1);
        assert_eq!(stats.total_timeouts, 0);
    }
}
