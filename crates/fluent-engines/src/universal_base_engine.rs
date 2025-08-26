use crate::cache_manager::global_cache_manager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

/// Universal base engine that provides common functionality for all engines
pub struct UniversalBaseEngine {
    pub config: EngineConfig,
    pub engine_type: String,
    pub client: Client,
    pub neo4j_client: Option<Arc<Neo4jClient>>,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    pub default_model: String,
    pub pricing_rates: Option<(f64, f64)>, // (prompt_rate, completion_rate) per 1M tokens
}

impl UniversalBaseEngine {
    /// Create a new universal base engine
    pub async fn new(
        config: EngineConfig,
        engine_type: String,
        supports_vision: bool,
        supports_streaming: bool,
        default_model: String,
        pricing_rates: Option<(f64, f64)>,
    ) -> Result<Self> {
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

        Ok(Self {
            config,
            engine_type,
            client,
            neo4j_client,
            supports_vision,
            supports_streaming,
            default_model,
            pricing_rates,
        })
    }

    /// Build the complete URL for API requests
    pub fn build_url(&self, path: Option<&str>) -> String {
        let base_path = path.unwrap_or(&self.config.connection.request_path);
        format!(
            "{}://{}:{}{}",
            self.config.connection.protocol,
            self.config.connection.hostname,
            self.config.connection.port,
            base_path
        )
    }

    /// Get authentication headers for the engine
    pub fn get_auth_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();

