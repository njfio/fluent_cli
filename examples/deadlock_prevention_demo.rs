// Deadlock prevention demonstration
use fluent_core::deadlock_prevention::{DeadlockSafeLockManager, LockOrderRegistry};
use fluent_core::error::{FluentError, LockTimeoutConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), FluentError> {
    println!("🔒 Deadlock Prevention Demo");

    // Example 1: Demonstrate deadlock scenario and prevention
    demonstrate_deadlock_scenario().await?;

    // Example 2: Lock ordering enforcement
    demonstrate_lock_ordering().await?;

    // Example 3: Deadlock-safe lock manager
    demonstrate_deadlock_safe_manager().await?;

    // Example 4: Real-world scenario - security manager pattern
    demonstrate_security_manager_pattern().await?;

    // Example 5: Orchestrator pattern with multiple locks
    demonstrate_orchestrator_pattern().await?;

    println!("🎉 Deadlock prevention demo completed successfully!");
    Ok(())
}

/// Demonstrates a potential deadlock scenario and how to prevent it
async fn demonstrate_deadlock_scenario() -> Result<(), FluentError> {
    println!("\n⚠️  Example 1: Deadlock Scenario and Prevention");

    // Simulate the security manager deadlock scenario
    let active_sessions = Arc::new(RwLock::new(HashMap::<String, String>::new()));
    let rate_limiters = Arc::new(RwLock::new(HashMap::<String, u32>::new()));

    println!("🚫 Demonstrating potential deadlock scenario:");
    println!("   Task A: acquires active_sessions -> rate_limiters");
    println!("   Task B: acquires rate_limiters -> active_sessions");
    println!("   This could cause a deadlock!");

    // Safe approach: Always acquire locks in the same order
    println!("\n✅ Safe approach - consistent lock ordering:");
    
    let sessions_clone = active_sessions.clone();
    let limiters_clone = rate_limiters.clone();
    
    let task_a = tokio::spawn(async move {
        // SAFE: Always acquire active_sessions first, then rate_limiters
        let mut sessions = sessions_clone.write().await;
        let mut limiters = limiters_clone.write().await;
        
        sessions.insert("user_1".to_string(), "session_1".to_string());
        limiters.insert("user_1".to_string(), 10);
        
        println!("   ✅ Task A completed safely");
        sleep(Duration::from_millis(100)).await;
    });

    let sessions_clone2 = active_sessions.clone();
    let limiters_clone2 = rate_limiters.clone();
    
    let task_b = tokio::spawn(async move {
        // SAFE: Same order - active_sessions first, then rate_limiters
        let mut sessions = sessions_clone2.write().await;
        let mut limiters = limiters_clone2.write().await;
        
        sessions.insert("user_2".to_string(), "session_2".to_string());
        limiters.insert("user_2".to_string(), 20);
        
        println!("   ✅ Task B completed safely");
        sleep(Duration::from_millis(100)).await;
    });

    task_a.await.unwrap();
    task_b.await.unwrap();

    println!("✅ Both tasks completed without deadlock!");

    Ok(())
}

/// Demonstrates lock ordering enforcement
async fn demonstrate_lock_ordering() -> Result<(), FluentError> {
    println!("\n📋 Example 2: Lock Ordering Enforcement");

    let mut registry = LockOrderRegistry::new();
    registry.register_lock("lock_a", 10);
    registry.register_lock("lock_b", 20);
    registry.register_lock("lock_c", 30);

    println!("🔢 Registered locks with priorities:");
    println!("   lock_a: priority 10");
    println!("   lock_b: priority 20");
    println!("   lock_c: priority 30");

    // Valid lock order
    println!("\n✅ Testing valid lock order:");
    let result = registry.validate_lock_order(&["lock_a".to_string()], "lock_b");
    match result {
        Ok(()) => println!("   ✅ Valid: acquiring lock_b while holding lock_a"),
        Err(e) => println!("   ❌ Invalid: {}", e),
    }

    let result = registry.validate_lock_order(&["lock_a".to_string(), "lock_b".to_string()], "lock_c");
    match result {
        Ok(()) => println!("   ✅ Valid: acquiring lock_c while holding lock_a and lock_b"),
        Err(e) => println!("   ❌ Invalid: {}", e),
    }

    // Invalid lock order
    println!("\n❌ Testing invalid lock order:");
    let result = registry.validate_lock_order(&["lock_b".to_string()], "lock_a");
    match result {
        Ok(()) => println!("   ✅ Valid: acquiring lock_a while holding lock_b"),
        Err(e) => println!("   ❌ Invalid (expected): {}", e),
    }

    Ok(())
}

