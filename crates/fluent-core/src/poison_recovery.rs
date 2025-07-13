// Mutex poison recovery utilities
use crate::error::{FluentError, PoisonHandlingConfig, PoisonRecoveryStrategy};
use log::warn;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Utility functions for common mutex poison recovery patterns
pub struct PoisonRecoveryUtils;

impl PoisonRecoveryUtils {
    /// Safely execute an operation on a mutex with automatic poison recovery
    pub fn safe_execute<T, R, F>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: FnOnce(&mut T) -> Result<R, FluentError>,
    {
        let config = PoisonHandlingConfig::recover_data();
        Self::safe_execute_with_config(mutex, context, &config, operation)
    }

    /// Safely execute an operation on a mutex with configurable poison handling
    pub fn safe_execute_with_config<T, R, F>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
        config: &PoisonHandlingConfig,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: FnOnce(&mut T) -> Result<R, FluentError>,
    {
        match mutex.lock() {
            Ok(mut guard) => operation(&mut *guard),
            Err(poison_error) => {
                if config.log_poison_events {
                    eprintln!(
                        "⚠️  Mutex poisoned in {}: {}. Attempting recovery with strategy: {:?}",
                        context,
                        poison_error,
                        config.strategy
                    );
                }

                match config.strategy {
                    PoisonRecoveryStrategy::RecoverData => {
                        let mut guard = poison_error.into_inner();
                        operation(&mut *guard)
                    }
                    _ => Err(FluentError::Internal(format!(
                        "Mutex poisoned in {} and recovery strategy not supported: {}",
                        context, poison_error
                    ))),
                }
            }
        }
    }

    /// Safely read from a mutex with poison recovery
    pub fn safe_read<T: Clone>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
    ) -> Result<T, FluentError> {
        Self::safe_execute(mutex, context, |data| Ok(data.clone()))
    }

    /// Safely read from a mutex with default fallback
    pub fn safe_read_or_default<T: Clone + Default>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
    ) -> T {
        Self::safe_read(mutex, context).unwrap_or_default()
    }

    /// Safely modify a mutex value with poison recovery
    pub fn safe_modify<T, F>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
        modifier: F,
    ) -> Result<(), FluentError>
    where
        F: FnOnce(&mut T),
    {
        Self::safe_execute(mutex, context, |data| {
            modifier(data);
            Ok(())
        })
    }

    /// Safely replace a mutex value with poison recovery
    pub fn safe_replace<T>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
        new_value: T,
    ) -> Result<T, FluentError> {
        Self::safe_execute(mutex, context, |data| {
            Ok(std::mem::replace(data, new_value))
        })
    }

    /// Execute operation with retry on poison
    pub fn execute_with_retry<T, R, F>(
        mutex: &Arc<Mutex<T>>,
        context: &str,
        max_retries: u32,
        retry_delay: Duration,
        operation: F,
    ) -> Result<R, FluentError>
    where
        F: Fn(&mut T) -> Result<R, FluentError>,
    {
        let mut attempts = 0;
        loop {
            match mutex.lock() {
                Ok(mut guard) => {
                    return operation(&mut *guard);
                }
                Err(poison_error) => {
                    attempts += 1;
                    warn!(
                        "⚠️  Mutex poisoned in {} (attempt {}/{}): {}",
                        context,
                        attempts,
                        max_retries + 1,
                        poison_error
                    );

                    if attempts <= max_retries {
                        std::thread::sleep(retry_delay);
                        continue;
                    } else {
                        return Err(FluentError::Internal(format!(
                            "Mutex poisoned in {} after {} attempts: {}",
                            context, attempts, poison_error
                        )));
                    }
                }
            }
        }
    }

    /// Create a poison-resistant wrapper around a mutex
    pub fn create_poison_resistant<T>(value: T) -> PoisonResistantMutex<T> {
        PoisonResistantMutex::new(value)
    }
}

/// A wrapper around Arc<Mutex<T>> that provides poison-resistant operations
pub struct PoisonResistantMutex<T> {
    inner: Arc<Mutex<T>>,
    config: PoisonHandlingConfig,
}

impl<T> PoisonResistantMutex<T> {
    /// Create a new poison-resistant mutex
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            config: PoisonHandlingConfig::recover_data(),
        }
    }

    /// Create a new poison-resistant mutex with custom config
    pub fn with_config(value: T, config: PoisonHandlingConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            config,
        }
    }

    /// Execute an operation on the mutex data
    pub fn execute<R, F>(&self, context: &str, operation: F) -> Result<R, FluentError>
    where
        F: FnOnce(&mut T) -> Result<R, FluentError>,
    {
        PoisonRecoveryUtils::safe_execute_with_config(&self.inner, context, &self.config, operation)
    }

    /// Read the mutex data
    pub fn read(&self, context: &str) -> Result<T, FluentError>
    where
        T: Clone,
    {
        self.execute(context, |data| Ok(data.clone()))
    }

    /// Modify the mutex data
    pub fn modify<F>(&self, context: &str, modifier: F) -> Result<(), FluentError>
    where
        F: FnOnce(&mut T),
    {
        self.execute(context, |data| {
            modifier(data);
            Ok(())
        })
    }

    /// Replace the mutex data
    pub fn replace(&self, context: &str, new_value: T) -> Result<T, FluentError> {
        self.execute(context, |data| Ok(std::mem::replace(data, new_value)))
    }

    /// Get a clone of the inner Arc<Mutex<T>> for sharing
    pub fn clone_inner(&self) -> Arc<Mutex<T>> {
        self.inner.clone()
    }
}

impl<T> Clone for PoisonResistantMutex<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_poison_recovery_utils_safe_read() {
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let result = PoisonRecoveryUtils::safe_read(&mutex, "test_read");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_poison_recovery_utils_safe_modify() {
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let result = PoisonRecoveryUtils::safe_modify(&mutex, "test_modify", |data| {
            data.push(4);
        });
        assert!(result.is_ok());

        let data = PoisonRecoveryUtils::safe_read(&mutex, "test_read").unwrap();
        assert_eq!(data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_poison_resistant_mutex() {
        let mutex = PoisonResistantMutex::new(vec![1, 2, 3]);
        
        // Test read
        let result = mutex.read("test_read");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);

        // Test modify
        let result = mutex.modify("test_modify", |data| data.push(4));
        assert!(result.is_ok());

        // Verify modification
        let result = mutex.read("test_read_after_modify");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_poison_recovery_from_panic() {
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let mutex_clone = mutex.clone();

        // Create a poisoned mutex
        let handle = thread::spawn(move || {
            let mut guard = mutex_clone.lock().unwrap();
            guard.push(4);
            panic!("Poison the mutex");
        });
        let _ = handle.join();

        // Should still be able to read with recovery
        let result = PoisonRecoveryUtils::safe_read(&mutex, "test_poison_recovery");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_poison_resistant_mutex_recovery() {
        let mutex = PoisonResistantMutex::new(vec![1, 2, 3]);
        let inner_clone = mutex.clone_inner();

        // Create a poisoned mutex
        let handle = thread::spawn(move || {
            let mut guard = inner_clone.lock().unwrap();
            guard.push(4);
            panic!("Poison the mutex");
        });
        let _ = handle.join();

        // Should still be able to read with recovery
        let result = mutex.read("test_poison_resistant_recovery");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);
    }
}
