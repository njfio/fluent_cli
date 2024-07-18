use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::config::EngineConfig;
use log::debug;
use reqwest::Client;
use reqwest::multipart::{Form, Part};


pub struct StabilityAIEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
    download_dir: Option<String>, // Add this field
}

impl StabilityAIEngine {
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
            download_dir: None, // Initialize as None
        })
    }

    // Method to set the download directory
    pub fn set_download_dir(&mut self, dir: String) {
        self.download_dir = Some(dir);
    }
}

#[async_trait]
impl Engine for StabilityAIEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let url = "https://api.stability.ai/v2beta/stable-image/generate/ultra";

            let auth_token = self.config.parameters.get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            // Get output format from configuration, default to webp
            let output_format = self.config.parameters
                .get("output_format")
                .and_then(|v| v.as_str())
                .unwrap_or("webp");

            // Get accept header type, default to image/*
            let accept_header = self.config.parameters
                .get("accept")
                .and_then(|v| v.as_str())
                .unwrap_or("image/*");

            // Create multipart form data
            let mut form = Form::new()
                .part("none", Part::bytes(Vec::new()))
                .text("prompt", request.payload.clone())
                .text("output_format", output_format.to_string());

            // Add optional parameters from the config
            for (key, value) in &self.config.parameters {
                if key != "bearer_token"
                    && key != "sessionID"
                    && key != "output_format"
                    && key != "accept"
                {
                    if let Some(value_str) = value.as_str() {
                        form = form.text(key.clone(), value_str.to_owned());
                    } else {
                        debug!("Skipping non-string parameter: {}", key);
                    }
                }
            }

            // Send the request
            let response = self.client.post(url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .header("Accept", accept_header)  // Add Accept header
                .multipart(form)
                .send()
                .await?;

            // Check for successful response (200 OK)
            if !response.status().is_success() {
                return Err(anyhow!("Stability AI API request failed: {}", response.status()));
            }

            // Handle response based on Accept header
            let response_content = if accept_header == "application/json" {
                // Parse JSON response and extract base64 image data
                let json_response: Value = response.json().await?;
                let base64_image = json_response["artifacts"][0]["base64"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Failed to extract base64 image data from JSON response"))?;
                base64::decode(base64_image).context("Failed to decode base64 image data")?
            } else {
                // Get image bytes directly
                response.bytes().await?.to_vec()
            };

            // Get the download directory
            let download_dir = self.download_dir
                .as_ref()
                .ok_or_else(|| anyhow!("Download directory not set for StabilityAIEngine"))?;
            let download_path = Path::new(download_dir);

            // Create a unique file name
            let file_name = format!("stabilityai_image_{}.{}", uuid::Uuid::new_v4(), output_format);
            let full_path = download_path.join(file_name);

            // Write the image data to the file
            let mut file = File::create(&full_path).await.context("Failed to create image file")?;
            file.write_all(&response_content).await.context("Failed to write image data to file")?;

            Ok(Response {
                content: full_path.to_str().unwrap().to_string(),
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: "stabilityai-ultra".to_string(),
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
        // Extract image URLs from the response
        let image_urls: Vec<String> = value["artifacts"]
            .as_array()
            .ok_or_else(|| anyhow!("Failed to extract artifacts from Stability AI response"))
            .ok()?
            .iter()
            .filter_map(|artifact| artifact["base64"].as_str().map(|base64| format!("data:image/png;base64,{}", base64)))
            .collect();

        if image_urls.is_empty() {
            None
        } else {
            Some(ExtractedContent {
                main_content: image_urls.join("\n"),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
        }
    }

    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            // Stability AI doesn't support file uploads in the same way as OpenAI.
            // Instead, we'll read the file and encode it to base64.
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await.context("Failed to read file")?;
            let base64_image = STANDARD.encode(&buffer);
            Ok(base64_image)
        })
    }

    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await.context("Failed to read file")?;
            let base64_image = base64::encode(&buffer);
            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let payload = serde_json::json!({
                "text_prompts": [{"text": &request.payload}],
                "init_images": [base64_image],
                // Add other parameters as needed based on Stability AI's API
            });

            debug!("Stability AI Payload with file: {:?}", payload);

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

            debug!("Stability AI Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("Stability AI API error: {:?}", error));
            }

            // Extract image URLs from the response
            let image_urls: Vec<String> = response["artifacts"]
                .as_array()
                .ok_or_else(|| anyhow!("Failed to extract artifacts from Stability AI response"))?
                .iter()
                .filter_map(|artifact| artifact["base64"].as_str().map(|base64| format!("data:image/png;base64,{}", base64)))
                .collect();

            Ok(Response {
                content: image_urls.join("\n"),
                usage: Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                model: "stability-ai".to_string(), // Extract the model name from the response if available
                finish_reason: Some("success".to_string()),
            })
        })
    }
}