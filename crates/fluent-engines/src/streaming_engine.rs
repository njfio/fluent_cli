use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use log::{debug, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::pin::Pin;

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Unique identifier for this chunk
    pub id: String,
    /// Content of this chunk
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Token usage for this chunk (if available)
    pub token_usage: Option<ChunkTokenUsage>,
    /// Model that generated this chunk
    pub model: Option<String>,
    /// Finish reason (only present in final chunk)
    pub finish_reason: Option<String>,
}

/// Token usage information for a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkTokenUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// Stream of response chunks
pub type ResponseStream = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

/// Trait for engines that support streaming responses
#[async_trait]
pub trait StreamingEngine: Send + Sync {
    /// Execute a request and return a stream of response chunks
    async fn execute_stream(&self, request: &fluent_core::types::Request)
        -> Result<ResponseStream>;

    /// Check if streaming is supported for this engine
    fn supports_streaming(&self) -> bool;

    /// Get the streaming configuration for this engine
    fn get_streaming_config(&self) -> StreamingConfig;
}

/// Configuration for streaming behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Whether streaming is enabled
    pub enabled: bool,
    /// Buffer size for streaming chunks
    pub buffer_size: usize,
    /// Timeout for individual chunks
    pub chunk_timeout_ms: u64,
    /// Maximum number of chunks to buffer
    pub max_buffered_chunks: usize,
    /// Whether to include token usage in chunks
    pub include_token_usage: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 8192,
            chunk_timeout_ms: 5000,
            max_buffered_chunks: 100,
            include_token_usage: true,
        }
    }
}

/// OpenAI streaming implementation
pub struct OpenAIStreaming {
    client: Client,
    config: fluent_core::config::EngineConfig,
}

impl OpenAIStreaming {
    pub fn new(client: Client, config: fluent_core::config::EngineConfig) -> Self {
        Self { client, config }
    }

    /// Parse OpenAI streaming response
    fn parse_openai_chunk(line: &str) -> Result<Option<StreamChunk>> {
        if line.is_empty() || !line.starts_with("data: ") {
            return Ok(None);
        }

        let data = &line[6..]; // Remove "data: " prefix

        if data == "[DONE]" {
            return Ok(Some(StreamChunk {
                id: "final".to_string(),
                content: String::new(),
                is_final: true,
                token_usage: None,
                model: None,
                finish_reason: Some("stop".to_string()),
            }));
        }

        let chunk_data: Value = serde_json::from_str(data)?;

        let id = chunk_data["id"].as_str().unwrap_or("unknown").to_string();
        let model = chunk_data["model"].as_str().map(String::from);

        let empty_choices = vec![];
        let choices = chunk_data["choices"].as_array().unwrap_or(&empty_choices);
        if choices.is_empty() {
            return Ok(None);
        }

        let choice = &choices[0];
        let delta = &choice["delta"];
        let content = delta["content"].as_str().unwrap_or("").to_string();
        let finish_reason = choice["finish_reason"].as_str().map(String::from);

        let token_usage = chunk_data["usage"]
            .as_object()
            .map(|usage| ChunkTokenUsage {
                prompt_tokens: usage
                    .get("prompt_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
                completion_tokens: usage
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
                total_tokens: usage
                    .get("total_tokens")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32),
            });

        Ok(Some(StreamChunk {
            id,
            content,
            is_final: finish_reason.is_some(),
            token_usage,
            model,
            finish_reason,
        }))
    }
}

#[async_trait]
impl StreamingEngine for OpenAIStreaming {
    async fn execute_stream(
        &self,
        request: &fluent_core::types::Request,
    ) -> Result<ResponseStream> {
        let url = format!(
            "{}://{}:{}/v1/chat/completions",
            self.config.connection.protocol,
            self.config.connection.hostname,
            self.config.connection.port
        );

        // Get authentication token
        let bearer_token = self
            .config
            .parameters
            .get("bearer_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Bearer token not found"))?;

        // Build streaming payload
        let mut payload = json!({
            "model": self.config.parameters.get("model").unwrap_or(&json!("gpt-4")),
            "messages": [
                {
                    "role": "user",
                    "content": request.payload
                }
            ],
            "stream": true,
            "temperature": self.config.parameters.get("temperature").unwrap_or(&json!(0.7)),
            "max_tokens": self.config.parameters.get("max_tokens").unwrap_or(&json!(1000))
        });

