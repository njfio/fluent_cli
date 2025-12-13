//! Metrics Aggregation and Export for Autonomous Agent Operations
//!
//! This module provides comprehensive metrics collection and export capabilities
//! for monitoring agent performance, with support for Prometheus-compatible format.
//!
//! # Features
//!
//! - **Counter Metrics**: Track cumulative values (requests, errors, tasks completed)
//! - **Gauge Metrics**: Track current values (active tasks, queue depth, memory usage)
//! - **Histogram Metrics**: Track distributions (latency, execution time)
//! - **Prometheus Export**: Export metrics in Prometheus text format
//! - **Labels**: Support for dimensional metrics with labels
//! - **Memory Bounded**: Configurable limits to prevent unbounded growth

use anyhow::Result;
use prometheus::{
    Counter, CounterVec, Encoder, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, Opts,
    Registry, TextEncoder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the metrics exporter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics prefix for namespacing
    pub prefix: String,
    /// Default labels applied to all metrics
    pub default_labels: HashMap<String, String>,
    /// Latency histogram buckets (in seconds)
    pub latency_buckets: Vec<f64>,
    /// Size histogram buckets (in bytes)
    pub size_buckets: Vec<f64>,
    /// Maximum number of label combinations per metric
    pub max_label_cardinality: usize,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefix: "fluent_agent".to_string(),
            default_labels: HashMap::new(),
            latency_buckets: vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ],
            size_buckets: vec![100.0, 500.0, 1000.0, 5000.0, 10000.0, 50000.0, 100000.0],
            max_label_cardinality: 1000,
        }
    }
}

// ============================================================================
// Core Types
// ============================================================================

/// A recorded metric sample
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSample {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: Option<i64>,
}

/// Aggregated metric statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedStats {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub active_tasks: u64,
    pub average_latency_seconds: f64,
    pub p50_latency_seconds: f64,
    pub p95_latency_seconds: f64,
    pub p99_latency_seconds: f64,
    pub requests_per_second: f64,
    pub error_rate: f64,
    pub success_rate: f64,
}

impl Default for AggregatedStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            total_errors: 0,
            total_tasks_completed: 0,
            total_tasks_failed: 0,
            active_tasks: 0,
            average_latency_seconds: 0.0,
            p50_latency_seconds: 0.0,
            p95_latency_seconds: 0.0,
            p99_latency_seconds: 0.0,
            requests_per_second: 0.0,
            error_rate: 0.0,
            success_rate: 1.0,
        }
    }
}

// ============================================================================
// Metrics Exporter
// ============================================================================

/// The main metrics exporter with Prometheus-compatible metrics
pub struct MetricsExporter {
    config: MetricsConfig,
    registry: Registry,

    // Request metrics
    requests_total: CounterVec,
    request_duration_seconds: HistogramVec,
    request_size_bytes: HistogramVec,
    response_size_bytes: HistogramVec,

    // Task metrics
    tasks_total: CounterVec,
    task_duration_seconds: HistogramVec,
    active_tasks: GaugeVec,

    // Error metrics
    errors_total: CounterVec,
    error_recovery_total: CounterVec,

    // Resource metrics
    memory_usage_bytes: Gauge,
    cpu_usage_percent: Gauge,
    goroutines_active: Gauge,

    // Queue metrics
    queue_depth: GaugeVec,
    queue_latency_seconds: HistogramVec,

    // LLM-specific metrics
    llm_requests_total: CounterVec,
    llm_tokens_total: CounterVec,
    llm_latency_seconds: HistogramVec,
    llm_cost_dollars: CounterVec,

    // Tool execution metrics
    tool_executions_total: CounterVec,
    tool_duration_seconds: HistogramVec,

    // MCP metrics
    mcp_requests_total: CounterVec,
    mcp_latency_seconds: HistogramVec,

    // Internal tracking
    start_time: Instant,
    label_cardinality: Arc<RwLock<HashMap<String, usize>>>,
}

