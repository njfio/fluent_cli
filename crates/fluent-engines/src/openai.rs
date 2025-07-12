use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine as Base64Engine;
use fluent_core::auth::EngineAuth;
use fluent_core::cache::{cache_key, RequestCache};
use fluent_core::config::EngineConfig;
use fluent_core::input_validator::InputValidator;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::{Engine, EngineConfigProcessor, OpenAIConfigProcessor};
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use log::debug;
use reqwest::multipart::{Form, Part};
use tokio::time::{timeout, Duration};

use serde_json::{json, Value};
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_util::codec::{BytesCodec, FramedRead};

pub struct OpenAIEngine {
    config: EngineConfig,
    config_processor: OpenAIConfigProcessor,
    neo4j_client: Option<Arc<Neo4jClient>>,
    cache: Option<RequestCache>,
    auth_client: reqwest::Client,
}

impl OpenAIEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        let cache = if std::env::var("FLUENT_CACHE").ok().as_deref() == Some("1") {
            let path =
                std::env::var("FLUENT_CACHE_DIR").unwrap_or_else(|_| "fluent_cache".to_string());
            Some(RequestCache::new(std::path::Path::new(&path))?)
        } else {
            None
        };

        // Create authenticated client
        let auth_client = EngineAuth::openai(&config.parameters)?.create_authenticated_client()?;

        Ok(Self {
            config,
            config_processor: OpenAIConfigProcessor,
            neo4j_client,
            cache,
            auth_client,
        })
    }

    fn pricing(model: &str) -> (f64, f64) {
        match model {
            m if m.contains("gpt-4o") => (0.000005, 0.000015),
            m if m.contains("gpt-4") => (0.00001, 0.00003),
            m if m.contains("gpt-3.5") => (0.0000015, 0.000002),
            _ => (0.0, 0.0),
        }
    }
}

impl Engine for OpenAIEngine {
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

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            // OpenAI supports embeddings through their embeddings API
            // This is a simplified implementation - in production you'd want to:
            // 1. Process the files in the request
            // 2. Generate embeddings using OpenAI's embeddings API
            // 3. Store them in a vector database
            // 4. Return proper status

            use fluent_core::error::{EngineError, FluentError};

            // For now, return an error indicating this needs proper implementation
            Err(FluentError::Engine(EngineError::UnsupportedOperation {
                engine: "openai".to_string(),
                operation: "upsert - requires embeddings API integration".to_string(),
            })
            .into())
        })
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        value
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .map(|content| ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }

    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            if let Some(cache) = &self.cache {
                if let Some(cached) = cache.get(&cache_key(&request.payload))? {
                    return Ok(cached);
                }
            }

            debug!("Config: {:?}", self.config);

            let mut payload = self.config_processor.process_config(&self.config)?;
            debug!("OpenAI Processed Config Payload: {:#?}", payload);

            // Add the user's request to the messages
            payload["messages"] = json!([
                {
                    "role": "user",
                    "content": request.payload
                }
            ]);

            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path
            );

            // Use the pre-authenticated client (no need to extract token manually)
            let res = timeout(
                Duration::from_secs(300), // 5 minute timeout for API calls
                self.auth_client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
            )
            .await
            .map_err(|_| anyhow!("OpenAI API request timed out after 5 minutes"))??;

            let response_body = timeout(
                Duration::from_secs(30), // 30 second timeout for response parsing
                res.json::<serde_json::Value>()
            )
            .await
            .map_err(|_| anyhow!("Response parsing timed out after 30 seconds"))??;
            debug!("Response: {:?}", response_body);

            if let Some(error) = response_body.get("error") {
                return Err(anyhow!("OpenAI API error: {:?}", error));
            }

            let content = response_body["choices"][0]["message"]["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from OpenAI response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response_body["usage"]["prompt_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                completion_tokens: response_body["usage"]["completion_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                total_tokens: response_body["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
            };

            let model = response_body["model"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let finish_reason = response_body["choices"][0]["finish_reason"]
                .as_str()
                .map(String::from);

            let (prompt_rate, completion_rate) = OpenAIEngine::pricing(&model);
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

    fn upload_file<'a>(
        &'a self,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            // Security validation
            InputValidator::validate_file_upload(file_path).await?;

            let url = "https://api.openai.com/v1/files";

            let file_name = file_path
                .file_name()
                .ok_or_else(|| anyhow!("Invalid file name"))?
                .to_str()
                .ok_or_else(|| anyhow!("File name is not valid UTF-8"))?;

            // Sanitize filename
            let sanitized_filename = InputValidator::sanitize_filename(file_name);

            let file = File::open(file_path).await?;
            let stream = FramedRead::new(file, BytesCodec::new());
            let file_part =
                Part::stream(reqwest::Body::wrap_stream(stream)).file_name(sanitized_filename);

            let form = Form::new()
                .part("file", file_part)
                .text("purpose", "assistants");

            // Use the pre-authenticated client with timeout
            let response = timeout(
                Duration::from_secs(600), // 10 minute timeout for file uploads
                self.auth_client.post(url).multipart(form).send()
            )
            .await
            .map_err(|_| anyhow!("File upload timed out after 10 minutes"))??;

            let response_body = timeout(
                Duration::from_secs(30), // 30 second timeout for response parsing
                response.json::<serde_json::Value>()
            )
            .await
            .map_err(|_| anyhow!("Response parsing timed out after 30 seconds"))??;

            response_body["id"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract file ID from OpenAI response"))
                .map(|id| id.to_string())
        })
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let key_input = format!("{}:{}", request.payload, file_path.display());
            if let Some(cache) = &self.cache {
                if let Some(cached) = cache.get(&cache_key(&key_input))? {
                    return Ok(cached);
                }
            }
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

            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                "/v1/chat/completions" // Use the chat completions endpoint for vision tasks
            );

            let payload = serde_json::json!({
                "model": "gpt-4-vision-preview",
                "messages": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "text",
                                "text": &request.payload
                            },
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:image/png;base64,{}", base64_image)
                                }
                            }
                        ]
                    }
                ],
                "max_tokens": 300
            });

            // Use the pre-authenticated client with timeout
            let response = timeout(
                Duration::from_secs(300), // 5 minute timeout for vision API calls
                self.auth_client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&payload)
                    .send()
            )
            .await
            .map_err(|_| anyhow!("Vision API request timed out after 5 minutes"))??;

            let response_body = timeout(
                Duration::from_secs(30), // 30 second timeout for response parsing
                response.json::<serde_json::Value>()
            )
            .await
            .map_err(|_| anyhow!("Response parsing timed out after 30 seconds"))??;

            // Debug print the response
            debug!("OpenAI Response: {:?}", response_body);

            if let Some(error) = response_body.get("error") {
                return Err(anyhow!("OpenAI API error: {:?}", error));
            }

            let content = response_body["choices"][0]["message"]["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from OpenAI response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response_body["usage"]["prompt_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                completion_tokens: response_body["usage"]["completion_tokens"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                total_tokens: response_body["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
            };

            let model = response_body["model"]
                .as_str()
                .unwrap_or("gpt-4-vision-preview")
                .to_string();
            let finish_reason = response_body["choices"][0]["finish_reason"]
                .as_str()
                .map(String::from);

            let (prompt_rate, completion_rate) = OpenAIEngine::pricing(&model);
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
