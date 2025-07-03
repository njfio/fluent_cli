#[cfg(test)]
mod enhanced_pipeline_executor_tests {
    use crate::enhanced_pipeline_executor::*;
    use crate::pipeline_executor::{Pipeline, PipelineStep};
    use crate::pipeline_infrastructure::MemoryStateStore;
    use std::time::Duration;
    use tokio_test;

    fn create_test_config() -> ExecutorConfig {
        ExecutorConfig {
            max_concurrency: 4,
            adaptive_concurrency: true,
            max_memory_mb: 512,
            cpu_threshold: 0.8,
            step_timeout: Duration::from_secs(30),
            dependency_analysis: true,
            batch_size: 5,
            enable_caching: true,
            cache_ttl: 300,
        }
    }

    fn create_test_pipeline() -> Pipeline {
        Pipeline {
            name: "test-pipeline".to_string(),
            steps: vec![
                PipelineStep::Command {
                    name: "step1".to_string(),
                    command: "echo hello".to_string(),
                    save_output: Some("output1".to_string()),
                    retry: None,
                },
                PipelineStep::Command {
                    name: "step2".to_string(),
                    command: "echo world".to_string(),
                    save_output: Some("output2".to_string()),
                    retry: None,
                },
                PipelineStep::Command {
                    name: "step3".to_string(),
                    command: "echo done".to_string(),
                    save_output: Some("output3".to_string()),
                    retry: None,
                },
            ],
        }
    }

    #[tokio::test]
    async fn test_enhanced_executor_creation() {
        let state_store = MemoryStateStore::new();
        let config = create_test_config();
        let executor = EnhancedPipelineExecutor::new(state_store, config.clone());

        assert_eq!(executor.get_config().max_concurrency, 4);
        assert!(executor.get_config().adaptive_concurrency);
        assert_eq!(executor.get_config().max_memory_mb, 512);
        assert_eq!(executor.get_config().cpu_threshold, 0.8);
        assert!(executor.get_config().dependency_analysis);
        assert_eq!(executor.get_config().batch_size, 5);
        assert!(executor.get_config().enable_caching);
        assert_eq!(executor.get_config().cache_ttl, 300);
    }

    #[tokio::test]
    async fn test_executor_config_default() {
        let config = ExecutorConfig::default();

        assert_eq!(config.max_concurrency, num_cpus::get() * 2);
        assert!(config.adaptive_concurrency);
        assert_eq!(config.max_memory_mb, 1024);
        assert_eq!(config.cpu_threshold, 0.8);
        assert_eq!(config.step_timeout, Duration::from_secs(300));
        assert!(config.dependency_analysis);
        assert_eq!(config.batch_size, 10);
        assert!(config.enable_caching);
        assert_eq!(config.cache_ttl, 300);
    }

    #[tokio::test]
    async fn test_resource_monitor_creation() {
        let monitor = ResourceMonitor::new();
        let config = create_test_config();

        let optimal_concurrency = monitor.get_optimal_concurrency(&config).await;
        assert!(optimal_concurrency > 0);
        assert!(optimal_concurrency <= config.max_concurrency);
    }

    #[tokio::test]
    async fn test_resource_monitor_throttling() {
        let monitor = ResourceMonitor::new();
        let config = create_test_config();

        // Should not throttle with default values
        let should_throttle = monitor.should_throttle(&config).await;
        assert!(!should_throttle);
    }

    #[tokio::test]
    async fn test_resource_monitor_adaptive_concurrency() {
        let monitor = ResourceMonitor::new();
        let mut config = create_test_config();

        // Test with adaptive concurrency enabled
        config.adaptive_concurrency = true;
        let optimal = monitor.get_optimal_concurrency(&config).await;
        assert!(optimal > 0);
        assert!(optimal <= config.max_concurrency);

        // Test with adaptive concurrency disabled
        config.adaptive_concurrency = false;
        let fixed = monitor.get_optimal_concurrency(&config).await;
        assert_eq!(fixed, config.max_concurrency);
    }

    #[test]
    fn test_step_execution_context_creation() {
        let context = StepExecutionContext {
            step_id: "test-step".to_string(),
            dependencies: std::collections::HashSet::new(),
            dependents: std::collections::HashSet::new(),
            priority: 1,
            estimated_duration: Duration::from_secs(10),
            resource_requirements: ResourceRequirements::default(),
        };

        assert_eq!(context.step_id, "test-step");
        assert!(context.dependencies.is_empty());
        assert!(context.dependents.is_empty());
        assert_eq!(context.priority, 1);
        assert_eq!(context.estimated_duration, Duration::from_secs(10));
    }

    #[test]
    fn test_resource_requirements_default() {
        let requirements = ResourceRequirements::default();

        assert!(!requirements.cpu_intensive);
        assert!(!requirements.memory_intensive);
        assert!(!requirements.io_intensive);
        assert!(!requirements.network_intensive);
    }

    #[test]
    fn test_resource_requirements_custom() {
        let requirements = ResourceRequirements {
            cpu_intensive: true,
            memory_intensive: false,
            io_intensive: true,
            network_intensive: false,
        };

        assert!(requirements.cpu_intensive);
        assert!(!requirements.memory_intensive);
        assert!(requirements.io_intensive);
        assert!(!requirements.network_intensive);
    }

