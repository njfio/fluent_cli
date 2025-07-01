use crate::optimized_state_store::{OptimizedStateStore, StateStoreConfig};
use crate::pipeline_executor::{FileStateStore, PipelineState, StateStore};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

/// Benchmark results for state store performance
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub store_type: String,
    pub operations: usize,
    pub total_time: Duration,
    pub avg_time_per_op: Duration,
    pub ops_per_second: f64,
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub num_operations: usize,
    pub state_size: usize, // Number of data entries per state
    pub concurrent_operations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            num_operations: 1000,
            state_size: 10,
            concurrent_operations: 1,
        }
    }
}

/// State store performance benchmarker
pub struct StateStoreBenchmark;

impl StateStoreBenchmark {
    /// Run comprehensive benchmarks comparing different state stores
    pub async fn run_comparison(config: BenchmarkConfig) -> Result<Vec<BenchmarkResults>> {
        let mut results = Vec::new();

        // Benchmark FileStateStore
        let file_results = Self::benchmark_file_store(&config).await?;
        results.push(file_results);

        // Benchmark OptimizedStateStore with different configurations
        let optimized_results = Self::benchmark_optimized_store(&config, StateStoreConfig::default()).await?;
        results.push(optimized_results);

        // Benchmark OptimizedStateStore with write-through disabled
        let mut write_back_config = StateStoreConfig::default();
        write_back_config.write_through = false;
        let write_back_results = Self::benchmark_optimized_store(&config, write_back_config).await?;
        results.push(write_back_results);

        // Benchmark OptimizedStateStore with compression disabled
        let mut no_compression_config = StateStoreConfig::default();
        no_compression_config.enable_compression = false;
        let no_compression_results = Self::benchmark_optimized_store(&config, no_compression_config).await?;
        results.push(no_compression_results);

        Ok(results)
    }

    /// Benchmark the original FileStateStore
    async fn benchmark_file_store(config: &BenchmarkConfig) -> Result<BenchmarkResults> {
        let temp_dir = TempDir::new()?;
        let store = FileStateStore {
            directory: temp_dir.path().to_path_buf(),
        };

        let start_time = Instant::now();
        
        for i in 0..config.num_operations {
            let state = Self::create_test_state(i, config.state_size);
            let key = format!("test-key-{}", i);
            
            // Save and then load to simulate real usage
            store.save_state(&key, &state).await?;
            let _loaded = store.load_state(&key).await?;
        }

        let total_time = start_time.elapsed();
        let ops_per_second = (config.num_operations * 2) as f64 / total_time.as_secs_f64(); // *2 for save+load

        Ok(BenchmarkResults {
            store_type: "FileStateStore".to_string(),
            operations: config.num_operations * 2,
            total_time,
            avg_time_per_op: total_time / (config.num_operations * 2) as u32,
            ops_per_second,
        })
    }

    /// Benchmark the OptimizedStateStore
    async fn benchmark_optimized_store(
        config: &BenchmarkConfig,
        store_config: StateStoreConfig,
    ) -> Result<BenchmarkResults> {
        let temp_dir = TempDir::new()?;
        let store = OptimizedStateStore::new(temp_dir.path().to_path_buf(), store_config.clone())?;

        let store_type = format!(
            "OptimizedStateStore(write_through={}, compression={})",
            store_config.write_through,
            store_config.enable_compression
        );

        let start_time = Instant::now();
        
        for i in 0..config.num_operations {
            let state = Self::create_test_state(i, config.state_size);
            let key = format!("test-key-{}", i);
            
            // Save and then load to simulate real usage
            store.save_state(&key, &state).await?;
            let _loaded = store.load_state(&key).await?;
        }

        // Force flush to ensure all data is written
        store.force_flush().await?;

        let total_time = start_time.elapsed();
        let ops_per_second = (config.num_operations * 2) as f64 / total_time.as_secs_f64(); // *2 for save+load

        Ok(BenchmarkResults {
            store_type,
            operations: config.num_operations * 2,
            total_time,
            avg_time_per_op: total_time / (config.num_operations * 2) as u32,
            ops_per_second,
        })
    }

    /// Benchmark concurrent operations
    pub async fn benchmark_concurrent_operations(config: BenchmarkConfig) -> Result<Vec<BenchmarkResults>> {
        let mut results = Vec::new();

        // Test FileStateStore with concurrent operations
        let file_results = Self::benchmark_file_store_concurrent(&config).await?;
        results.push(file_results);

        // Test OptimizedStateStore with concurrent operations
        let optimized_results = Self::benchmark_optimized_store_concurrent(&config).await?;
        results.push(optimized_results);

        Ok(results)
    }

