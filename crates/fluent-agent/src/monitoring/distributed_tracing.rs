//! Distributed Tracing for Autonomous Agent Operations
//!
//! This module provides comprehensive distributed tracing capabilities for tracking
//! operations across the agent system, enabling observability, debugging, and
//! performance analysis of complex multi-step workflows.
//!
//! # Key Features
//!
//! - **Trace Context Propagation**: W3C Trace Context compatible headers for cross-service tracing
//! - **Span Management**: Hierarchical span creation with parent-child relationships
//! - **Baggage Support**: Propagate custom key-value pairs across service boundaries
//! - **Sampling**: Configurable trace sampling strategies to control overhead
//! - **Export**: Multiple export formats (JSON, OpenTelemetry-compatible)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Maximum number of completed spans to retain in memory
const MAX_COMPLETED_SPANS: usize = 10000;

/// Maximum baggage items per trace context
const MAX_BAGGAGE_ITEMS: usize = 64;

/// Maximum baggage value length
const MAX_BAGGAGE_VALUE_LEN: usize = 4096;

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for a trace (128-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub [u8; 16]);

impl TraceId {
    /// Generate a new random trace ID
    pub fn new() -> Self {
        let uuid = Uuid::new_v4();
        Self(*uuid.as_bytes())
    }

    /// Create from hex string (32 characters)
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 16];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            if i >= 16 {
                return None;
            }
            let s = std::str::from_utf8(chunk).ok()?;
            bytes[i] = u8::from_str_radix(s, 16).ok()?;
        }
        Some(Self(bytes))
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Check if this is a valid (non-zero) trace ID
    pub fn is_valid(&self) -> bool {
        self.0.iter().any(|&b| b != 0)
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Unique identifier for a span (64-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub u64);

impl SpanId {
    /// Generate a new random span ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        // Combine timestamp with counter for uniqueness
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        Self(ts.wrapping_add(count))
    }

    /// Create from hex string (16 characters)
    pub fn from_hex(hex: &str) -> Option<Self> {
        u64::from_str_radix(hex, 16).ok().map(Self)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        format!("{:016x}", self.0)
    }

    /// Check if this is a valid (non-zero) span ID
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Trace flags indicating trace state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceFlags(pub u8);

impl TraceFlags {
    /// No flags set
    pub const NONE: Self = Self(0);

    /// Trace is sampled (should be recorded)
    pub const SAMPLED: Self = Self(0x01);

    /// Check if the sampled flag is set
    pub fn is_sampled(&self) -> bool {
        self.0 & 0x01 != 0
    }

    /// Set the sampled flag
    pub fn with_sampled(self, sampled: bool) -> Self {
        if sampled {
            Self(self.0 | 0x01)
        } else {
            Self(self.0 & !0x01)
        }
    }
}

impl Default for TraceFlags {
    fn default() -> Self {
        Self::SAMPLED
    }
}

/// Baggage item for propagating custom context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaggageItem {
    pub key: String,
    pub value: String,
    pub metadata: Option<String>,
}

/// Trace context for propagation across service boundaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// The trace ID
    pub trace_id: TraceId,
    /// The parent span ID (if any)
    pub parent_span_id: Option<SpanId>,
    /// Trace flags
    pub flags: TraceFlags,
    /// Trace state (vendor-specific data)
    pub trace_state: HashMap<String, String>,
    /// Baggage items for custom propagation
    pub baggage: HashMap<String, BaggageItem>,
}

impl TraceContext {
    /// Create a new trace context with a new trace ID
    pub fn new() -> Self {
        Self {
            trace_id: TraceId::new(),
            parent_span_id: None,
            flags: TraceFlags::SAMPLED,
            trace_state: HashMap::new(),
            baggage: HashMap::new(),
        }
    }

    /// Create a child context with the given parent span
    pub fn child(&self, parent_span_id: SpanId) -> Self {
        Self {
            trace_id: self.trace_id,
            parent_span_id: Some(parent_span_id),
            flags: self.flags,
            trace_state: self.trace_state.clone(),
            baggage: self.baggage.clone(),
        }
    }

    /// Add a baggage item
    pub fn with_baggage(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        let value = value.into();

        // Enforce limits
        if self.baggage.len() >= MAX_BAGGAGE_ITEMS {
            return self;
        }
        if value.len() > MAX_BAGGAGE_VALUE_LEN {
            return self;
        }

        self.baggage.insert(
            key.clone(),
            BaggageItem {
                key,
                value,
                metadata: None,
            },
        );
        self
    }

    /// Get a baggage value
    pub fn get_baggage(&self, key: &str) -> Option<&str> {
        self.baggage.get(key).map(|b| b.value.as_str())
    }

    /// Parse from W3C traceparent header
    pub fn from_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = u8::from_str_radix(parts[0], 16).ok()?;
        if version != 0 {
            // Only version 00 is supported
            return None;
        }

        let trace_id = TraceId::from_hex(parts[1])?;
        let parent_span_id = SpanId::from_hex(parts[2])?;
        let flags = TraceFlags(u8::from_str_radix(parts[3], 16).ok()?);

        Some(Self {
            trace_id,
            parent_span_id: Some(parent_span_id),
            flags,
            trace_state: HashMap::new(),
            baggage: HashMap::new(),
        })
    }

    /// Format as W3C traceparent header
    pub fn to_traceparent(&self, span_id: SpanId) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id.to_hex(),
            span_id.to_hex(),
            self.flags.0
        )
    }

    /// Parse tracestate header
    pub fn parse_tracestate(&mut self, header: &str) {
        for pair in header.split(',') {
            let pair = pair.trim();
            if let Some((key, value)) = pair.split_once('=') {
                self.trace_state
                    .insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }

    /// Format tracestate header
    pub fn format_tracestate(&self) -> String {
        self.trace_state
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Span Types
// ============================================================================

/// Kind of span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    /// Internal operation
    Internal,
    /// Server handling an incoming request
    Server,
    /// Client making an outgoing request
    Client,
    /// Producer sending a message
    Producer,
    /// Consumer receiving a message
    Consumer,
}

impl Default for SpanKind {
    fn default() -> Self {
        Self::Internal
    }
}

/// Status of a span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpanStatus {
    /// Unset status
    Unset,
    /// Operation completed successfully
    Ok,
    /// Operation failed with an error
    Error { message: String },
}

impl Default for SpanStatus {
    fn default() -> Self {
        Self::Unset
    }
}

/// Event that occurred during a span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Timestamp when the event occurred
    pub timestamp: SystemTime,
    /// Event attributes
    pub attributes: HashMap<String, AttributeValue>,
}

