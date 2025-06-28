// crates/fluent-engines/src/mistral.rs
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{json, Value};


use fluent_core::types::{
    Cost, ExtractedContent, Request, Response, UpsertRequest, UpsertResponse,
    Usage,
};
use fluent_core::neo4j_client::Neo4jClient;
use fluent_core::traits::Engine;
use fluent_core::config::EngineConfig;
use log::debug;
use reqwest::Client;

pub struct MistralEngine {
    config: EngineConfig,
    client: Client,
    neo4j_client: Option<Arc<Neo4jClient>>,
}

impl MistralEngine {
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

    async fn send_mistral_request(&self, messages: Vec<Value>) -> Result<Value> {
        let url = format!("{}://{}:{}{}",
                          self.config.connection.protocol,
                          self.config.connection.hostname,
                          self.config.connection.port,
                          self.config.connection.request_path
        );

        let payload = json!({
            "model": self.config.parameters.get("model").and_then(|v| v.as_str()).unwrap_or("mistral-7b-instruct"),
            "messages": messages,
            "temperature": self.config.parameters.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7),
            "max_tokens": self.config.parameters.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(1024),
            "top_p": self.config.parameters.get("top_p").and_then(|v| v.as_f64()).unwrap_or(1.0),
            "stream": self.config.parameters.get("stream").and_then(|v| v.as_bool()).unwrap_or(false),
        });

        debug!("Mistral Request: {:?}", payload);

        let auth_token = self.config.parameters.get("bearer_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Bearer token not found in configuration"))?;

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", auth_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            debug!("Error response body: {}", error_text);
            return Err(anyhow!("Request failed with status: {}", status));
        }

        let response_body: Value = response.json().await?;
        debug!("Mistral Response: {:?}", response_body);

        Ok(response_body)
    }
}

#[async_trait]
impl Engine for MistralEngine {
    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            let messages = vec![json!({
                "role": "user",
                "content": request.payload
            })];

            let response = self.send_mistral_request(messages).await?;

            let content = response["choices"][0]["message"]["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to extract content from Mistral response"))?
                .to_string();

            let usage = Usage {
                prompt_tokens: response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: response["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: response["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
            };

            let model = response["model"].as_str().unwrap_or("unknown").to_string();
            let finish_reason = response["choices"][0]["finish_reason"].as_str().map(String::from);

            let (prompt_rate, completion_rate) = (0.0_f64, 0.0_f64);
            let prompt_cost = usage.prompt_tokens as f64 * prompt_rate;
            let completion_cost = usage.completion_tokens as f64 * completion_rate;
            let total_cost = prompt_cost + completion_cost;

            Ok(Response {
                content,
                usage,
                model,
                finish_reason,
                cost: Cost {
                    prompt_cost,
                    completion_cost,
                    total_cost,
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
        value["choices"][0]["message"]["content"]
            .as_str()
            .map(|content| ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }

    fn upload_file<'a>(&'a self, _file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File upload not supported for Mistral engine"))
        })
    }

    fn process_request_with_file<'a>(&'a self, _request: &'a Request, _file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            Err(anyhow!("File processing not supported for Mistral engine"))
        })
    }
}
