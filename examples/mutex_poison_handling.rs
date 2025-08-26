// Comprehensive mutex poison handling example
use fluent_core::error::{FluentError, PoisonHandlingConfig};
use fluent_core::{safe_lock, safe_lock_with_config, safe_lock_with_default, safe_lock_with_retry, poison_resistant_operation};
use std::sync::{Arc, Mutex};
use std::thread;


fn main() -> Result<(), FluentError> {
    println!("🧪 Comprehensive Mutex Poison Handling Demo");

    // Example 1: Basic poison handling (fail-fast)
    demonstrate_fail_fast_handling()?;

    // Example 2: Data recovery from poisoned mutex
    demonstrate_data_recovery()?;

    // Example 3: Retry with delay strategy
    demonstrate_retry_strategy()?;

    // Example 4: Default value fallback
    demonstrate_default_fallback()?;

    // Example 5: Poison-resistant operations
    demonstrate_poison_resistant_operations()?;

    println!("🎉 Mutex poison handling demo completed successfully!");
    Ok(())
}

/// Demonstrates fail-fast poison handling (default behavior)
fn demonstrate_fail_fast_handling() -> Result<(), FluentError> {
    println!("\n🚨 Example 1: Fail-Fast Poison Handling");

    let shared_data = Arc::new(Mutex::new(vec![1, 2, 3]));
    let data_clone = shared_data.clone();

    // Spawn a thread that will panic and poison the mutex
    let handle = thread::spawn(move || {
        let _guard = data_clone.lock().unwrap();
        panic!("Intentional panic to poison mutex!");
    });

    // Wait for the thread to panic
    let _ = handle.join();

    // Now try to access the poisoned mutex with fail-fast strategy
    match safe_lock!(shared_data, "fail_fast_example") {
        Ok(_) => println!("❌ Unexpected success - mutex should be poisoned"),
        Err(e) => println!("✅ Correctly detected poisoned mutex: {}", e),
    }

    Ok(())
}

/// Demonstrates data recovery from poisoned mutex
fn demonstrate_data_recovery() -> Result<(), FluentError> {
    println!("\n🔧 Example 2: Data Recovery from Poisoned Mutex");

    let shared_data = Arc::new(Mutex::new(vec![1, 2, 3, 4, 5]));
    let data_clone = shared_data.clone();

    // Spawn a thread that will panic and poison the mutex
    let handle = thread::spawn(move || {
        let mut guard = data_clone.lock().unwrap();
        guard.push(6); // Add data before panicking
        panic!("Intentional panic to poison mutex!");
    });

    // Wait for the thread to panic
    let _ = handle.join();

    // Try to recover data from the poisoned mutex
    let config = PoisonHandlingConfig::recover_data();
    match safe_lock_with_config!(shared_data, "data_recovery_example", &config) {
        Ok(guard) => {
            println!("✅ Successfully recovered data from poisoned mutex: {:?}", *guard);
            println!("   Data includes the value added before panic: {}", guard.contains(&6));
        }
        Err(e) => println!("❌ Failed to recover data: {}", e),
    }

    Ok(())
}

/// Demonstrates retry strategy with delay
fn demonstrate_retry_strategy() -> Result<(), FluentError> {
    println!("\n🔄 Example 3: Retry Strategy with Delay");

    let shared_counter = Arc::new(Mutex::new(0));
    let config = PoisonHandlingConfig::retry_with_delay(3, 50);

    // Simulate a retry operation
    let result = safe_lock_with_retry!(
        &shared_counter,
        "retry_example",
        &config,
        |counter| {
            *counter += 1;
            println!("✅ Successfully incremented counter to: {}", *counter);
            Ok(())
        }
    );

    match result {
        Ok(()) => println!("✅ Retry operation completed successfully"),
        Err(e) => println!("❌ Retry operation failed: {}", e),
    }

    Ok(())
}