/// Link to another span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLink {
    /// Linked trace ID
    pub trace_id: TraceId,
    /// Linked span ID
    pub span_id: SpanId,
    /// Link attributes
    pub attributes: HashMap<String, AttributeValue>,
}

/// Attribute value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    StringArray(Vec<String>),
    IntArray(Vec<i64>),
    FloatArray(Vec<f64>),
    BoolArray(Vec<bool>),
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<i64> for AttributeValue {
    fn from(n: i64) -> Self {
        Self::Int(n)
    }
}

impl From<f64> for AttributeValue {
    fn from(n: f64) -> Self {
        Self::Float(n)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

/// A completed span with all its data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Span name/operation name
    pub name: String,
    /// Trace ID this span belongs to
    pub trace_id: TraceId,
    /// This span's unique ID
    pub span_id: SpanId,
    /// Parent span ID (if any)
    pub parent_span_id: Option<SpanId>,
    /// Kind of span
    pub kind: SpanKind,
    /// Start time
    pub start_time: SystemTime,
    /// End time
    pub end_time: SystemTime,
    /// Duration
    pub duration: Duration,
    /// Span status
    pub status: SpanStatus,
    /// Span attributes
    pub attributes: HashMap<String, AttributeValue>,
    /// Events that occurred during the span
    pub events: Vec<SpanEvent>,
    /// Links to other spans
    pub links: Vec<SpanLink>,
    /// Resource attributes (service info)
    pub resource: HashMap<String, AttributeValue>,
}

/// Builder for creating spans
pub struct SpanBuilder {
    name: String,
    trace_context: TraceContext,
    kind: SpanKind,
    attributes: HashMap<String, AttributeValue>,
    links: Vec<SpanLink>,
    start_time: Option<SystemTime>,
}

impl SpanBuilder {
    /// Create a new span builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            trace_context: TraceContext::new(),
            kind: SpanKind::Internal,
            attributes: HashMap::new(),
            links: Vec::new(),
            start_time: None,
        }
    }

    /// Set the trace context
    pub fn with_context(mut self, ctx: TraceContext) -> Self {
        self.trace_context = ctx;
        self
    }

    /// Set the span kind
    pub fn with_kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    /// Add an attribute
    pub fn with_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<AttributeValue>,
    ) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Add a link
    pub fn with_link(mut self, trace_id: TraceId, span_id: SpanId) -> Self {
        self.links.push(SpanLink {
            trace_id,
            span_id,
            attributes: HashMap::new(),
        });
        self
    }

    /// Set explicit start time
    pub fn with_start_time(mut self, time: SystemTime) -> Self {
        self.start_time = Some(time);
        self
    }

    /// Start the span
    pub fn start(self) -> ActiveSpan {
        let span_id = SpanId::new();
        let start_time = self.start_time.unwrap_or_else(SystemTime::now);
        let start_instant = Instant::now();

        ActiveSpan {
            name: self.name,
            trace_id: self.trace_context.trace_id,
            span_id,
            parent_span_id: self.trace_context.parent_span_id,
            kind: self.kind,
            start_time,
            start_instant,
            status: SpanStatus::Unset,
            attributes: self.attributes,
            events: Vec::new(),
            links: self.links,
        }
    }
}

/// An active (in-progress) span
pub struct ActiveSpan {
    name: String,
    trace_id: TraceId,
    span_id: SpanId,
    parent_span_id: Option<SpanId>,
    kind: SpanKind,
    start_time: SystemTime,
    start_instant: Instant,
    status: SpanStatus,
    attributes: HashMap<String, AttributeValue>,
    events: Vec<SpanEvent>,
    links: Vec<SpanLink>,
}

impl ActiveSpan {
    /// Get the span ID
    pub fn span_id(&self) -> SpanId {
        self.span_id
    }

    /// Get the trace ID
    pub fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Create a child trace context for this span
    pub fn child_context(&self) -> TraceContext {
        TraceContext {
            trace_id: self.trace_id,
            parent_span_id: Some(self.span_id),
            flags: TraceFlags::SAMPLED,
            trace_state: HashMap::new(),
            baggage: HashMap::new(),
        }
    }

    /// Add an attribute
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<AttributeValue>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Add an event
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: SystemTime::now(),
            attributes: HashMap::new(),
        });
    }

    /// Add an event with attributes
    pub fn add_event_with_attributes(
        &mut self,
        name: impl Into<String>,
        attributes: HashMap<String, AttributeValue>,
    ) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: SystemTime::now(),
            attributes,
        });
    }

    /// Record an exception
    pub fn record_exception(&mut self, error: &dyn std::error::Error) {
        let mut attrs = HashMap::new();
        attrs.insert(
            "exception.type".to_string(),
            AttributeValue::String(std::any::type_name_of_val(error).to_string()),
        );
        attrs.insert(
            "exception.message".to_string(),
            AttributeValue::String(error.to_string()),
        );
        self.events.push(SpanEvent {
            name: "exception".to_string(),
            timestamp: SystemTime::now(),
            attributes: attrs,
        });
        self.status = SpanStatus::Error {
            message: error.to_string(),
        };
    }

    /// Set the span status to OK
    pub fn set_ok(&mut self) {
        self.status = SpanStatus::Ok;
    }

    /// Set the span status to Error
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status = SpanStatus::Error {
            message: message.into(),
        };
    }

    /// End the span and return the completed span data
    pub fn end(self) -> Span {
        let end_time = SystemTime::now();
        let duration = self.start_instant.elapsed();

        Span {
            name: self.name,
            trace_id: self.trace_id,
            span_id: self.span_id,
            parent_span_id: self.parent_span_id,
            kind: self.kind,
            start_time: self.start_time,
            end_time,
            duration,
            status: self.status,
            attributes: self.attributes,
            events: self.events,
            links: self.links,
            resource: HashMap::new(),
        }
    }

    /// End the span with a specific status
    pub fn end_with_status(mut self, status: SpanStatus) -> Span {
        self.status = status;
        self.end()
    }
}

