use anyhow::{anyhow, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::{Duration, Instant};

/// Configuration for optimized parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionConfig {
    /// Maximum number of concurrent tasks
    pub max_concurrency: usize,
    /// Task timeout duration
    pub task_timeout: Duration,
    /// Enable adaptive concurrency based on system load
    pub adaptive_concurrency: bool,
    /// Resource monitoring interval
    pub monitoring_interval: Duration,
    /// Maximum memory usage threshold (in MB)
    pub max_memory_mb: usize,
    /// CPU usage threshold for throttling
    pub cpu_threshold: f64,
}

impl Default for ParallelExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrency: num_cpus::get() * 2,
            task_timeout: Duration::from_secs(300), // 5 minutes
            adaptive_concurrency: true,
            monitoring_interval: Duration::from_secs(1),
            max_memory_mb: 1024, // 1GB
            cpu_threshold: 0.8,  // 80%
        }
    }
}

/// Task priority levels for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Execution task with metadata
#[derive(Debug, Clone)]
pub struct ExecutionTask<T> {
    pub id: String,
    pub priority: TaskPriority,
    pub estimated_duration: Option<Duration>,
    pub memory_requirement: Option<usize>, // in MB
    pub payload: T,
    pub dependencies: Vec<String>,
}

/// Task execution result
#[derive(Debug, Clone)]
pub struct TaskResult<R> {
    pub task_id: String,
    pub result: std::result::Result<R, String>, // Use String for error to make it cloneable
    pub execution_time: Duration,
    pub memory_used: Option<usize>,
}

/// Resource monitoring metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub cpu_usage: f64,
    pub memory_usage_mb: usize,
    pub active_tasks: usize,
    pub queued_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
}

/// Optimized parallel executor with resource management and adaptive scheduling
pub struct OptimizedParallelExecutor<T, R> {
    config: ParallelExecutionConfig,
    semaphore: Arc<Semaphore>,
    metrics: Arc<RwLock<ResourceMetrics>>,
    task_queue: Arc<Mutex<Vec<ExecutionTask<T>>>>,
    completed_tasks: Arc<RwLock<HashMap<String, TaskResult<R>>>>,
    _phantom: std::marker::PhantomData<(T, R)>,
}

