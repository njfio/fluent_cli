//! Monitoring and performance tracking for autonomous agents
//!
//! This module provides comprehensive monitoring capabilities for the agent system:
//!
//! - **Performance Monitoring**: Track execution metrics, quality scores, and efficiency
//! - **Distributed Tracing**: W3C Trace Context compatible tracing across service boundaries
//! - **Metrics Export**: Prometheus-compatible metrics aggregation and export
//! - **Circuit Breaker**: Prevent cascading failures with configurable circuit breakers
//! - **Error Recovery**: Automatic error detection and recovery strategies
//! - **Adaptive Strategy**: Dynamic strategy adjustment based on performance

pub mod adaptive_strategy;
pub mod circuit_breaker;
pub mod distributed_tracing;
pub mod error_recovery;
pub mod metrics_exporter;
pub mod performance_monitor;

pub use adaptive_strategy::AdaptiveStrategySystem;
pub use circuit_breaker::{
    with_circuit_breaker, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError,
    CircuitBreakerStats, CircuitState,
};
pub use distributed_tracing::{
    extract_context_from_headers, inject_context_to_headers, span_from_context, ActiveSpan,
    AttributeValue, DistributedTracer, Sampler, SamplingDecision, Span, SpanBuilder, SpanId,
    SpanKind, SpanLink, SpanStatus, TraceContext, TraceFlags, TraceId, TracerConfig, TracerStats,
};
pub use error_recovery::{
    ErrorInstance, ErrorRecoverySystem, ErrorSeverity, ErrorType, RecoveryConfig, RecoveryResult,
};
pub use metrics_exporter::{
    AggregatedStats, MetricSample, MetricsConfig, MetricsExporter, TimerGuard, TimerMetricType,
};
pub use performance_monitor::{PerformanceMetrics, PerformanceMonitor, QualityMetrics};
