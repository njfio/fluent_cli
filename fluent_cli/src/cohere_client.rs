use reqwest::{Client, multipart};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use std::env;
use clap::ArgMatches;

use log::{debug, error};
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use serde::ser::StdError;
use tokio::fs::File as TokioFile; // Alias to avoid confusion with std::fs::File
use tokio::io::AsyncReadExt as TokioAsyncReadExt;

use tokio::time::{Instant, sleep};

use crate::client::{handle_openai_assistant_response, resolve_env_var};
use crate::config::{FlowConfig, replace_with_env_var};

#[derive(Debug, Serialize)]
struct CohereRequest {
    model: String,
    message: String,
    temperature: f32,
    chat_history: Vec<ChatHistory>,
    prompt_truncation: String,
    stream: bool,
    connectors: Vec<Connector>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Connector {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CohereResponse {
    text: Option<String>,
    response_id: Option<String>,
    generation_id: Option<String>,
    citations: Option<Vec<Citation>>,
    documents: Option<Vec<Document>>,
    is_search_required: Option<bool>,
    search_queries: Option<Vec<SearchQuery>>,
    search_results: Option<Vec<SearchResult>>,
    finish_reason: Option<String>,
    continue_on_failure: Option<bool>,
    tool_calls: Option<Vec<ToolCall>>,
    chat_history: Vec<ChatHistory>,
    meta: Option<Meta>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Citation {
    start: i32,
    end: i32,
    text: String,
    document_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Document {
    id: String,
    additional_prop: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    text: String,
    generation_id: String,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    search_query: SearchQuery,
    connector: Connector,
    document_ids: Vec<String>,
    error_message: Option<String>,
    continue_on_failure: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ToolCall {
    name: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ToolResult {
    call: ToolCall,
    outputs: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    api_version: ApiVersion,
    billed_units: BilledUnits,
    tokens: Tokens,
    warnings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ApiVersion {
    version: String,
    is_deprecated: Option<bool>,
    is_experimental: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BilledUnits {
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    search_units: Option<i32>,
    classifications: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Tokens {
    input_tokens: i32,
    output_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatHistory {
    role: String,
    message: String,
}
impl ChatHistory {
    fn new(role: &str, message: &str) -> Self {
        ChatHistory {
            role: role.to_string(),
            message: message.to_string(),
        }
    }
}

pub async fn send_cohere_request(
    prompt: &str,
    api_key: &str,
    url: &str,
    model: &str,
    temperature: f32,
    chat_history: &[ChatHistory],
) -> Result<CohereResponse, Box<dyn StdError + Send + Sync>> {
    let client = reqwest::Client::new();
    let request_body = CohereRequest {
        model: model.to_string(),
        message: prompt.to_string(),
        temperature,
        chat_history: chat_history.to_vec(),
        prompt_truncation: "AUTO".to_string(),
        stream: false,
        connectors: vec![Connector { id: "web-search".to_string() }],
    };
    debug!("Request body: {:?}", request_body);
    let response = client
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    let response_text = response.text().await?;
    debug!("Response body: {}", response_text);

    match serde_json::from_str::<CohereResponse>(&response_text) {
        Ok(cohere_response) => {
            debug!("Cohere response: {:?}", cohere_response);
            Ok(cohere_response)
        },
        Err(e) => {
            error!("Failed to parse Cohere response: {}", e);
            error!("Response text: {}", response_text);
            Err(Box::new(e))
        }
    }
}


async fn encode_image(image_path: &Path) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut file = TokioFile::open(image_path).await?;
    let mut buffer = Vec::new();
    TokioAsyncReadExt::read_to_end(&mut file, &mut buffer).await?;
    Ok(STANDARD.encode(&buffer))
}


use std::sync::Arc;
use crate::neo4j_client::{Neo4jClient, LlmProvider, capture_llm_interaction};

pub async fn handle_cohere_agent(
    prompt: &str,
    flow: &FlowConfig,
    matches: &ArgMatches,
) -> Result<String, Box<dyn StdError + Send + Sync>> {
    let mut flow = flow.clone();
    replace_with_env_var(&mut flow.override_config);

    let api_key = resolve_env_var(&flow.bearer_token)?;

    debug!("Using Cohere API key: {}", api_key);

    let url = format!("{}://{}:{}/{}", flow.protocol, flow.hostname, flow.port, flow.request_path);

    let model = flow.override_config["modelName"].as_str().unwrap_or("command-r");
    let temperature = flow.override_config["temperature"].as_f64().unwrap_or(0.7) as f32;
    let max_iterations = flow.override_config["max_iterations"].as_u64().unwrap_or(10) as usize;

    let mut chat_history = Vec::new();
    chat_history.push(ChatHistory::new("USER", prompt));

    if let Some(file_path) = matches.get_one::<String>("upload-image-path") {
        let path = Path::new(file_path);
        let encoded_image = encode_image(path).await?;
        chat_history.push(ChatHistory::new("USER", &format!("data:image/jpeg;base64,{}", encoded_image)));
    }

    let mut full_response = String::new();
    for _ in 0..max_iterations {
        let cohere_response = send_cohere_request(prompt, &api_key, &url, model, temperature, &chat_history).await?;
        debug!("Cohere response: {:?}", cohere_response);

        if let Some(text) = cohere_response.text.clone() {
            full_response.push_str(&text);
            chat_history.push(ChatHistory::new("CHATBOT", &text));

// Prepare the full response JSON
            let full_response_json = serde_json::json!({
                "model": model,
                "generated_text": text,
                "finish_reason": cohere_response.finish_reason,
                "usage": {
                    "prompt_tokens": cohere_response.meta.as_ref().map(|m| m.tokens.input_tokens),
                    "completion_tokens": cohere_response.meta.as_ref().map(|m| m.tokens.output_tokens),
                    "total_tokens": cohere_response.meta.as_ref().map(|m| m.tokens.input_tokens + m.tokens.output_tokens)
                }
            });

            // Initialize Neo4jClient
            let neo4j_client = Arc::new(Neo4jClient::initialize().await?);

            // Capture the LLM interaction in Neo4j
            if let Err(e) = capture_llm_interaction(
                neo4j_client,
                &flow,
                prompt,
                &text,
                model,
                &serde_json::to_string(&full_response_json).unwrap(),
                LlmProvider::Cohere,
            ).await {
                error!("Failed to capture LLM interaction: {:?}", e);
            }
        }

        if let Some(error) = cohere_response.error.clone() {
            return Err(format!("Cohere API error: {}", error).into());
        }

        if cohere_response.finish_reason.as_deref() == Some("COMPLETE") {
            break;
        }
    }

    debug!("Full response: {}", full_response);
    println!("{}", full_response);
    Ok(full_response)
}