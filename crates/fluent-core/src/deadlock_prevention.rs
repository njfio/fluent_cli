// Deadlock prevention utilities and lock ordering enforcement
use crate::error::{FluentError, LockTimeoutConfig};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

/// Lock ordering registry to prevent deadlocks through consistent lock acquisition order
#[derive(Debug)]
pub struct LockOrderRegistry {
    /// Maps lock names to their ordering priority (lower numbers acquired first)
    lock_priorities: HashMap<String, u64>,
    /// Counter for assigning unique priorities
    next_priority: AtomicU64,
}

impl LockOrderRegistry {
    /// Create a new lock order registry
    pub fn new() -> Self {
        Self {
            lock_priorities: HashMap::new(),
            next_priority: AtomicU64::new(1),
        }
    }

    /// Register a lock with a specific priority (lower numbers acquired first)
    pub fn register_lock(&mut self, lock_name: &str, priority: u64) {
        self.lock_priorities.insert(lock_name.to_string(), priority);
    }

    /// Register a lock with auto-assigned priority
    pub fn register_lock_auto(&mut self, lock_name: &str) -> u64 {
        let priority = self.next_priority.fetch_add(1, Ordering::Relaxed);
        self.lock_priorities.insert(lock_name.to_string(), priority);
        priority
    }

    /// Get the priority of a lock
    pub fn get_priority(&self, lock_name: &str) -> Option<u64> {
        self.lock_priorities.get(lock_name).copied()
    }

    /// Validate that locks are being acquired in the correct order
    pub fn validate_lock_order(&self, current_locks: &[String], new_lock: &str) -> Result<(), FluentError> {
        let new_priority = self.get_priority(new_lock)
            .ok_or_else(|| FluentError::Internal(format!("Unregistered lock: {}", new_lock)))?;

        for current_lock in current_locks {
            let current_priority = self.get_priority(current_lock)
                .ok_or_else(|| FluentError::Internal(format!("Unregistered lock: {}", current_lock)))?;
            
            if new_priority <= current_priority {
                return Err(FluentError::Internal(format!(
                    "Lock ordering violation: attempting to acquire '{}' (priority {}) while holding '{}' (priority {}). Locks must be acquired in ascending priority order.",
                    new_lock, new_priority, current_lock, current_priority
                )));
            }
        }

        Ok(())
    }
}

impl Default for LockOrderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Deadlock-safe lock manager that enforces lock ordering
pub struct DeadlockSafeLockManager {
    registry: Arc<RwLock<LockOrderRegistry>>,
    timeout_config: LockTimeoutConfig,
}

impl DeadlockSafeLockManager {
    /// Create a new deadlock-safe lock manager
    pub fn new(timeout_config: LockTimeoutConfig) -> Self {
        Self {
            registry: Arc::new(RwLock::new(LockOrderRegistry::new())),
            timeout_config,
        }
    }

    /// Register locks with their priorities
    pub async fn register_locks(&self, locks: Vec<(&str, u64)>) -> Result<(), FluentError> {
        let mut registry = self.registry.write().await;
        for (lock_name, priority) in locks {
            registry.register_lock(lock_name, priority);
        }
        Ok(())
    }

    /// Acquire multiple locks in the correct order to prevent deadlocks
    pub async fn acquire_locks_ordered<'a, T>(
        &self,
        locks: Vec<(&str, &'a Arc<Mutex<T>>)>,
    ) -> Result<Vec<tokio::sync::MutexGuard<'a, T>>, FluentError> {
        // Sort locks by priority to ensure consistent ordering
        let registry = self.registry.read().await;
        let mut lock_info: Vec<_> = locks.into_iter().collect();
        
        lock_info.sort_by_key(|(name, _)| {
            registry.get_priority(name).unwrap_or(u64::MAX)
        });

        // Acquire locks in order
        let mut guards = Vec::new();
        for (lock_name, mutex) in lock_info {
            let start_time = Instant::now();
            
            match tokio::time::timeout(self.timeout_config.timeout, mutex.lock()).await {
                Ok(guard) => {
                    if self.timeout_config.log_timeout_events {
                        let elapsed = start_time.elapsed();
                        if elapsed > Duration::from_millis(100) {
                            eprintln!(
                                "🔒 Acquired lock '{}' after {:?} (ordered acquisition)",
                                lock_name, elapsed
                            );
                        }
                    }
                    guards.push(guard);
                }
                Err(_) => {
                    let elapsed = start_time.elapsed();
                    return Err(FluentError::LockTimeout(format!(
                        "Timeout acquiring lock '{}' after {:?} during ordered acquisition",
                        lock_name, elapsed
                    )));
                }
            }
        }