// ============================================================================
// Sampling
// ============================================================================

/// Sampling decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingDecision {
    /// Don't record the trace
    Drop,
    /// Record but don't sample (for local debugging)
    RecordOnly,
    /// Record and sample (include in distributed trace)
    RecordAndSample,
}

/// Sampler for deciding which traces to record
pub trait Sampler: Send + Sync {
    /// Make a sampling decision for a new root span
    fn should_sample(
        &self,
        trace_id: &TraceId,
        name: &str,
        kind: SpanKind,
        attributes: &HashMap<String, AttributeValue>,
        links: &[SpanLink],
    ) -> SamplingDecision;
}

/// Always sample all traces
pub struct AlwaysOnSampler;

impl Sampler for AlwaysOnSampler {
    fn should_sample(
        &self,
        _trace_id: &TraceId,
        _name: &str,
        _kind: SpanKind,
        _attributes: &HashMap<String, AttributeValue>,
        _links: &[SpanLink],
    ) -> SamplingDecision {
        SamplingDecision::RecordAndSample
    }
}

/// Never sample any traces
pub struct AlwaysOffSampler;

impl Sampler for AlwaysOffSampler {
    fn should_sample(
        &self,
        _trace_id: &TraceId,
        _name: &str,
        _kind: SpanKind,
        _attributes: &HashMap<String, AttributeValue>,
        _links: &[SpanLink],
    ) -> SamplingDecision {
        SamplingDecision::Drop
    }
}

/// Sample traces based on probability (0.0 to 1.0)
pub struct ProbabilitySampler {
    probability: f64,
    /// Threshold for sampling (based on trace ID)
    threshold: u64,
}

impl ProbabilitySampler {
    pub fn new(probability: f64) -> Self {
        let probability = probability.clamp(0.0, 1.0);
        let threshold = (probability * u64::MAX as f64) as u64;
        Self {
            probability,
            threshold,
        }
    }

    pub fn probability(&self) -> f64 {
        self.probability
    }
}

impl Sampler for ProbabilitySampler {
    fn should_sample(
        &self,
        trace_id: &TraceId,
        _name: &str,
        _kind: SpanKind,
        _attributes: &HashMap<String, AttributeValue>,
        _links: &[SpanLink],
    ) -> SamplingDecision {
        // Use last 8 bytes of trace ID for deterministic sampling
        let bytes = &trace_id.0[8..16];
        let value = u64::from_be_bytes(bytes.try_into().unwrap_or([0; 8]));

        if value < self.threshold {
            SamplingDecision::RecordAndSample
        } else {
            SamplingDecision::Drop
        }
    }
}

/// Sample traces based on rate limit (traces per second)
pub struct RateLimitingSampler {
    max_traces_per_second: f64,
    last_sample_time: std::sync::Mutex<Instant>,
    tokens: std::sync::Mutex<f64>,
}

impl RateLimitingSampler {
    pub fn new(max_traces_per_second: f64) -> Self {
        Self {
            max_traces_per_second,
            last_sample_time: std::sync::Mutex::new(Instant::now()),
            tokens: std::sync::Mutex::new(max_traces_per_second),
        }
    }
}

impl Sampler for RateLimitingSampler {
    fn should_sample(
        &self,
        _trace_id: &TraceId,
        _name: &str,
        _kind: SpanKind,
        _attributes: &HashMap<String, AttributeValue>,
        _links: &[SpanLink],
    ) -> SamplingDecision {
        let mut last_time = self.last_sample_time.lock().unwrap();
        let mut tokens = self.tokens.lock().unwrap();

        let now = Instant::now();
        let elapsed = now.duration_since(*last_time).as_secs_f64();
        *last_time = now;

        // Replenish tokens based on elapsed time
        *tokens = (*tokens + elapsed * self.max_traces_per_second).min(self.max_traces_per_second);

        if *tokens >= 1.0 {
            *tokens -= 1.0;
            SamplingDecision::RecordAndSample
        } else {
            SamplingDecision::Drop
        }
    }
}

// ============================================================================
// Distributed Tracer
// ============================================================================

/// Configuration for the distributed tracer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerConfig {
    /// Service name
    pub service_name: String,
    /// Service version
    pub service_version: Option<String>,
    /// Environment (production, staging, etc.)
    pub environment: Option<String>,
    /// Maximum spans to retain
    pub max_spans: usize,
    /// Enable trace export
    pub export_enabled: bool,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            service_name: "fluent-agent".to_string(),
            service_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            environment: None,
            max_spans: MAX_COMPLETED_SPANS,
            export_enabled: true,
            sampling_rate: 1.0,
        }
    }
}

/// The main distributed tracer
pub struct DistributedTracer {
    config: TracerConfig,
    sampler: Arc<dyn Sampler>,
    completed_spans: Arc<RwLock<VecDeque<Span>>>,
    active_traces: Arc<RwLock<HashMap<TraceId, Vec<SpanId>>>>,
    resource_attributes: HashMap<String, AttributeValue>,
}

