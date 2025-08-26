use crate::enhanced_cache::{CacheConfig, CacheKey, EnhancedCache};
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine as Base64Engine;
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::{AnthropicConfigProcessor, Engine, EngineConfigProcessor};
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use log::debug;
use mime_guess::from_path;
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use tokio::time::{timeout, Duration};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct AnthropicEngine {
    config: EngineConfig,
    config_processor: AnthropicConfigProcessor,
    neo4j_client: Option<Arc<Neo4jClient>>,
    client: Client,                    // Reusable HTTP client
    cache: Option<Arc<EnhancedCache>>, // Response caching
}

impl AnthropicEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        // Create reusable HTTP client with optimized settings
        let mut client_builder = Client::builder()
            .timeout(std::time::Duration::from_secs(60))  // Increased from 30 to 60 seconds
            .connect_timeout(std::time::Duration::from_secs(30))  // Increased from 10 to 30 seconds
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(60));
            
        // Check for proxy settings from environment variables
        if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                client_builder = client_builder.proxy(proxy);
                debug!("Using HTTPS proxy");
            }
        } else if let Ok(proxy_url) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                client_builder = client_builder.proxy(proxy);
                debug!("Using HTTP proxy");
            }
        }
            
        let client = client_builder
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        // Initialize cache if enabled
        let cache = if std::env::var("FLUENT_CACHE").ok().as_deref() == Some("1") {
            let cache_config = CacheConfig {
                disk_cache_dir: Some("fluent_cache_anthropic".to_string()),
                ..Default::default()
            };
            Some(Arc::new(EnhancedCache::new(cache_config)?))
        } else {
            None
        };

        Ok(Self {
            config,
            config_processor: AnthropicConfigProcessor,
            neo4j_client,
            client,
            cache,
        })
    }

    fn pricing(model: &str) -> (f64, f64) {
        if model.contains("haiku") {
            (0.00000025, 0.00000125)
        } else if model.contains("sonnet") {
            (0.000003, 0.000015)
        } else if model.contains("opus") {
            (0.000015, 0.000075)
        } else {
            (0.0, 0.0)
        }
    }
}

