use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::types::{
    ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use log::debug;
use reqwest::Client;
use serde_json::{json, Value};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct CohereEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl CohereEngine {
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
impl Engine for CohereEngine {
    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path
            );

            let payload = json!({
                "message": &request.payload,
                "model": self.config.parameters.get("modelName").and_then(|v| v.as_str()).unwrap_or("command-r-plus"),
                "stream": self.config.parameters.get("stream").and_then(|v| v.as_bool()).unwrap_or(false),
                "preamble": self.config.parameters.get("preamble").and_then(|v| v.as_str()),
                "chat_history": self.config.parameters.get("chat_history"),
                "conversation_id": self.config.parameters.get("conversation_id").and_then(|v| v.as_str()),
                "prompt_truncation": self.config.parameters.get("prompt_truncation").and_then(|v| v.as_str()).unwrap_or("AUTO"),
                "connectors": self.config.parameters.get("connectors"),
                "documents": self.config.parameters.get("documents"),
                "citation_quality": self.config.parameters.get("citation_quality").and_then(|v| v.as_str()).unwrap_or("accurate"),
                "temperature": self.config.parameters.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.3),
                "max_tokens": self.config.parameters.get("max_tokens").and_then(|v| v.as_u64()),
                "k": self.config.parameters.get("k").and_then(|v| v.as_u64()).unwrap_or(0),
                "p": self.config.parameters.get("p").and_then(|v| v.as_f64()).unwrap_or(0.75),
                "frequency_penalty": self.config.parameters.get("frequency_penalty").and_then(|v| v.as_f64()).unwrap_or(0.0),
                "presence_penalty": self.config.parameters.get("presence_penalty").and_then(|v| v.as_f64()).unwrap_or(0.0),
                "tools": self.config.parameters.get("tools"),
                "tool_results": self.config.parameters.get("tool_results"),
            });

            debug!("Cohere Payload: {:?}", payload);

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            debug!("Cohere Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("Cohere API error: {:?}", error));
            }

            let content = response["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from Cohere response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response
                    .get("meta")
                    .and_then(|meta| meta.get("billed_units"))
                    .and_then(|billed_units| billed_units.get("input_tokens"))
                    .and_then(|input_tokens| input_tokens.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: response
                    .get("meta")
                    .and_then(|meta| meta.get("billed_units"))
                    .and_then(|billed_units| billed_units.get("output_tokens"))
                    .and_then(|output_tokens| output_tokens.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: response
                    .get("meta")
                    .and_then(|meta| meta.get("billed_units"))
                    .map(|billed_units| {
                        let input = billed_units
                            .get("input_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0);
                        let output = billed_units
                            .get("output_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0);
                        input + output
                    })
                    .unwrap_or(0) as u32,
            };

            let model = "cohere".to_string(); // Or extract from response if available
            let finish_reason = response["finish_reason"].as_str().map(String::from);

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
            })
        })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            // Cohere doesn't have a direct upsert functionality, so we return an empty response
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
            .get("text")
            .and_then(|text| text.as_str())
            .map(|content| ExtractedContent {
                main_content: content.to_string(),
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
        Box::new(async move {
            // Cohere doesn't have a direct file upload API, so we'll read the file and return its content as a base64 string
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .await
                .context("Failed to read file")?;
            let base64_content = STANDARD.encode(&buffer);
            Ok(base64_content)
        })
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let base64_content = Pin::from(self.upload_file(file_path)).await?;

            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path
            );

            let payload = json!({
                "message": &request.payload,
                "model": self.config.parameters.get("modelName").and_then(|v| v.as_str()).unwrap_or("command-r-plus"),
                "documents": [{
                    "text": base64_content,
                    "title": file_path.file_name().unwrap_or_default().to_string_lossy()
                }],
                // Include other parameters as needed
            });

            debug!("Cohere Payload with file: {:?}", payload);

            let auth_token = self
                .config
                .parameters
                .get("bearer_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", auth_token))
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?
                .json::<Value>()
                .await?;

            debug!("Cohere Response: {:?}", response);

            if let Some(error) = response.get("error") {
                return Err(anyhow!("Cohere API error: {:?}", error));
            }

            let content = response["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from Cohere response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response
                    .get("meta")
                    .and_then(|meta| meta.get("billed_units"))
                    .and_then(|billed_units| billed_units.get("input_tokens"))
                    .and_then(|input_tokens| input_tokens.as_u64())
                    .unwrap_or(0) as u32,
                completion_tokens: response
                    .get("meta")
                    .and_then(|meta| meta.get("billed_units"))
                    .and_then(|billed_units| billed_units.get("output_tokens"))
                    .and_then(|output_tokens| output_tokens.as_u64())
                    .unwrap_or(0) as u32,
                total_tokens: response
                    .get("meta")
                    .and_then(|meta| meta.get("billed_units"))
                    .map(|billed_units| {
                        let input = billed_units
                            .get("input_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0);
                        let output = billed_units
                            .get("output_tokens")
                            .and_then(|t| t.as_u64())
                            .unwrap_or(0);
                        input + output
                    })
                    .unwrap_or(0) as u32,
            };

            let model = "cohere".to_string(); // Or extract from response if available
            let finish_reason = response["finish_reason"].as_str().map(String::from);

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
            })
        })
    }
}
