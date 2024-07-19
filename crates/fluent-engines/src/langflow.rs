use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse, Usage};
use fluent_core::traits::{Engine, EngineConfigProcessor};
use fluent_core::config::EngineConfig;
use fluent_core::neo4j_client::Neo4jClient;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};
use log::debug;

pub struct LangflowEngine {
    config: EngineConfig,
    config_processor: LangflowConfigProcessor,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl LangflowEngine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let neo4j_client = if let Some(neo4j_config) = &config.neo4j {
            Some(Arc::new(Neo4jClient::new(neo4j_config).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            config_processor: LangflowConfigProcessor,
            neo4j_client,
        })
    }
}

pub struct LangflowConfigProcessor;

impl EngineConfigProcessor for LangflowConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value> {
        debug!("LangflowConfigProcessor::process_config");
        debug!("Config: {:#?}", config);

        let mut payload = json!({
            "input_value": "",  // This will be filled later with the actual request
            "output_type": "chat",
            "input_type": "chat",
            "tweaks": {}
        });

        // Process all parameters and add them to tweaks
        for (key, value) in &config.parameters {
            match value {
                serde_json::Value::Object(obj) => {
                    // Handle nested objects
                    payload["tweaks"][key] = json!(obj);
                },
                _ => {
                    // For non-object values, add them directly to the root of the payload
                    payload[key] = value.clone();
                }
            }
        }

        debug!("Langflow Payload: {:#?}", payload);
        Ok(payload)
    }


}

#[async_trait::async_trait]
impl Engine for LangflowEngine {
    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>> {
        self.neo4j_client.as_ref()
    }

    fn get_session_id(&self) -> Option<String> {
        self.config.parameters.get("sessionID").and_then(|v| v.as_str()).map(String::from)
    }

    fn upsert<'a>(&'a self, request: &'a UpsertRequest) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move {
            // Implement Langflow-specific upsert logic here if needed
            Ok(UpsertResponse {
                processed_files: vec![],
                errors: vec![],
            })
        })
    }


    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let client = Client::new();
            debug!("Config: {:?}", self.config);

            let mut payload = self.config_processor.process_config(&self.config)?;
            payload["input_value"] = json!(request.payload);

            let url = format!("{}://{}:{}{}",
                              self.config.connection.protocol,
                              self.config.connection.hostname,
                              self.config.connection.port,
                              self.config.connection.request_path
            );

            let res = client.post(&url)
                .json(&payload)
                .send()
                .await?;

            let response_body = res.json::<serde_json::Value>().await?;
            debug!("Response: {:?}", response_body);

            if let Some(error) = response_body.get("error") {
                return Err(anyhow!("Langflow API error: {:?}", error));
            }

            let extracted_content = self.extract_content(&response_body)
                .ok_or_else(|| anyhow!("Failed to extract content from Langflow response"))?;

            let estimated_tokens = (extracted_content.main_content.len() as f32 / 4.0).ceil() as u32;
            let usage = Usage {
                prompt_tokens: estimated_tokens / 2,
                completion_tokens: estimated_tokens / 2,
                total_tokens: estimated_tokens,
            };

            let model = format!("{}_langflow_chain", self.config.name);
            let finish_reason = Some("stop".to_string());

            Ok(Response {
                content: extracted_content.main_content,
                usage,
                model,
                finish_reason,
            })
        })
    }

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent> {
        fn extract_recursive(value: &Value) -> Option<ExtractedContent> {
            match value {
                Value::Object(map) => {
                    let mut content = ExtractedContent::default();

                    if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                        content.main_content = text.to_string();
                    } else if let Some(message) = map.get("message").and_then(|v| v.as_str()) {
                        content.main_content = message.to_string();
                    }

                    content.sentiment = map.get("sentiment").and_then(|v| v.as_str()).map(String::from);
                    content.clusters = map.get("clusters").and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());
                    content.themes = map.get("themes").and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());
                    content.keywords = map.get("keywords").and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

                    if !content.main_content.is_empty() {
                        Some(content)
                    } else {
                        for (_, v) in map {
                            if let Some(extracted) = extract_recursive(v) {
                                return Some(extracted);
                            }
                        }
                        None
                    }
                }
                Value::Array(arr) => {
                    for v in arr {
                        if let Some(extracted) = extract_recursive(v) {
                            return Some(extracted);
                        }
                    }
                    None
                }
                _ => None,
            }
        }

        extract_recursive(value)
    }

    fn upload_file<'a>(&'a self, _file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File upload not implemented for Langflow engine"))
        })
    }

    fn process_request_with_file<'a>(&'a self, _request: &'a Request, _file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File processing not implemented for Langflow engine"))
        })
    }
}