use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::config::EngineConfig;
use log::debug;
use reqwest::Client;

pub struct DalleEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl DalleEngine {
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
}


#[async_trait]
impl Engine for DalleEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let payload = serde_json::json!({
                "model": self.config.parameters.get("modelName").and_then(|v| v.as_str()).unwrap_or("dall-e-3"),
                "prompt": &request.payload,
                "n": self.config.parameters.get("n").and_then(|v| v.as_u64()).unwrap_or(1),
                "size": self.config.parameters.get("size").and_then(|v| v.as_str()).unwrap_or("1024x1024"),
                "response_format": self.config.parameters.get("response_format").and_then(|v| v.as_str()).unwrap_or("url"),
                "quality": self.config.parameters.get("quality").and_then(|v| v.as_str()).unwrap_or("standard"),
                "style": self.config.parameters.get("style").and_then(|v| v.as_str()).unwrap_or("vivid")
            });

            debug!("DALL-E Payload: {:?}", payload);


            debug!("Size, {:?}", self.config.parameters.get("size"));

            let auth_token = self.config.parameters.get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = self.client.post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            debug!("DALL-E Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("DALL-E API error: {:?}", error));
            }

            let content = response["data"][0]["url"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract image URL from DALL-E response"))?
                .to_string();

            Ok(Response {
                content,
                usage: Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                model: "dall-e".to_string(),
                finish_reason: Some("success".to_string()),
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
        self.config.parameters.get("sessionID").and_then(|v| v.as_str()).map(String::from)
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        value.get("data")
            .and_then(|data| data.as_array())
            .and_then(|array| array.first())
            .and_then(|first| first.get("url"))
            .and_then(|url| url.as_str())
            .map(|url| ExtractedContent {
                main_content: url.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }


    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            // Read and encode the file
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await.context("Failed to read file")?;
            let base64_image = STANDARD.encode(&buffer);

            let url = format!("{}://{}:{}/v1/images/edits",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port
            );

            let payload = serde_json::json!({
                "model": self.config.parameters.get("modelName").and_then(|v| v.as_str()).unwrap_or("dall-e-3"),
                "image": format!("data:image/png;base64,{}", base64_image),
                "prompt": &request.payload,
                "n": self.config.parameters.get("n").and_then(|v| v.as_u64()).unwrap_or(1),
                "size": self.config.parameters.get("size").and_then(|v| v.as_str()).unwrap_or("1024x1024"),
                "response_format": self.config.parameters.get("response_format").and_then(|v| v.as_str()).unwrap_or("url"),
                "quality": self.config.parameters.get("quality").and_then(|v| v.as_str()).unwrap_or("standard"),
                "style": self.config.parameters.get("style").and_then(|v| v.as_str()).unwrap_or("vivid")
            });

            debug!("DALL-E Payload: {:?}", payload);

            let auth_token = self.config.parameters.get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = self.client.post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            debug!("DALL-E Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("DALL-E API error: {:?}", error));
            }

            let content = response["data"][0]["url"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract image URL from DALL-E response"))?
                .to_string();

            Ok(Response {
                content,
                usage: Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                model: "dall-e".to_string(),
                finish_reason: Some("success".to_string()),
            })
        })
    }

    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            // DALL-E doesn't support file uploads in the same way as OpenAI.
            // Instead, we'll read the file and encode it to base64.
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await.context("Failed to read file")?;
            let base64_image = STANDARD.encode(&buffer);
            Ok(base64_image)
        })
    }
}