use std::time::Duration;

pub mod cache;
pub mod connection_pool;

/// Performance configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    pub connection_pool: ConnectionPoolConfig,
    pub cache: CacheConfig,
    pub batch: BatchConfig,
    pub metrics: MetricsConfig,
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            min_connections: 10,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(3600),
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub l1_max_capacity: u64,
    pub l1_ttl: Duration,
    pub l2_enabled: bool,
    pub l2_url: Option<String>,
    pub l2_ttl: Duration,
    pub l3_enabled: bool,
    pub l3_database_url: Option<String>,
    pub l3_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_max_capacity: 10000,
            l1_ttl: Duration::from_secs(300),
            l2_enabled: false,
            l2_url: None,
            l2_ttl: Duration::from_secs(3600),
            l3_enabled: false,
            l3_database_url: None,
            l3_ttl: Duration::from_secs(86400),
        }
    }
}

/// Batch processing configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub batch_timeout: Duration,
    pub max_concurrent_batches: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            batch_timeout: Duration::from_millis(100),
            max_concurrent_batches: 10,
        }
    }
}

/// Metrics configuration
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub collection_interval: Duration,
    pub export_endpoint: Option<String>,
    pub histogram_buckets: Vec<f64>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Duration::from_secs(60),
            export_endpoint: None,
            histogram_buckets: vec![0.001, 0.01, 0.1, 1.0, 10.0],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            connection_pool: ConnectionPoolConfig::default(),
            cache: CacheConfig::default(),
            batch: BatchConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

/// Performance optimization utilities
pub mod utils {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Instant;

    /// Performance counter for tracking metrics
    pub struct PerformanceCounter {
        requests: AtomicU64,
        errors: AtomicU64,
        total_duration: AtomicU64,
        last_reset: std::sync::Mutex<Instant>,
    }

    impl PerformanceCounter {
        pub fn new() -> Self {
            Self {
                requests: AtomicU64::new(0),
                errors: AtomicU64::new(0),
                total_duration: AtomicU64::new(0),
                last_reset: std::sync::Mutex::new(Instant::now()),
            }
        }

        pub fn record_request(&self, duration: Duration, is_error: bool) {
            self.requests.fetch_add(1, Ordering::Relaxed);
            self.total_duration
                .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);

            if is_error {
                self.errors.fetch_add(1, Ordering::Relaxed);
            }
        }

        pub fn get_stats(&self) -> PerformanceStats {
            let requests = self.requests.load(Ordering::Relaxed);
            let errors = self.errors.load(Ordering::Relaxed);
            let total_duration = self.total_duration.load(Ordering::Relaxed);

            let avg_duration = if requests > 0 {
                Duration::from_millis(total_duration / requests)
            } else {
                Duration::ZERO
            };

            let error_rate = if requests > 0 {
                (errors as f64) / (requests as f64)
            } else {
                0.0
            };

            PerformanceStats {
                total_requests: requests,
                total_errors: errors,
                error_rate,
                average_duration: avg_duration,
            }
        }

