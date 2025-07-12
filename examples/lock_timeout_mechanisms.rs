// Lock timeout mechanisms example
use fluent_core::error::{FluentError, LockTimeoutConfig};
use fluent_core::lock_timeout::{LockContentionMonitor, LockTimeoutUtils};
use fluent_core::{safe_tokio_lock_with_timeout, safe_tokio_lock_short_timeout, safe_tokio_lock_medium_timeout};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), FluentError> {
    println!("⏰ Lock Timeout Mechanisms Demo");

    // Example 1: Basic timeout handling
    demonstrate_basic_timeout_handling().await?;

    // Example 2: Lock contention monitoring
    demonstrate_lock_contention_monitoring().await?;

    // Example 3: Different timeout strategies
    demonstrate_timeout_strategies().await?;

    // Example 4: RwLock timeout handling
    demonstrate_rwlock_timeout_handling().await?;

    // Example 5: High contention scenarios
    demonstrate_high_contention_scenarios().await?;

    println!("🎉 Lock timeout mechanisms demo completed successfully!");
    Ok(())
}

/// Demonstrates basic timeout handling
async fn demonstrate_basic_timeout_handling() -> Result<(), FluentError> {
    println!("\n⏰ Example 1: Basic Timeout Handling");

    let shared_data = Arc::new(Mutex::new(vec![1, 2, 3]));
    let data_clone = shared_data.clone();

    // Spawn a task that holds the lock for a long time
    let long_running_task = tokio::spawn(async move {
        let _guard = data_clone.lock().await;
        println!("🔒 Long-running task acquired lock, sleeping for 3 seconds...");
        sleep(Duration::from_secs(3)).await;
        println!("🔓 Long-running task releasing lock");
    });

    // Give the long-running task time to acquire the lock
    sleep(Duration::from_millis(100)).await;

    // Try to acquire the lock with a short timeout
    let timeout_result = safe_tokio_lock_with_timeout!(
        &shared_data,
        "basic_timeout_example",
        Duration::from_secs(1)
    );

    match timeout_result {
        Ok(_) => println!("❌ Unexpected success - should have timed out"),
        Err(FluentError::LockTimeout(msg)) => {
            println!("✅ Lock timeout handled correctly: {}", msg);
        }
        Err(e) => println!("❌ Unexpected error: {}", e),
    }

    // Wait for the long-running task to complete
    long_running_task.await.unwrap();

    // Now try again with a longer timeout - should succeed
    let success_result = safe_tokio_lock_with_timeout!(
        &shared_data,
        "basic_timeout_success",
        Duration::from_secs(5)
    );

    match success_result {
        Ok(guard) => {
            println!("✅ Successfully acquired lock after long-running task completed: {:?}", *guard);
        }
        Err(e) => println!("❌ Unexpected error: {}", e),
    }

    Ok(())
}

