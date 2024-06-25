use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use reqwest::Client;
use std::error::Error;
use anyhow::Context;
use clap::ArgMatches;
use log::debug;
use tokio::time::Instant;

use crate::config::{FlowConfig, replace_with_env_var};
use crate::custom_error::FluentError;
use crate::db;
use crate::db::{get_connection, log_anthropic_interaction, log_interaction};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AnthropicRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: usize,
    temperature: f32,
    stop_sequences: Vec<String>,
    stream: bool,
    system: Option<String>, // Updated to be optional
    tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    role: String,
    content: Value, // Updated to Value to handle both string and array content types
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tool {
    name: String,
    description: String,
    input_schema: HashMap<String, ToolSchema>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ToolSchema {
    r#type: String,
    description: String,
}


#[derive(Debug, Deserialize, Clone)]
pub struct AnthropicResponse {
    pub content: Vec<ContentBlock>,
    pub id: String,
    pub response_type: Option<String>,
    pub model: String,
    pub role: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContentBlock {
    pub r#type: String,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Usage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}


pub async fn send_anthropic_request(
    messages: Vec<Message>,
    api_key: &str,
    url: &str,
    model: &str,
    temperature: f32,
    max_tokens: usize,
    stop_sequences: Vec<String>,
    system: Option<String>,
    tools: Option<Vec<Tool>>,
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let request_body = AnthropicRequest {
        model: model.to_string(),
        messages,
        max_tokens,
        temperature,
        stop_sequences,
        stream: false,
        system,
        tools: tools.or(Some(vec![])), // Ensure tools is always a valid list
    };
    debug!("Request body: {:?}", request_body);
    let response = client
        .post(url)
        .header("anthropic-version", "2023-06-01")
        .header("x-api-key", format!("{}", api_key))
        .json(&request_body)
        .send()
        .await?
        .text()
        .await?;
    debug!("Response: {}", response);
    Ok(response)
}

pub async fn handle_anthropic_agent(prompt: &str, flow: &FlowConfig, _matches: &ArgMatches) -> Result<String, Box<dyn std::error::Error>> {

    let start_time = Instant::now();

    let duration = start_time.elapsed().as_secs_f64();
    let original_flow_clone = flow.clone();
    let mut flow = flow.clone();

    replace_with_env_var(&mut flow.override_config);

    let api_key = env::var("FLUENT_ANTHROPIC_KEY_01").expect("FLUENT_ANTHROPIC_KEY_01 not set");
    debug!("Using Anthropic API key: {}", api_key);

    let url = format!("{}://{}:{}/{}", flow.protocol, flow.hostname, flow.port, flow.request_path);
    let max_iterations = flow.override_config["max_iterations"].as_u64().unwrap_or(10) as usize;

    let model = flow.override_config["modelName"].as_str().unwrap_or("claude-3");
    let temperature = flow.override_config["temperature"].as_f64().unwrap_or(0.7) as f32;
    let max_tokens = flow.override_config["max_tokens"].as_u64().unwrap_or(2048) as usize;
    let stop_sequences: Vec<String> = flow.override_config["stop_sequences"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|val| val.as_str().map(String::from))
        .collect();
    let system = flow.override_config["systemMessage"].as_str().map(String::from);
    let tools: Option<Vec<Tool>> = flow.override_config.get("tools")
        .and_then(|tools| serde_json::from_value(tools.clone()).ok());
    debug!("Model: {}", model);
    debug!("Temperature: {}", temperature);
    debug!("Max tokens: {}", max_tokens);
    debug!("Stop sequences: {:?}", stop_sequences);
    debug!("System message: {:?}", system);
    debug!("Tools: {:?}", tools);
    debug!("URL: {}", url);
    let mut messages = vec![
        Message { role: "user".to_string(), content: Value::String(prompt.to_string()) },
    ];

    let mut full_response = String::new();
    for _ in 0..max_iterations {
        let anthropic_response = send_anthropic_request(messages.clone(), &api_key, &url, model, temperature, max_tokens, stop_sequences.clone(), system.clone(), tools.clone()).await?;

        debug!("Anthropic response: {}", anthropic_response);
        // Parse response as JSON
        let response_json: AnthropicResponse = serde_json::from_str(&anthropic_response)?;
        debug!("Response JSON: {:?}", response_json);
        let response_json_clone = &response_json.clone();
        // Process the response messages
        for block in response_json.content {
            if let Some(text) = block.text {
                full_response.push_str(&text);
                messages.push(Message { role: "assistant".to_string(), content: Value::String(text) });
            }
        }

        // Process the usage
        if let Some(usage) = response_json.usage {
            debug!("Input tokens: {}", usage.input_tokens);
            debug!("Output tokens: {}", usage.output_tokens);
        }
        // Collect content into a string first

        let mut content_string = String::new();
        for block in &response_json_clone.content {
            if let Some(text) = &block.text {
                content_string.push_str(text);
            }
        }

        let mut full_response = String::new();
        for block in &response_json_clone.content {
            if let Some(text) = &block.text {
                full_response.push_str(text);
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: serde_json::Value::String(text.clone())
                });
            }
        }

        if let Some(stop_reason) = &response_json_clone.stop_reason {
            if stop_reason != "max_tokens" {
                debug!("Stop reason: {}", stop_reason);
                let conn = get_connection().await?;

                log_anthropic_interaction(&conn, &flow.name, &original_flow_clone, prompt, &response_json_clone, &flow.engine, duration)?;


                return Ok(full_response);
            }
        }
    }
    debug!("Full response: {}", full_response);
    Ok(full_response)
}