/// Demonstrates the deadlock-safe lock manager
async fn demonstrate_deadlock_safe_manager() -> Result<(), FluentError> {
    println!("\n🛡️  Example 3: Deadlock-Safe Lock Manager");

    let config = LockTimeoutConfig::medium_timeout();
    let manager = DeadlockSafeLockManager::new(config);

    // Register locks with priorities
    manager.register_locks(vec![
        ("resource_a", 10),
        ("resource_b", 20),
        ("resource_c", 30),
    ]).await?;

    // Create test resources
    let resource_a = Arc::new(Mutex::new("Resource A".to_string()));
    let resource_b = Arc::new(Mutex::new("Resource B".to_string()));
    let resource_c = Arc::new(Mutex::new("Resource C".to_string()));

    println!("🔒 Acquiring locks in random order - manager will reorder them:");

    // Request locks in "wrong" order - manager will reorder them
    let locks = vec![
        ("resource_c", &resource_c), // Priority 30
        ("resource_a", &resource_a), // Priority 10
        ("resource_b", &resource_b), // Priority 20
    ];

    let guards = manager.acquire_locks_ordered(locks).await?;
    
    println!("✅ Successfully acquired {} locks in correct order:", guards.len());
    println!("   Lock order enforced: resource_a (10) -> resource_b (20) -> resource_c (30)");
    
    // Use the resources
    for (i, guard) in guards.iter().enumerate() {
        println!("   Resource {}: {}", i + 1, **guard);
    }

    Ok(())
}