/// Demonstrates lock contention monitoring
async fn demonstrate_lock_contention_monitoring() -> Result<(), FluentError> {
    println!("\n📊 Example 2: Lock Contention Monitoring");

    let shared_data = Arc::new(Mutex::new(0));
    let monitor = Arc::new(LockContentionMonitor::new());
    let config = Arc::new(LockTimeoutConfig::medium_timeout());

    // Spawn multiple tasks that compete for the lock
    let mut handles = Vec::new();
    for i in 0..5 {
        let data_clone = shared_data.clone();
        let monitor_clone = monitor.clone();
        let config_clone = config.clone();
        let handle = tokio::spawn(async move {
            let result = LockTimeoutUtils::execute_with_timeout_and_monitoring(
                &data_clone,
                &format!("contention_task_{}", i),
                &config_clone,
                Some(&*monitor_clone),
                |counter| {
                    *counter += 1;
                    // Simulate some work
                    std::thread::sleep(Duration::from_millis(100));
                    Ok(*counter)
                },
            ).await;

            match result {
                Ok(value) => println!("✅ Task {} completed with value: {}", i, value),
                Err(e) => println!("❌ Task {} failed: {}", i, e),
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Display contention statistics
    let stats = monitor.get_stats();
    println!("📈 Lock Contention Statistics:");
    println!("   Total acquisitions: {}", stats.total_acquisitions);
    println!("   Total timeouts: {}", stats.total_timeouts);
    println!("   Timeout rate: {:.2}%", stats.timeout_rate);
    println!("   Average wait time: {:.2}ms", stats.average_wait_time_ms);
    println!("   Max concurrent waiters: {}", stats.max_concurrent_waiters);
    println!("   Contention warnings: {}", stats.contention_warnings);

    Ok(())
}

/// Demonstrates different timeout strategies
async fn demonstrate_timeout_strategies() -> Result<(), FluentError> {
    println!("\n🎯 Example 3: Different Timeout Strategies");

    let shared_data = Arc::new(Mutex::new("shared_resource".to_string()));

    // Short timeout for quick operations
    println!("🏃 Testing short timeout (5 seconds):");
    let short_result = safe_tokio_lock_short_timeout!(&shared_data, "short_timeout_test");
    match short_result {
        Ok(guard) => println!("✅ Short timeout succeeded: {}", *guard),
        Err(e) => println!("❌ Short timeout failed: {}", e),
    }

    // Medium timeout for normal operations
    println!("🚶 Testing medium timeout (30 seconds):");
    let medium_result = safe_tokio_lock_medium_timeout!(&shared_data, "medium_timeout_test");
    match medium_result {
        Ok(guard) => println!("✅ Medium timeout succeeded: {}", *guard),
        Err(e) => println!("❌ Medium timeout failed: {}", e),
    }

    // Custom timeout configuration
    let custom_config = LockTimeoutConfig {
        timeout: Duration::from_millis(500),
        log_timeout_events: true,
        monitor_contention: true,
        max_waiters_warning_threshold: 3,
    };

    println!("⚙️  Testing custom timeout (500ms):");
    let custom_result = fluent_core::error::ThreadSafeErrorHandler::handle_tokio_mutex_lock_with_timeout(
        &shared_data,
        "custom_timeout_test",
        &custom_config,
    ).await;

    match custom_result {
        Ok(guard) => println!("✅ Custom timeout succeeded: {}", *guard),
        Err(e) => println!("❌ Custom timeout failed: {}", e),
    }

    Ok(())
}

/// Demonstrates RwLock timeout handling
async fn demonstrate_rwlock_timeout_handling() -> Result<(), FluentError> {
    println!("\n📖 Example 4: RwLock Timeout Handling");

    let shared_config = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let config = LockTimeoutConfig::short_timeout();

    // Initialize some data
    {
        let mut write_guard = fluent_core::error::ThreadSafeErrorHandler::handle_tokio_rwlock_write_with_timeout(
            &shared_config,
            "rwlock_init",
            &config,
        ).await?;
        write_guard.insert("setting1".to_string(), "value1".to_string());
        write_guard.insert("setting2".to_string(), "value2".to_string());
        println!("✅ Initialized RwLock with data");
    }

    // Spawn multiple readers
    let mut reader_handles = Vec::new();
    for i in 0..3 {
        let config_clone = shared_config.clone();
        let config_ref = config.clone();
        let handle = tokio::spawn(async move {
            let read_result = fluent_core::error::ThreadSafeErrorHandler::handle_tokio_rwlock_read_with_timeout(
                &config_clone,
                &format!("rwlock_reader_{}", i),
                &config_ref,
            ).await;

            match read_result {
                Ok(guard) => {
                    println!("✅ Reader {} accessed config: {} items", i, guard.len());
                    // Simulate reading work
                    sleep(Duration::from_millis(50)).await;
                }
                Err(e) => println!("❌ Reader {} failed: {}", i, e),
            }
        });
        reader_handles.push(handle);
    }

    // Wait for readers to complete
    for handle in reader_handles {
        handle.await.unwrap();
    }

    // Test write with timeout
    let write_result = fluent_core::error::ThreadSafeErrorHandler::handle_tokio_rwlock_write_with_timeout(
        &shared_config,
        "rwlock_writer",
        &config,
    ).await;

    match write_result {
        Ok(mut guard) => {
            guard.insert("setting3".to_string(), "value3".to_string());
            println!("✅ Writer updated config: {} items", guard.len());
        }
        Err(e) => println!("❌ Writer failed: {}", e),
    }

    Ok(())
}

/// Demonstrates high contention scenarios
async fn demonstrate_high_contention_scenarios() -> Result<(), FluentError> {
    println!("\n🔥 Example 5: High Contention Scenarios");

    let shared_counter = Arc::new(Mutex::new(0));
    let monitor = Arc::new(LockContentionMonitor::new());

    // Create a config with low warning threshold to trigger contention warnings
    let config = Arc::new(LockTimeoutConfig {
        timeout: Duration::from_secs(10),
        log_timeout_events: true,
        monitor_contention: true,
        max_waiters_warning_threshold: 2, // Low threshold to trigger warnings
    });

    // Spawn many tasks to create high contention
    let mut handles = Vec::new();
    for i in 0..10 {
        let counter_clone = shared_counter.clone();
        let monitor_clone = monitor.clone();
        let config_clone = config.clone();
        let handle = tokio::spawn(async move {
            let result = LockTimeoutUtils::execute_with_timeout_and_monitoring(
                &counter_clone,
                &format!("high_contention_task_{}", i),
                &config_clone,
                Some(&*monitor_clone),
                |counter| {
                    *counter += 1;
                    // Simulate work that takes time
                    std::thread::sleep(Duration::from_millis(200));
                    Ok(*counter)
                },
            ).await;

            match result {
                Ok(value) => println!("✅ High contention task {} completed: {}", i, value),
                Err(e) => println!("❌ High contention task {} failed: {}", i, e),
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Display final statistics
    let stats = monitor.get_stats();
    println!("🔥 High Contention Statistics:");
    println!("   Total acquisitions: {}", stats.total_acquisitions);
    println!("   Total timeouts: {}", stats.total_timeouts);
    println!("   Timeout rate: {:.2}%", stats.timeout_rate);
    println!("   Average wait time: {:.2}ms", stats.average_wait_time_ms);
    println!("   Max concurrent waiters: {}", stats.max_concurrent_waiters);
    println!("   Contention warnings: {}", stats.contention_warnings);

    if stats.contention_warnings > 0 {
        println!("⚠️  High contention detected - consider optimizing lock usage");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_handling() {
        let mutex = Arc::new(Mutex::new(42));
        let config = LockTimeoutConfig::with_timeout(Duration::from_millis(100));

        // This should succeed quickly
        let result = fluent_core::error::ThreadSafeErrorHandler::handle_tokio_mutex_lock_with_timeout(
            &mutex,
            "test_timeout",
            &config,
        ).await;

        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_contention_monitoring() {
        let monitor = LockContentionMonitor::new();
        let mutex = Arc::new(Mutex::new(0));
        let config = LockTimeoutConfig::short_timeout();

        let result = LockTimeoutUtils::execute_with_timeout_and_monitoring(
            &mutex,
            "test_monitoring",
            &config,
            Some(&monitor),
            |data| {
                *data += 1;
                Ok(*data)
            },
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        let stats = monitor.get_stats();
        assert_eq!(stats.total_acquisitions, 1);
        assert_eq!(stats.total_timeouts, 0);
    }
}
