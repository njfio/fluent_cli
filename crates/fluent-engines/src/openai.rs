use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine as Base64Engine;
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::{Engine, EngineConfigProcessor, OpenAIConfigProcessor};
use fluent_core::types::{
    ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use log::debug;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
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
}

impl OpenAIEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            config_processor: OpenAIConfigProcessor,
            neo4j_client,
        })
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
            // Implement OpenAI-specific upsert logic here
            // For now, we'll just return a placeholder response
            Ok(UpsertResponse {
                processed_files: vec![],
                errors: vec![],
            })
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
            let client = Client::new();
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

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let res = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

            let response_body = res.json::<serde_json::Value>().await?;
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

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
            })
        })
    }

    fn upload_file<'a>(
        &'a self,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            let client = reqwest::Client::new();
            let url = "https://api.openai.com/v1/files";

            let file_name = file_path
                .file_name()
                .ok_or_else(|| anyhow!("Invalid file name"))?
                .to_str()
                .ok_or_else(|| anyhow!("File name is not valid UTF-8"))?;

            let file = File::open(file_path).await?;
            let stream = FramedRead::new(file, BytesCodec::new());
            let file_part =
                Part::stream(reqwest::Body::wrap_stream(stream)).file_name(file_name.to_owned());

            let form = Form::new()
                .part("file", file_part)
                .text("purpose", "assistants");

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = client
                .post(url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .multipart(form)
                .send()
                .await?;

            let response_body = response.json::<serde_json::Value>().await?;

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
            // Read and encode the file
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .await
                .context("Failed to read file")?;
            let base64_image = Base64.encode(&buffer);

            let client = reqwest::Client::new();
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

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

            let response_body = response.json::<serde_json::Value>().await?;

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

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
            })
        })
    }
}