impl Engine for AnthropicEngine {
    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }

    fn get_session_id(&self) -> Option<String> {
        self.config
            .parameters
            .get("sessionID")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        let main_content = value.get("completion").and_then(|v| v.as_str())?;

        let sentiment = value
            .get("sentiment")
            .and_then(|v| v.as_str())
            .map(String::from);

        let clusters = value.get("clusters").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

        let themes = value.get("themes").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

        let keywords = value.get("keywords").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });

        Some(ExtractedContent {
            main_content: main_content.to_string(),
            sentiment,
            clusters,
            themes,
            keywords,
        })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            use fluent_core::error::{EngineError, FluentError};

            // Anthropic doesn't have a native upsert/embedding API
            // They focus on conversational AI rather than embeddings
            Err(FluentError::Engine(EngineError::UnsupportedOperation {
                engine: "anthropic".to_string(),
                operation: "upsert".to_string(),
            })
            .into())
        })
    }

    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            // Check cache first if enabled
            if let Some(cache) = &self.cache {
                let cache_key = CacheKey::new(&request.payload, "anthropic")
                    .with_model("claude-sonnet-4-20250514"); // Default model

                if let Ok(Some(cached_response)) = cache.get(&cache_key).await {
                    debug!("Cache hit for Anthropic request");
                    return Ok(cached_response);
                }
            }

            debug!("Config: {:?}", self.config);

            let mut payload = self.config_processor.process_config(&self.config)?;

            // Add the user's request to the messages
            payload["messages"][0]["content"] = json!(request.payload);

            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path
            );

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let res = timeout(
                Duration::from_secs(600), // Increased from 300 to 600 seconds (10 minutes) for API calls
                self.client
                    .post(&url)
                    .header("x-api-key", auth_token)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&payload)
                    .send()
            )
            .await
            .map_err(|e| {
                debug!("Anthropic API request timeout error: {:?}", e);
                anyhow!("Anthropic API request timed out after 10 minutes or failed with timeout error: {:?}", e)
            })??;
            
            let response_body = timeout(
                Duration::from_secs(30), // 30 second timeout for response parsing
                res.json::<serde_json::Value>()
            )
            .await
            .map_err(|e| {
                debug!("Response parsing error: {:?}", e);
                anyhow!("Response parsing timed out after 30 seconds or failed with error: {:?}", e)
            })??;
            debug!("Response: {:?}", response_body);

            if let Some(error) = response_body.get("error") {
                return Err(anyhow!("Anthropic API error: {:?}", error));
            }

            let content = response_body["content"][0]["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from Anthropic response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response_body["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: response_body["usage"]["output_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                total_tokens: (response_body["usage"]["input_tokens"].as_u64().unwrap_or(0)
                    + response_body["usage"]["output_tokens"]
                        .as_u64()
                        .unwrap_or(0)) as u32,
            };

            let model = response_body["model"]
                .as_str()
                .unwrap_or("claude-sonnet-4-20250514")
                .to_string();
            let finish_reason = response_body["stop_reason"].as_str().map(String::from);

            let (prompt_rate, completion_rate) = AnthropicEngine::pricing(&model);
            let prompt_cost = usage.prompt_tokens as f64 * prompt_rate;
            let completion_cost = usage.completion_tokens as f64 * completion_rate;
            let total_cost = prompt_cost + completion_cost;

            let response = Response {
                content,
                usage,
                model: model.clone(),
                finish_reason,
                cost: Cost {
                    prompt_cost,
                    completion_cost,
                    total_cost,
                },
            };

            // Cache the response if caching is enabled
            if let Some(cache) = &self.cache {
                let cache_key = CacheKey::new(&request.payload, "anthropic").with_model(&model);

                if let Err(e) = cache.insert(&cache_key, &response).await {
                    debug!("Failed to cache response: {}", e);
                }
            }

            Ok(response)
        })
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            use fluent_core::error::{EngineError, FluentError};

            // Anthropic doesn't have a separate file upload API
            // Files are processed inline with vision requests
            Err(FluentError::Engine(EngineError::UnsupportedOperation {
                engine: "anthropic".to_string(),
                operation: "file_upload".to_string(),
            })
            .into())
        })
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            // Read and encode the file with timeout
            let mut file = timeout(
                Duration::from_secs(30), // 30 second timeout for file opening
                File::open(file_path)
            )
            .await
            .map_err(|_| anyhow!("File open timed out after 30 seconds"))?
            .context("Failed to open file")?;

            let mut buffer = Vec::new();
            timeout(
                Duration::from_secs(60), // 1 minute timeout for file reading
                file.read_to_end(&mut buffer)
            )
            .await
            .map_err(|_| anyhow!("File read timed out after 1 minute"))?
            .context("Failed to read file")?;
            let base64_image = Base64.encode(&buffer);

            // Guess the MIME type of the file
            let mime_type = from_path(file_path).first_or_octet_stream().to_string();

            // Use the reusable HTTP client
            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path
            );

            let payload = serde_json::json!({
                "model": "claude-sonnet-4-20250514",
                "max_tokens": 1024,
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": mime_type,
                                    "data": base64_image,
                                }
                            },
                            {
                                "type": "text",
                                "text": &request.payload
                            }
                        ]
                    }
                ]
            });

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = timeout(
                Duration::from_secs(600), // Increased from 300 to 600 seconds (10 minutes) for vision API calls
                self.client
                    .post(&url)
                    .header("x-api-key", auth_token)
                    .header("anthropic-version", "2023-06-01")
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
            )
            .await
            .map_err(|e| {
                debug!("Vision API request timeout error: {:?}", e);
                anyhow!("Vision API request timed out after 10 minutes or failed with timeout error: {:?}", e)
            })??;
            
            let response_body = timeout(
                Duration::from_secs(30), // 30 second timeout for response parsing
                response.json::<serde_json::Value>()
            )
            .await
            .map_err(|e| {
                debug!("Vision API response parsing error: {:?}", e);
                anyhow!("Vision API response parsing timed out after 30 seconds or failed with error: {:?}", e)
            })??;

            // Debug print the response
            debug!("Anthropic Response: {:?}", response_body);

            if let Some(error) = response_body.get("error") {
                return Err(anyhow!("Anthropic API error: {:?}", error));
            }

            let content = response_body["content"][0]["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from Anthropic response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response_body["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: response_body["usage"]["output_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                total_tokens: (response_body["usage"]["input_tokens"].as_u64().unwrap_or(0)
                    + response_body["usage"]["output_tokens"]
                        .as_u64()
                        .unwrap_or(0)) as u32,
            };

            let model = response_body["model"]
                .as_str()
                .unwrap_or("claude-3-5-sonnet-20240620")
                .to_string();
            let finish_reason = response_body["stop_reason"].as_str().map(String::from);

            let (prompt_rate, completion_rate) = AnthropicEngine::pricing(&model);
            let prompt_cost = usage.prompt_tokens as f64 * prompt_rate;
            let completion_cost = usage.completion_tokens as f64 * completion_rate;
            let total_cost = prompt_cost + completion_cost;

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
                cost: Cost {
                    prompt_cost,
                    completion_cost,
                    total_cost,
                },
            })
        })
    }
}
