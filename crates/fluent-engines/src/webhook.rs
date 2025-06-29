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
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse,
    Usage,
};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::config::EngineConfig;
use log::debug;
use reqwest::Client;

pub struct WebhookEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl WebhookEngine {
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

    async fn prepare_payload(&self, request: &Request, file_content: Option<String>) -> Value {
        let mut payload = json!({
            "input": request.payload,
            "chat_id": self.config.parameters.get("chat_id").and_then(|v| v.as_str()).unwrap_or(""),
            "sessionId": self.config.parameters.get("sessionId").and_then(|v| v.as_str()).unwrap_or(""),
        });

        if let Some(content) = file_content {
            payload["file_content"] = json!(content);
        }

        // Add overrideConfig parameters
        if let Some(override_config) = self.config.parameters.get("overrideConfig") {
            if let Some(obj) = override_config.as_object() {
                for (key, value) in obj {
                    payload[key] = value.clone();
                }
            }
        }

        // Add tweaks
        if let Some(tweaks) = self.config.parameters.get("tweaks") {
            if let Some(obj) = tweaks.as_object() {
                for (key, value) in obj {
                    payload["tweaks"][key] = value.clone();
                }
            }
        }

        payload
    }
}

#[async_trait]
impl Engine for WebhookEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let payload = self.prepare_payload(request, None).await;

            debug!("Webhook Payload: {:?}", payload);

            let auth_token = self.config.parameters.get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let timeout = self.config.parameters.get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(60000);
            debug!("url: {}, payload: {}, timeout: {}", url, payload, timeout);
            let response = self.client.post(&url)
                .timeout(std::time::Duration::from_millis(timeout))
                .header("Authorization", format!("Bearer {}", auth_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            debug!("Webhook Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("Webhook error: {:?}", error));
            }

            let content = serde_json::to_string(&response)
                .context("Failed to serialize webhook response")?;

            Ok(Response {
                content,
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: self.config.name.clone(),
                finish_reason: Some("webhook_complete".to_string()),
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
        self.config.parameters.get("sessionId").and_then(|v| v.as_str()).map(String::from)
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        Some(ExtractedContent {
            main_content: serde_json::to_string(value).ok()?,
            sentiment: None,
            clusters: None,
            themes: None,
            keywords: None,
        })
    }

    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await.context("Failed to read file")?;
            let base64_content = STANDARD.encode(&buffer);
            Ok(base64_content)
        })
    }

    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let file_content = Pin::from(self.upload_file(file_path)).await?;

            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let payload = self.prepare_payload(request, Some(file_content)).await;

            debug!("Webhook Payload with file: {:?}", payload);

            let auth_token = self.config.parameters.get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let timeout = self.config.parameters.get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(60000);

            debug!("Url: {}, payload: {:?}, timeout: {}", url, payload, timeout);
            let response = self.client.post(&url)
                .timeout(std::time::Duration::from_millis(timeout))
                .header("Authorization", format!("Bearer {}", auth_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            debug!("Webhook Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("Webhook error: {:?}", error));
            }

            let content = serde_json::to_string(&response)
                .context("Failed to serialize webhook response")?;

            Ok(Response {
                content,
                usage: Usage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
                model: self.config.name.clone(),
                finish_reason: Some("webhook_complete".to_string()),
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
            })
        })
    }

}