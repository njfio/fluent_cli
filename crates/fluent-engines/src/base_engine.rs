use crate::connection_pool::{get_pooled_client, return_pooled_client};
use crate::enhanced_cache::{CacheKey, EnhancedCache};
use crate::shared::*;
use anyhow::{anyhow, Result};
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse};
use serde_json::Value;

use std::path::Path;
use std::sync::Arc;
use tokio::sync::OnceCell;

/// Base engine configuration for common functionality
#[derive(Debug, Clone)]
pub struct BaseEngineConfig {
    pub engine_type: String,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    pub supports_file_upload: bool,
    pub supports_embeddings: bool,
    pub default_model: String,
    pub pricing_rates: Option<(f64, f64)>, // (prompt_rate, completion_rate) per 1M tokens
}

impl BaseEngineConfig {
    pub fn openai() -> Self {
        Self {
            engine_type: "openai".to_string(),
            supports_vision: true,
            supports_streaming: true,
            supports_file_upload: false,
            supports_embeddings: true,
            default_model: "gpt-3.5-turbo".to_string(),
            pricing_rates: Some((0.0015, 0.002)), // GPT-3.5-turbo rates
        }
    }

    pub fn anthropic() -> Self {
        Self {
            engine_type: "anthropic".to_string(),
            supports_vision: true,
            supports_streaming: true,
            supports_file_upload: false,
            supports_embeddings: false,
            default_model: "claude-3-sonnet".to_string(),
            pricing_rates: Some((0.003, 0.015)), // Claude-3-sonnet rates
        }
    }

    pub fn google_gemini() -> Self {
        Self {
            engine_type: "google_gemini".to_string(),
            supports_vision: true,
            supports_streaming: true,
            supports_file_upload: false,
            supports_embeddings: true,
            default_model: "gemini-1.5-flash".to_string(),
            pricing_rates: Some((0.00025, 0.00075)), // Gemini-1.5-flash rates
        }
    }

    pub fn webhook() -> Self {
        Self {
            engine_type: "webhook".to_string(),
            supports_vision: false,
            supports_streaming: false,
            supports_file_upload: true,
            supports_embeddings: false,
            default_model: "webhook".to_string(),
            pricing_rates: None,
        }
    }
}

/// Base engine implementation that provides common functionality
pub struct BaseEngine {
    pub config: EngineConfig,
    pub base_config: BaseEngineConfig,
    pub neo4j_client: Option<Arc<Neo4jClient>>,
    pub cache: OnceCell<Arc<EnhancedCache>>,
}

