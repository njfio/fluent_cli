use crate::connection_pool::{get_pooled_client, return_pooled_client};
use crate::shared::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use serde_json::Value;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

/// Example OpenAI engine using connection pooling
pub struct PooledOpenAIEngine {
    config: EngineConfig,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl PooledOpenAIEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            neo4j_client,
        })
    }

    /// Execute request using pooled connection
    async fn execute_with_pool(&self, request: &Request) -> Result<Response> {
        // Get a client from the connection pool
        let client = get_pooled_client(&self.config).await?;

        // Build URL and payload
        let url = UrlBuilder::build_default_url(&self.config);
        let payload = PayloadBuilder::build_chat_payload(request, None);

        // Send request
        let response = client.post(&url).json(&payload).send().await?;

        // Return client to pool for reuse
        return_pooled_client(&self.config, client).await;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let response_json: Value = response.json().await?;

        // Parse response using shared utilities
        let content = ResponseParser::extract_content_openai(&response_json)
            .ok_or_else(|| anyhow!("Failed to extract content from response"))?;

        // Extract usage information
        let usage = Usage {
            prompt_tokens: response_json["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: response_json["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        // Calculate cost
        let cost = self.calculate_cost(&usage);

        let response = Response {
            content: content.main_content,
            usage,
            cost,
            model: self
                .config
                .parameters
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("gpt-3.5-turbo")
                .to_string(),
            finish_reason: response_json["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
        };

        Ok(response)
    }

    /// Process file request using pooled connection
    async fn process_file_with_pool(
        &self,
        request: &Request,
        file_path: &Path,
    ) -> Result<Response> {
        // Get a client from the connection pool
        let client = get_pooled_client(&self.config).await?;

        // Validate and read file
        FileHandler::validate_file_size(file_path, 10).await?; // 10MB limit
        let base64_data = FileHandler::encode_file_base64(file_path).await?;
        let image_format = FileHandler::get_image_format(file_path);

        // Build vision payload
        let payload =
            PayloadBuilder::build_vision_payload(&request.payload, &base64_data, &image_format);

        // Build URL
        let url = UrlBuilder::build_default_url(&self.config);

        // Send request
        let response = client.post(&url).json(&payload).send().await?;

        // Return client to pool for reuse
        return_pooled_client(&self.config, client).await;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let response_json: Value = response.json().await?;

        // Parse response
        let content = ResponseParser::extract_content_openai(&response_json)
            .ok_or_else(|| anyhow!("Failed to extract content from response"))?;

        // Extract usage and calculate cost
        let usage = Usage {
            prompt_tokens: response_json["usage"]["prompt_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            completion_tokens: response_json["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let cost = self.calculate_cost(&usage);

        let response = Response {
            content: content.main_content,
            usage,
            cost,
            model: "gpt-4-vision-preview".to_string(),
            finish_reason: response_json["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
        };

        Ok(response)
    }

    fn calculate_cost(&self, usage: &Usage) -> Cost {
        let model = self
            .config
            .parameters
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("gpt-3.5-turbo");

        let (prompt_rate, completion_rate) = match model {
            m if m.contains("gpt-4o") => (0.005, 0.015),
            m if m.contains("gpt-4") => (0.01, 0.03),
            m if m.contains("gpt-3.5-turbo") => (0.0015, 0.002),
            _ => (0.001, 0.002), // Default rates
        };

        let prompt_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * prompt_rate;
        let completion_cost = (usage.completion_tokens as f64 / 1_000_000.0) * completion_rate;

        Cost {
            prompt_cost,
            completion_cost,
            total_cost: prompt_cost + completion_cost,
        }
    }
}

#[async_trait]
impl Engine for PooledOpenAIEngine {
    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move { self.execute_with_pool(request).await })
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move { self.process_file_with_pool(request, file_path).await })
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move { Err(anyhow!("File upload not supported for OpenAI engine")) })
    }

    fn upsert<'a>(
        &'a self,
        request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            Ok(UpsertResponse {
                processed_files: vec![request.input.clone()],
                errors: vec![],
            })
        })
    }

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }

    fn get_session_id(&self) -> Option<String> {
        self.config.session_id.clone()
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        ResponseParser::extract_content_openai(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_pool::global_pool;
    use std::collections::HashMap;

    fn create_test_config() -> EngineConfig {
        let mut parameters = HashMap::new();
        parameters.insert("bearer_token".to_string(), serde_json::json!("test-token"));
        parameters.insert("model".to_string(), serde_json::json!("gpt-3.5-turbo"));

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
    async fn test_pooled_engine_creation() {
        let config = create_test_config();
        let engine = PooledOpenAIEngine::new(config).await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_connection_pool_usage() {
        let config = create_test_config();

        // Get initial pool stats
        let initial_stats = global_pool().get_stats();

        // Create engine and simulate getting a client
        let _engine = PooledOpenAIEngine::new(config.clone()).await.unwrap();
        let client = get_pooled_client(&config).await.unwrap();
        return_pooled_client(&config, client).await;

        // Check that pool stats changed
        let final_stats = global_pool().get_stats().await;
        assert!(final_stats.total_clients_created >= initial_stats.await.total_clients_created);
    }

    #[test]
    fn test_cost_calculation() {
        let config = create_test_config();
        let engine = PooledOpenAIEngine {
            config,
            neo4j_client: None,
        };

        let usage = Usage {
            prompt_tokens: 1000000,    // 1M tokens
            completion_tokens: 500000, // 0.5M tokens
            total_tokens: 1500000,
        };

        let cost = engine.calculate_cost(&usage);
        assert_eq!(cost.prompt_cost, 0.0015); // $0.0015 for 1M prompt tokens
        assert_eq!(cost.completion_cost, 0.001); // $0.002 for 0.5M completion tokens
        assert_eq!(cost.total_cost, 0.0025);
    }
}
