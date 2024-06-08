use reqwest::Client;
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::env;
use clap::ArgMatches;
use colored::Colorize;
use log::debug;
use serde::de::StdError;
use termimad::crossterm::style::Stylize;
use tokio::time::Instant;
use crate::client;
use crate::client::handle_openai_response;
use crate::config::{FlowConfig, replace_with_env_var};

#[derive(Debug, Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Option<Message>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub total_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
}

pub async fn send_openai_request(
    messages: Vec<Message>,
    api_key: &str,
    url: &str,
    model: &str,
    temperature: f32,
) -> Result<OpenAIResponse, Box<dyn StdError + Send + Sync>> {
    let client = reqwest::Client::new();
    let request_body = OpenAIRequest {
        model: model.to_string(),
        messages,
        temperature,
    };

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    Ok(response)
}


const MAX_ITERATIONS: usize = 10;

pub async fn handle_openai_agent(prompt: &str, flow: &FlowConfig, matches: &ArgMatches) -> Result<String, Box<dyn StdError + Send + Sync>> {
    let mut flow = flow.clone();
    replace_with_env_var(&mut flow.override_config);

    let api_key = env::var("FLUENT_OPENAI_API_KEY_01").expect("FLUENT_OPENAI_API_KEY_01 not set");
    debug!("Using OpenAI API key: {}", api_key);

    let url = format!("{}://{}:{}/{}", flow.protocol, flow.hostname, flow.port, flow.request_path);

    let model = flow.override_config["modelName"]["chatOpenAICustom_0"].as_str().unwrap_or("gpt-4o");
    let temperature = flow.override_config["temperature"].as_f64().unwrap_or(0.7) as f32;
    let max_iterations = flow.override_config["max_iterations"].as_u64().unwrap_or(10) as usize;

    let mut messages = vec![
        Message { role: "system".to_string(), content: flow.override_config["systemMessage"].as_str().unwrap_or("You are a helpful assistant.").to_string() },
        Message { role: "user".to_string(), content: prompt.to_string() },
    ];
    debug!("Messages: {:?}", messages);
    debug!("Model: {}", model);
    debug!("Temperature: {}", temperature);
    debug!("Max iterations: {}", max_iterations);
    debug!("URL: {}", url);


    let mut full_response = String::new();
    for _ in 0..max_iterations {
        let openai_response = send_openai_request(messages.clone(), &api_key, &url, model, temperature).await?;
        debug!("OpenAI response: {:?}", openai_response);
        for choice in openai_response.choices {
            debug!("Choice: {:?}", &choice);
            if let Some(message) = choice.message {
                full_response.push_str(&message.content);
                messages.push(Message { role: "assistant".to_string(), content: message.content.clone() });
            }
            debug!("Full response: {}", full_response);
            debug!("Messages: {:?}", messages);

            debug!("Choice finish reason: {:?}", choice.finish_reason);
            if choice.finish_reason.as_deref() != Some("length") {
                // If finish_reason is not "length", we have the complete response
                return Ok(full_response);
            }
        }
    }

    debug!("Full response: {}", full_response);
    Ok(full_response)
}


