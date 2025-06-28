use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse,
    Usage,
};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::config::EngineConfig;
use log::debug;
use reqwest::Client;
use reqwest::multipart::{Form, Part};
use base64::Engine as Base64Engine;
use base64::engine::general_purpose::STANDARD as Base64;


pub struct ReplicateEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
    download_dir: Option<String>,
}

impl ReplicateEngine {
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
            download_dir: None,
        })
    }

    // Method to set the download directory
    pub fn set_download_dir(&mut self, dir: String) {
        self.download_dir = Some(dir);
    }
}

#[async_trait]
impl Engine for ReplicateEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let model = self.config.parameters.get("model")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Model not found in configuration"))?;

            let api_token = self.config.parameters.get("api_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("API token not found in configuration"))?;

            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path // This now includes the model path
            );

            let mut input_payload = serde_json::json!({
                "prompt": request.payload
            });
            debug!("Sending request to Replicate: {:?}", input_payload);
            // Add additional parameters from the configuration
            for (key, value) in &self.config.parameters {
                if key != "api_token" && key != "model" {
                    if let Some(value_str) = value.as_str() {
                        // Send all parameters as strings
                        input_payload[key] = Value::String(value_str.to_string());
                    } else {
                        debug!("Skipping non-string parameter: {}", key);
                    }
                }
            }
            debug!("Sending request to Replicate: {:?}", input_payload);
            // Create the main payload
            let payload = serde_json::json!({
                "input": input_payload
             });
            debug!("Sending request to Replicate: {:?}", payload);
            let response = self.client.post(&url)
                .header("Authorization", format!("Token {}", api_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

            // Check for successful response (201 Created)
            if !response.status().is_success() {
                return Err(anyhow!("Replicate API request failed: {}", response.status()));
            }

            let response_json: Value = response.json().await?;
            debug!("Replicate API response: {:?}", response_json);
            let prediction_id = response_json["id"].as_str().ok_or_else(|| anyhow!("Failed to extract prediction ID"))?;
            debug!("Prediction ID: {}", prediction_id);

            // Poll for prediction completion
            let output_url = loop {
                let status_url = format!("https://api.replicate.com/v1/predictions/{}", prediction_id);
                debug!("Prediction status URL: {}", status_url);
                let status_response = self.client.get(&status_url)
                    .header("Authorization", format!("Token {}", api_token))
                    .send()
                    .await?;

                let status_json: Value = status_response.json().await?;
                debug!("Prediction status: {:?}", status_json);

                if status_json["status"].as_str().unwrap_or("starting") == "succeeded" {
                    // Extract the output URL
                    let output_url = status_json["output"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Failed to extract output URL"))?;

                    debug!("Output URL: {}", output_url);
                    break output_url.to_string();
                }

                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            };

// Download the image **data** (this is the key change)
            debug!("Saving image: {}", output_url);
            let image_response = self.client.get(&output_url).send().await?;
            debug!("Saved image to:");
            let image_data = image_response.bytes().await?;
            debug!("Saved image to: ");
            // Download the image
            let download_dir = self.download_dir
                .as_ref()
                .ok_or_else(|| anyhow!("Download directory not set for ReplicateEngine"))?;
            let download_path = Path::new(download_dir);

            // Create a unique file name
            let file_name = format!("replicate_image_{}.png", uuid::Uuid::new_v4());
            let full_path = download_path.join(file_name);

            // Save the image data to a file
            let mut file = File::create(&full_path).await.context("Failed to create image file")?;
            file.write_all(&image_data).await.context("Failed to write image data to file")?;

            Ok(Response {
                content: full_path.to_str().unwrap().to_string(),
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: model.to_string(),
                finish_reason: Some("success".to_string()),
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
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
        let image_urls: Vec<String> = value["output"]
            .as_array()
            .ok_or_else(|| anyhow!("Failed to extract output from Replicate response"))
            .ok()?
            .iter()
            .filter_map(|output| output.as_str().map(|url| url.to_string()))
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
            // Replicate doesn't directly support file uploads for image generation.
            // This method is left unimplemented for now.
            Err(anyhow!("File uploads are not directly supported for image generation with Replicate."))
        })
    }

    fn process_request_with_file<'a>(&'a self, _request: &'a Request, _file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            // Replicate doesn't directly support file uploads for image generation.
            // This method is left unimplemented for now.
            Err(anyhow!("File uploads are not directly supported for image generation with Replicate."))
        })
    }
}