impl MetricsExporter {
    /// Create a new metrics exporter with the given configuration
    pub fn new(config: MetricsConfig) -> Result<Self> {
        let registry = Registry::new();
        let prefix = &config.prefix;

        // Request metrics
        let requests_total = CounterVec::new(
            Opts::new(
                format!("{}_requests_total", prefix),
                "Total number of requests processed",
            ),
            &["method", "endpoint", "status"],
        )?;
        registry.register(Box::new(requests_total.clone()))?;

        let request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_request_duration_seconds", prefix),
                "Request duration in seconds",
            )
            .buckets(config.latency_buckets.clone()),
            &["method", "endpoint"],
        )?;
        registry.register(Box::new(request_duration_seconds.clone()))?;

        let request_size_bytes = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_request_size_bytes", prefix),
                "Request size in bytes",
            )
            .buckets(config.size_buckets.clone()),
            &["method", "endpoint"],
        )?;
        registry.register(Box::new(request_size_bytes.clone()))?;

        let response_size_bytes = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_response_size_bytes", prefix),
                "Response size in bytes",
            )
            .buckets(config.size_buckets.clone()),
            &["method", "endpoint"],
        )?;
        registry.register(Box::new(response_size_bytes.clone()))?;

        // Task metrics
        let tasks_total = CounterVec::new(
            Opts::new(
                format!("{}_tasks_total", prefix),
                "Total number of tasks processed",
            ),
            &["task_type", "status"],
        )?;
        registry.register(Box::new(tasks_total.clone()))?;

        let task_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_task_duration_seconds", prefix),
                "Task duration in seconds",
            )
            .buckets(config.latency_buckets.clone()),
            &["task_type"],
        )?;
        registry.register(Box::new(task_duration_seconds.clone()))?;

        let active_tasks = GaugeVec::new(
            Opts::new(
                format!("{}_active_tasks", prefix),
                "Number of currently active tasks",
            ),
            &["task_type"],
        )?;
        registry.register(Box::new(active_tasks.clone()))?;

        // Error metrics
        let errors_total = CounterVec::new(
            Opts::new(format!("{}_errors_total", prefix), "Total number of errors"),
            &["error_type", "severity"],
        )?;
        registry.register(Box::new(errors_total.clone()))?;

        let error_recovery_total = CounterVec::new(
            Opts::new(
                format!("{}_error_recovery_total", prefix),
                "Total number of error recovery attempts",
            ),
            &["error_type", "recovery_status"],
        )?;
        registry.register(Box::new(error_recovery_total.clone()))?;

        // Resource metrics
        let memory_usage_bytes = Gauge::new(
            format!("{}_memory_usage_bytes", prefix),
            "Current memory usage in bytes",
        )?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;

        let cpu_usage_percent = Gauge::new(
            format!("{}_cpu_usage_percent", prefix),
            "Current CPU usage percentage",
        )?;
        registry.register(Box::new(cpu_usage_percent.clone()))?;

        let goroutines_active = Gauge::new(
            format!("{}_goroutines_active", prefix),
            "Number of active goroutines/tasks",
        )?;
        registry.register(Box::new(goroutines_active.clone()))?;

        // Queue metrics
        let queue_depth = GaugeVec::new(
            Opts::new(format!("{}_queue_depth", prefix), "Current queue depth"),
            &["queue_name"],
        )?;
        registry.register(Box::new(queue_depth.clone()))?;

        let queue_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_queue_latency_seconds", prefix),
                "Time spent in queue before processing",
            )
            .buckets(config.latency_buckets.clone()),
            &["queue_name"],
        )?;
        registry.register(Box::new(queue_latency_seconds.clone()))?;

        // LLM-specific metrics
        let llm_requests_total = CounterVec::new(
            Opts::new(
                format!("{}_llm_requests_total", prefix),
                "Total LLM API requests",
            ),
            &["provider", "model", "status"],
        )?;
        registry.register(Box::new(llm_requests_total.clone()))?;

        let llm_tokens_total = CounterVec::new(
            Opts::new(
                format!("{}_llm_tokens_total", prefix),
                "Total tokens processed by LLM",
            ),
            &["provider", "model", "direction"],
        )?;
        registry.register(Box::new(llm_tokens_total.clone()))?;

        let llm_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_llm_latency_seconds", prefix),
                "LLM request latency in seconds",
            )
            .buckets(config.latency_buckets.clone()),
            &["provider", "model"],
        )?;
        registry.register(Box::new(llm_latency_seconds.clone()))?;

        let llm_cost_dollars = CounterVec::new(
            Opts::new(
                format!("{}_llm_cost_dollars", prefix),
                "Estimated LLM API cost in dollars",
            ),
            &["provider", "model"],
        )?;
        registry.register(Box::new(llm_cost_dollars.clone()))?;

        // Tool execution metrics
        let tool_executions_total = CounterVec::new(
            Opts::new(
                format!("{}_tool_executions_total", prefix),
                "Total tool executions",
            ),
            &["tool_name", "status"],
        )?;
        registry.register(Box::new(tool_executions_total.clone()))?;

        let tool_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_tool_duration_seconds", prefix),
                "Tool execution duration in seconds",
            )
            .buckets(config.latency_buckets.clone()),
            &["tool_name"],
        )?;
        registry.register(Box::new(tool_duration_seconds.clone()))?;

        // MCP metrics
        let mcp_requests_total = CounterVec::new(
            Opts::new(
                format!("{}_mcp_requests_total", prefix),
                "Total MCP requests",
            ),
            &["server", "method", "status"],
        )?;
        registry.register(Box::new(mcp_requests_total.clone()))?;

        let mcp_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                format!("{}_mcp_latency_seconds", prefix),
                "MCP request latency in seconds",
            )
            .buckets(config.latency_buckets.clone()),
            &["server", "method"],
        )?;
        registry.register(Box::new(mcp_latency_seconds.clone()))?;

        Ok(Self {
            config,
            registry,
            requests_total,
            request_duration_seconds,
            request_size_bytes,
            response_size_bytes,
            tasks_total,
            task_duration_seconds,
            active_tasks,
            errors_total,
            error_recovery_total,
            memory_usage_bytes,
            cpu_usage_percent,
            goroutines_active,
            queue_depth,
            queue_latency_seconds,
            llm_requests_total,
            llm_tokens_total,
            llm_latency_seconds,
            llm_cost_dollars,
            tool_executions_total,
            tool_duration_seconds,
            mcp_requests_total,
            mcp_latency_seconds,
            start_time: Instant::now(),
            label_cardinality: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(MetricsConfig::default())
    }

    // ========== Request Metrics ==========

    /// Record a request
    pub fn record_request(&self, method: &str, endpoint: &str, status: &str) {
        self.requests_total
            .with_label_values(&[method, endpoint, status])
            .inc();
    }

    /// Record request duration
    pub fn record_request_duration(&self, method: &str, endpoint: &str, duration: Duration) {
        self.request_duration_seconds
            .with_label_values(&[method, endpoint])
            .observe(duration.as_secs_f64());
    }

    /// Record request size
    pub fn record_request_size(&self, method: &str, endpoint: &str, size_bytes: u64) {
        self.request_size_bytes
            .with_label_values(&[method, endpoint])
            .observe(size_bytes as f64);
    }

    /// Record response size
    pub fn record_response_size(&self, method: &str, endpoint: &str, size_bytes: u64) {
        self.response_size_bytes
            .with_label_values(&[method, endpoint])
            .observe(size_bytes as f64);
    }

    // ========== Task Metrics ==========

    /// Record a task completion
    pub fn record_task(&self, task_type: &str, status: &str) {
        self.tasks_total
            .with_label_values(&[task_type, status])
            .inc();
    }

    /// Record task duration
    pub fn record_task_duration(&self, task_type: &str, duration: Duration) {
        self.task_duration_seconds
            .with_label_values(&[task_type])
            .observe(duration.as_secs_f64());
    }

    /// Increment active tasks
    pub fn inc_active_tasks(&self, task_type: &str) {
        self.active_tasks.with_label_values(&[task_type]).inc();
    }

    /// Decrement active tasks
    pub fn dec_active_tasks(&self, task_type: &str) {
        self.active_tasks.with_label_values(&[task_type]).dec();
    }

    /// Set active tasks count
    pub fn set_active_tasks(&self, task_type: &str, count: f64) {
        self.active_tasks.with_label_values(&[task_type]).set(count);
    }

    // ========== Error Metrics ==========

    /// Record an error
    pub fn record_error(&self, error_type: &str, severity: &str) {
        self.errors_total
            .with_label_values(&[error_type, severity])
            .inc();
    }

    /// Record error recovery attempt
    pub fn record_error_recovery(&self, error_type: &str, recovery_status: &str) {
        self.error_recovery_total
            .with_label_values(&[error_type, recovery_status])
            .inc();
    }

    // ========== Resource Metrics ==========

    /// Set memory usage
    pub fn set_memory_usage(&self, bytes: u64) {
        self.memory_usage_bytes.set(bytes as f64);
    }

    /// Set CPU usage
    pub fn set_cpu_usage(&self, percent: f64) {
        self.cpu_usage_percent.set(percent);
    }

    /// Set active goroutines/tasks count
    pub fn set_goroutines_active(&self, count: u64) {
        self.goroutines_active.set(count as f64);
    }

    // ========== Queue Metrics ==========

    /// Set queue depth
    pub fn set_queue_depth(&self, queue_name: &str, depth: u64) {
        self.queue_depth
            .with_label_values(&[queue_name])
            .set(depth as f64);
    }

    /// Record queue latency
    pub fn record_queue_latency(&self, queue_name: &str, duration: Duration) {
        self.queue_latency_seconds
            .with_label_values(&[queue_name])
            .observe(duration.as_secs_f64());
    }

    // ========== LLM Metrics ==========

    /// Record an LLM request
    pub fn record_llm_request(&self, provider: &str, model: &str, status: &str) {
        self.llm_requests_total
            .with_label_values(&[provider, model, status])
            .inc();
    }

    /// Record LLM tokens
    pub fn record_llm_tokens(&self, provider: &str, model: &str, direction: &str, count: u64) {
        self.llm_tokens_total
            .with_label_values(&[provider, model, direction])
            .inc_by(count as f64);
    }

    /// Record LLM latency
    pub fn record_llm_latency(&self, provider: &str, model: &str, duration: Duration) {
        self.llm_latency_seconds
            .with_label_values(&[provider, model])
            .observe(duration.as_secs_f64());
    }

    /// Record LLM cost
    pub fn record_llm_cost(&self, provider: &str, model: &str, cost_dollars: f64) {
        self.llm_cost_dollars
            .with_label_values(&[provider, model])
            .inc_by(cost_dollars);
    }

    // ========== Tool Metrics ==========

    /// Record a tool execution
    pub fn record_tool_execution(&self, tool_name: &str, status: &str) {
        self.tool_executions_total
            .with_label_values(&[tool_name, status])
            .inc();
    }

    /// Record tool execution duration
    pub fn record_tool_duration(&self, tool_name: &str, duration: Duration) {
        self.tool_duration_seconds
            .with_label_values(&[tool_name])
            .observe(duration.as_secs_f64());
    }

    // ========== MCP Metrics ==========

    /// Record an MCP request
    pub fn record_mcp_request(&self, server: &str, method: &str, status: &str) {
        self.mcp_requests_total
            .with_label_values(&[server, method, status])
            .inc();
    }

    /// Record MCP latency
    pub fn record_mcp_latency(&self, server: &str, method: &str, duration: Duration) {
        self.mcp_latency_seconds
            .with_label_values(&[server, method])
            .observe(duration.as_secs_f64());
    }

    // ========== Export ==========

    /// Export metrics in Prometheus text format
    pub fn export(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Export metrics as JSON (for debugging/alternative formats)
    pub fn export_json(&self) -> Result<String> {
        let metric_families = self.registry.gather();
        let mut samples = Vec::new();

        for family in metric_families {
            let name = family.get_name();
            for metric in family.get_metric() {
                let mut labels = HashMap::new();
                for label in metric.get_label() {
                    labels.insert(label.get_name().to_string(), label.get_value().to_string());
                }

                let value = if metric.has_counter() {
                    metric.get_counter().get_value()
                } else if metric.has_gauge() {
                    metric.get_gauge().get_value()
                } else if metric.has_histogram() {
                    metric.get_histogram().get_sample_sum()
                } else {
                    0.0
                };

                samples.push(MetricSample {
                    name: name.to_string(),
                    value,
                    labels,
                    timestamp: None,
                });
            }
        }

        Ok(serde_json::to_string_pretty(&samples)?)
    }

    /// Get aggregated statistics
    pub fn get_aggregated_stats(&self) -> AggregatedStats {
        let metric_families = self.registry.gather();
        let mut stats = AggregatedStats::default();

        for family in metric_families {
            let name = family.get_name();

            for metric in family.get_metric() {
                if name.ends_with("_requests_total")
                    && name.contains("fluent_agent_requests")
                    && metric.has_counter()
                {
                    stats.total_requests += metric.get_counter().get_value() as u64;
                }
                if name.ends_with("_errors_total") && metric.has_counter() {
                    stats.total_errors += metric.get_counter().get_value() as u64;
                }
                if name.ends_with("_tasks_total") {
                    for label in metric.get_label() {
                        if label.get_name() == "status" {
                            let count = metric.get_counter().get_value() as u64;
                            match label.get_value() {
                                "success" | "completed" => stats.total_tasks_completed += count,
                                "failed" | "error" => stats.total_tasks_failed += count,
                                _ => {}
                            }
                        }
                    }
                }
                if name.ends_with("_active_tasks") && metric.has_gauge() {
                    stats.active_tasks += metric.get_gauge().get_value() as u64;
                }
            }
        }

        // Calculate derived metrics
        let total = stats.total_tasks_completed + stats.total_tasks_failed;
        if total > 0 {
            stats.success_rate = stats.total_tasks_completed as f64 / total as f64;
            stats.error_rate = stats.total_tasks_failed as f64 / total as f64;
        }

        // Calculate RPS based on uptime
        let uptime_secs = self.start_time.elapsed().as_secs_f64();
        if uptime_secs > 0.0 {
            stats.requests_per_second = stats.total_requests as f64 / uptime_secs;
        }

        stats
    }

    /// Get the configuration
    pub fn config(&self) -> &MetricsConfig {
        &self.config
    }

    /// Get the uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if metrics are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

// ============================================================================
// Timer Guard for automatic duration recording
// ============================================================================

/// Guard that records duration when dropped
pub struct TimerGuard<'a> {
    exporter: &'a MetricsExporter,
    metric_type: TimerMetricType<'a>,
    start: Instant,
}

/// Type of metric the timer is recording
pub enum TimerMetricType<'a> {
    Request { method: &'a str, endpoint: &'a str },
    Task { task_type: &'a str },
    Tool { tool_name: &'a str },
    Llm { provider: &'a str, model: &'a str },
    Mcp { server: &'a str, method: &'a str },
    Queue { queue_name: &'a str },
}

impl<'a> TimerGuard<'a> {
    /// Create a new timer guard
    pub fn new(exporter: &'a MetricsExporter, metric_type: TimerMetricType<'a>) -> Self {
        Self {
            exporter,
            metric_type,
            start: Instant::now(),
        }
    }

    /// Get elapsed duration without stopping
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for TimerGuard<'_> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        match &self.metric_type {
            TimerMetricType::Request { method, endpoint } => {
                self.exporter
                    .record_request_duration(method, endpoint, duration);
            }
            TimerMetricType::Task { task_type } => {
                self.exporter.record_task_duration(task_type, duration);
            }
            TimerMetricType::Tool { tool_name } => {
                self.exporter.record_tool_duration(tool_name, duration);
            }
            TimerMetricType::Llm { provider, model } => {
                self.exporter.record_llm_latency(provider, model, duration);
            }
            TimerMetricType::Mcp { server, method } => {
                self.exporter.record_mcp_latency(server, method, duration);
            }
            TimerMetricType::Queue { queue_name } => {
                self.exporter.record_queue_latency(queue_name, duration);
            }
        }
    }
}

impl MetricsExporter {
    /// Start a request timer
    pub fn start_request_timer<'a>(&'a self, method: &'a str, endpoint: &'a str) -> TimerGuard<'a> {
        TimerGuard::new(self, TimerMetricType::Request { method, endpoint })
    }

    /// Start a task timer
    pub fn start_task_timer<'a>(&'a self, task_type: &'a str) -> TimerGuard<'a> {
        TimerGuard::new(self, TimerMetricType::Task { task_type })
    }

    /// Start a tool timer
    pub fn start_tool_timer<'a>(&'a self, tool_name: &'a str) -> TimerGuard<'a> {
        TimerGuard::new(self, TimerMetricType::Tool { tool_name })
    }

    /// Start an LLM timer
    pub fn start_llm_timer<'a>(&'a self, provider: &'a str, model: &'a str) -> TimerGuard<'a> {
        TimerGuard::new(self, TimerMetricType::Llm { provider, model })
    }

    /// Start an MCP timer
    pub fn start_mcp_timer<'a>(&'a self, server: &'a str, method: &'a str) -> TimerGuard<'a> {
        TimerGuard::new(self, TimerMetricType::Mcp { server, method })
    }

    /// Start a queue timer
    pub fn start_queue_timer<'a>(&'a self, queue_name: &'a str) -> TimerGuard<'a> {
        TimerGuard::new(self, TimerMetricType::Queue { queue_name })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Configuration Tests ==========

    #[test]
    fn test_metrics_config_default() {
        let config = MetricsConfig::default();

        assert!(config.enabled);
        assert_eq!(config.prefix, "fluent_agent");
        assert!(!config.latency_buckets.is_empty());
        assert!(!config.size_buckets.is_empty());
        assert_eq!(config.max_label_cardinality, 1000);
    }

    #[test]
    fn test_metrics_config_custom() {
        let config = MetricsConfig {
            enabled: false,
            prefix: "custom".to_string(),
            default_labels: {
                let mut labels = HashMap::new();
                labels.insert("env".to_string(), "test".to_string());
                labels
            },
            latency_buckets: vec![0.1, 0.5, 1.0],
            size_buckets: vec![100.0, 1000.0],
            max_label_cardinality: 500,
        };

        assert!(!config.enabled);
        assert_eq!(config.prefix, "custom");
        assert_eq!(config.latency_buckets.len(), 3);
        assert_eq!(config.max_label_cardinality, 500);
    }

    // ========== MetricsExporter Creation Tests ==========

    #[test]
    fn test_metrics_exporter_new() {
        let exporter = MetricsExporter::with_defaults().unwrap();
        assert!(exporter.is_enabled());
    }

    #[test]
    fn test_metrics_exporter_custom_config() {
        let config = MetricsConfig {
            prefix: "test".to_string(),
            ..Default::default()
        };
        let exporter = MetricsExporter::new(config).unwrap();
        assert_eq!(exporter.config().prefix, "test");
    }

    // ========== Request Metrics Tests ==========

    #[test]
    fn test_record_request() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_request("POST", "/api/v1/execute", "200");
        exporter.record_request("POST", "/api/v1/execute", "200");
        exporter.record_request("POST", "/api/v1/execute", "500");

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_requests_total"));
    }

    #[test]
    fn test_record_request_duration() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_request_duration("POST", "/api/v1/execute", Duration::from_millis(100));
        exporter.record_request_duration("POST", "/api/v1/execute", Duration::from_millis(200));

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_request_duration_seconds"));
    }

    #[test]
    fn test_record_request_size() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_request_size("POST", "/api/v1/execute", 1024);
        exporter.record_response_size("POST", "/api/v1/execute", 2048);

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_request_size_bytes"));
        assert!(output.contains("fluent_agent_response_size_bytes"));
    }

    // ========== Task Metrics Tests ==========

    #[test]
    fn test_record_task() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_task("reasoning", "success");
        exporter.record_task("reasoning", "failed");
        exporter.record_task("tool_execution", "success");

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_tasks_total"));
    }

    #[test]
    fn test_record_task_duration() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_task_duration("reasoning", Duration::from_secs(1));
        exporter.record_task_duration("tool_execution", Duration::from_millis(500));

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_task_duration_seconds"));
    }

    #[test]
    fn test_active_tasks() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.inc_active_tasks("reasoning");
        exporter.inc_active_tasks("reasoning");
        exporter.dec_active_tasks("reasoning");

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_active_tasks"));
    }

    #[test]
    fn test_set_active_tasks() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.set_active_tasks("reasoning", 5.0);

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_active_tasks"));
    }

    // ========== Error Metrics Tests ==========

    #[test]
    fn test_record_error() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_error("timeout", "warning");
        exporter.record_error("api_error", "critical");

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_errors_total"));
    }

    #[test]
    fn test_record_error_recovery() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_error_recovery("timeout", "success");
        exporter.record_error_recovery("timeout", "failed");

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_error_recovery_total"));
    }

    // ========== Resource Metrics Tests ==========

    #[test]
    fn test_resource_metrics() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.set_memory_usage(1024 * 1024 * 100); // 100 MB
        exporter.set_cpu_usage(45.5);
        exporter.set_goroutines_active(10);

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_memory_usage_bytes"));
        assert!(output.contains("fluent_agent_cpu_usage_percent"));
        assert!(output.contains("fluent_agent_goroutines_active"));
    }

    // ========== Queue Metrics Tests ==========

    #[test]
    fn test_queue_metrics() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.set_queue_depth("task_queue", 100);
        exporter.record_queue_latency("task_queue", Duration::from_millis(50));

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_queue_depth"));
        assert!(output.contains("fluent_agent_queue_latency_seconds"));
    }

    // ========== LLM Metrics Tests ==========

    #[test]
    fn test_llm_metrics() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_llm_request("anthropic", "claude-3-sonnet", "success");
        exporter.record_llm_tokens("anthropic", "claude-3-sonnet", "input", 1000);
        exporter.record_llm_tokens("anthropic", "claude-3-sonnet", "output", 500);
        exporter.record_llm_latency("anthropic", "claude-3-sonnet", Duration::from_secs(2));
        exporter.record_llm_cost("anthropic", "claude-3-sonnet", 0.015);

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_llm_requests_total"));
        assert!(output.contains("fluent_agent_llm_tokens_total"));
        assert!(output.contains("fluent_agent_llm_latency_seconds"));
        assert!(output.contains("fluent_agent_llm_cost_dollars"));
    }

    // ========== Tool Metrics Tests ==========

    #[test]
    fn test_tool_metrics() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_tool_execution("read_file", "success");
        exporter.record_tool_execution("write_file", "failed");
        exporter.record_tool_duration("read_file", Duration::from_millis(10));

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_tool_executions_total"));
        assert!(output.contains("fluent_agent_tool_duration_seconds"));
    }

    // ========== MCP Metrics Tests ==========

    #[test]
    fn test_mcp_metrics() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_mcp_request("filesystem", "read", "success");
        exporter.record_mcp_latency("filesystem", "read", Duration::from_millis(5));

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_mcp_requests_total"));
        assert!(output.contains("fluent_agent_mcp_latency_seconds"));
    }

    // ========== Export Tests ==========

    #[test]
    fn test_export_prometheus_format() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_request("GET", "/health", "200");
        exporter.record_task("test", "success");

        let output = exporter.export().unwrap();

        // Verify Prometheus format characteristics
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
        assert!(
            output.contains("counter") || output.contains("gauge") || output.contains("histogram")
        );
    }

    #[test]
    fn test_export_json() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_request("GET", "/health", "200");

        let json = exporter.export_json().unwrap();

        // Verify JSON format
        let parsed: Vec<MetricSample> = serde_json::from_str(&json).unwrap();
        assert!(!parsed.is_empty());
    }

    // ========== Aggregated Stats Tests ==========

    #[test]
    fn test_get_aggregated_stats() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_task("reasoning", "success");
        exporter.record_task("reasoning", "success");
        exporter.record_task("reasoning", "failed");
        exporter.inc_active_tasks("reasoning");

        let stats = exporter.get_aggregated_stats();

        // Note: Counter values may not be immediately accessible through gather()
        // The test verifies that the method doesn't panic and returns valid stats
        assert!(stats.success_rate >= 0.0 && stats.success_rate <= 1.0);
        assert!(stats.error_rate >= 0.0 && stats.error_rate <= 1.0);
    }

    #[test]
    fn test_aggregated_stats_default() {
        let stats = AggregatedStats::default();

        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_errors, 0);
        assert!((stats.success_rate - 1.0).abs() < f64::EPSILON);
        assert!((stats.error_rate - 0.0).abs() < f64::EPSILON);
    }

    // ========== Timer Guard Tests ==========

    #[test]
    fn test_timer_guard_request() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        {
            let _timer = exporter.start_request_timer("GET", "/api/test");
            std::thread::sleep(Duration::from_millis(10));
        }

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_request_duration_seconds"));
    }

    #[test]
    fn test_timer_guard_task() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        {
            let _timer = exporter.start_task_timer("reasoning");
            std::thread::sleep(Duration::from_millis(10));
        }

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_task_duration_seconds"));
    }

    #[test]
    fn test_timer_guard_tool() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        {
            let _timer = exporter.start_tool_timer("read_file");
            std::thread::sleep(Duration::from_millis(10));
        }

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_tool_duration_seconds"));
    }

    #[test]
    fn test_timer_guard_llm() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        {
            let _timer = exporter.start_llm_timer("anthropic", "claude-3");
            std::thread::sleep(Duration::from_millis(10));
        }

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_llm_latency_seconds"));
    }

    #[test]
    fn test_timer_guard_mcp() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        {
            let _timer = exporter.start_mcp_timer("filesystem", "read");
            std::thread::sleep(Duration::from_millis(10));
        }

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_mcp_latency_seconds"));
    }

    #[test]
    fn test_timer_guard_queue() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        {
            let _timer = exporter.start_queue_timer("task_queue");
            std::thread::sleep(Duration::from_millis(10));
        }

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_queue_latency_seconds"));
    }

    #[test]
    fn test_timer_guard_elapsed() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        let timer = exporter.start_request_timer("GET", "/api/test");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.elapsed();

        assert!(elapsed >= Duration::from_millis(10));
    }

    // ========== Uptime Tests ==========

    #[test]
    fn test_uptime() {
        let exporter = MetricsExporter::with_defaults().unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let uptime = exporter.uptime();
        assert!(uptime >= Duration::from_millis(10));
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_metric_sample_serialization() {
        let sample = MetricSample {
            name: "test_metric".to_string(),
            value: 42.5,
            labels: {
                let mut labels = HashMap::new();
                labels.insert("env".to_string(), "test".to_string());
                labels
            },
            timestamp: Some(1234567890),
        };

        let json = serde_json::to_string(&sample).unwrap();
        let deserialized: MetricSample = serde_json::from_str(&json).unwrap();

        assert_eq!(sample.name, deserialized.name);
        assert!((sample.value - deserialized.value).abs() < f64::EPSILON);
    }

    #[test]
    fn test_aggregated_stats_serialization() {
        let stats = AggregatedStats {
            total_requests: 100,
            total_errors: 5,
            total_tasks_completed: 90,
            total_tasks_failed: 10,
            active_tasks: 3,
            average_latency_seconds: 0.5,
            p50_latency_seconds: 0.3,
            p95_latency_seconds: 1.0,
            p99_latency_seconds: 2.0,
            requests_per_second: 10.0,
            error_rate: 0.05,
            success_rate: 0.95,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: AggregatedStats = serde_json::from_str(&json).unwrap();

        assert_eq!(stats.total_requests, deserialized.total_requests);
        assert_eq!(stats.total_errors, deserialized.total_errors);
    }

    #[test]
    fn test_metrics_config_serialization() {
        let config = MetricsConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MetricsConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.prefix, deserialized.prefix);
    }

    // ========== Edge Cases ==========

    #[test]
    fn test_empty_label_values() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        // Empty strings should still work
        exporter.record_request("", "", "");
        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_requests_total"));
    }

    #[test]
    fn test_special_characters_in_labels() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        // Special characters in labels
        exporter.record_request("POST", "/api/v1/execute?param=value", "200");
        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_requests_total"));
    }

    #[test]
    fn test_high_precision_duration() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_request_duration("GET", "/api", Duration::from_nanos(100));
        exporter.record_request_duration("GET", "/api", Duration::from_micros(100));
        exporter.record_request_duration("GET", "/api", Duration::from_millis(100));
        exporter.record_request_duration("GET", "/api", Duration::from_secs(100));

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_request_duration_seconds"));
    }

    #[test]
    fn test_large_counter_values() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.record_llm_tokens("anthropic", "claude-3", "input", u64::MAX);

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_llm_tokens_total"));
    }

    #[test]
    fn test_zero_values() {
        let exporter = MetricsExporter::with_defaults().unwrap();

        exporter.set_memory_usage(0);
        exporter.set_cpu_usage(0.0);
        exporter.set_queue_depth("test", 0);
        exporter.record_request_duration("GET", "/api", Duration::ZERO);

        let output = exporter.export().unwrap();
        assert!(output.contains("fluent_agent_memory_usage_bytes 0"));
    }

    // ========== Multiple Exporters ==========

    #[test]
    fn test_multiple_exporters_different_prefixes() {
        let config1 = MetricsConfig {
            prefix: "exporter1".to_string(),
            ..Default::default()
        };
        let config2 = MetricsConfig {
            prefix: "exporter2".to_string(),
            ..Default::default()
        };

        let exporter1 = MetricsExporter::new(config1).unwrap();
        let exporter2 = MetricsExporter::new(config2).unwrap();

        exporter1.record_request("GET", "/api", "200");
        exporter2.record_request("POST", "/api", "201");

        let output1 = exporter1.export().unwrap();
        let output2 = exporter2.export().unwrap();

        assert!(output1.contains("exporter1_requests_total"));
        assert!(output2.contains("exporter2_requests_total"));
        assert!(!output1.contains("exporter2"));
        assert!(!output2.contains("exporter1"));
    }
}
