use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine as Base64Engine;
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use log::{debug, info};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub struct ImagineProEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
    download_dir: Option<String>,
}

impl ImagineProEngine {
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

    pub fn set_download_dir(&mut self, dir: String) {
        self.download_dir = Some(dir);
    }

    async fn get_image_result(&self, message_id: &str) -> Result<String> {
        let url = format!(
            "{}://{}:{}/api/v1/midjourney/message/{}",
            self.config.connection.protocol,
            self.config.connection.hostname,
            self.config.connection.port,
            message_id
        );

        let auth_token = self
            .config
            .parameters
            .get("bearer_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

        let max_attempts = 30; // Adjust as needed
        let delay = Duration::from_secs(10); // Adjust as needed

        for _ in 0..max_attempts {
            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .send()
                .await?
                .json::<Value>()
                .await?;

            match response["status"].as_str() {
                Some("DONE") => {
                    return response["uri"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Image URI not found in response"))
                        .map(String::from)
                }
                Some("PROCESSING") | Some("QUEUED") => {
                    info!("Job still processing. Progress: {}%", response["progress"]);
                    tokio::time::sleep(delay).await;
                }
                Some("FAIL") => return Err(anyhow!("Job failed: {:?}", response["error"])),
                _ => return Err(anyhow!("Unexpected job status: {:?}", response["status"])),
            }
        }

        Err(anyhow!("Timed out waiting for image generation"))
    }

    async fn download_image(&self, uri: &str) -> Result<String> {
        let response = self.client.get(uri).send().await?;
        let content = response.bytes().await?;

        let download_dir = self
            .download_dir
            .as_ref()
            .ok_or_else(|| anyhow!("Download directory not set"))?;
        let file_name = format!("imaginepro_{}.png", Uuid::new_v4());
        let file_path = Path::new(download_dir).join(file_name);

        let mut file = File::create(&file_path).await?;
        file.write_all(&content).await?;

        Ok(file_path.to_string_lossy().into_owned())
    }

    async fn upload_file_internal(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path).await.context("Failed to open file")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .context("Failed to read file")?;
        let base64_image = Base64.encode(&buffer);
        let mime_type = mime_guess::from_path(file_path)
            .first_or_octet_stream()
            .to_string();
        Ok(format!("data:{};base64,{}", mime_type, base64_image))
    }

    fn extract_prompt(&self, payload: &str) -> String {
        // Look for the prompt between quotes after "**Image Prompt:**"
        if let Some(start) = payload.find("**Image Prompt:**") {
            if let Some(quote_start) = payload[start..].find('"') {
                if let Some(quote_end) = payload[start + quote_start + 1..].find('"') {
                    return payload[start + quote_start + 1..start + quote_start + 1 + quote_end]
                        .to_string();
                }
            }
        }

        payload.trim().to_string()
    }
}

#[async_trait]
impl Engine for ImagineProEngine {
    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let url = format!(
                "{}://{}:{}/api/v1/midjourney/imagine",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port
            );

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            // Extract the actual prompt from the payload
            let clean_prompt = self.extract_prompt(&request.payload);

            let payload = json!({
                "prompt": clean_prompt,
                "ref": self.config.parameters.get("ref"),
                "webhookOverride": self.config.parameters.get("webhookOverride"),
            });

            debug!("ImaginePro Payload: {:?}", payload);

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            if !response["success"].as_bool().unwrap_or(false) {
                return Err(anyhow!(
                    "ImaginePro API request failed: {:?}",
                    response["error"]
                ));
            }

            let message_id = response["messageId"]
                .as_str()
                .ok_or_else(|| anyhow!("MessageId not found in response"))?;

            // Wait for the image to be generated
            let image_url = self.get_image_result(message_id).await?;

            // Download the image
            let local_image_path = self.download_image(&image_url).await?;

            Ok(Response {
                content: local_image_path,
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: "imaginepro-midjourney".to_string(),
                finish_reason: Some("success".to_string()),
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
            })
        })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
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
        self.config
            .parameters
            .get("sessionID")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        value
            .get("imageUrl")
            .and_then(|url| url.as_str())
            .map(|url| ExtractedContent {
                main_content: url.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }

    fn upload_file<'a>(
        &'a self,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(self.upload_file_internal(file_path))
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let data_url = self.upload_file_internal(file_path).await?;
            let prompt = format!("{} {}", data_url, request.payload);

            let new_request = Request {
                flowname: request.flowname.clone(),
                payload: prompt,
            };

            // Manually implement the execute logic to avoid recursive async calls
            let url = format!(
                "{}://{}:{}/api/v1/midjourney/imagine",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port
            );

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let clean_prompt = self.extract_prompt(&new_request.payload);

            let payload = json!({
                "prompt": clean_prompt,
                "ref": self.config.parameters.get("ref"),
                "webhookOverride": self.config.parameters.get("webhookOverride"),
            });

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            let content = response.to_string();
            let usage = Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            };

            Ok(Response {
                content,
                usage,
                model: "midjourney".to_string(),
                finish_reason: None,
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
            })
        })
    }
}