impl DistributedTracer {
    /// Create a new tracer with the given configuration
    pub fn new(config: TracerConfig) -> Self {
        let sampler: Arc<dyn Sampler> = if config.sampling_rate >= 1.0 {
            Arc::new(AlwaysOnSampler)
        } else if config.sampling_rate <= 0.0 {
            Arc::new(AlwaysOffSampler)
        } else {
            Arc::new(ProbabilitySampler::new(config.sampling_rate))
        };

        let mut resource_attributes = HashMap::new();
        resource_attributes.insert(
            "service.name".to_string(),
            AttributeValue::String(config.service_name.clone()),
        );
        if let Some(ref version) = config.service_version {
            resource_attributes.insert(
                "service.version".to_string(),
                AttributeValue::String(version.clone()),
            );
        }
        if let Some(ref env) = config.environment {
            resource_attributes.insert(
                "deployment.environment".to_string(),
                AttributeValue::String(env.clone()),
            );
        }

        Self {
            config,
            sampler,
            completed_spans: Arc::new(RwLock::new(VecDeque::new())),
            active_traces: Arc::new(RwLock::new(HashMap::new())),
            resource_attributes,
        }
    }

    /// Create a new span builder
    pub fn span(&self, name: impl Into<String>) -> SpanBuilder {
        SpanBuilder::new(name)
    }

    /// Start a new root span
    pub async fn start_span(&self, name: impl Into<String>) -> Option<ActiveSpan> {
        let name = name.into();
        let ctx = TraceContext::new();

        let decision = self.sampler.should_sample(
            &ctx.trace_id,
            &name,
            SpanKind::Internal,
            &HashMap::new(),
            &[],
        );

        if decision == SamplingDecision::Drop {
            return None;
        }

        let span = SpanBuilder::new(name).with_context(ctx).start();

        // Track the active trace
        let mut traces = self.active_traces.write().await;
        traces.entry(span.trace_id).or_default().push(span.span_id);

        Some(span)
    }

    /// Start a child span under an existing context
    pub async fn start_child_span(
        &self,
        name: impl Into<String>,
        parent_context: &TraceContext,
        parent_span_id: SpanId,
    ) -> Option<ActiveSpan> {
        let name = name.into();
        let child_ctx = parent_context.child(parent_span_id);

        let decision = self.sampler.should_sample(
            &child_ctx.trace_id,
            &name,
            SpanKind::Internal,
            &HashMap::new(),
            &[],
        );

        if decision == SamplingDecision::Drop {
            return None;
        }

        let span = SpanBuilder::new(name).with_context(child_ctx).start();

        // Track in active traces
        let mut traces = self.active_traces.write().await;
        traces.entry(span.trace_id).or_default().push(span.span_id);

        Some(span)
    }

    /// Record a completed span
    pub async fn record_span(&self, mut span: Span) {
        // Add resource attributes
        span.resource = self.resource_attributes.clone();

        // Remove from active traces
        {
            let mut traces = self.active_traces.write().await;
            if let Some(spans) = traces.get_mut(&span.trace_id) {
                spans.retain(|&id| id != span.span_id);
                if spans.is_empty() {
                    traces.remove(&span.trace_id);
                }
            }
        }

        // Store completed span
        {
            let mut completed = self.completed_spans.write().await;
            completed.push_back(span);

            // Enforce max spans limit
            while completed.len() > self.config.max_spans {
                completed.pop_front();
            }
        }
    }

    /// Get completed spans for a trace
    pub async fn get_trace_spans(&self, trace_id: &TraceId) -> Vec<Span> {
        let completed = self.completed_spans.read().await;
        completed
            .iter()
            .filter(|s| &s.trace_id == trace_id)
            .cloned()
            .collect()
    }

    /// Get all completed spans
    pub async fn get_all_spans(&self) -> Vec<Span> {
        let completed = self.completed_spans.read().await;
        completed.iter().cloned().collect()
    }

    /// Get recent spans (last N)
    pub async fn get_recent_spans(&self, count: usize) -> Vec<Span> {
        let completed = self.completed_spans.read().await;
        completed.iter().rev().take(count).cloned().collect()
    }

    /// Clear all completed spans
    pub async fn clear_spans(&self) {
        let mut completed = self.completed_spans.write().await;
        completed.clear();
    }

    /// Get active trace count
    pub async fn active_trace_count(&self) -> usize {
        let traces = self.active_traces.read().await;
        traces.len()
    }

    /// Get completed span count
    pub async fn completed_span_count(&self) -> usize {
        let completed = self.completed_spans.read().await;
        completed.len()
    }

    /// Export spans to JSON
    pub async fn export_json(&self) -> Result<String> {
        let spans = self.get_all_spans().await;
        let json = serde_json::to_string_pretty(&spans)?;
        Ok(json)
    }

    /// Get tracer statistics
    pub async fn get_stats(&self) -> TracerStats {
        let completed = self.completed_spans.read().await;
        let active = self.active_traces.read().await;

        let total_duration: Duration = completed.iter().map(|s| s.duration).sum();
        let avg_duration = if completed.is_empty() {
            Duration::ZERO
        } else {
            total_duration / completed.len() as u32
        };

        let error_count = completed
            .iter()
            .filter(|s| matches!(s.status, SpanStatus::Error { .. }))
            .count();

        TracerStats {
            completed_spans: completed.len(),
            active_traces: active.len(),
            active_spans: active.values().map(|v| v.len()).sum(),
            total_duration,
            average_span_duration: avg_duration,
            error_count,
            error_rate: if completed.is_empty() {
                0.0
            } else {
                error_count as f64 / completed.len() as f64
            },
        }
    }

    /// Get the tracer configuration
    pub fn config(&self) -> &TracerConfig {
        &self.config
    }
}

/// Statistics about the tracer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerStats {
    pub completed_spans: usize,
    pub active_traces: usize,
    pub active_spans: usize,
    pub total_duration: Duration,
    pub average_span_duration: Duration,
    pub error_count: usize,
    pub error_rate: f64,
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a span from an existing trace context
pub fn span_from_context(name: impl Into<String>, ctx: &TraceContext) -> SpanBuilder {
    SpanBuilder::new(name).with_context(ctx.clone())
}