/// Demonstrates default value fallback
fn demonstrate_default_fallback() -> Result<(), FluentError> {
    println!("\n🔄 Example 4: Default Value Fallback");

    let shared_data = Arc::new(Mutex::new(vec![1, 2, 3]));
    let data_clone = shared_data.clone();

    // Spawn a thread that will panic and poison the mutex
    let handle = thread::spawn(move || {
        let _guard = data_clone.lock().unwrap();
        panic!("Intentional panic to poison mutex!");
    });

    // Wait for the thread to panic
    let _ = handle.join();

    // Try to get data with default fallback
    let config = PoisonHandlingConfig::use_default();
    match safe_lock_with_default!(&shared_data, "default_fallback_example", &config) {
        Ok(data) => {
            println!("✅ Got data (possibly default): {:?}", data);
            if data.is_empty() {
                println!("   Used default empty vector due to poison");
            }
        }
        Err(e) => println!("❌ Failed to get data with default: {}", e),
    }

    Ok(())
}

/// Demonstrates poison-resistant operations
fn demonstrate_poison_resistant_operations() -> Result<(), FluentError> {
    println!("\n🛡️  Example 5: Poison-Resistant Operations");

    let shared_data = Arc::new(Mutex::new(vec![1, 2, 3]));
    let data_clone = shared_data.clone();

    // Spawn a thread that will panic and poison the mutex
    let handle = thread::spawn(move || {
        let mut guard = data_clone.lock().unwrap();
        guard.push(999); // Add data before panicking
        panic!("Intentional panic to poison mutex!");
    });

    // Wait for the thread to panic
    let _ = handle.join();

    // Use poison-resistant operation to modify the data
    let result = poison_resistant_operation!(
        &shared_data,
        "poison_resistant_example",
        |data| {
            data.push(42);
            println!("✅ Successfully added 42 to poisoned mutex data");
            println!("   Current data: {:?}", *data);
            Ok(())
        }
    );

    match result {
        Ok(()) => println!("✅ Poison-resistant operation completed successfully"),
        Err(e) => println!("❌ Poison-resistant operation failed: {}", e),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poison_handling_config_creation() {
        let config = PoisonHandlingConfig::fail_fast();
        assert_eq!(config.strategy, fluent_core::error::PoisonRecoveryStrategy::FailFast);

        let config = PoisonHandlingConfig::recover_data();
        assert_eq!(config.strategy, fluent_core::error::PoisonRecoveryStrategy::RecoverData);

        let config = PoisonHandlingConfig::retry_with_delay(5, 200);
        assert_eq!(config.strategy, fluent_core::error::PoisonRecoveryStrategy::RetryWithDelay);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay_ms, 200);

        let config = PoisonHandlingConfig::use_default();
        assert_eq!(config.strategy, fluent_core::error::PoisonRecoveryStrategy::UseDefault);
    }

    #[test]
    fn test_data_recovery_from_poison() {
        use fluent_core::error::ThreadSafeErrorHandler;
        
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let mutex_clone = mutex.clone();

        // Create a poisoned mutex
        let handle = thread::spawn(move || {
            let mut guard = mutex_clone.lock().unwrap();
            guard.push(4);
            panic!("Poison the mutex");
        });
        let _ = handle.join();

        // Try to recover data
        let config = PoisonHandlingConfig::recover_data();
        let result = ThreadSafeErrorHandler::handle_mutex_lock_with_config(
            mutex.lock(),
            "test_recovery",
            &config,
        );

        assert!(result.is_ok());
        let guard = result.unwrap();
        assert_eq!(*guard, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_default_value_fallback() {
        use fluent_core::error::ThreadSafeErrorHandler;
        
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let mutex_clone = mutex.clone();

        // Create a poisoned mutex
        let handle = thread::spawn(move || {
            let _guard = mutex_clone.lock().unwrap();
            panic!("Poison the mutex");
        });
        let _ = handle.join();

        // Try to get default value
        let config = PoisonHandlingConfig::use_default();
        let result = ThreadSafeErrorHandler::handle_mutex_lock_with_default(
            &mutex,
            "test_default",
            &config,
        );

        // Should get empty vector as default
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data, Vec::<i32>::default());
    }

    #[test]
    fn test_normal_mutex_operations() {
        use fluent_core::error::ThreadSafeErrorHandler;
        
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        
        // Normal operation should work fine
        let config = PoisonHandlingConfig::fail_fast();
        let result = ThreadSafeErrorHandler::handle_mutex_lock_with_config(
            mutex.lock(),
            "test_normal",
            &config,
        );

        assert!(result.is_ok());
        let guard = result.unwrap();
        assert_eq!(*guard, vec![1, 2, 3]);
    }
}