    async fn benchmark_file_store_concurrent(config: &BenchmarkConfig) -> Result<BenchmarkResults> {
        let temp_dir = TempDir::new()?;
        let store = FileStateStore {
            directory: temp_dir.path().to_path_buf(),
        };

        let start_time = Instant::now();
        let mut handles = Vec::new();

        let ops_per_task = config.num_operations / config.concurrent_operations;
        
        for task_id in 0..config.concurrent_operations {
            let store_clone = store.clone();
            let state_size = config.state_size;
            
            let handle = tokio::spawn(async move {
                for i in 0..ops_per_task {
                    let state = Self::create_test_state(task_id * ops_per_task + i, state_size);
                    let key = format!("test-key-{}-{}", task_id, i);
                    
                    store_clone.save_state(&key, &state).await?;
                    let _loaded = store_clone.load_state(&key).await?;
                }
                Ok::<(), anyhow::Error>(())
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await??;
        }

        let total_time = start_time.elapsed();
        let total_ops = config.num_operations * 2; // *2 for save+load
        let ops_per_second = total_ops as f64 / total_time.as_secs_f64();

        Ok(BenchmarkResults {
            store_type: format!("FileStateStore(concurrent={})", config.concurrent_operations),
            operations: total_ops,
            total_time,
            avg_time_per_op: total_time / total_ops as u32,
            ops_per_second,
        })
    }

    async fn benchmark_optimized_store_concurrent(config: &BenchmarkConfig) -> Result<BenchmarkResults> {
        let temp_dir = TempDir::new()?;
        let store = Arc::new(OptimizedStateStore::with_defaults(temp_dir.path().to_path_buf())?);

        let start_time = Instant::now();
        let mut handles = Vec::new();

        let ops_per_task = config.num_operations / config.concurrent_operations;

        for task_id in 0..config.concurrent_operations {
            let store_clone = Arc::clone(&store);
            let state_size = config.state_size;
            
            let handle = tokio::spawn(async move {
                for i in 0..ops_per_task {
                    let state = Self::create_test_state(task_id * ops_per_task + i, state_size);
                    let key = format!("test-key-{}-{}", task_id, i);
                    
                    store_clone.save_state(&key, &state).await?;
                    let _loaded = store_clone.load_state(&key).await?;
                }
                Ok::<(), anyhow::Error>(())
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await??;
        }

        let total_time = start_time.elapsed();
        let total_ops = config.num_operations * 2; // *2 for save+load
        let ops_per_second = total_ops as f64 / total_time.as_secs_f64();

        Ok(BenchmarkResults {
            store_type: format!("OptimizedStateStore(concurrent={})", config.concurrent_operations),
            operations: total_ops,
            total_time,
            avg_time_per_op: total_time / total_ops as u32,
            ops_per_second,
        })
    }

    /// Create a test state with specified size
    fn create_test_state(index: usize, data_size: usize) -> PipelineState {
        let mut data = HashMap::new();
        
        for i in 0..data_size {
            data.insert(
                format!("key_{}", i),
                format!("value_{}_{}", index, i),
            );
        }

        PipelineState {
            current_step: index % 10,
            data,
            run_id: format!("run-{}", index),
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Print benchmark results in a formatted table
    pub fn print_results(results: &[BenchmarkResults]) {
        println!("\n=== State Store Performance Benchmark Results ===");
        println!("{:<40} {:<10} {:<12} {:<15} {:<12}", 
                 "Store Type", "Ops", "Total Time", "Avg Time/Op", "Ops/Sec");
        println!("{}", "-".repeat(90));

        for result in results {
            println!("{:<40} {:<10} {:<12.2?} {:<15.2?} {:<12.1}",
                     result.store_type,
                     result.operations,
                     result.total_time,
                     result.avg_time_per_op,
                     result.ops_per_second);
        }
        println!();
    }

    /// Run a quick performance test
    pub async fn quick_test() -> Result<()> {
        let config = BenchmarkConfig {
            num_operations: 100,
            state_size: 5,
            concurrent_operations: 1,
        };

        println!("Running quick state store performance test...");
        let results = Self::run_comparison(config).await?;
        Self::print_results(&results);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_execution() {
        let config = BenchmarkConfig {
            num_operations: 10,
            state_size: 3,
            concurrent_operations: 1,
        };

        let results = StateStoreBenchmark::run_comparison(config).await.unwrap();
        assert!(!results.is_empty());
        
        // Verify all benchmarks completed
        for result in &results {
            assert!(result.operations > 0);
            assert!(result.total_time.as_nanos() > 0);
            assert!(result.ops_per_second > 0.0);
        }
    }

    #[tokio::test]
    async fn test_concurrent_benchmark() {
        let config = BenchmarkConfig {
            num_operations: 20,
            state_size: 2,
            concurrent_operations: 4,
        };

        let results = StateStoreBenchmark::benchmark_concurrent_operations(config).await.unwrap();
        assert_eq!(results.len(), 2); // FileStateStore + OptimizedStateStore
    }
}