        match self.engine_type.as_str() {
            "openai" => {
                if let Some(token) = self
                    .config
                    .parameters
                    .get("bearer_token")
                    .and_then(|v| v.as_str())
                {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                } else {
                    return Err(anyhow!("Bearer token not found for OpenAI"));
                }
            }
            "anthropic" => {
                if let Some(token) = self
                    .config
                    .parameters
                    .get("bearer_token")
                    .and_then(|v| v.as_str())
                {
                    headers.insert("x-api-key".to_string(), token.to_string());
                    headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
                } else {
                    return Err(anyhow!("Bearer token not found for Anthropic"));
                }
            }
            "google_gemini" => {
                if let Some(token) = self
                    .config
                    .parameters
                    .get("bearer_token")
                    .and_then(|v| v.as_str())
                {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                } else {
                    return Err(anyhow!("Bearer token not found for Google Gemini"));
                }
            }
            _ => {
                // Generic bearer token handling for other engines
                if let Some(token) = self
                    .config
                    .parameters
                    .get("bearer_token")
                    .and_then(|v| v.as_str())
                {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", token));
                }
            }
        }

        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(headers)
    }

    /// Execute a request with caching and error handling
    pub async fn execute_request(&self, request: &Request) -> Result<Response> {
        // Check cache first
        let cache_manager = global_cache_manager().await;
        let model = self.get_model_name();

        if let Ok(Some(cached_response)) = cache_manager
            .get_cached_response(
                &self.engine_type,
                request,
                Some(&model),
                Some(&self.config.parameters),
            )
            .await
        {
            debug!("Cache hit for {} request", self.engine_type);
            return Ok(cached_response);
        }

        // Build payload based on engine type
        let payload = self.build_chat_payload(request)?;

        // Send HTTP request
        let response_data = self.send_http_request(&payload).await?;

        // Parse response based on engine type
        let response = self.parse_response(&response_data, request)?;

        // Cache the response
        if let Err(e) = cache_manager
            .cache_response(
                &self.engine_type,
                request,
                &response,
                Some(&model),
                Some(&self.config.parameters),
            )
            .await
        {
            debug!("Failed to cache response: {}", e);
        }

        Ok(response)
    }

    /// Send HTTP request with proper authentication and error handling
    async fn send_http_request(&self, payload: &Value) -> Result<Value> {
        let url = self.build_url(None);
        let headers = self.get_auth_headers()?;

        let mut request_builder = self.client.post(&url).json(payload);

        // Add authentication headers
        for (key, value) in headers {
            request_builder = request_builder.header(&key, &value);
        }

        let response = request_builder.send().await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let redacted = fluent_core::redaction::redact_secrets_in_text(&error_text);
            return Err(anyhow!("{} API error: {}", self.engine_type, redacted));
        }

        let response_body = response.json::<Value>().await?;
        Ok(response_body)
    }

    /// Build chat payload based on engine type
    fn build_chat_payload(&self, request: &Request) -> Result<Value> {
        let model = self.get_model_name();

        match self.engine_type.as_str() {
            "openai" => Ok(json!({
                "model": model,
                "messages": [
                    {
                        "role": "user",
                        "content": request.payload
                    }
                ],
                "temperature": self.config.parameters.get("temperature").unwrap_or(&json!(0.7)),
                "max_tokens": self.config.parameters.get("max_tokens").unwrap_or(&json!(1000))
            })),
            "anthropic" => Ok(json!({
                "model": model,
                "messages": [
                    {
                        "role": "user",
                        "content": request.payload
                    }
                ],
                "max_tokens": self.config.parameters.get("max_tokens").unwrap_or(&json!(1000))
            })),
            "google_gemini" => Ok(json!({
                "contents": [
                    {
                        "parts": [
                            {
                                "text": request.payload
                            }
                        ]
                    }
                ],
                "generationConfig": {
                    "temperature": self.config.parameters.get("temperature").unwrap_or(&json!(0.7)),
                    "maxOutputTokens": self.config.parameters.get("max_tokens").unwrap_or(&json!(1000))
                }
            })),
            _ => {
                // Generic payload format
                Ok(json!({
                    "model": model,
                    "prompt": request.payload,
                    "temperature": self.config.parameters.get("temperature").unwrap_or(&json!(0.7)),
                    "max_tokens": self.config.parameters.get("max_tokens").unwrap_or(&json!(1000))
                }))
            }
        }
    }

    /// Parse response based on engine type
    fn parse_response(&self, response_data: &Value, _request: &Request) -> Result<Response> {
        match self.engine_type.as_str() {
            "openai" => self.parse_openai_response(response_data),
            "anthropic" => self.parse_anthropic_response(response_data),
            "google_gemini" => self.parse_gemini_response(response_data),
            _ => self.parse_generic_response(response_data),
        }
    }

    /// Parse OpenAI response format
    fn parse_openai_response(&self, response_data: &Value) -> Result<Response> {
        let content = response_data["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to extract content from OpenAI response"))?
            .to_string();

        let usage = Usage {
            prompt_tokens: response_data["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: response_data["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: response_data["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let model = response_data["model"]
            .as_str()
            .unwrap_or(&self.default_model)
            .to_string();
        let finish_reason = response_data["choices"][0]["finish_reason"]
            .as_str()
            .map(String::from);

        let cost = self.calculate_cost(&usage);

        Ok(Response {
            content,
            usage,
            model,
            finish_reason,
            cost,
        })
    }

    /// Parse Anthropic response format
    fn parse_anthropic_response(&self, response_data: &Value) -> Result<Response> {
        let content = response_data["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to extract content from Anthropic response"))?
            .to_string();

        let usage = Usage {
            prompt_tokens: response_data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response_data["usage"]["output_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: (response_data["usage"]["input_tokens"].as_u64().unwrap_or(0)
                + response_data["usage"]["output_tokens"]
                    .as_u64()
                    .unwrap_or(0)) as u32,
        };

        let model = response_data["model"]
            .as_str()
            .unwrap_or(&self.default_model)
            .to_string();
        let finish_reason = response_data["stop_reason"].as_str().map(String::from);

        let cost = self.calculate_cost(&usage);

        Ok(Response {
            content,
            usage,
            model,
            finish_reason,
            cost,
        })
    }

    /// Parse Google Gemini response format
    fn parse_gemini_response(&self, response_data: &Value) -> Result<Response> {
        let content = response_data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to extract content from Gemini response"))?
            .to_string();

        // Gemini doesn't always provide detailed usage stats
        let usage = Usage {
            prompt_tokens: response_data["usageMetadata"]["promptTokenCount"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: response_data["usageMetadata"]["candidatesTokenCount"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: response_data["usageMetadata"]["totalTokenCount"]
                .as_u64()
                .unwrap_or(0) as u32,
        };

        let model = self.default_model.clone();
        let finish_reason = response_data["candidates"][0]["finishReason"]
            .as_str()
            .map(String::from);

        let cost = self.calculate_cost(&usage);

        Ok(Response {
            content,
            usage,
            model,
            finish_reason,
            cost,
        })
    }

    /// Parse generic response format
    fn parse_generic_response(&self, response_data: &Value) -> Result<Response> {
        // Try common response field patterns
        let content = response_data
            .get("text")
            .or_else(|| response_data.get("content"))
            .or_else(|| response_data.get("response"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Failed to extract content from response"))?
            .to_string();

        let usage = Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };

        let cost = Cost {
            prompt_cost: 0.0,
            completion_cost: 0.0,
            total_cost: 0.0,
        };

        Ok(Response {
            content,
            usage,
            model: self.default_model.clone(),
            finish_reason: None,
            cost,
        })
    }

    /// Calculate cost based on usage and pricing rates
    fn calculate_cost(&self, usage: &Usage) -> Cost {
        if let Some((prompt_rate, completion_rate)) = self.pricing_rates {
            let prompt_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * prompt_rate;
            let completion_cost = (usage.completion_tokens as f64 / 1_000_000.0) * completion_rate;
            let total_cost = prompt_cost + completion_cost;

            Cost {
                prompt_cost,
                completion_cost,
                total_cost,
            }
        } else {
            Cost {
                prompt_cost: 0.0,
                completion_cost: 0.0,
                total_cost: 0.0,
            }
        }
    }

    /// Get the model name from configuration or use default
    pub fn get_model_name(&self) -> String {
        self.config
            .parameters
            .get("model")
            .or_else(|| self.config.parameters.get("modelName"))
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_model)
            .to_string()
    }

    /// Get Neo4j client reference
    pub fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::config::{ConnectionConfig, EngineConfig};
    use serde_json::json;

    fn create_test_config() -> EngineConfig {
        EngineConfig {
            name: "test-engine".to_string(),
            engine: "test".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.test.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters: {
                let mut params = std::collections::HashMap::new();
                params.insert("bearer_token".to_string(), json!("test-token"));
                params.insert("model".to_string(), json!("test-model"));
                params
            },
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    #[tokio::test]
    async fn test_universal_base_engine_creation() {
        let config = create_test_config();
        let engine = UniversalBaseEngine::new(
            config,
            "test".to_string(),
            false,
            false,
            "test-model".to_string(),
            Some((0.001, 0.002)),
        )
        .await
        .unwrap();

        assert_eq!(engine.engine_type, "test");
        assert_eq!(engine.default_model, "test-model");
        assert!(!engine.supports_vision);
    }

    #[test]
    fn test_url_building() {
        let config = create_test_config();
        let engine = UniversalBaseEngine {
            config: config.clone(),
            engine_type: "test".to_string(),
            client: Client::new(),
            neo4j_client: None,
            supports_vision: false,
            supports_streaming: false,
            default_model: "test-model".to_string(),
            pricing_rates: None,
        };

        let url = engine.build_url(None);
        assert_eq!(url, "https://api.test.com:443/v1/chat/completions");

        let custom_url = engine.build_url(Some("/custom/path"));
        assert_eq!(custom_url, "https://api.test.com:443/custom/path");
    }
}

/// Wrapper struct that implements the Engine trait using UniversalBaseEngine
pub struct UniversalEngine {
    base: UniversalBaseEngine,
}

impl UniversalEngine {
    /// Create a new universal engine for OpenAI
    pub async fn openai(config: EngineConfig) -> Result<Self> {
        let base = UniversalBaseEngine::new(
            config,
            "openai".to_string(),
            true, // supports_vision
            true, // supports_streaming
            "gpt-4".to_string(),
            Some((0.01, 0.03)), // OpenAI GPT-4 pricing per 1M tokens
        )
        .await?;
        Ok(Self { base })
    }

    /// Create a new universal engine for Anthropic
    pub async fn anthropic(config: EngineConfig) -> Result<Self> {
        let base = UniversalBaseEngine::new(
            config,
            "anthropic".to_string(),
            true,  // supports_vision
            false, // supports_streaming
            "claude-sonnet-4-20250514".to_string(),
            Some((0.003, 0.015)), // Anthropic Claude pricing per 1M tokens
        )
        .await?;
        Ok(Self { base })
    }

    /// Create a new universal engine for Google Gemini
    pub async fn google_gemini(config: EngineConfig) -> Result<Self> {
        let base = UniversalBaseEngine::new(
            config,
            "google_gemini".to_string(),
            true, // supports_vision
            true, // supports_streaming
            "gemini-1.5-flash".to_string(),
            Some((0.00025, 0.00075)), // Gemini pricing per 1M tokens
        )
        .await?;
        Ok(Self { base })
    }
}

#[async_trait]
impl Engine for UniversalEngine {
    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move { self.base.execute_request(request).await })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move { Err(anyhow!("Upsert not implemented for universal engine")) })
    }

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.base.get_neo4j_client()
    }

    fn get_session_id(&self) -> Option<String> {
        self.base.config.session_id.clone()
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        // Extract content based on engine type and response structure
        match self.base.engine_type.as_str() {
            "openai" => self.extract_openai_content(value),
            "anthropic" => self.extract_anthropic_content(value),
            "google" => self.extract_google_content(value),
            _ => self.extract_generic_content(value),
        }
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File upload not implemented for universal engine"))
        })
    }

    fn process_request_with_file<'a>(
        &'a self,
        _request: &'a Request,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File processing not implemented for universal engine"))
        })
    }
}