/// Demonstrates the security manager pattern with deadlock prevention
async fn demonstrate_security_manager_pattern() -> Result<(), FluentError> {
    println!("\n🔐 Example 4: Security Manager Pattern");

    // Simulate the fixed security manager
    struct SafeSecurityManager {
        active_sessions: Arc<RwLock<HashMap<String, String>>>,
        rate_limiters: Arc<RwLock<HashMap<String, u32>>>,
    }

    impl SafeSecurityManager {
        fn new() -> Self {
            Self {
                active_sessions: Arc::new(RwLock::new(HashMap::new())),
                rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        async fn create_session(&self, user_id: &str, session_id: &str) -> Result<(), FluentError> {
            // SAFE: Consistent lock order - always sessions before limiters
            let mut sessions = self.active_sessions.write().await;
            let mut limiters = self.rate_limiters.write().await;
            
            sessions.insert(user_id.to_string(), session_id.to_string());
            limiters.insert(user_id.to_string(), 100); // Default rate limit
            
            Ok(())
        }

        async fn remove_session(&self, user_id: &str) -> Result<(), FluentError> {
            // SAFE: Same lock order - sessions before limiters
            let mut sessions = self.active_sessions.write().await;
            let mut limiters = self.rate_limiters.write().await;
            
            sessions.remove(user_id);
            limiters.remove(user_id);
            
            Ok(())
        }

        async fn get_session_count(&self) -> usize {
            let sessions = self.active_sessions.read().await;
            sessions.len()
        }
    }

    let manager = Arc::new(SafeSecurityManager::new());

    // Spawn multiple tasks that create and remove sessions
    let mut handles = Vec::new();
    for i in 0..5 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let user_id = format!("user_{}", i);
            let session_id = format!("session_{}", i);

            // Create session
            manager_clone.create_session(&user_id, &session_id).await.unwrap();
            println!("   ✅ Created session for {}", user_id);

            // Simulate some work
            sleep(Duration::from_millis(50)).await;

            // Remove session
            manager_clone.remove_session(&user_id).await.unwrap();
            println!("   ✅ Removed session for {}", user_id);
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    println!("✅ All security manager operations completed without deadlock!");

    Ok(())
}

/// Demonstrates orchestrator pattern with multiple locks
async fn demonstrate_orchestrator_pattern() -> Result<(), FluentError> {
    println!("\n🎭 Example 5: Orchestrator Pattern");

    // Simulate orchestrator components
    struct SafeOrchestrator {
        agent_state: Arc<RwLock<String>>,
        metrics: Arc<RwLock<u64>>,
        reflection_engine: Arc<RwLock<String>>,
    }

    impl SafeOrchestrator {
        fn new() -> Self {
            Self {
                agent_state: Arc::new(RwLock::new("Initial State".to_string())),
                metrics: Arc::new(RwLock::new(0)),
                reflection_engine: Arc::new(RwLock::new("Ready".to_string())),
            }
        }

        async fn record_reasoning_step(&self, step_info: &str) -> Result<(), FluentError> {
            // SAFE: Consistent lock order - state before metrics
            let mut state = self.agent_state.write().await;
            let mut metrics = self.metrics.write().await;
            
            *state = format!("Reasoning: {}", step_info);
            *metrics += 1;
            
            println!("   📝 Recorded reasoning step: {}", step_info);
            Ok(())
        }

        async fn trigger_reflection(&self, reason: &str) -> Result<(), FluentError> {
            // SAFE: Acquire reflection engine and state in consistent order
            let mut reflection = self.reflection_engine.write().await;
            let state = self.agent_state.read().await;
            
            *reflection = format!("Reflecting on: {} (State: {})", reason, *state);
            
            println!("   🤔 Triggered reflection: {}", reason);
            Ok(())
        }

        async fn get_status(&self) -> (String, u64, String) {
            // SAFE: Read locks can be acquired in any order (no deadlock risk)
            let state = self.agent_state.read().await;
            let metrics = self.metrics.read().await;
            let reflection = self.reflection_engine.read().await;
            
            (state.clone(), *metrics, reflection.clone())
        }
    }

    let orchestrator = Arc::new(SafeOrchestrator::new());

    // Spawn multiple tasks that interact with the orchestrator
    let mut handles = Vec::new();
    for i in 0..3 {
        let orch_clone = orchestrator.clone();
        let handle = tokio::spawn(async move {
            // Record reasoning steps
            orch_clone.record_reasoning_step(&format!("Step {}", i)).await.unwrap();
            
            // Trigger reflection
            orch_clone.trigger_reflection(&format!("Reason {}", i)).await.unwrap();
            
            // Get status
            let (state, metrics, reflection) = orch_clone.get_status().await;
            println!("   📊 Status - State: {}, Metrics: {}, Reflection: {}", 
                     state, metrics, reflection);
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    println!("✅ All orchestrator operations completed without deadlock!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deadlock_prevention() {
        let config = LockTimeoutConfig::short_timeout();
        let manager = DeadlockSafeLockManager::new(config);

        manager.register_locks(vec![
            ("lock_a", 10),
            ("lock_b", 20),
        ]).await.unwrap();

        let mutex_a = Arc::new(Mutex::new(1));
        let mutex_b = Arc::new(Mutex::new(2));

        // Test that locks are acquired in correct order regardless of request order
        let guards = manager.acquire_locks_ordered(vec![
            ("lock_b", &mutex_b), // Requested first but has higher priority
            ("lock_a", &mutex_a), // Requested second but has lower priority
        ]).await.unwrap();

        // Verify correct order: lock_a (priority 10) then lock_b (priority 20)
        assert_eq!(*guards[0], 1); // lock_a
        assert_eq!(*guards[1], 2); // lock_b
    }

    #[test]
    fn test_lock_order_validation() {
        let mut registry = LockOrderRegistry::new();
        registry.register_lock("first", 10);
        registry.register_lock("second", 20);

        // Valid order
        assert!(registry.validate_lock_order(&["first".to_string()], "second").is_ok());

        // Invalid order
        assert!(registry.validate_lock_order(&["second".to_string()], "first").is_err());
    }
}