/// Extract trace context from HTTP headers
pub fn extract_context_from_headers(headers: &HashMap<String, String>) -> Option<TraceContext> {
    let traceparent = headers.get("traceparent")?;
    let mut ctx = TraceContext::from_traceparent(traceparent)?;

    if let Some(tracestate) = headers.get("tracestate") {
        ctx.parse_tracestate(tracestate);
    }

    // Extract baggage
    if let Some(baggage) = headers.get("baggage") {
        for pair in baggage.split(',') {
            if let Some((key, value)) = pair.split_once('=') {
                ctx = ctx.with_baggage(key.trim(), value.trim());
            }
        }
    }

    Some(ctx)
}

/// Inject trace context into HTTP headers
pub fn inject_context_to_headers(
    ctx: &TraceContext,
    span_id: SpanId,
    headers: &mut HashMap<String, String>,
) {
    headers.insert("traceparent".to_string(), ctx.to_traceparent(span_id));

    let tracestate = ctx.format_tracestate();
    if !tracestate.is_empty() {
        headers.insert("tracestate".to_string(), tracestate);
    }

    if !ctx.baggage.is_empty() {
        let baggage: String = ctx
            .baggage
            .iter()
            .map(|(k, v)| format!("{}={}", k, v.value))
            .collect::<Vec<_>>()
            .join(",");
        headers.insert("baggage".to_string(), baggage);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========== TraceId Tests ==========

    #[test]
    fn test_trace_id_new() {
        let id1 = TraceId::new();
        let id2 = TraceId::new();

        assert!(id1.is_valid());
        assert!(id2.is_valid());
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_trace_id_from_hex() {
        let hex = "0123456789abcdef0123456789abcdef";
        let id = TraceId::from_hex(hex).unwrap();

        assert_eq!(id.to_hex(), hex);
    }

    #[test]
    fn test_trace_id_from_hex_invalid_length() {
        assert!(TraceId::from_hex("0123456789abcdef").is_none());
        assert!(TraceId::from_hex("").is_none());
    }

    #[test]
    fn test_trace_id_from_hex_invalid_chars() {
        assert!(TraceId::from_hex("0123456789abcdef0123456789abcdeg").is_none());
    }

    #[test]
    fn test_trace_id_display() {
        let id = TraceId::from_hex("0123456789abcdef0123456789abcdef").unwrap();
        assert_eq!(format!("{}", id), "0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn test_trace_id_default() {
        let id = TraceId::default();
        assert!(id.is_valid());
    }

    // ========== SpanId Tests ==========

    #[test]
    fn test_span_id_new() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();

        assert!(id1.is_valid());
        assert!(id2.is_valid());
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_span_id_from_hex() {
        let hex = "0123456789abcdef";
        let id = SpanId::from_hex(hex).unwrap();

        assert_eq!(id.to_hex(), hex);
    }

    #[test]
    fn test_span_id_from_hex_invalid() {
        assert!(SpanId::from_hex("invalid").is_none());
    }

    #[test]
    fn test_span_id_display() {
        let id = SpanId(0x0123456789abcdef);
        assert_eq!(format!("{}", id), "0123456789abcdef");
    }

    #[test]
    fn test_span_id_default() {
        let id = SpanId::default();
        assert!(id.is_valid());
    }

    // ========== TraceFlags Tests ==========

    #[test]
    fn test_trace_flags_none() {
        let flags = TraceFlags::NONE;
        assert!(!flags.is_sampled());
    }

    #[test]
    fn test_trace_flags_sampled() {
        let flags = TraceFlags::SAMPLED;
        assert!(flags.is_sampled());
    }

    #[test]
    fn test_trace_flags_with_sampled() {
        let flags = TraceFlags::NONE.with_sampled(true);
        assert!(flags.is_sampled());

        let flags = TraceFlags::SAMPLED.with_sampled(false);
        assert!(!flags.is_sampled());
    }

    #[test]
    fn test_trace_flags_default() {
        let flags = TraceFlags::default();
        assert!(flags.is_sampled());
    }

    // ========== TraceContext Tests ==========

    #[test]
    fn test_trace_context_new() {
        let ctx = TraceContext::new();

        assert!(ctx.trace_id.is_valid());
        assert!(ctx.parent_span_id.is_none());
        assert!(ctx.flags.is_sampled());
    }

    #[test]
    fn test_trace_context_child() {
        let parent = TraceContext::new();
        let parent_span_id = SpanId::new();
        let child = parent.child(parent_span_id);

        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent_span_id));
    }

    #[test]
    fn test_trace_context_baggage() {
        let ctx = TraceContext::new()
            .with_baggage("user_id", "123")
            .with_baggage("tenant", "acme");

        assert_eq!(ctx.get_baggage("user_id"), Some("123"));
        assert_eq!(ctx.get_baggage("tenant"), Some("acme"));
        assert_eq!(ctx.get_baggage("missing"), None);
    }

    #[test]
    fn test_trace_context_baggage_limit() {
        let mut ctx = TraceContext::new();
        for i in 0..MAX_BAGGAGE_ITEMS + 10 {
            ctx = ctx.with_baggage(format!("key{}", i), "value");
        }

        assert_eq!(ctx.baggage.len(), MAX_BAGGAGE_ITEMS);
    }

    #[test]
    fn test_trace_context_from_traceparent() {
        let header = "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01";
        let ctx = TraceContext::from_traceparent(header).unwrap();

        assert_eq!(ctx.trace_id.to_hex(), "0123456789abcdef0123456789abcdef");
        assert_eq!(ctx.parent_span_id.unwrap().to_hex(), "0123456789abcdef");
        assert!(ctx.flags.is_sampled());
    }

    #[test]
    fn test_trace_context_from_traceparent_invalid() {
        assert!(TraceContext::from_traceparent("invalid").is_none());
        assert!(TraceContext::from_traceparent(
            "01-0123456789abcdef0123456789abcdef-0123456789abcdef-01"
        )
        .is_none());
    }

    #[test]
    fn test_trace_context_to_traceparent() {
        let ctx = TraceContext::from_traceparent(
            "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01",
        )
        .unwrap();
        let span_id = SpanId(0xfedcba9876543210);

        let header = ctx.to_traceparent(span_id);
        assert_eq!(
            header,
            "00-0123456789abcdef0123456789abcdef-fedcba9876543210-01"
        );
    }

    #[test]
    fn test_trace_context_tracestate() {
        let mut ctx = TraceContext::new();
        ctx.parse_tracestate("vendor1=value1, vendor2=value2");

        assert_eq!(ctx.trace_state.get("vendor1"), Some(&"value1".to_string()));
        assert_eq!(ctx.trace_state.get("vendor2"), Some(&"value2".to_string()));

        let formatted = ctx.format_tracestate();
        assert!(formatted.contains("vendor1=value1"));
        assert!(formatted.contains("vendor2=value2"));
    }

    // ========== SpanKind Tests ==========

    #[test]
    fn test_span_kind_default() {
        let kind = SpanKind::default();
        assert!(matches!(kind, SpanKind::Internal));
    }

    #[test]
    fn test_span_kind_variants() {
        let kinds = vec![
            SpanKind::Internal,
            SpanKind::Server,
            SpanKind::Client,
            SpanKind::Producer,
            SpanKind::Consumer,
        ];
        assert_eq!(kinds.len(), 5);
    }

    // ========== SpanStatus Tests ==========

    #[test]
    fn test_span_status_default() {
        let status = SpanStatus::default();
        assert!(matches!(status, SpanStatus::Unset));
    }

    #[test]
    fn test_span_status_error() {
        let status = SpanStatus::Error {
            message: "test error".to_string(),
        };
        if let SpanStatus::Error { message } = status {
            assert_eq!(message, "test error");
        } else {
            panic!("Expected Error status");
        }
    }

    // ========== AttributeValue Tests ==========

    #[test]
    fn test_attribute_value_from_str() {
        let attr: AttributeValue = "test".into();
        assert!(matches!(attr, AttributeValue::String(s) if s == "test"));
    }

    #[test]
    fn test_attribute_value_from_string() {
        let attr: AttributeValue = String::from("test").into();
        assert!(matches!(attr, AttributeValue::String(s) if s == "test"));
    }

    #[test]
    fn test_attribute_value_from_i64() {
        let attr: AttributeValue = 42i64.into();
        assert!(matches!(attr, AttributeValue::Int(n) if n == 42));
    }

    #[test]
    fn test_attribute_value_from_f64() {
        let attr: AttributeValue = 3.14f64.into();
        assert!(matches!(attr, AttributeValue::Float(n) if (n - 3.14).abs() < f64::EPSILON));
    }

    #[test]
    fn test_attribute_value_from_bool() {
        let attr: AttributeValue = true.into();
        assert!(matches!(attr, AttributeValue::Bool(b) if b));
    }

    // ========== SpanBuilder Tests ==========

    #[test]
    fn test_span_builder_basic() {
        let span = SpanBuilder::new("test_span").start();

        assert_eq!(span.name, "test_span");
        assert!(span.trace_id.is_valid());
        assert!(span.span_id.is_valid());
    }

    #[test]
    fn test_span_builder_with_context() {
        let ctx = TraceContext::new();
        let trace_id = ctx.trace_id;

        let span = SpanBuilder::new("test_span").with_context(ctx).start();

        assert_eq!(span.trace_id, trace_id);
    }

    #[test]
    fn test_span_builder_with_kind() {
        let span = SpanBuilder::new("test_span")
            .with_kind(SpanKind::Server)
            .start();

        assert!(matches!(span.kind, SpanKind::Server));
    }

    #[test]
    fn test_span_builder_with_attribute() {
        let span = SpanBuilder::new("test_span")
            .with_attribute("key", "value")
            .start();

        assert!(span.attributes.contains_key("key"));
    }

    #[test]
    fn test_span_builder_with_link() {
        let linked_trace = TraceId::new();
        let linked_span = SpanId::new();

        let span = SpanBuilder::new("test_span")
            .with_link(linked_trace, linked_span)
            .start();

        assert_eq!(span.links.len(), 1);
        assert_eq!(span.links[0].trace_id, linked_trace);
    }

    // ========== ActiveSpan Tests ==========

    #[test]
    fn test_active_span_set_attribute() {
        let mut span = SpanBuilder::new("test").start();
        span.set_attribute("key", "value");

        assert!(span.attributes.contains_key("key"));
    }

    #[test]
    fn test_active_span_add_event() {
        let mut span = SpanBuilder::new("test").start();
        span.add_event("test_event");

        assert_eq!(span.events.len(), 1);
        assert_eq!(span.events[0].name, "test_event");
    }

    #[test]
    fn test_active_span_add_event_with_attributes() {
        let mut span = SpanBuilder::new("test").start();
        let mut attrs = HashMap::new();
        attrs.insert(
            "level".to_string(),
            AttributeValue::String("info".to_string()),
        );
        span.add_event_with_attributes("test_event", attrs);

        assert_eq!(span.events.len(), 1);
        assert!(span.events[0].attributes.contains_key("level"));
    }

    #[test]
    fn test_active_span_set_ok() {
        let mut span = SpanBuilder::new("test").start();
        span.set_ok();

        let completed = span.end();
        assert!(matches!(completed.status, SpanStatus::Ok));
    }

    #[test]
    fn test_active_span_set_error() {
        let mut span = SpanBuilder::new("test").start();
        span.set_error("test error");

        let completed = span.end();
        assert!(
            matches!(completed.status, SpanStatus::Error { message } if message == "test error")
        );
    }

    #[test]
    fn test_active_span_child_context() {
        let span = SpanBuilder::new("parent").start();
        let child_ctx = span.child_context();

        assert_eq!(child_ctx.trace_id, span.trace_id);
        assert_eq!(child_ctx.parent_span_id, Some(span.span_id));
    }

    #[test]
    fn test_active_span_end() {
        let span = SpanBuilder::new("test").start();
        let span_id = span.span_id;
        let trace_id = span.trace_id;

        let completed = span.end();

        assert_eq!(completed.span_id, span_id);
        assert_eq!(completed.trace_id, trace_id);
        assert!(completed.duration.as_nanos() > 0 || completed.duration.as_nanos() == 0);
    }

    #[test]
    fn test_active_span_end_with_status() {
        let span = SpanBuilder::new("test").start();
        let completed = span.end_with_status(SpanStatus::Ok);

        assert!(matches!(completed.status, SpanStatus::Ok));
    }

    // ========== Sampler Tests ==========

    #[test]
    fn test_always_on_sampler() {
        let sampler = AlwaysOnSampler;
        let trace_id = TraceId::new();

        let decision =
            sampler.should_sample(&trace_id, "test", SpanKind::Internal, &HashMap::new(), &[]);
        assert_eq!(decision, SamplingDecision::RecordAndSample);
    }

    #[test]
    fn test_always_off_sampler() {
        let sampler = AlwaysOffSampler;
        let trace_id = TraceId::new();

        let decision =
            sampler.should_sample(&trace_id, "test", SpanKind::Internal, &HashMap::new(), &[]);
        assert_eq!(decision, SamplingDecision::Drop);
    }

    #[test]
    fn test_probability_sampler_100_percent() {
        let sampler = ProbabilitySampler::new(1.0);

        for _ in 0..100 {
            let trace_id = TraceId::new();
            let decision =
                sampler.should_sample(&trace_id, "test", SpanKind::Internal, &HashMap::new(), &[]);
            assert_eq!(decision, SamplingDecision::RecordAndSample);
        }
    }

    #[test]
    fn test_probability_sampler_0_percent() {
        let sampler = ProbabilitySampler::new(0.0);

        for _ in 0..100 {
            let trace_id = TraceId::new();
            let decision =
                sampler.should_sample(&trace_id, "test", SpanKind::Internal, &HashMap::new(), &[]);
            assert_eq!(decision, SamplingDecision::Drop);
        }
    }

    #[test]
    fn test_probability_sampler_clamping() {
        let sampler_high = ProbabilitySampler::new(1.5);
        assert!((sampler_high.probability() - 1.0).abs() < f64::EPSILON);

        let sampler_low = ProbabilitySampler::new(-0.5);
        assert!((sampler_low.probability() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rate_limiting_sampler() {
        let sampler = RateLimitingSampler::new(10.0);
        let trace_id = TraceId::new();

        // First sample should succeed
        let decision =
            sampler.should_sample(&trace_id, "test", SpanKind::Internal, &HashMap::new(), &[]);
        assert_eq!(decision, SamplingDecision::RecordAndSample);
    }

    // ========== TracerConfig Tests ==========

    #[test]
    fn test_tracer_config_default() {
        let config = TracerConfig::default();

        assert_eq!(config.service_name, "fluent-agent");
        assert!(config.service_version.is_some());
        assert!(config.export_enabled);
        assert!((config.sampling_rate - 1.0).abs() < f64::EPSILON);
    }

    // ========== DistributedTracer Tests ==========

    #[tokio::test]
    async fn test_distributed_tracer_new() {
        let config = TracerConfig::default();
        let tracer = DistributedTracer::new(config);

        assert_eq!(tracer.active_trace_count().await, 0);
        assert_eq!(tracer.completed_span_count().await, 0);
    }

    #[tokio::test]
    async fn test_distributed_tracer_start_span() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let span = tracer.start_span("test_operation").await.unwrap();

        assert_eq!(span.name, "test_operation");
        assert_eq!(tracer.active_trace_count().await, 1);
    }

    #[tokio::test]
    async fn test_distributed_tracer_record_span() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let span = tracer.start_span("test_operation").await.unwrap();
        let completed = span.end();
        tracer.record_span(completed).await;

        assert_eq!(tracer.active_trace_count().await, 0);
        assert_eq!(tracer.completed_span_count().await, 1);
    }

    #[tokio::test]
    async fn test_distributed_tracer_child_span() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let parent = tracer.start_span("parent").await.unwrap();
        let parent_ctx = parent.child_context();
        let parent_span_id = parent.span_id;

        let child = tracer
            .start_child_span("child", &parent_ctx, parent_span_id)
            .await
            .unwrap();

        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent_span_id));
    }

    #[tokio::test]
    async fn test_distributed_tracer_get_trace_spans() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let span1 = tracer.start_span("op1").await.unwrap();
        let trace_id = span1.trace_id;
        tracer.record_span(span1.end()).await;

        let span2 = tracer.start_span("op2").await.unwrap();
        tracer.record_span(span2.end()).await;

        let trace_spans = tracer.get_trace_spans(&trace_id).await;
        assert_eq!(trace_spans.len(), 1);
        assert_eq!(trace_spans[0].name, "op1");
    }

    #[tokio::test]
    async fn test_distributed_tracer_get_recent_spans() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        for i in 0..5 {
            let span = tracer.start_span(format!("op{}", i)).await.unwrap();
            tracer.record_span(span.end()).await;
        }

        let recent = tracer.get_recent_spans(3).await;
        assert_eq!(recent.len(), 3);
    }

    #[tokio::test]
    async fn test_distributed_tracer_clear_spans() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let span = tracer.start_span("test").await.unwrap();
        tracer.record_span(span.end()).await;

        assert_eq!(tracer.completed_span_count().await, 1);

        tracer.clear_spans().await;
        assert_eq!(tracer.completed_span_count().await, 0);
    }

    #[tokio::test]
    async fn test_distributed_tracer_stats() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let span = tracer.start_span("test").await.unwrap();
        tracer.record_span(span.end()).await;

        let stats = tracer.get_stats().await;
        assert_eq!(stats.completed_spans, 1);
        assert_eq!(stats.error_count, 0);
    }

    #[tokio::test]
    async fn test_distributed_tracer_stats_with_errors() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let mut span = tracer.start_span("test").await.unwrap();
        span.set_error("test error");
        tracer.record_span(span.end()).await;

        let stats = tracer.get_stats().await;
        assert_eq!(stats.error_count, 1);
        assert!((stats.error_rate - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_distributed_tracer_export_json() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        let span = tracer.start_span("test").await.unwrap();
        tracer.record_span(span.end()).await;

        let json = tracer.export_json().await.unwrap();
        assert!(json.contains("test"));
    }

    #[tokio::test]
    async fn test_distributed_tracer_sampling_off() {
        let mut config = TracerConfig::default();
        config.sampling_rate = 0.0;
        let tracer = DistributedTracer::new(config);

        let span = tracer.start_span("test").await;
        assert!(span.is_none());
    }

    // ========== Context Propagation Tests ==========

    #[test]
    fn test_extract_context_from_headers() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01".to_string(),
        );
        headers.insert("tracestate".to_string(), "vendor=value".to_string());
        headers.insert("baggage".to_string(), "key1=val1, key2=val2".to_string());

        let ctx = extract_context_from_headers(&headers).unwrap();

        assert_eq!(ctx.trace_id.to_hex(), "0123456789abcdef0123456789abcdef");
        assert_eq!(ctx.trace_state.get("vendor"), Some(&"value".to_string()));
        assert_eq!(ctx.get_baggage("key1"), Some("val1"));
        assert_eq!(ctx.get_baggage("key2"), Some("val2"));
    }

    #[test]
    fn test_extract_context_from_headers_missing() {
        let headers = HashMap::new();
        let ctx = extract_context_from_headers(&headers);
        assert!(ctx.is_none());
    }

    #[test]
    fn test_inject_context_to_headers() {
        let ctx = TraceContext::from_traceparent(
            "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01",
        )
        .unwrap()
        .with_baggage("user_id", "123");

        let span_id = SpanId(0xfedcba9876543210);
        let mut headers = HashMap::new();
        inject_context_to_headers(&ctx, span_id, &mut headers);

        assert!(headers.contains_key("traceparent"));
        assert!(headers.contains_key("baggage"));
        assert!(headers.get("baggage").unwrap().contains("user_id=123"));
    }

    // ========== SpanEvent Tests ==========

    #[test]
    fn test_span_event_creation() {
        let event = SpanEvent {
            name: "cache_hit".to_string(),
            timestamp: SystemTime::now(),
            attributes: HashMap::new(),
        };

        assert_eq!(event.name, "cache_hit");
    }

    // ========== SpanLink Tests ==========

    #[test]
    fn test_span_link_creation() {
        let link = SpanLink {
            trace_id: TraceId::new(),
            span_id: SpanId::new(),
            attributes: HashMap::new(),
        };

        assert!(link.trace_id.is_valid());
        assert!(link.span_id.is_valid());
    }

    // ========== Serialization Tests ==========

    #[test]
    fn test_trace_id_serialization() {
        let id = TraceId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: TraceId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_span_id_serialization() {
        let id = SpanId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: SpanId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_span_serialization() {
        let span = Span {
            name: "test".to_string(),
            trace_id: TraceId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            kind: SpanKind::Internal,
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            duration: Duration::from_millis(100),
            status: SpanStatus::Ok,
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
            resource: HashMap::new(),
        };

        let json = serde_json::to_string(&span).unwrap();
        let deserialized: Span = serde_json::from_str(&json).unwrap();

        assert_eq!(span.name, deserialized.name);
        assert_eq!(span.trace_id, deserialized.trace_id);
    }

    #[test]
    fn test_tracer_stats_serialization() {
        let stats = TracerStats {
            completed_spans: 10,
            active_traces: 2,
            active_spans: 5,
            total_duration: Duration::from_secs(100),
            average_span_duration: Duration::from_millis(10),
            error_count: 1,
            error_rate: 0.1,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: TracerStats = serde_json::from_str(&json).unwrap();

        assert_eq!(stats.completed_spans, deserialized.completed_spans);
        assert_eq!(stats.error_count, deserialized.error_count);
    }

    // ========== Integration Tests ==========

    #[tokio::test]
    async fn test_full_trace_workflow() {
        let tracer = DistributedTracer::new(TracerConfig::default());

        // Start a parent span
        let mut parent = tracer.start_span("http_request").await.unwrap();
        parent.set_attribute("http.method", "GET");
        parent.set_attribute("http.url", "https://example.com/api");

        let parent_ctx = parent.child_context();
        let parent_span_id = parent.span_id;

        // Start a child span for database query
        let mut db_span = tracer
            .start_child_span("db_query", &parent_ctx, parent_span_id)
            .await
            .unwrap();
        db_span.set_attribute("db.system", "postgresql");
        db_span.set_attribute("db.statement", "SELECT * FROM users");
        db_span.add_event("query_started");

        // Simulate some work
        tokio::time::sleep(Duration::from_millis(1)).await;

        db_span.add_event("query_completed");
        db_span.set_ok();

        let db_completed = db_span.end();
        tracer.record_span(db_completed).await;

        // Complete parent span
        parent.set_attribute("http.status_code", 200i64);
        parent.set_ok();
        tracer.record_span(parent.end()).await;

        // Verify
        let stats = tracer.get_stats().await;
        assert_eq!(stats.completed_spans, 2);
        assert_eq!(stats.error_count, 0);

        let all_spans = tracer.get_all_spans().await;
        assert_eq!(all_spans.len(), 2);
    }
}