        Ok(guards)
    }

    /// Acquire multiple RwLocks for reading in the correct order
    pub async fn acquire_read_locks_ordered<'a, T>(
        &self,
        locks: Vec<(&str, &'a Arc<RwLock<T>>)>,
    ) -> Result<Vec<tokio::sync::RwLockReadGuard<'a, T>>, FluentError> {
        // Sort locks by priority
        let registry = self.registry.read().await;
        let mut lock_info: Vec<_> = locks.into_iter().collect();
        
        lock_info.sort_by_key(|(name, _)| {
            registry.get_priority(name).unwrap_or(u64::MAX)
        });

        // Acquire read locks in order
        let mut guards = Vec::new();
        for (lock_name, rwlock) in lock_info {
            let start_time = Instant::now();
            
            match tokio::time::timeout(self.timeout_config.timeout, rwlock.read()).await {
                Ok(guard) => {
                    if self.timeout_config.log_timeout_events {
                        let elapsed = start_time.elapsed();
                        if elapsed > Duration::from_millis(100) {
                            eprintln!(
                                "🔒 Acquired read lock '{}' after {:?} (ordered acquisition)",
                                lock_name, elapsed
                            );
                        }
                    }
                    guards.push(guard);
                }
                Err(_) => {
                    let elapsed = start_time.elapsed();
                    return Err(FluentError::LockTimeout(format!(
                        "Timeout acquiring read lock '{}' after {:?} during ordered acquisition",
                        lock_name, elapsed
                    )));
                }
            }
        }

        Ok(guards)
    }
}

/// Utility for safe lock acquisition with deadlock prevention
pub struct DeadlockPreventionUtils;

impl DeadlockPreventionUtils {
    /// Execute an operation that requires multiple locks in a deadlock-safe manner
    pub async fn execute_with_ordered_locks<'a, T, R, F>(
        lock_manager: &DeadlockSafeLockManager,
        locks: Vec<(&str, &'a Arc<Mutex<T>>)>,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: FnOnce(&mut [&mut T]) -> Result<R, FluentError>,
    {
        let mut guards = lock_manager.acquire_locks_ordered(locks).await?;
        
        // Convert guards to mutable references
        let mut refs: Vec<&mut T> = guards.iter_mut().map(|g| &mut **g).collect();
        
        operation(&mut refs)
    }

    /// Execute a read operation that requires multiple RwLocks in a deadlock-safe manner
    pub async fn execute_with_ordered_read_locks<'a, T, R, F>(
        lock_manager: &DeadlockSafeLockManager,
        locks: Vec<(&str, &'a Arc<RwLock<T>>)>,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: FnOnce(&[&T]) -> Result<R, FluentError>,
    {
        let guards = lock_manager.acquire_read_locks_ordered(locks).await?;
        
        // Convert guards to references
        let refs: Vec<&T> = guards.iter().map(|g| &**g).collect();
        
        operation(&refs)
    }

    /// Create a standard lock ordering for common patterns
    pub fn create_standard_lock_ordering() -> HashMap<String, u64> {
        let mut ordering = HashMap::new();
        
        // Security-related locks (highest priority - acquired first)
        ordering.insert("security_policies".to_string(), 10);
        ordering.insert("active_sessions".to_string(), 20);
        ordering.insert("rate_limiters".to_string(), 30);
        
        // State management locks
        ordering.insert("agent_state".to_string(), 100);
        ordering.insert("execution_context".to_string(), 110);
        ordering.insert("state_history".to_string(), 120);
        
        // Engine and processing locks
        ordering.insert("reflection_engine".to_string(), 200);
        ordering.insert("reasoning_engine".to_string(), 210);
        ordering.insert("action_planner".to_string(), 220);
        
        // Metrics and monitoring locks (lowest priority - acquired last)
        ordering.insert("orchestration_metrics".to_string(), 900);
        ordering.insert("performance_metrics".to_string(), 910);
        ordering.insert("cache_metrics".to_string(), 920);
        
        // MCP client locks
        ordering.insert("mcp_response_handlers".to_string(), 300);
        ordering.insert("mcp_tools".to_string(), 310);
        ordering.insert("mcp_resources".to_string(), 320);
        ordering.insert("mcp_stdin".to_string(), 330);
        
        ordering
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_lock_order_registry() {
        let mut registry = LockOrderRegistry::new();
        registry.register_lock("lock_a", 10);
        registry.register_lock("lock_b", 20);

        // Valid order: acquiring lock_b while holding lock_a
        assert!(registry.validate_lock_order(&["lock_a".to_string()], "lock_b").is_ok());

        // Invalid order: acquiring lock_a while holding lock_b
        assert!(registry.validate_lock_order(&["lock_b".to_string()], "lock_a").is_err());
    }

    #[tokio::test]
    async fn test_deadlock_safe_lock_manager() {
        let config = LockTimeoutConfig::short_timeout();
        let manager = DeadlockSafeLockManager::new(config);

        // Register locks
        manager.register_locks(vec![
            ("lock_a", 10),
            ("lock_b", 20),
        ]).await.unwrap();

        // Test ordered acquisition
        let mutex_a = Arc::new(Mutex::new(1));
        let mutex_b = Arc::new(Mutex::new(2));

        let guards = manager.acquire_locks_ordered(vec![
            ("lock_b", &mutex_b), // Higher priority, but should be reordered
            ("lock_a", &mutex_a), // Lower priority
        ]).await.unwrap();

        assert_eq!(guards.len(), 2);
        // Verify the locks were acquired in the correct order
        assert_eq!(*guards[0], 1); // lock_a (priority 10)
        assert_eq!(*guards[1], 2); // lock_b (priority 20)
    }
}