impl<T, R> OptimizedParallelExecutor<T, R>
where
    T: Clone + Send + Sync + 'static,
    R: Clone + Send + Sync + 'static,
{
    /// Create a new optimized parallel executor
    pub fn new(config: ParallelExecutionConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));

        Self {
            semaphore,
            config,
            metrics: Arc::new(RwLock::new(ResourceMetrics {
                cpu_usage: 0.0,
                memory_usage_mb: 0,
                active_tasks: 0,
                queued_tasks: 0,
                completed_tasks: 0,
                failed_tasks: 0,
            })),
            task_queue: Arc::new(Mutex::new(Vec::new())),
            completed_tasks: Arc::new(RwLock::new(HashMap::new())),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Execute tasks in parallel with optimized scheduling
    pub async fn execute_tasks<F, Fut>(
        &self,
        tasks: Vec<ExecutionTask<T>>,
        executor_fn: F,
    ) -> Result<Vec<TaskResult<R>>>
    where
        F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<R>> + Send + 'static,
    {
        info!("Starting parallel execution of {} tasks", tasks.len());

        // Start resource monitoring
        let monitor_handle = self.start_resource_monitoring().await;

        // Sort tasks by priority and dependencies
        let sorted_tasks = self.sort_tasks_by_priority_and_dependencies(tasks).await?;

        // Execute tasks in batches based on dependencies
        let mut all_results = Vec::new();
        let task_batches = self.create_execution_batches(sorted_tasks).await?;

        for batch in task_batches {
            let batch_results = self.execute_task_batch(batch, executor_fn.clone()).await?;
            all_results.extend(batch_results);
        }

        // Stop monitoring
        monitor_handle.abort();

        info!(
            "Completed parallel execution of {} tasks",
            all_results.len()
        );
        Ok(all_results)
    }

    /// Execute a batch of tasks that can run in parallel
    async fn execute_task_batch<F, Fut>(
        &self,
        tasks: Vec<ExecutionTask<T>>,
        executor_fn: F,
    ) -> Result<Vec<TaskResult<R>>>
    where
        F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<R>> + Send + 'static,
    {
        let mut futures = FuturesUnordered::new();

        // Update queued tasks count
        {
            let mut metrics = self.metrics.write().await;
            metrics.queued_tasks = tasks.len();
        }

        for task in tasks {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let metrics = self.metrics.clone();
            let executor = executor_fn.clone();
            let timeout_duration = self.config.task_timeout;

            futures.push(tokio::spawn(async move {
                let _permit = permit; // Hold permit for task duration
                let start_time = Instant::now();

                // Update active tasks count
                {
                    let mut m = metrics.write().await;
                    m.active_tasks += 1;
                    m.queued_tasks = m.queued_tasks.saturating_sub(1);
                }

                // Execute task with timeout
                let result =
                    tokio::time::timeout(timeout_duration, executor(task.payload.clone())).await;

                let execution_time = start_time.elapsed();
                let task_result = match result {
                    Ok(Ok(value)) => {
                        debug!(
                            "Task {} completed successfully in {:?}",
                            task.id, execution_time
                        );
                        TaskResult {
                            task_id: task.id.clone(),
                            result: Ok(value),
                            execution_time,
                            memory_used: task.memory_requirement,
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("Task {} failed: {}", task.id, e);
                        TaskResult {
                            task_id: task.id.clone(),
                            result: Err(e.to_string()),
                            execution_time,
                            memory_used: task.memory_requirement,
                        }
                    }
                    Err(_) => {
                        error!("Task {} timed out after {:?}", task.id, timeout_duration);
                        TaskResult {
                            task_id: task.id.clone(),
                            result: Err("Task timed out".to_string()),
                            execution_time,
                            memory_used: task.memory_requirement,
                        }
                    }
                };

                // Update metrics
                {
                    let mut m = metrics.write().await;
                    m.active_tasks = m.active_tasks.saturating_sub(1);
                    if task_result.result.is_ok() {
                        m.completed_tasks += 1;
                    } else {
                        m.failed_tasks += 1;
                    }
                }

                task_result
            }));
        }

        // Collect results
        let mut results = Vec::new();
        while let Some(result) = futures.next().await {
            match result {
                Ok(task_result) => results.push(task_result),
                Err(e) => {
                    error!("Task join error: {}", e);
                    // Create error result for failed join
                    results.push(TaskResult {
                        task_id: "unknown".to_string(),
                        result: Err(format!("Task join failed: {}", e)),
                        execution_time: Duration::from_secs(0),
                        memory_used: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Sort tasks by priority and resolve dependencies
    async fn sort_tasks_by_priority_and_dependencies(
        &self,
        mut tasks: Vec<ExecutionTask<T>>,
    ) -> Result<Vec<ExecutionTask<T>>> {
        // Sort by priority first (higher priority first)
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        // TODO: Implement topological sort for dependencies
        // For now, just return priority-sorted tasks
        Ok(tasks)
    }

    /// Create execution batches based on dependencies
    async fn create_execution_batches(
        &self,
        tasks: Vec<ExecutionTask<T>>,
    ) -> Result<Vec<Vec<ExecutionTask<T>>>> {
        // Simple implementation: group tasks without dependencies first
        let mut batches = Vec::new();
        let mut remaining_tasks = tasks;
        let mut completed_task_ids = std::collections::HashSet::new();

        while !remaining_tasks.is_empty() {
            let mut current_batch = Vec::new();
            let mut indices_to_remove = Vec::new();

            for (i, task) in remaining_tasks.iter().enumerate() {
                // Check if all dependencies are satisfied
                let dependencies_satisfied = task
                    .dependencies
                    .iter()
                    .all(|dep| completed_task_ids.contains(dep));

                if dependencies_satisfied {
                    current_batch.push(task.clone());
                    indices_to_remove.push(i);
                }
            }

            if current_batch.is_empty() {
                return Err(anyhow!("Circular dependency detected in tasks"));
            }

            // Mark tasks as completed for dependency resolution
            for task in &current_batch {
                completed_task_ids.insert(task.id.clone());
            }

            // Remove processed tasks
            for &i in indices_to_remove.iter().rev() {
                remaining_tasks.remove(i);
            }

            batches.push(current_batch);
        }

        Ok(batches)
    }

    /// Start resource monitoring task
    async fn start_resource_monitoring(&self) -> tokio::task::JoinHandle<()> {
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let semaphore = self.semaphore.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.monitoring_interval);

            loop {
                interval.tick().await;

                // Get system metrics (simplified - in real implementation, use system monitoring crates)
                let cpu_usage = Self::get_cpu_usage().await;
                let memory_usage = Self::get_memory_usage().await;

                // Update metrics
                {
                    let mut m = metrics.write().await;
                    m.cpu_usage = cpu_usage;
                    m.memory_usage_mb = memory_usage;
                }

                // Log resource usage
                debug!(
                    "Resource usage - CPU: {:.1}%, Memory: {}MB",
                    cpu_usage, memory_usage
                );

                // Implement adaptive concurrency based on resource usage
                let current_concurrency = semaphore.available_permits();
                let target_concurrency = Self::calculate_optimal_concurrency(cpu_usage, memory_usage as u64, &config);

                if target_concurrency != current_concurrency {
                    info!(
                        "Adjusting concurrency from {} to {} based on resource usage (CPU: {:.1}%, Memory: {}MB)",
                        current_concurrency, target_concurrency, cpu_usage, memory_usage
                    );
                    // Note: Dynamic semaphore adjustment is complex and would require
                    // a more sophisticated implementation with permit management
                    debug!("Concurrency adjustment requested but not implemented in this simplified version");
                }

                if cpu_usage > config.cpu_threshold {
                    warn!("High CPU usage detected: {:.1}%", cpu_usage);
                }

                if memory_usage > config.max_memory_mb {
                    warn!("High memory usage detected: {}MB", memory_usage);
                }
            }
        })
    }

    /// Get current CPU usage (simplified implementation)
    async fn get_cpu_usage() -> f64 {
        // In a real implementation, use a system monitoring crate like `sysinfo`
        0.0 // Placeholder
    }

    /// Get current memory usage in MB (simplified implementation)
    async fn get_memory_usage() -> usize {
        // In a real implementation, use a system monitoring crate like `sysinfo`
        0 // Placeholder
    }

    /// Calculate optimal concurrency based on resource usage
    fn calculate_optimal_concurrency(cpu_usage: f64, memory_usage: u64, config: &ParallelExecutionConfig) -> usize {
        let mut target_concurrency = config.max_concurrency;

        // Reduce concurrency if CPU usage is high
        if cpu_usage > config.cpu_threshold {
            let reduction_factor = (cpu_usage - config.cpu_threshold) / (100.0 - config.cpu_threshold);
            target_concurrency = ((target_concurrency as f64) * (1.0 - reduction_factor * 0.5)) as usize;
        }

        // Reduce concurrency if memory usage is high
        if memory_usage > config.max_memory_mb as u64 {
            let reduction_factor = (memory_usage as f64 - config.max_memory_mb as f64) / (config.max_memory_mb as f64);
            target_concurrency = ((target_concurrency as f64) * (1.0 - reduction_factor * 0.3)) as usize;
        }

        // Ensure minimum concurrency of 1
        target_concurrency.max(1).min(config.max_concurrency)
    }

    /// Get current execution metrics
    pub async fn get_metrics(&self) -> ResourceMetrics {
        self.metrics.read().await.clone()
    }

    /// Get the number of queued tasks
    pub async fn get_queue_size(&self) -> usize {
        self.task_queue.lock().await.len()
    }

    /// Get completed task results
    pub async fn get_completed_tasks(&self) -> HashMap<String, TaskResult<R>> {
        self.completed_tasks.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_optimized_parallel_executor() {
        let config = ParallelExecutionConfig {
            max_concurrency: 2,
            task_timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let executor: OptimizedParallelExecutor<String, String> =
            OptimizedParallelExecutor::new(config);

        let tasks = vec![
            ExecutionTask {
                id: "task1".to_string(),
                priority: TaskPriority::High,
                estimated_duration: Some(Duration::from_millis(100)),
                memory_requirement: Some(10),
                payload: "test1".to_string(),
                dependencies: vec![],
            },
            ExecutionTask {
                id: "task2".to_string(),
                priority: TaskPriority::Normal,
                estimated_duration: Some(Duration::from_millis(200)),
                memory_requirement: Some(20),
                payload: "test2".to_string(),
                dependencies: vec![],
            },
        ];

        let results = executor
            .execute_tasks(tasks, |payload: String| async move {
                sleep(Duration::from_millis(50)).await;
                Ok(format!("processed_{}", payload))
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.result.is_ok()));
    }

    #[tokio::test]
    async fn test_task_priority_ordering() {
        let config = ParallelExecutionConfig::default();
        let executor: OptimizedParallelExecutor<String, String> =
            OptimizedParallelExecutor::new(config);

        let tasks = vec![
            ExecutionTask {
                id: "low".to_string(),
                priority: TaskPriority::Low,
                estimated_duration: None,
                memory_requirement: None,
                payload: "low_task".to_string(),
                dependencies: vec![],
            },
            ExecutionTask {
                id: "high".to_string(),
                priority: TaskPriority::High,
                estimated_duration: None,
                memory_requirement: None,
                payload: "high_task".to_string(),
                dependencies: vec![],
            },
            ExecutionTask {
                id: "critical".to_string(),
                priority: TaskPriority::Critical,
                estimated_duration: None,
                memory_requirement: None,
                payload: "critical_task".to_string(),
                dependencies: vec![],
            },
        ];

        let sorted = executor
            .sort_tasks_by_priority_and_dependencies(tasks)
            .await
            .unwrap();

        // Should be sorted by priority: Critical, High, Low
        assert_eq!(sorted[0].priority, TaskPriority::Critical);
        assert_eq!(sorted[1].priority, TaskPriority::High);
        assert_eq!(sorted[2].priority, TaskPriority::Low);
    }
}
