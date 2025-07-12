// crates/fluent-core/src/types.rs

//! Core types for the Fluent CLI system
//!
//! This module defines the fundamental data structures used throughout
//! the Fluent CLI for communication with LLM engines, handling requests
//! and responses, and managing usage statistics.

use serde::{Deserialize, Serialize};

/// Represents a request to an LLM engine
///
/// A `Request` contains the flow name (identifying the type of operation)
/// and the payload (the actual content to be processed by the LLM).
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::Request;
///
/// let request = Request {
///     flowname: "chat".to_string(),
///     payload: "Hello, how are you?".to_string(),
/// };
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Request {
    /// The flow name identifying the type of operation
    pub flowname: String,
    /// The content to be processed by the LLM
    pub payload: String,
}

/// Represents a response from an LLM engine
///
/// A `Response` contains the generated content, usage statistics,
/// model information, and cost details from an LLM request.
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::{Response, Usage, Cost};
///
/// let response = Response {
///     content: "Hello! I'm doing well, thank you.".to_string(),
///     usage: Usage {
///         prompt_tokens: 10,
///         completion_tokens: 15,
///         total_tokens: 25,
///     },
///     model: "gpt-4".to_string(),
///     finish_reason: Some("stop".to_string()),
///     cost: Cost {
///         prompt_cost: 0.001,
///         completion_cost: 0.002,
///         total_cost: 0.003,
///     },
/// };
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Response {
    /// The generated content from the LLM
    pub content: String,
    /// Token usage statistics for this request
    pub usage: Usage,
    /// The model that generated this response
    pub model: String,
    /// The reason the generation finished (e.g., "stop", "length")
    pub finish_reason: Option<String>,
    /// Cost breakdown for this request
    pub cost: Cost,
}

/// Token usage statistics for an LLM request
///
/// Tracks the number of tokens used in the prompt, completion,
/// and total for billing and monitoring purposes.
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::Usage;
///
/// let usage = Usage {
///     prompt_tokens: 100,
///     completion_tokens: 50,
///     total_tokens: 150,
/// };
///
/// assert_eq!(usage.total_tokens, usage.prompt_tokens + usage.completion_tokens);
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Usage {
    /// Number of tokens in the input prompt
    pub prompt_tokens: u32,
    /// Number of tokens in the generated completion
    pub completion_tokens: u32,
    /// Total tokens used (prompt + completion)
    pub total_tokens: u32,
}

/// Cost breakdown for an LLM request
///
/// Tracks the cost in USD for prompt tokens, completion tokens,
/// and the total cost for billing and budget monitoring.
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::Cost;
///
/// let cost = Cost {
///     prompt_cost: 0.001,
///     completion_cost: 0.002,
///     total_cost: 0.003,
/// };
///
/// assert!((cost.total_cost - (cost.prompt_cost + cost.completion_cost)).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cost {
    /// Cost in USD for prompt tokens
    pub prompt_cost: f64,
    /// Cost in USD for completion tokens
    pub completion_cost: f64,
    /// Total cost in USD (prompt + completion)
    pub total_cost: f64,
}

/// Request for upserting data into a knowledge base
///
/// Used for storing conversation history, document content,
/// or other data that needs to be persisted and searchable.
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::UpsertRequest;
///
/// let request = UpsertRequest {
///     input: "What is the capital of France?".to_string(),
///     output: "The capital of France is Paris.".to_string(),
///     metadata: vec!["geography".to_string(), "europe".to_string()],
/// };
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpsertRequest {
    /// The input/query that was processed
    pub input: String,
    /// The output/response that was generated
    pub output: String,
    /// Additional metadata tags for categorization
    pub metadata: Vec<String>,
}

/// Response from an upsert operation
///
/// Contains information about what was successfully processed
/// and any errors that occurred during the operation.
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::UpsertResponse;
///
/// let response = UpsertResponse {
///     processed_files: vec!["conversation_1.json".to_string()],
///     errors: vec![],
/// };
/// ```
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpsertResponse {
    /// List of files that were successfully processed
    pub processed_files: Vec<String>,
    /// List of errors that occurred during processing
    pub errors: Vec<String>,
}

/// Statistics about documents in the knowledge base
///
/// Provides metrics about the stored documents including counts,
/// average sizes, and processing statistics.
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::DocumentStatistics;
///
/// let stats = DocumentStatistics {
///     document_count: 100,
///     avg_content_length: 1500.5,
///     chunk_count: 500,
///     embedding_count: 500,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct DocumentStatistics {
    /// Total number of documents stored
    pub document_count: i64,
    /// Average length of document content in characters
    pub avg_content_length: f64,
    /// Total number of text chunks created
    pub chunk_count: i64,
    /// Total number of embeddings generated
    pub embedding_count: i64,
}

/// Content extracted and analyzed from documents
///
/// Contains the main content along with optional analysis results
/// such as sentiment, themes, and keywords extracted by LLMs.
///
/// # Examples
///
/// ```rust
/// use fluent_core::types::ExtractedContent;
///
/// let content = ExtractedContent {
///     main_content: "This is a positive article about technology.".to_string(),
///     sentiment: Some("positive".to_string()),
///     clusters: Some(vec!["technology".to_string()]),
///     themes: Some(vec!["innovation".to_string(), "progress".to_string()]),
///     keywords: Some(vec!["technology".to_string(), "positive".to_string()]),
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ExtractedContent {
    /// The main text content
    pub main_content: String,
    /// Detected sentiment (positive, negative, neutral)
    pub sentiment: Option<String>,
    /// Content clusters or categories
    pub clusters: Option<Vec<String>>,
    /// Identified themes in the content
    pub themes: Option<Vec<String>>,
    /// Extracted keywords
    pub keywords: Option<Vec<String>>,
}
