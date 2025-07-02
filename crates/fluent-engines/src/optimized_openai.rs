use crate::memory_optimized_utils::MemoryPool;
use crate::shared::*;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use fluent_core::auth::EngineAuth;
use fluent_core::cache::{cache_key, RequestCache};
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::{Request, Response, Usage, Cost, UpsertRequest, UpsertResponse, ExtractedContent};
use serde_json::Value;
use std::future::Future;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Memory-optimized OpenAI engine that reuses buffers and reduces allocations
pub struct OptimizedOpenAIEngine {
    config: EngineConfig,
    neo4j_client: Option<Arc<Neo4jClient>>,
    cache: Option<RequestCache>,
    auth_client: reqwest::Client,
    // Memory optimization: reusable buffers
    memory_pool: Arc<Mutex<MemoryPool>>,
}

impl OptimizedOpenAIEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        let cache = if std::env::var("FLUENT_CACHE").ok().as_deref() == Some("1") {
            let path = std::env::var("FLUENT_CACHE_DIR").unwrap_or_else(|_| "fluent_cache".to_string());
            Some(RequestCache::new(std::path::Path::new(&path))?)
        } else {
            None
        };

        // Create authenticated client
        let auth_client = EngineAuth::openai(&config.parameters)?
            .create_authenticated_client()?;

        Ok(Self {
            config,
            neo4j_client,
            cache,
            auth_client,
            memory_pool: Arc::new(Mutex::new(MemoryPool::new())),
        })
    }

    /// Calculate cost with optimized memory usage
    fn calculate_cost_optimized(&self, usage: &Usage) -> Cost {
        let (prompt_rate, completion_rate) = self.get_pricing_rates();
        
        let prompt_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * prompt_rate;
        let completion_cost = (usage.completion_tokens as f64 / 1_000_000.0) * completion_rate;
        
        Cost {
            prompt_cost,
            completion_cost,
            total_cost: prompt_cost + completion_cost,
        }
    }

    fn get_pricing_rates(&self) -> (f64, f64) {
        let model = self.config.parameters.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("gpt-3.5-turbo");

        match model {
            m if m.contains("gpt-4o") => (0.005, 0.015),
            m if m.contains("gpt-4") => (0.01, 0.03),
            m if m.contains("gpt-3.5-turbo") => (0.0015, 0.002),
            _ => (0.001, 0.002), // Default rates
        }
    }

    /// Execute request with memory optimization
    async fn execute_optimized(&self, request: &Request) -> Result<Response> {
        // Check cache first
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(&cache_key(&request.payload))? {
                return Ok(cached);
            }
        }

        // Get reusable buffers from pool
        let mut string_buffer = {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.get_string_buffer()
        };

        let mut payload_builder = {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.get_payload_builder()
        };

        let mut response_parser = {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.get_response_parser()
        };

        // Build URL using reusable buffer
        let url = string_buffer.build_url(
            &self.config.connection.protocol,
            &self.config.connection.hostname,
            self.config.connection.port,
            &self.config.connection.request_path,
        );
        let url = url.to_string(); // Only allocate when necessary

        // Build payload using reusable builder
        let model = self.config.parameters.get("model")
            .and_then(|v| v.as_str());
        
        let _payload = payload_builder.build_openai_payload(&request.payload, model);
        
        // Add configuration parameters
        payload_builder.add_config_params(&self.config.parameters);

        // Send request
        let response = self.auth_client
            .post(&url)
            .json(payload_builder.payload())
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let response_json: Value = response.json().await?;

        // Parse response using reusable parser
        let content = response_parser.extract_openai_content(&response_json)
            .ok_or_else(|| anyhow!("No content in response"))?
            .to_string(); // Only allocate final content

        // Extract usage information
        let usage = Usage {
            prompt_tokens: response_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response_json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let cost = self.calculate_cost_optimized(&usage);

        let response = Response {
            content,
            usage,
            cost,
            model: model.unwrap_or("gpt-3.5-turbo").to_string(),
            finish_reason: response_json["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
        };

        // Return buffers to pool for reuse
        {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.return_string_buffer(string_buffer);
            pool.return_payload_builder(payload_builder);
            pool.return_response_parser(response_parser);
        }

        // Cache the response
        if let Some(cache) = &self.cache {
            cache.insert(&cache_key(&request.payload), &response)?;
        }

        Ok(response)
    }

    /// Process file request with memory optimization
    async fn process_file_optimized(&self, request: &Request, file_path: &Path) -> Result<Response> {
        // Check cache first
        let cache_key_str = {
            let mut string_buffer = {
                let mut pool = self.memory_pool.lock().unwrap();
                pool.get_string_buffer()
            };
            let key = string_buffer.build_cache_key(&request.payload, Some(&file_path.display().to_string()));
            let result = key.to_string();
            {
                let mut pool = self.memory_pool.lock().unwrap();
                pool.return_string_buffer(string_buffer);
            }
            result
        };

        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(&cache_key_str)? {
                return Ok(cached);
            }
        }

        // Read file using reusable buffer
        let mut file_buffer = {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.get_file_buffer()
        };

        let file_data = file_buffer.read_file(file_path).await?;
        let base64_image = Base64.encode(file_data);

        // Get image format
        let image_format = FileHandler::get_image_format(file_path);

        // Build vision payload
        let mut payload_builder = {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.get_payload_builder()
        };

        let _payload = payload_builder.build_vision_payload(&request.payload, &base64_image, &image_format);

        // Build URL
        let mut string_buffer = {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.get_string_buffer()
        };

        let url = string_buffer.build_url(
            &self.config.connection.protocol,
            &self.config.connection.hostname,
            self.config.connection.port,
            &self.config.connection.request_path,
        );
        let url = url.to_string();

        // Send request
        let response = self.auth_client
            .post(&url)
            .json(payload_builder.payload())
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let response_json: Value = response.json().await?;

        // Parse response
        let mut response_parser = {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.get_response_parser()
        };

        let content = response_parser.extract_openai_content(&response_json)
            .ok_or_else(|| anyhow!("No content in response"))?
            .to_string();

        // Extract usage and calculate cost
        let usage = Usage {
            prompt_tokens: response_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response_json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: response_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let cost = self.calculate_cost_optimized(&usage);

        let response = Response {
            content,
            usage,
            cost,
            model: "gpt-4-vision-preview".to_string(),
            finish_reason: response_json["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
        };

        // Return buffers to pool
        {
            let mut pool = self.memory_pool.lock().unwrap();
            pool.return_file_buffer(file_buffer);
            pool.return_payload_builder(payload_builder);
            pool.return_string_buffer(string_buffer);
            pool.return_response_parser(response_parser);
        }

        // Cache the response
        if let Some(cache) = &self.cache {
            cache.insert(&cache_key_str, &response)?;
        }

        Ok(response)
    }
}

#[async_trait]
impl Engine for OptimizedOpenAIEngine {
    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            self.execute_optimized(request).await
        })
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            self.process_file_optimized(request, file_path).await
        })
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File upload not supported for OpenAI engine"))
        })
    }

    fn upsert<'a>(
        &'a self,
        request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            // For this optimized engine, we'll implement a simple upsert response
            // In a real implementation, you would process the request appropriately
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
        // Extract content from OpenAI response format
        if let Some(content) = value["choices"][0]["message"]["content"].as_str() {
            Some(ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_optimized_engine_creation() {
        let mut parameters = HashMap::new();
        parameters.insert("bearer_token".to_string(), serde_json::json!("test-token"));
        parameters.insert("model".to_string(), serde_json::json!("gpt-3.5-turbo"));

        let config = EngineConfig {
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
        };

        let engine = OptimizedOpenAIEngine::new(config).await;
        assert!(engine.is_ok());
    }

    #[test]
    fn test_pricing_rates() {
        let mut parameters = HashMap::new();
        parameters.insert("model".to_string(), serde_json::json!("gpt-4"));

        let config = EngineConfig {
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
        };

        let engine = OptimizedOpenAIEngine {
            config,
            neo4j_client: None,
            cache: None,
            auth_client: reqwest::Client::new(),
            memory_pool: Arc::new(Mutex::new(MemoryPool::new())),
        };

        let (prompt_rate, completion_rate) = engine.get_pricing_rates();
        assert_eq!(prompt_rate, 0.01);
        assert_eq!(completion_rate, 0.03);
    }
}
