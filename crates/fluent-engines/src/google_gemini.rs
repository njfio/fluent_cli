use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::config::EngineConfig;
use log::debug;
use reqwest::Client;

pub struct GoogleGeminiEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl GoogleGeminiEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            client: Client::new(),
            neo4j_client,
        })
    }

    async fn encode_image(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path).await.context("Failed to open file")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.context("Failed to read file")?;
        Ok(STANDARD.encode(&buffer))
    }

    async fn send_gemini_request(&self, prompt: &str, encoded_image: Option<String>) -> Result<Value> {
        let api_key = self.config.parameters.get("bearer_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("API key not found in configuration"))?;

        let model = self.config.parameters.get("modelName")
            .and_then(|v| v.as_str())
            .unwrap_or("gemini-1.5-pro-latest");

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model,
            api_key
        );

        let mut content = vec![json!({
            "parts": [{ "text": prompt }]
        })];

        if let Some(image) = encoded_image {
            content.push(json!({
                "parts": [{
                    "inline_data": {
                        "mime_type": "image/jpeg",
                        "data": image
                    }
                }]
            }));
        }

        let request_body = json!({
            "contents": content,
            "generationConfig": {
                "temperature": self.config.parameters.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7),
                "topK": self.config.parameters.get("top_k").and_then(|v| v.as_u64()).unwrap_or(40),
                "topP": self.config.parameters.get("top_p").and_then(|v| v.as_f64()).unwrap_or(0.95),
                "maxOutputTokens": self.config.parameters.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(1024),
            }
        });

        debug!("Google Gemini Request: {:?}", request_body);

        let response = self.client.post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            debug!("Error response body: {}", error_text);
            return Err(anyhow!("Request failed with status: {}", status));
        }

        let response_body: Value = response.json().await?;
        debug!("Google Gemini Response: {:?}", response_body);

        Ok(response_body)
    }
}

#[async_trait]
impl Engine for GoogleGeminiEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let response = self.send_gemini_request(&request.payload, None).await?;

            let generated_text = response["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract generated text from Gemini response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                completion_tokens: response["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                total_tokens: response["usageMetadata"]["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            };

            let model = self.config.parameters.get("modelName")
                .and_then(|v| v.as_str())
                .unwrap_or("gemini-1.5-pro-latest")
                .to_string();

            let finish_reason = response["candidates"][0]["finishReason"]
                .as_str()
                .map(String::from);

            Ok(Response {
                content: generated_text,
                usage,
                model,
                finish_reason,
            })
        })
    }

    fn upsert<'a>(&'a self, _request: &'a UpsertRequest) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            Ok(UpsertResponse {
                processed_files: vec![],
                errors: vec![],
            })
        })
    }

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }

    fn get_session_id(&self) -> Option<String> {
        self.config.parameters.get("sessionId").and_then(|v| v.as_str()).map(String::from)
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        value["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|content| ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }

    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            self.encode_image(file_path).await
        })
    }

    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let encoded_image = Pin::from(self.upload_file(file_path)).await?;
            let response = self.send_gemini_request(&request.payload, Some(encoded_image)).await?;

            let generated_text = response["candidates"][0]["content"]["parts"][0]["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract generated text from Gemini response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                completion_tokens: response["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                total_tokens: response["usageMetadata"]["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            };

            let model = self.config.parameters.get("modelName")
                .and_then(|v| v.as_str())
                .unwrap_or("gemini-1.5-pro-latest")
                .to_string();

            let finish_reason = response["candidates"][0]["finishReason"]
                .as_str()
                .map(String::from);

            Ok(Response {
                content: generated_text,
                usage,
                model,
                finish_reason,
            })
        })
    }
}