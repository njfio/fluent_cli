use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, anyhow, Context};
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::config::EngineConfig;
use log::debug;
use reqwest::Client;

pub struct LeonardoAIEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl LeonardoAIEngine {
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

    async fn upload_file_internal(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path).await.context("Failed to open file")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.context("Failed to read file")?;
        let base64_image = STANDARD.encode(&buffer);

        let url = format!("{}://{}:{}/api/rest/v1/init-image",
                          self.config.connection.protocol,
                          self.config.connection.hostname,
                          self.config.connection.port
        );

        let auth_token = self.config.parameters.get("bearer_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

        let payload = json!({
            "extension": file_path.extension().and_then(|e| e.to_str()).unwrap_or("png"),
            "file": base64_image,
        });

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", auth_token))
            .json(&payload)
            .send()
            .await?
            .json::<Value>()
            .await?;

        response["uploadInitImage"]["id"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to extract uploaded image ID"))
            .map(String::from)
    }

    fn create_payload(&self, request: &Request, image_id: Option<String>) -> Value {
        let mut payload: Map<String, Value> = self.config.parameters
            .iter()
            .filter(|(k, _)| *k != "bearer_token" && *k != "sessionID" && *k != "user")  // Exclude bearer_token
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Add or override specific fields
        payload.insert("prompt".to_string(), json!(request.payload));

        if let Some(id) = image_id {
            payload.insert("initImageId".to_string(), json!(id));
            payload.insert("imagePrompts".to_string(), json!([id]));
        }

        // Convert to Value and remove null values
        let payload_value = Value::Object(payload);
        remove_null_values(payload_value)
    }

    fn get_bearer_token(&self) -> Result<String, anyhow::Error> {
        self.config.parameters.get("bearer_token")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Bearer token not found in configuration"))
    }
}

fn remove_null_values(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = Map::new();
            for (k, v) in map {
                let new_v = remove_null_values(v);
                if new_v != Value::Null {
                    new_map.insert(k, new_v);
                }
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            let new_arr: Vec<Value> = arr
                .into_iter()
                .map(remove_null_values)
                .filter(|v| *v != Value::Null)
                .collect();
            Value::Array(new_arr)
        }
        _ => value,
    }
}

#[async_trait]
impl Engine for LeonardoAIEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let payload = self.create_payload(request, None);

            debug!("Leonardo AI Payload: {:?}", payload);

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

            debug!("Leonardo AI Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("Leonardo AI API error: {:?}", error));
            }

            let generation_id = response["sdGenerationJob"]["generationId"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract generation ID from Leonardo AI response"))?;

            // Poll for results
            let mut image_urls = Vec::new();
            for _ in 0..60 { // Poll for up to 5 minutes
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let status_url = format!("{}://{}:{}/api/rest/v1/generations/{}",
                                         self.config.connection.protocol,
                                         self.config.connection.hostname,
                                         self.config.connection.port,
                                         generation_id
                );

                let status_response = self.client.get(&status_url)
                    .header("Authorization", format!("Bearer {}", auth_token))
                    .send()
                    .await?
                    .json::<Value>()
                    .await?;

                if status_response["generations_by_pk"]["status"].as_str() == Some("COMPLETE") {
                    image_urls = status_response["generations_by_pk"]["generated_images"]
                        .as_array()
                        .ok_or_else(|| anyhow!("No generated images found"))?
                        .iter()
                        .filter_map(|img| img["url"].as_str())
                        .map(String::from)
                        .collect();
                    break;
                }
            }

            if image_urls.is_empty() {
                return Err(anyhow!("Timed out waiting for Leonardo AI to generate images"));
            }

            Ok(Response {
                content: image_urls.join("\n"),
                usage: Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                model: "leonardo-ai".to_string(),
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
        value["generations_by_pk"]["generated_images"]
            .as_array()
            .and_then(|images| {
                let urls: Vec<String> = images.iter()
                    .filter_map(|img| img["url"].as_str().map(String::from))
                    .collect();
                if urls.is_empty() {
                    None
                } else {
                    Some(ExtractedContent {
                        main_content: urls.join("\n"),
                        sentiment: None,
                        clusters: None,
                        themes: None,
                        keywords: None,
                    })
                }
            })
    }

    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            self.upload_file_internal(file_path).await
        })
    }




    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let image_id = self.upload_file_internal(file_path).await?;

            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let payload = self.create_payload(request, None);

            debug!("Leonardo AI Payload with file: {:?}", payload);

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

            debug!("Leonardo AI Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("Leonardo AI API error: {:?}", error));
            }

            let generation_id = response["sdGenerationJob"]["generationId"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract generation ID from Leonardo AI response"))?;

            // Poll for results
            let mut image_urls = Vec::new();
            for _ in 0..60 { // Poll for up to 5 minutes
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let status_url = format!("{}://{}:{}/api/rest/v1/generations/{}",
                                         self.config.connection.protocol,
                                         self.config.connection.hostname,
                                         self.config.connection.port,
                                         generation_id
                );

                let status_response = self.client.get(&status_url)
                    .header("Authorization", format!("Bearer {}", auth_token))
                    .send()
                    .await?
                    .json::<Value>()
                    .await?;

                if status_response["generations_by_pk"]["status"].as_str() == Some("COMPLETE") {
                    image_urls = status_response["generations_by_pk"]["generated_images"]
                        .as_array()
                        .ok_or_else(|| anyhow!("No generated images found"))?
                        .iter()
                        .filter_map(|img| img["url"].as_str())
                        .map(String::from)
                        .collect();
                    break;
                }
            }

            if image_urls.is_empty() {
                return Err(anyhow!("Timed out waiting for Leonardo AI to generate images"));
            }

            Ok(Response {
                content: image_urls.join("\n"),
                usage: Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
                model: "leonardo-ai".to_string(),
                finish_reason: Some("success".to_string()),
            })
        })
    }
}


