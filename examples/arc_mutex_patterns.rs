// Arc/Mutex usage patterns example
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

/// Example of improved Arc/Mutex patterns
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔒 Arc/Mutex Usage Patterns Demo");

    // Example 1: Proper tokio::sync::Mutex usage (not Arc<Mutex<Option<T>>>)
    demonstrate_proper_mutex_usage().await?;

    // Example 2: Efficient RwLock usage for read-heavy workloads
    demonstrate_rwlock_patterns().await?;

    // Example 3: Avoiding nested Arc patterns
    demonstrate_nested_arc_avoidance().await?;

    // Example 4: Lock scope management
    demonstrate_lock_scope_management().await?;

    println!("🎉 Arc/Mutex patterns demo completed successfully!");
    Ok(())
}

/// Demonstrates proper mutex usage patterns
async fn demonstrate_proper_mutex_usage() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📝 Example 1: Proper Mutex Usage");

    // ❌ BAD: Arc<Mutex<Option<T>>> - unnecessary Option wrapper
    // let bad_pattern = Arc::new(Mutex::new(Option::<String>::None));

    // ✅ GOOD: Direct Arc<Mutex<T>> with proper initialization
    let shared_data = Arc::new(Mutex::new(String::from("initial_value")));

    // Spawn multiple tasks that modify the shared data
    let mut handles = Vec::new();

    for i in 0..3 {
        let data_clone = shared_data.clone();
        let handle = tokio::spawn(async move {
            // Proper lock scope - acquire, modify, release quickly
            {
                let mut guard = data_clone.lock().await;
                guard.push_str(&format!("_task_{}", i));
            } // Lock is released here
            
            // Simulate some work outside the lock
            sleep(Duration::from_millis(10)).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }

    let final_value = shared_data.lock().await;
    println!("✅ Final shared data: {}", *final_value);

    Ok(())
}

/// Demonstrates efficient RwLock usage for read-heavy workloads
async fn demonstrate_rwlock_patterns() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📖 Example 2: RwLock for Read-Heavy Workloads");

    // Configuration that's read frequently but written rarely
    let config = Arc::new(RwLock::new(std::collections::HashMap::new()));
    
    // Initialize config
    {
        let mut write_guard = config.write().await;
        write_guard.insert("max_connections".to_string(), "100".to_string());
        write_guard.insert("timeout_ms".to_string(), "5000".to_string());
    }

    // Spawn multiple readers
    let mut reader_handles = Vec::new();
    for i in 0..5 {
        let config_clone = config.clone();
        let handle = tokio::spawn(async move {
            // Multiple readers can acquire read locks simultaneously
            let read_guard = config_clone.read().await;
            if let Some(timeout) = read_guard.get("timeout_ms") {
                println!("✅ Reader {}: timeout = {}", i, timeout);
            }
        });
        reader_handles.push(handle);
    }

    // Wait for readers
    for handle in reader_handles {
        handle.await?;
    }

    // Single writer updates config
    {
        let mut write_guard = config.write().await;
        write_guard.insert("timeout_ms".to_string(), "10000".to_string());
        println!("✅ Config updated by writer");
    }

    Ok(())
}

/// Demonstrates avoiding nested Arc patterns
async fn demonstrate_nested_arc_avoidance() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Example 3: Avoiding Nested Arc Patterns");

    // ❌ BAD: Nested Arc pattern
    // struct BadStateManager {
    //     state: Arc<RwLock<State>>,
    //     history: Arc<RwLock<Vec<State>>>,
    // }
    // let bad_manager = Arc::new(BadStateManager { ... });

    // ✅ GOOD: Single Arc with internal synchronization
    #[derive(Debug, Clone)]
    struct State {
        value: i32,
        timestamp: std::time::SystemTime,
    }

    struct GoodStateManager {
        state: RwLock<State>,
        history: RwLock<Vec<State>>,
    }

    impl GoodStateManager {
        fn new() -> Self {
            Self {
                state: RwLock::new(State {
                    value: 0,
                    timestamp: std::time::SystemTime::now(),
                }),
                history: RwLock::new(Vec::new()),
            }
        }

        async fn update_state(&self, new_value: i32) {
            let new_state = State {
                value: new_value,
                timestamp: std::time::SystemTime::now(),
            };

            // Update current state
            {
                let mut state_guard = self.state.write().await;
                *state_guard = new_state.clone();
            }

            // Add to history
            {
                let mut history_guard = self.history.write().await;
                history_guard.push(new_state);
            }
        }

        async fn get_current_value(&self) -> i32 {
            let state_guard = self.state.read().await;
            state_guard.value
        }
    }

    let manager = Arc::new(GoodStateManager::new());

    // Spawn tasks that update state
    let mut handles = Vec::new();
    for i in 1..=3 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            manager_clone.update_state(i * 10).await;
            let current = manager_clone.get_current_value().await;
            println!("✅ Task {}: updated state to {}", i, current);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    Ok(())
}

/// Demonstrates proper lock scope management
async fn demonstrate_lock_scope_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⏱️  Example 4: Lock Scope Management");

    let shared_counter = Arc::new(Mutex::new(0));

    // ❌ BAD: Holding lock across await points
    // let guard = shared_counter.lock().await;
    // some_async_operation().await; // Lock held during async operation!
    // *guard += 1;

    // ✅ GOOD: Minimize lock scope
    let counter_clone = shared_counter.clone();
    tokio::spawn(async move {
        // Acquire lock, do work, release quickly
        {
            let mut guard = counter_clone.lock().await;
            *guard += 1;
            println!("✅ Counter incremented to: {}", *guard);
        } // Lock released here

        // Do async work outside the lock
        sleep(Duration::from_millis(100)).await;
        println!("✅ Async work completed");
    }).await?;

    // ✅ GOOD: Pattern for complex operations
    let counter_clone = shared_counter.clone();
    tokio::spawn(async move {
        // Read current value
        let current_value = {
            let guard = counter_clone.lock().await;
            *guard
        };

        // Do computation outside the lock
        let new_value = current_value * 2;
        sleep(Duration::from_millis(50)).await; // Simulate async work

        // Update with computed value
        {
            let mut guard = counter_clone.lock().await;
            *guard = new_value;
            println!("✅ Counter updated to: {}", *guard);
        }
    }).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proper_mutex_usage() {
        let shared_data = Arc::new(Mutex::new(0));
        let data_clone = shared_data.clone();

        let handle = tokio::spawn(async move {
            let mut guard = data_clone.lock().await;
            *guard = 42;
        });

        handle.await.unwrap();
        let final_value = *shared_data.lock().await;
        assert_eq!(final_value, 42);
    }

    #[tokio::test]
    async fn test_rwlock_concurrent_reads() {
        let data = Arc::new(RwLock::new(100));
        
        // Multiple concurrent readers
        let mut handles = Vec::new();
        for _ in 0..5 {
            let data_clone = data.clone();
            let handle = tokio::spawn(async move {
                let guard = data_clone.read().await;
                *guard
            });
            handles.push(handle);
        }

        // All readers should get the same value
        for handle in handles {
            let value = handle.await.unwrap();
            assert_eq!(value, 100);
        }
    }
}
