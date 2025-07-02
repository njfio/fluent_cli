use crate::streaming_engine::{StreamingEngine, OpenAIStreaming, StreamingUtils, ResponseStream};
use fluent_core::config::EngineConfig;
use fluent_core::traits::Engine;
use fluent_core::types::{Request, Response, UpsertRequest, UpsertResponse, ExtractedContent};
use fluent_core::neo4j_client::Neo4jClient;
use anyhow::{Result, anyhow};
use serde_json::Value;
use std::sync::Arc;
use std::path::Path;
use std::future::Future;
use async_trait::async_trait;
use reqwest::Client;
use log::debug;

/// OpenAI engine with streaming support
pub struct OpenAIStreamingEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
    streaming: OpenAIStreaming,
}

impl OpenAIStreamingEngine {
    /// Create a new OpenAI streaming engine
    pub async fn new(config: EngineConfig) -> Result<Self> {
        // Create optimized HTTP client
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        // Initialize Neo4j client if configured
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        // Create streaming implementation
        let streaming = OpenAIStreaming::new(client.clone(), config.clone());

        Ok(Self {
            config,
            client,
            neo4j_client,
            streaming,
        })
    }

    /// Execute request with streaming support
    pub async fn execute_streaming(&self, request: &Request) -> Result<ResponseStream> {
        debug!("Executing OpenAI request with streaming");
        self.streaming.execute_stream(request).await
    }

    /// Execute request and collect streaming response into a single response
    pub async fn execute_collected(&self, request: &Request) -> Result<Response> {
        let stream = self.execute_streaming(request).await?;
        StreamingUtils::collect_stream(stream).await
    }

    /// Execute request with progress callback
    pub async fn execute_with_progress<F>(&self, request: &Request, mut progress_callback: F) -> Result<Response>
    where
        F: FnMut(&str) + Send + 'static,
    {
        let mut stream = self.execute_streaming(request).await?;
        let mut content = String::new();
        let mut total_prompt_tokens = 0u32;
        let mut total_completion_tokens = 0u32;
        let mut model = String::new();
        let mut finish_reason = None;

        while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
            let chunk = chunk_result?;
            
            if !chunk.content.is_empty() {
                content.push_str(&chunk.content);
                progress_callback(&chunk.content);
            }
            
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
        
        Ok(Response {
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

    /// Check if streaming is enabled in configuration
    pub fn is_streaming_enabled(&self) -> bool {
        self.config.parameters.get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}

#[async_trait]
impl Engine for OpenAIStreamingEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            if self.is_streaming_enabled() {
                // Use streaming and collect the result
                self.execute_collected(request).await
            } else {
                // Fall back to non-streaming implementation
                self.execute_collected(request).await
            }
        })
    }

    fn upsert<'a>(&'a self, _request: &'a UpsertRequest) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("Upsert not implemented for OpenAI streaming engine"))
        })
    }

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }

    fn get_session_id(&self) -> Option<String> {
        self.config.session_id.clone()
    }

    fn extract_content(&self, _value: &Value) -> Option<ExtractedContent> {
        None // TODO: Implement content extraction
    }

    fn upload_file<'a>(&'a self, _file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File upload not implemented for OpenAI streaming engine"))
        })
    }

    fn process_request_with_file<'a>(&'a self, _request: &'a Request, _file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File processing not implemented for OpenAI streaming engine"))
        })
    }
}

#[async_trait]
impl StreamingEngine for OpenAIStreamingEngine {
    async fn execute_stream(&self, request: &Request) -> Result<ResponseStream> {
        self.streaming.execute_stream(request).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn get_streaming_config(&self) -> crate::streaming_engine::StreamingConfig {
        self.streaming.get_streaming_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::config::{EngineConfig, ConnectionConfig};
    use serde_json::json;

    fn create_openai_config() -> EngineConfig {
        EngineConfig {
            name: "openai-streaming-test".to_string(),
            engine: "openai".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.openai.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert("bearer_token".to_string(), json!("test-token"));
                params.insert("model".to_string(), json!("gpt-4"));
                params.insert("stream".to_string(), json!(true));
                params
            },
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    #[tokio::test]
    async fn test_openai_streaming_engine_creation() {
        let config = create_openai_config();
        let engine = OpenAIStreamingEngine::new(config).await.unwrap();
        
        assert!(engine.supports_streaming());
        assert!(engine.is_streaming_enabled());
    }

    #[tokio::test]
    async fn test_streaming_config() {
        let config = create_openai_config();
        let engine = OpenAIStreamingEngine::new(config).await.unwrap();
        
        let streaming_config = engine.get_streaming_config();
        assert!(streaming_config.enabled);
        assert_eq!(streaming_config.buffer_size, 8192);
    }

    #[test]
    fn test_streaming_enabled_detection() {
        let mut config = create_openai_config();
        
        // Test with streaming enabled
        config.parameters.insert("stream".to_string(), json!(true));
        let engine = tokio_test::block_on(OpenAIStreamingEngine::new(config.clone())).unwrap();
        assert!(engine.is_streaming_enabled());
        
        // Test with streaming disabled
        config.parameters.insert("stream".to_string(), json!(false));
        let engine = tokio_test::block_on(OpenAIStreamingEngine::new(config)).unwrap();
        assert!(!engine.is_streaming_enabled());
    }
}

/// Usage examples for the streaming engine
/// 
/// ```rust
/// use fluent_engines::openai_streaming::OpenAIStreamingEngine;
/// use fluent_core::types::Request;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = create_openai_config();
///     let engine = OpenAIStreamingEngine::new(config).await?;
///     
///     let request = Request {
///         flowname: "test".to_string(),
///         payload: "Hello, how are you?".to_string(),
///     };
///     
///     // Option 1: Use streaming with progress callback
///     let response = engine.execute_with_progress(&request, |chunk| {
///         print!("{}", chunk); // Print each chunk as it arrives
///         std::io::stdout().flush().unwrap();
///     }).await?;
///     
///     // Option 2: Use streaming and collect into single response
///     let response = engine.execute_collected(&request).await?;
///     
///     // Option 3: Use raw streaming
///     let mut stream = engine.execute_streaming(&request).await?;
///     while let Some(chunk) = stream.next().await {
///         let chunk = chunk?;
///         if !chunk.content.is_empty() {
///             print!("{}", chunk.content);
///         }
///         if chunk.is_final {
///             break;
///         }
///     }
///     
///     Ok(())
/// }
/// ```
pub struct _UsageExamples;