        // Add any additional parameters
        for (key, value) in &self.config.parameters {
            if !["bearer_token", "model", "temperature", "max_tokens"].contains(&key.as_str()) {
                payload[key] = value.clone();
            }
        }

        debug!("Starting OpenAI streaming request to: {}", url);

        // Send streaming request
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", bearer_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        // Create stream from response - simplified approach
        let stream = async_stream::stream! {
            let mut buffer = String::new();
            let mut bytes_stream = response.bytes_stream();

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Process complete lines
                        let current_buffer = buffer.clone();
                        let lines: Vec<&str> = current_buffer.lines().collect();

                        if !buffer.ends_with('\n') && !lines.is_empty() {
                            // Keep the last incomplete line in buffer
                            let last_line = lines.last().unwrap();
                            buffer = last_line.to_string();

                            // Process all but the last line
                            for line in &lines[..lines.len()-1] {
                                match Self::parse_openai_chunk(line) {
                                    Ok(Some(chunk)) => yield Ok(chunk),
                                    Ok(None) => continue,
                                    Err(e) => {
                                        warn!("Failed to parse chunk: {}", e);
                                        continue;
                                    }
                                }
                            }
                        } else {
                            buffer.clear();

                            // Process all lines
                            for line in lines {
                                match Self::parse_openai_chunk(line) {
                                    Ok(Some(chunk)) => yield Ok(chunk),
                                    Ok(None) => continue,
                                    Err(e) => {
                                        warn!("Failed to parse chunk: {}", e);
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow!("Stream error: {}", e));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn get_streaming_config(&self) -> StreamingConfig {
        StreamingConfig::default()
    }
}

/// Anthropic streaming implementation
pub struct AnthropicStreaming {
    client: Client,
    config: fluent_core::config::EngineConfig,
}

impl AnthropicStreaming {
    pub fn new(client: Client, config: fluent_core::config::EngineConfig) -> Self {
        Self { client, config }
    }

    /// Parse Anthropic streaming response
    fn parse_anthropic_chunk(line: &str) -> Result<Option<StreamChunk>> {
        if line.is_empty() || !line.starts_with("data: ") {
            return Ok(None);
        }

        let data = &line[6..]; // Remove "data: " prefix

        if data == "[DONE]" {
            return Ok(Some(StreamChunk {
                id: "final".to_string(),
                content: String::new(),
                is_final: true,
                token_usage: None,
                model: None,
                finish_reason: Some("end_turn".to_string()),
            }));
        }

        let chunk_data: Value = serde_json::from_str(data)?;

        let event_type = chunk_data["type"].as_str().unwrap_or("");

        match event_type {
            "content_block_delta" => {
                let delta = &chunk_data["delta"];
                let content = delta["text"].as_str().unwrap_or("").to_string();

                Ok(Some(StreamChunk {
                    id: chunk_data["index"].as_u64().unwrap_or(0).to_string(),
                    content,
                    is_final: false,
                    token_usage: None,
                    model: None,
                    finish_reason: None,
                }))
            }
            "message_stop" => Ok(Some(StreamChunk {
                id: "final".to_string(),
                content: String::new(),
                is_final: true,
                token_usage: None,
                model: None,
                finish_reason: Some("end_turn".to_string()),
            })),
            _ => Ok(None),
        }
    }
}

#[async_trait]
impl StreamingEngine for AnthropicStreaming {
    async fn execute_stream(
        &self,
        request: &fluent_core::types::Request,
    ) -> Result<ResponseStream> {
        let url = format!(
            "{}://{}:{}/v1/messages",
            self.config.connection.protocol,
            self.config.connection.hostname,
            self.config.connection.port
        );

        // Get authentication token
        let api_key = self
            .config
            .parameters
            .get("bearer_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("API key not found"))?;

        // Build streaming payload
        let payload = json!({
            "model": self.config.parameters.get("model").unwrap_or(&json!("claude-3-5-sonnet-20240620")),
            "messages": [
                {
                    "role": "user",
                    "content": request.payload
                }
            ],
            "stream": true,
            "max_tokens": self.config.parameters.get("max_tokens").unwrap_or(&json!(1000))
        });

        debug!("Starting Anthropic streaming request to: {}", url);

        // Send streaming request
        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Anthropic API error: {}", error_text));
        }

        // Create stream from response - simplified approach
        let stream = async_stream::stream! {
            let mut buffer = String::new();
            let mut bytes_stream = response.bytes_stream();

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Process complete lines
                        let current_buffer = buffer.clone();
                        let lines: Vec<&str> = current_buffer.lines().collect();

                        if !buffer.ends_with('\n') && !lines.is_empty() {
                            // Keep the last incomplete line in buffer
                            let last_line = lines.last().unwrap();
                            buffer = last_line.to_string();

                            // Process all but the last line
                            for line in &lines[..lines.len()-1] {
                                match Self::parse_anthropic_chunk(line) {
                                    Ok(Some(chunk)) => yield Ok(chunk),
                                    Ok(None) => continue,
                                    Err(e) => {
                                        warn!("Failed to parse chunk: {}", e);
                                        continue;
                                    }
                                }
                            }
                        } else {
                            buffer.clear();

                            // Process all lines
                            for line in lines {
                                match Self::parse_anthropic_chunk(line) {
                                    Ok(Some(chunk)) => yield Ok(chunk),
                                    Ok(None) => continue,
                                    Err(e) => {
                                        warn!("Failed to parse chunk: {}", e);
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow!("Stream error: {}", e));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn get_streaming_config(&self) -> StreamingConfig {
        StreamingConfig::default()
    }
}

/// Utility functions for streaming
pub struct StreamingUtils;

impl StreamingUtils {
    /// Collect a stream into a single response
    pub async fn collect_stream(
        mut stream: ResponseStream,
    ) -> Result<fluent_core::types::Response> {
        let mut content = String::new();
        let mut total_prompt_tokens = 0u32;
        let mut total_completion_tokens = 0u32;
        let mut model = String::new();
        let mut finish_reason = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            content.push_str(&chunk.content);

            if let Some(usage) = chunk.token_usage {
                if let Some(prompt) = usage.prompt_tokens {
                    total_prompt_tokens = prompt;
                }
                if let Some(completion) = usage.completion_tokens {
                    total_completion_tokens += completion;
                }
            }

            if let Some(chunk_model) = chunk.model {
                model = chunk_model;
            }

            if chunk.is_final {
                finish_reason = chunk.finish_reason;
                break;
            }
        }

        let total_tokens = total_prompt_tokens + total_completion_tokens;

        Ok(fluent_core::types::Response {
            content,
            usage: fluent_core::types::Usage {
                prompt_tokens: total_prompt_tokens,
                completion_tokens: total_completion_tokens,
                total_tokens,
            },
            model,
            finish_reason,
            cost: fluent_core::types::Cost {
                prompt_cost: 0.0, // TODO: Calculate based on pricing
                completion_cost: 0.0,
                total_cost: 0.0,
            },
        })
    }

    /// Create a progress callback for streaming
    pub fn create_progress_callback<F>(mut callback: F) -> impl FnMut(StreamChunk) -> Result<()>
    where
        F: FnMut(&str) + Send + 'static,
    {
        move |chunk: StreamChunk| {
            if !chunk.content.is_empty() {
                callback(&chunk.content);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openai_chunk() {
        let chunk_line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"delta":{"content":"Hello"},"index":0,"finish_reason":null}]}"#;

        let result = OpenAIStreaming::parse_openai_chunk(chunk_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_parse_openai_done() {
        let done_line = "data: [DONE]";

        let result = OpenAIStreaming::parse_openai_chunk(done_line).unwrap();
        assert!(result.is_some());

        let chunk = result.unwrap();
        assert!(chunk.is_final);
        assert_eq!(chunk.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.buffer_size, 8192);
        assert_eq!(config.chunk_timeout_ms, 5000);
    }
}

// Include comprehensive test suite
#[path = "streaming_engine_tests.rs"]
mod streaming_engine_tests;