        pub fn reset(&self) {
            self.requests.store(0, Ordering::Relaxed);
            self.errors.store(0, Ordering::Relaxed);
            self.total_duration.store(0, Ordering::Relaxed);
            *self.last_reset.lock().unwrap() = Instant::now();
        }
    }

    impl Default for PerformanceCounter {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Performance statistics
    #[derive(Debug, Clone)]
    pub struct PerformanceStats {
        pub total_requests: u64,
        pub total_errors: u64,
        pub error_rate: f64,
        pub average_duration: Duration,
    }

    /// Memory usage tracker
    pub struct MemoryTracker {
        peak_usage: AtomicU64,
        current_usage: AtomicU64,
    }

    impl MemoryTracker {
        pub fn new() -> Self {
            Self {
                peak_usage: AtomicU64::new(0),
                current_usage: AtomicU64::new(0),
            }
        }

        pub fn allocate(&self, size: u64) {
            let new_usage = self.current_usage.fetch_add(size, Ordering::Relaxed) + size;

            // Update peak usage if necessary
            let mut peak = self.peak_usage.load(Ordering::Relaxed);
            while new_usage > peak {
                match self.peak_usage.compare_exchange_weak(
                    peak,
                    new_usage,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(current) => peak = current,
                }
            }
        }

        pub fn deallocate(&self, size: u64) {
            self.current_usage.fetch_sub(size, Ordering::Relaxed);
        }

        pub fn get_current_usage(&self) -> u64 {
            self.current_usage.load(Ordering::Relaxed)
        }

        pub fn get_peak_usage(&self) -> u64 {
            self.peak_usage.load(Ordering::Relaxed)
        }

        pub fn reset_peak(&self) {
            let current = self.current_usage.load(Ordering::Relaxed);
            self.peak_usage.store(current, Ordering::Relaxed);
        }
    }

    impl Default for MemoryTracker {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Resource limiter for controlling resource usage
    pub struct ResourceLimiter {
        max_memory: u64,
        max_connections: usize,
        current_memory: AtomicU64,
        current_connections: AtomicU64,
    }

    impl ResourceLimiter {
        pub fn new(max_memory: u64, max_connections: usize) -> Self {
            Self {
                max_memory,
                max_connections,
                current_memory: AtomicU64::new(0),
                current_connections: AtomicU64::new(0),
            }
        }

        pub fn try_allocate_memory(&self, size: u64) -> bool {
            let current = self.current_memory.load(Ordering::Relaxed);
            if current + size <= self.max_memory {
                self.current_memory.fetch_add(size, Ordering::Relaxed);
                true
            } else {
                false
            }
        }

        pub fn deallocate_memory(&self, size: u64) {
            self.current_memory.fetch_sub(size, Ordering::Relaxed);
        }

        pub fn try_acquire_connection(&self) -> bool {
            let current = self.current_connections.load(Ordering::Relaxed);
            if current < self.max_connections as u64 {
                self.current_connections.fetch_add(1, Ordering::Relaxed);
                true
            } else {
                false
            }
        }

        pub fn release_connection(&self) {
            self.current_connections.fetch_sub(1, Ordering::Relaxed);
        }

        pub fn get_memory_usage(&self) -> (u64, u64) {
            (self.current_memory.load(Ordering::Relaxed), self.max_memory)
        }

        pub fn get_connection_usage(&self) -> (u64, usize) {
            (
                self.current_connections.load(Ordering::Relaxed),
                self.max_connections,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;

    #[test]
    fn test_performance_config_defaults() {
        let config = PerformanceConfig::default();
        assert_eq!(config.connection_pool.max_connections, 100);
        assert_eq!(config.cache.l1_max_capacity, 10000);
        assert_eq!(config.batch.max_batch_size, 100);
        assert!(config.metrics.enabled);
    }

    #[test]
    fn test_performance_counter() {
        let counter = PerformanceCounter::new();

        counter.record_request(Duration::from_millis(100), false);
        counter.record_request(Duration::from_millis(200), true);

        let stats = counter.get_stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.error_rate, 0.5);
        assert_eq!(stats.average_duration, Duration::from_millis(150));
    }

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new();

        tracker.allocate(1000);
        assert_eq!(tracker.get_current_usage(), 1000);
        assert_eq!(tracker.get_peak_usage(), 1000);

        tracker.allocate(500);
        assert_eq!(tracker.get_current_usage(), 1500);
        assert_eq!(tracker.get_peak_usage(), 1500);

        tracker.deallocate(200);
        assert_eq!(tracker.get_current_usage(), 1300);
        assert_eq!(tracker.get_peak_usage(), 1500);
    }

    #[test]
    fn test_resource_limiter() {
        let limiter = ResourceLimiter::new(1000, 5);

        assert!(limiter.try_allocate_memory(500));
        assert!(limiter.try_allocate_memory(400));
        assert!(!limiter.try_allocate_memory(200)); // Would exceed limit

        assert!(limiter.try_acquire_connection());
        assert!(limiter.try_acquire_connection());

        let (memory_used, memory_max) = limiter.get_memory_usage();
        assert_eq!(memory_used, 900);
        assert_eq!(memory_max, 1000);

        let (conn_used, conn_max) = limiter.get_connection_usage();
        assert_eq!(conn_used, 2);
        assert_eq!(conn_max, 5);
    }
}
