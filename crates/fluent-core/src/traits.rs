use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use log::debug;
use crate::config::EngineConfig;
use crate::neo4j_client::Neo4jClient;
use crate::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse};


#[async_trait]

pub trait FileUpload: Send + Sync {
    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a>;
    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;
}



#[async_trait]
pub trait Engine: Send + Sync {

    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;

    fn upsert<'a>(&'a self, request: &'a UpsertRequest) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a>;

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>>;

    fn get_session_id(&self) -> Option<String>;  // New method

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent>;

    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a>;
    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;


}

pub trait EngineConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value>;
}



pub struct AnthropicConfigProcessor;
impl EngineConfigProcessor for AnthropicConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value> {
        debug!("AnthropicConfigProcessor::process_config");
        debug!("Config: {:#?}", config);

        let mut payload = json!({
            "messages": [
                {
                    "role": "user",
                    "content": "" // This will be filled later with the actual request
                }
            ],
            "model": config.parameters.get("modelName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Model name not specified or not a string"))?,
        });

        // Add other parameters, excluding 'sessionID'
        for (key, value) in &config.parameters {
            if key != "sessionID" && !["modelName", "bearer_token"].contains(&key.as_str()) {
                match key.as_str() {
                    "max_tokens" => {
                        if let Some(num) = value.as_str().and_then(|s| s.parse::<i64>().ok()) {
                            payload[key] = json!(num);
                        } else if let Some(num) = value.as_i64() {
                            payload[key] = json!(num);
                        }
                    },
                    "temperature" | "top_p" => {
                        if let Some(num) = value.as_str().and_then(|s| s.parse::<f64>().ok()) {
                            payload[key] = json!(num);
                        } else if let Some(num) = value.as_f64() {
                            payload[key] = json!(num);
                        }
                    },
                    _ => {
                        payload[key] = value.clone();
                    }
                }
            }
        }

        debug!("Anthropic Payload: {:#?}", payload);
        Ok(payload)
    }
}


pub struct OpenAIConfigProcessor;
impl EngineConfigProcessor for OpenAIConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value> {
        debug!("OpenAIConfigProcessor::process_config");
        debug!("Config: {:#?}", config);

        let mut payload = json!({
            "model": config.parameters.get("modelName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Model not specified or not a string"))?,
            "messages": [], // This will be filled later with the actual request
        });

        // Handle temperature
        if let Some(temperature) = config.parameters.get("temperature") {
            if let Some(temp) = temperature.as_f64() {
                payload["temperature"] = json!(temp);
            }
        }

        // Handle max_tokens
        if let Some(max_tokens) = config.parameters.get("max_tokens") {
            if let Some(max_tokens_str) = max_tokens.as_str() {
                if let Ok(max_tokens_int) = max_tokens_str.parse::<u64>() {
                    payload["max_tokens"] = json!(max_tokens_int);
                }
            } else if let Some(max_tokens_int) = max_tokens.as_u64() {
                payload["max_tokens"] = json!(max_tokens_int);
            }
        }

        // Add optional parameters if they exist in the configuration
        for &param in &["frequency_penalty", "presence_penalty", "top_p", "n", "stream", "stop"] {
            if let Some(value) = config.parameters.get(param) {
                payload[param] = value.clone();
            }
        }

        debug!("OpenAI Payload: {:#?}", payload);
        Ok(payload)
    }
}