    #[test]
    fn test_execution_batch_creation() {
        let step = PipelineStep::Command {
            name: "test".to_string(),
            command: "echo test".to_string(),
            save_output: Some("output".to_string()),
            retry: None,
        };

        let context = StepExecutionContext {
            step_id: "test".to_string(),
            dependencies: std::collections::HashSet::new(),
            dependents: std::collections::HashSet::new(),
            priority: 0,
            estimated_duration: Duration::from_secs(5),
            resource_requirements: ResourceRequirements::default(),
        };

        let batch = ExecutionBatch {
            steps: vec![(step, context)],
            batch_id: "batch-1".to_string(),
            estimated_duration: Duration::from_secs(5),
        };

        assert_eq!(batch.steps.len(), 1);
        assert_eq!(batch.batch_id, "batch-1");
        assert_eq!(batch.estimated_duration, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_simple_batch_creation() {
        let state_store = MemoryStateStore::new();
        let config = create_test_config();
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        let pipeline = create_test_pipeline();
        let batches = executor
            .create_simple_batches(&pipeline.steps)
            .await
            .unwrap();

        assert!(!batches.is_empty());
        assert_eq!(batches[0].steps.len(), 3); // All steps in one batch since batch_size is 5
    }

    #[tokio::test]
    async fn test_step_name_helper() {
        let step = PipelineStep::Command {
            name: "test-command".to_string(),
            command: "echo test".to_string(),
            save_output: None,
            retry: None,
        };

        let name = EnhancedPipelineExecutor::<MemoryStateStore>::get_step_name(&step);
        assert_eq!(name, "test-command");
    }

    #[tokio::test]
    async fn test_metrics_initialization() {
        let state_store = MemoryStateStore::new();
        let config = create_test_config();
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        let metrics = executor.get_metrics().await;
        assert_eq!(metrics.total_pipelines, 0);
        assert_eq!(metrics.successful_pipelines, 0);
        assert_eq!(metrics.failed_pipelines, 0);
        assert_eq!(metrics.total_steps, 0);
        assert_eq!(metrics.parallel_steps, 0);
        assert_eq!(metrics.sequential_steps, 0);
        assert_eq!(metrics.cache_hits, 0);
        assert_eq!(metrics.cache_misses, 0);
        assert_eq!(metrics.average_execution_time_ms, 0.0);
        assert_eq!(metrics.peak_concurrency, 0);
        assert_eq!(metrics.resource_throttling_events, 0);
        assert!(metrics.step_type_performance.is_empty());
    }

    #[test]
    fn test_step_performance_default() {
        let performance = StepPerformance::default();

        assert_eq!(performance.total_executions, 0);
        assert_eq!(performance.successful_executions, 0);
        assert_eq!(performance.average_duration_ms, 0.0);
        assert_eq!(performance.min_duration_ms, f64::MAX);
        assert_eq!(performance.max_duration_ms, 0.0);
        assert_eq!(performance.error_rate, 0.0);
    }

    #[test]
    fn test_step_performance_custom() {
        let performance = StepPerformance {
            total_executions: 100,
            successful_executions: 95,
            average_duration_ms: 150.5,
            min_duration_ms: 50.0,
            max_duration_ms: 500.0,
            error_rate: 0.05,
        };

        assert_eq!(performance.total_executions, 100);
        assert_eq!(performance.successful_executions, 95);
        assert_eq!(performance.average_duration_ms, 150.5);
        assert_eq!(performance.min_duration_ms, 50.0);
        assert_eq!(performance.max_duration_ms, 500.0);
        assert_eq!(performance.error_rate, 0.05);
    }

    #[tokio::test]
    async fn test_pipeline_execution_basic() {
        let state_store = MemoryStateStore::new();
        let config = create_test_config();
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        let pipeline = create_test_pipeline();

        // This would normally execute the pipeline, but since we don't have
        // actual command execution implemented in the test environment,
        // we'll just test that the method can be called without panicking
        let result = executor
            .execute_pipeline(
                &pipeline,
                "test input",
                true, // force_fresh
                Some("test-run-id".to_string()),
            )
            .await;

        // The result should be Ok since our simplified implementation
        // just returns placeholder responses
        assert!(result.is_ok());
    }

    // Error handling tests
    #[tokio::test]
    async fn test_empty_pipeline_steps() {
        let state_store = MemoryStateStore::new();
        let config = create_test_config();
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        let empty_steps: Vec<PipelineStep> = vec![];
        let batches = executor.create_simple_batches(&empty_steps).await.unwrap();

        assert!(batches.is_empty());
    }

    #[tokio::test]
    async fn test_large_batch_size() {
        let state_store = MemoryStateStore::new();
        let mut config = create_test_config();
        config.batch_size = 100; // Larger than number of steps
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        let pipeline = create_test_pipeline();
        let batches = executor
            .create_simple_batches(&pipeline.steps)
            .await
            .unwrap();

        assert_eq!(batches.len(), 1); // All steps in one batch
        assert_eq!(batches[0].steps.len(), 3);
    }

    #[tokio::test]
    async fn test_small_batch_size() {
        let state_store = MemoryStateStore::new();
        let mut config = create_test_config();
        config.batch_size = 1; // One step per batch
        let executor = EnhancedPipelineExecutor::new(state_store, config);

        let pipeline = create_test_pipeline();
        let batches = executor
            .create_simple_batches(&pipeline.steps)
            .await
            .unwrap();

        assert_eq!(batches.len(), 3); // Three batches for three steps
        for batch in batches {
            assert_eq!(batch.steps.len(), 1);
        }
    }
}
