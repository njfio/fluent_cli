use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage};
use fluent_core::traits::{AnthropicConfigProcessor, Engine, EngineConfigProcessor};
use fluent_core::config::EngineConfig;
use anyhow::{Result, anyhow, Context};
use reqwest::Client;
use serde_json::{json, Value};
use log::debug;
use mime_guess::from_path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use fluent_core::neo4j_client::Neo4jClient;
use base64::Engine as Base64Engine;
use base64::engine::general_purpose::STANDARD as Base64;


pub struct AnthropicEngine {
    config: EngineConfig,
    config_processor: AnthropicConfigProcessor,
    neo4j_client: Option<Arc<Neo4jClient>>,

}

impl AnthropicEngine {

    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            config_processor: AnthropicConfigProcessor,
            neo4j_client,
        })
    }
}

impl Engine for AnthropicEngine {

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }

    fn get_session_id(&self) -> Option<String> {
        self.config.parameters.get("sessionID").and_then(|v| v.as_str()).map(String::from)
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        let main_content = value.get("completion").and_then(|v| v.as_str())?;

        let sentiment = value.get("sentiment")
            .and_then(|v| v.as_str())
            .map(String::from);

        let clusters = value.get("clusters")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect());

        let themes = value.get("themes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect());

        let keywords = value.get("keywords")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect());

        Some(ExtractedContent {
            main_content: main_content.to_string(),
            sentiment,
            clusters,
            themes,
            keywords,
        })
    }


    fn upsert<'a>(&'a self, _request: &'a UpsertRequest) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            // Implement Anthropic-specific upsert logic here
            // For now, we'll just return a placeholder response
            Ok(UpsertResponse {
                processed_files: vec![],
                errors: vec![],
            })
        })
    }

    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let client = Client::new();
            debug!("Config: {:?}", self.config);

            let mut payload = self.config_processor.process_config(&self.config)?;

            // Add the user's request to the messages
            payload["messages"][0]["content"] = json!(request.payload);

            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let auth_token = self.config.parameters.get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let res = client.post(&url)
                .header("x-api-key", auth_token)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&payload)
                .send()
                .await?;

            let response_body = res.json::<serde_json::Value>().await?;
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
                completion_tokens: response_body["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (response_body["usage"]["input_tokens"].as_u64().unwrap_or(0) +
                    response_body["usage"]["output_tokens"].as_u64().unwrap_or(0)) as u32,
            };

            let model = response_body["model"].as_str().unwrap_or("unknown").to_string();
            let finish_reason = response_body["stop_reason"].as_str().map(String::from);

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
            })
        })
    }

    fn upload_file<'a>(&'a self, _file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File upload not implemented for Anthropic engine"))
        })
    }



    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            // Read and encode the file
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await.context("Failed to read file")?;
            let base64_image = Base64.encode(&buffer);

            // Guess the MIME type of the file
            let mime_type = from_path(file_path)
                .first_or_octet_stream()
                .to_string();

            let client = reqwest::Client::new();
            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let payload = serde_json::json!({
                "model": "claude-3-5-sonnet-20240620",
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

            let auth_token = self.config.parameters.get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = client.post(&url)
                .header("x-api-key", auth_token)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

            let response_body = response.json::<serde_json::Value>().await?;

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
                completion_tokens: response_body["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (response_body["usage"]["input_tokens"].as_u64().unwrap_or(0) +
                    response_body["usage"]["output_tokens"].as_u64().unwrap_or(0)) as u32,
            };

            let model = response_body["model"].as_str().unwrap_or("claude-3-5-sonnet-20240620").to_string();
            let finish_reason = response_body["stop_reason"].as_str().map(String::from);

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
            })
        })
    }
}





