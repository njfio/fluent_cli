// Thread-safe error handling example
use fluent_core::error::{ErrorContext, FluentError, ThreadSafeErrorHandler};
use fluent_core::{safe_lock, safe_read_lock, safe_write_lock};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), FluentError> {
    println!("🔒 Thread-Safe Error Handling Demo");

    // Example 1: Safe mutex locking
    let shared_data = Arc::new(Mutex::new(vec![1, 2, 3, 4, 5]));
    let shared_data_clone = shared_data.clone();

    let handle = thread::spawn(move || -> Result<(), FluentError> {
        // Using the safe_lock! macro
        let mut data = safe_lock!(shared_data_clone, "shared_data_modification")?;
        data.push(6);
        println!("✅ Thread safely modified shared data: {:?}", *data);
        Ok(())
    });

    // Wait for thread to complete
    handle.join().unwrap()?;

    // Example 2: Safe RwLock usage
    let shared_config = Arc::new(RwLock::new("initial_config".to_string()));
    let config_clone = shared_config.clone();

    let reader_handle = thread::spawn(move || -> Result<(), FluentError> {
        // Using the safe_read_lock! macro
        let config = safe_read_lock!(config_clone, "config_reading")?;
        println!("✅ Thread safely read config: {}", *config);
        Ok(())
    });

    reader_handle.join().unwrap()?;

    // Example 3: Error context demonstration
    let context = ErrorContext::new()
        .with_operation("database_transaction")
        .with_operation("user_authentication")
        .with_metadata("user_id", "12345")
        .with_metadata("session_id", "abc-def-ghi");

    let error = FluentError::Internal("Database connection failed".to_string());
    let contextual_error = ThreadSafeErrorHandler::create_error_with_context(error, context);

    println!("🔍 Error with context: {}", contextual_error);

    // Example 4: Manual mutex handling with proper error handling
    let manual_mutex = Arc::new(Mutex::new(42));
    match ThreadSafeErrorHandler::handle_mutex_lock(manual_mutex.lock(), "manual_lock_example") {
        Ok(guard) => println!("✅ Manual mutex lock successful: {}", *guard),
        Err(e) => println!("❌ Manual mutex lock failed: {}", e),
    }

    println!("🎉 Thread-safe error handling demo completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new()
            .with_operation("test_operation")
            .with_metadata("test_key", "test_value");

        assert!(context.operation_stack.contains(&"test_operation".to_string()));
        assert_eq!(context.metadata.get("test_key"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_thread_safe_error_handler() {
        let mutex = Arc::new(Mutex::new(100));
        let result = ThreadSafeErrorHandler::handle_mutex_lock(mutex.lock(), "test_context");
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), 100);
    }

    #[test]
    fn test_safe_lock_macro() {
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let result = safe_lock!(mutex, "test_macro");
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), vec![1, 2, 3]);
    }
}