impl UniversalEngine {
    /// Extract content from OpenAI response format
    fn extract_openai_content(&self, value: &Value) -> Option<ExtractedContent> {
        if let Some(choices) = value.get("choices").and_then(|c| c.as_array()) {
            if let Some(first_choice) = choices.first() {
                if let Some(message) = first_choice.get("message") {
                    let content = message.get("content")?.as_str()?.to_string();

                    return Some(ExtractedContent {
                        main_content: content,
                        sentiment: None,
                        clusters: None,
                        themes: None,
                        keywords: None,
                    });
                }
            }
        }
        None
    }

    /// Extract content from Anthropic response format
    fn extract_anthropic_content(&self, value: &Value) -> Option<ExtractedContent> {
        if let Some(content_array) = value.get("content").and_then(|c| c.as_array()) {
            if let Some(first_content) = content_array.first() {
                if let Some(text) = first_content.get("text").and_then(|t| t.as_str()) {
                    return Some(ExtractedContent {
                        main_content: text.to_string(),
                        sentiment: None,
                        clusters: None,
                        themes: None,
                        keywords: None,
                    });
                }
            }
        }
        None
    }

    /// Extract content from Google response format
    fn extract_google_content(&self, value: &Value) -> Option<ExtractedContent> {
        if let Some(candidates) = value.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first_candidate) = candidates.first() {
                if let Some(content) = first_candidate.get("content") {
                    if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                        if let Some(first_part) = parts.first() {
                            if let Some(text) = first_part.get("text").and_then(|t| t.as_str()) {
                                return Some(ExtractedContent {
                                    main_content: text.to_string(),
                                    sentiment: None,
                                    clusters: None,
                                    themes: None,
                                    keywords: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract content from generic response format
    fn extract_generic_content(&self, value: &Value) -> Option<ExtractedContent> {
        // Try common content fields
        if let Some(text) = value.get("text").and_then(|t| t.as_str()) {
            return Some(ExtractedContent {
                main_content: text.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            });
        }

        if let Some(content) = value.get("content").and_then(|c| c.as_str()) {
            return Some(ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            });
        }

        if let Some(response) = value.get("response").and_then(|r| r.as_str()) {
            return Some(ExtractedContent {
                main_content: response.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            });
        }

        None
    }

}