impl BaseEngine {
    /// Create a new base engine
    pub async fn new(config: EngineConfig, base_config: BaseEngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            base_config,
            neo4j_client,
            cache: OnceCell::new(),
        })
    }

    /// Get or create the cache instance
    pub async fn get_cache(&self) -> Result<&Arc<EnhancedCache>> {
        self.cache
            .get_or_try_init(|| async {
                let cache_config = crate::enhanced_cache::CacheConfig {
                    disk_cache_dir: Some(format!("fluent_cache_{}", self.base_config.engine_type)),
                    ..Default::default()
                };
                Ok(Arc::new(EnhancedCache::new(cache_config)?))
            })
            .await
    }

    /// Execute a standard chat request
    pub async fn execute_chat_request(&self, request: &Request) -> Result<Response> {
        // Check cache first
        let cache_key = CacheKey::new(&request.payload, &self.base_config.engine_type)
            .with_model(&self.get_model_name())
            .with_parameters(&self.config.parameters);

        if let Ok(cache) = self.get_cache().await {
            if let Ok(Some(cached_response)) = cache.get(&cache_key).await {
                return Ok(cached_response);
            }
        }

        // Get pooled HTTP client
        let client = get_pooled_client(&self.config).await?;

        // Build request
        let url = UrlBuilder::build_default_url(&self.config);
        let payload = self.build_chat_payload(request)?;

        // Send request
        let response = client.post(&url).json(&payload).send().await?;

        // Return client to pool
        return_pooled_client(&self.config, client).await;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let redacted = fluent_core::redaction::redact_secrets_in_text(&error_text);
            return Err(anyhow!(
                "{} API error: {}",
                self.base_config.engine_type,
                redacted
            ));
        }

        let response_json: Value = response.json().await?;

        // Parse response based on engine type
        let parsed_response = self.parse_response(&response_json)?;

        // Cache the response
        if let Ok(cache) = self.get_cache().await {
            let _ = cache.insert(&cache_key, &parsed_response).await;
        }

        Ok(parsed_response)
    }

    /// Execute a vision request with file
    pub async fn execute_vision_request(
        &self,
        request: &Request,
        file_path: &Path,
    ) -> Result<Response> {
        if !self.base_config.supports_vision {
            return Err(anyhow!(
                "{} does not support vision requests",
                self.base_config.engine_type
            ));
        }

        // Check cache with file
        let cache_key = CacheKey::new(&request.payload, &self.base_config.engine_type)
            .with_model(&self.get_model_name())
            .with_file(file_path)?
            .with_parameters(&self.config.parameters);

        if let Ok(cache) = self.get_cache().await {
            if let Ok(Some(cached_response)) = cache.get(&cache_key).await {
                return Ok(cached_response);
            }
        }

        // Validate and encode file
        FileHandler::validate_file_size(file_path, 10).await?; // 10MB limit
        let base64_data = FileHandler::encode_file_base64(file_path).await?;
        let image_format = FileHandler::get_image_format(file_path);

        // Get pooled HTTP client
        let client = get_pooled_client(&self.config).await?;

        // Build vision payload
        let payload = self.build_vision_payload(request, &base64_data, &image_format)?;
        let url = UrlBuilder::build_default_url(&self.config);

        // Send request
        let response = client.post(&url).json(&payload).send().await?;

        // Return client to pool
        return_pooled_client(&self.config, client).await;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let redacted = fluent_core::redaction::redact_secrets_in_text(&error_text);
            return Err(anyhow!(
                "{} API error: {}",
                self.base_config.engine_type,
                redacted
            ));
        }

        let response_json: Value = response.json().await?;
        let parsed_response = self.parse_response(&response_json)?;

        // Cache the response
        if let Ok(cache) = self.get_cache().await {
            let _ = cache.insert(&cache_key, &parsed_response).await;
        }

        Ok(parsed_response)
    }

    /// Handle upsert requests
    pub async fn handle_upsert(&self, request: &UpsertRequest) -> Result<UpsertResponse> {
        if !self.base_config.supports_embeddings {
            return Ok(UpsertResponse {
                processed_files: vec![],
                errors: vec![format!(
                    "{} does not support embeddings",
                    self.base_config.engine_type
                )],
            });
        }

        // For engines that support embeddings, this would be implemented
        // For now, return a placeholder response
        Ok(UpsertResponse {
            processed_files: vec![request.input.clone()],
            errors: vec![],
        })
    }

    /// Get the model name from configuration
    pub fn get_model_name(&self) -> String {
        self.config
            .parameters
            .get("model")
            .or_else(|| self.config.parameters.get("modelName"))
            .and_then(|v| v.as_str())
            .unwrap_or(&self.base_config.default_model)
            .to_string()
    }

    /// Build chat payload based on engine type
    fn build_chat_payload(&self, request: &Request) -> Result<Value> {
        match self.base_config.engine_type.as_str() {
            "openai" => {
                let mut payload =
                    PayloadBuilder::build_chat_payload(request, Some(&self.get_model_name()));
                self.add_config_parameters(&mut payload);
                Ok(payload)
            }
            "anthropic" => {
                // Use the generic chat payload for Anthropic
                let mut payload =
                    PayloadBuilder::build_chat_payload(request, Some(&self.get_model_name()));
                // Anthropic uses "max_tokens" instead of "max_completion_tokens"
                if let Some(max_tokens) = self.config.parameters.get("max_tokens") {
                    payload["max_tokens"] = max_tokens.clone();
                } else {
                    payload["max_tokens"] = serde_json::json!(4096);
                }
                self.add_config_parameters(&mut payload);
                Ok(payload)
            }
            "google_gemini" => {
                // Use the generic chat payload for Gemini
                let mut payload =
                    PayloadBuilder::build_chat_payload(request, Some(&self.get_model_name()));
                self.add_config_parameters(&mut payload);
                Ok(payload)
            }
            _ => {
                // Generic payload for other engines
                let mut payload =
                    PayloadBuilder::build_chat_payload(request, Some(&self.get_model_name()));
                self.add_config_parameters(&mut payload);
                Ok(payload)
            }
        }
    }

    /// Build vision payload based on engine type
    fn build_vision_payload(
        &self,
        request: &Request,
        base64_data: &str,
        image_format: &str,
    ) -> Result<Value> {
        match self.base_config.engine_type.as_str() {
            "openai" | "anthropic" => Ok(PayloadBuilder::build_vision_payload(
                &request.payload,
                base64_data,
                image_format,
            )),
            "google_gemini" => {
                // Use the standard vision payload for Gemini as well
                Ok(PayloadBuilder::build_vision_payload(
                    &request.payload,
                    base64_data,
                    image_format,
                ))
            }
            _ => Err(anyhow!(
                "Vision not supported for engine type: {}",
                self.base_config.engine_type
            )),
        }
    }

    /// Parse response based on engine type
    fn parse_response(&self, response_json: &Value) -> Result<Response> {
        match self.base_config.engine_type.as_str() {
            "openai" => {
                let pricing = self.base_config.pricing_rates.unwrap_or((0.001, 0.002));
                ResponseParser::parse_openai_chat_response(
                    response_json,
                    &self.get_model_name(),
                    Some(pricing),
                )
            }
            "anthropic" => {
                let pricing = self.base_config.pricing_rates.unwrap_or((0.003, 0.015));
                ResponseParser::parse_anthropic_response(
                    response_json,
                    &self.get_model_name(),
                    Some(pricing),
                )
            }
            "google_gemini" => {
                let pricing = self.base_config.pricing_rates.unwrap_or((0.00025, 0.00075));
                ResponseParser::parse_gemini_response(
                    response_json,
                    &self.get_model_name(),
                    Some(pricing),
                )
            }
            _ => {
                // Generic response parsing
                let content = response_json
                    .get("content")
                    .or_else(|| response_json.get("text"))
                    .or_else(|| response_json.get("response"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("No content found")
                    .to_string();

                Ok(ResponseParser::parse_simple_response(
                    content,
                    &self.get_model_name(),
                    None,
                ))
            }
        }
    }

    /// Add configuration parameters to payload
    fn add_config_parameters(&self, payload: &mut Value) {
        for (key, value) in &self.config.parameters {
            match key.as_str() {
                "temperature" | "max_tokens" | "top_p" | "frequency_penalty"
                | "presence_penalty" => {
                    payload[key] = value.clone();
                }
                _ => {} // Skip other parameters
            }
        }
    }

    /// Extract content for the engine type
    pub fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        match self.base_config.engine_type.as_str() {
            "openai" => ResponseParser::extract_content_openai(value),
            "anthropic" => ResponseParser::extract_content_anthropic(value),
            "google_gemini" => ResponseParser::extract_content_gemini(value),
            _ => ResponseParser::extract_content_generic(value),
        }
    }

    /// Get session ID from configuration
    pub fn get_session_id(&self) -> Option<String> {
        self.config.session_id.clone().or_else(|| {
            self.config
                .parameters
                .get("sessionID")
                .or_else(|| self.config.parameters.get("session_id"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
    }

    /// Get Neo4j client reference
    pub fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> EngineConfig {
        let mut parameters = HashMap::new();
        parameters.insert("model".to_string(), serde_json::json!("gpt-4"));
        parameters.insert("temperature".to_string(), serde_json::json!(0.7));

        EngineConfig {
            name: "test".to_string(),
            engine: "openai".to_string(),
            connection: fluent_core::config::ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.openai.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters,
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    #[tokio::test]
    async fn test_base_engine_creation() {
        let config = create_test_config();
        let base_config = BaseEngineConfig::openai();
        let engine = BaseEngine::new(config, base_config).await.unwrap();

        assert_eq!(engine.get_model_name(), "gpt-4");
        assert_eq!(engine.base_config.engine_type, "openai");
        assert!(engine.base_config.supports_vision);
    }

    #[test]
    fn test_base_engine_configs() {
        let openai_config = BaseEngineConfig::openai();
        assert_eq!(openai_config.engine_type, "openai");
        assert!(openai_config.supports_vision);
        assert!(openai_config.supports_embeddings);

        let anthropic_config = BaseEngineConfig::anthropic();
        assert_eq!(anthropic_config.engine_type, "anthropic");
        assert!(anthropic_config.supports_vision);
        assert!(!anthropic_config.supports_embeddings);
    }

    #[tokio::test]
    async fn test_model_name_extraction() {
        let config = create_test_config();
        let base_config = BaseEngineConfig::openai();
        let engine = BaseEngine::new(config, base_config).await.unwrap();

        assert_eq!(engine.get_model_name(), "gpt-4");
    }
}
