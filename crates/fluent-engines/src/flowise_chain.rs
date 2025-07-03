use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine as Base64Engine;
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::{Engine, EngineConfigProcessor};
use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage,
};
use log::{debug, warn};
use mime_guess::from_path;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct FlowiseChainEngine {
    config: EngineConfig,
    config_processor: FlowiseChainConfigProcessor,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl FlowiseChainEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            config_processor: FlowiseChainConfigProcessor,
            neo4j_client,
        })
    }

    async fn _create_upload_payload(file_path: &Path) -> Result<serde_json::Value> {
        debug!("Creating upload payload for file: {}", file_path.display());
        let mut file = File::open(file_path).await.context("Failed to open file")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .context("Failed to read file")?;
        let base64_image = Base64.encode(&buffer);

        let mime_type = from_path(file_path).first_or_octet_stream().to_string();

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.file")
            .to_string();

        Ok(serde_json::json!({
            "data": format!("data:{};base64,{}", mime_type, base64_image),
            "type": "file",
            "name": file_name,
            "mime": mime_type
        }))
    }
}

pub struct FlowiseChainConfigProcessor;

impl EngineConfigProcessor for FlowiseChainConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value> {
        debug!("FlowiseConfigProcessor::process_config");
        debug!("Config: {:#?}", config);

        let mut payload = json!({
            "question": "", // This will be filled later with the actual request
            "overrideConfig": {}
        });

        // Process all parameters and add them to overrideConfig
        for (key, value) in &config.parameters {
            match value {
                Value::Object(obj) => {
                    // Handle nested objects (like openAIApiKey with multiple keys)
                    let nested_config: HashMap<String, Value> = obj.clone().into_iter().collect();
                    payload["overrideConfig"][key] = json!(nested_config);
                }
                _ => {
                    // For non-object values, add them directly
                    payload["overrideConfig"][key] = value.clone();
                }
            }
        }

        debug!("Flowise Payload: {:#?}", payload);
        Ok(payload)
    }
}

#[async_trait::async_trait]
impl Engine for FlowiseChainEngine {
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
        let mut content = ExtractedContent::default();

        if let Some(outputs) = value.get("outputs").and_then(|v| v.as_array()) {
            if let Some(first_output) = outputs.first() {
                content.main_content = first_output
                    .get("output")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                content.sentiment = first_output
                    .get("sentiment")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                content.clusters =
                    first_output
                        .get("clusters")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        });
                content.themes = first_output
                    .get("themes")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });
                content.keywords =
                    first_output
                        .get("keywords")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        });
            }
        }

        if content.main_content.is_empty() {
            None
        } else {
            Some(content)
        }
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            use fluent_core::error::{EngineError, FluentError};

            // Flowise Chain doesn't have a native upsert/embedding API
            Err(FluentError::Engine(EngineError::UnsupportedOperation {
                engine: "flowise_chain".to_string(),
                operation: "upsert".to_string(),
            })
            .into())
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

            // Add the user's request to the payload
            payload["question"] = json!(request.payload);

            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path
            );

            let res = client.post(&url).json(&payload).send().await?;

            let response_body = res.json::<serde_json::Value>().await?;
            debug!("Response: {:?}", response_body);

            if let Some(error) = response_body.get("error") {
                return Err(anyhow!("FlowiseAI API error: {:?}", error));
            }

            let content = response_body["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from FlowiseAI response"))?
                .to_string();

            // FlowiseAI doesn't provide token usage, so we'll estimate it based on content length
            let estimated_tokens = (content.len() as f32 / 4.0).ceil() as u32;
            let usage = Usage {
                prompt_tokens: estimated_tokens / 2,     // Rough estimate
                completion_tokens: estimated_tokens / 2, // Rough estimate
                total_tokens: estimated_tokens,
            };

            let model = format!("{}_flowise_chain", self.config.name);
            let finish_reason = Some("stop".to_string()); // Assuming normal completion

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
            })
        })
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            use fluent_core::error::{EngineError, FluentError};

            Err(FluentError::Engine(EngineError::UnsupportedOperation {
                engine: "flowise_chain".to_string(),
                operation: "file_upload".to_string(),
            })
            .into())
        })
    }

    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let mut file = File::open(file_path).await.context("Failed to open file")?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .await
                .context("Failed to read file")?;

            let encoded_image = base64::engine::general_purpose::STANDARD.encode(&buffer);
            debug!("Encoded image length: {} bytes", encoded_image.len());

            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown.file")
                .to_string();

            let payload = serde_json::json!({
                "question": &request.payload,
                "uploads": [{
                    "data": format!("data:image/png;base64,{}", encoded_image),
                    "type": "file",
                    "name": file_name,
                    "mime": "image/png"
                }]
            });

            debug!(
                "Data field prefix: {}",
                &payload["uploads"][0]["data"]
                    .as_str()
                    .unwrap_or("")
                    .split(',')
                    .next()
                    .unwrap_or("")
            );
            debug!(
                "Uploads array length: {}",
                payload["uploads"].as_array().map_or(0, |arr| arr.len())
            );
            debug!("File name in payload: {}", &payload["uploads"][0]["name"]);

            let client = reqwest::Client::new();
            let url = format!(
                "{}://{}:{}{}",
                self.config.connection.protocol,
                self.config.connection.hostname,
                self.config.connection.port,
                self.config.connection.request_path
            );

            debug!("Sending request to URL: {}", url);

            let response = client.post(&url).json(&payload).send().await?;

            debug!("Response status: {}", response.status());

            let response_body = response.json::<serde_json::Value>().await?;

            debug!("FlowiseAI Response: {:?}", response_body);

            if response_body.get("error").is_some()
                || response_body["text"]
                    .as_str()
                    .map_or(false, |s| s.contains("no image provided"))
            {
                warn!(
                    "FlowiseAI did not process the image. Full response: {:?}",
                    response_body
                );
            }

            let content = response_body["text"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from FlowiseAI response"))?
                .to_string();

            // FlowiseAI doesn't provide token usage, so we'll estimate it based on content length
            let estimated_tokens = (content.len() as f32 / 4.0).ceil() as u32;
            let usage = Usage {
                prompt_tokens: estimated_tokens / 2,     // Rough estimate
                completion_tokens: estimated_tokens / 2, // Rough estimate
                total_tokens: estimated_tokens,
            };

            let model = "flowise-chain".to_string(); // FlowiseAI doesn't provide model info
            let finish_reason = Some("stop".to_string()); // Assuming normal completion

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
                cost: Cost {
                    prompt_cost: 0.0,
                    completion_cost: 0.0,
                    total_cost: 0.0,
                },
            })
        })
    }
}
